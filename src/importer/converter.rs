//! Markdown to Patto converter
//!
//! Converts markdown content to patto format using pulldown-cmark for parsing.
//! Builds patto's AST directly for consistency with the native parser.

use super::options::{ImportMode, ImportOptions, MarkdownInputFlavor};
use super::report::{ConversionReport, ImportWarning, WarningKind};
use crate::parser::{AstNode, AstNodeKind, Deadline, Property, TaskStatus};
use crate::renderer::{PattoRenderer, Renderer};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::time::Instant;

/// Error type for import operations
#[derive(Debug, Clone)]
pub struct ImportError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ImportError {}

/// Result of markdown import
#[derive(Debug)]
pub struct ImportResult {
    /// Converted patto AST (root node)
    pub ast: AstNode,
    /// Converted patto content as string (for convenience)
    pub patto_content: String,
    /// Conversion report
    pub report: ConversionReport,
}

/// Markdown to Patto importer
pub struct MarkdownImporter {
    options: ImportOptions,
}

impl MarkdownImporter {
    /// Create a new importer with the given options
    pub fn new(options: ImportOptions) -> Self {
        Self { options }
    }

    /// Detect the markdown flavor from content
    pub fn detect_flavor(content: &str) -> MarkdownInputFlavor {
        // Check for Obsidian-specific syntax
        let obsidian_wikilink = Regex::new(r"\[\[[^\]]+\]\]").unwrap();
        let obsidian_block_ref = Regex::new(r"\s\^[a-zA-Z0-9-]+$").unwrap();
        let obsidian_dataview = Regex::new(r"\[due::\s*\d{4}-\d{2}-\d{2}\]").unwrap();
        let obsidian_task_emoji = Regex::new(r"📅\s*\d{4}-\d{2}-\d{2}").unwrap();

        if obsidian_wikilink.is_match(content)
            || obsidian_block_ref.is_match(content)
            || obsidian_dataview.is_match(content)
            || obsidian_task_emoji.is_match(content)
        {
            return MarkdownInputFlavor::Obsidian;
        }

        // Check for GitHub-specific syntax (task lists with mentions)
        let github_mention = Regex::new(r"@[a-zA-Z0-9_-]+").unwrap();
        let github_issue_ref = Regex::new(r"#\d+").unwrap();
        if github_mention.is_match(content) || github_issue_ref.is_match(content) {
            return MarkdownInputFlavor::GitHub;
        }

        MarkdownInputFlavor::Standard
    }

    /// Import markdown content to patto format
    pub fn import(
        &self,
        markdown: &str,
        input_path: &str,
        output_path: &str,
    ) -> Result<ImportResult, ImportError> {
        let start_time = Instant::now();

        // Detect or use specified flavor
        let flavor = self
            .options
            .flavor
            .unwrap_or_else(|| Self::detect_flavor(markdown));

        let mut report = ConversionReport::new(input_path, output_path, self.options.mode, flavor);
        report.statistics.total_lines = markdown.lines().count();

        // Convert to AST
        let ast = self.convert_to_ast(markdown, &mut report)?;

        // Render AST to patto string format using PattoRenderer
        let renderer = PattoRenderer::new();
        let mut patto_content = Vec::new();
        renderer
            .format(&ast, &mut patto_content)
            .map_err(|e| ImportError {
                line: 0,
                message: format!("Failed to render AST: {}", e),
            })?;
        let patto_content = String::from_utf8(patto_content).map_err(|e| ImportError {
            line: 0,
            message: format!("Invalid UTF-8 in output: {}", e),
        })?;

        report.statistics.converted_lines =
            report.statistics.total_lines - report.statistics.failed_lines;
        report.duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(ImportResult {
            ast,
            patto_content,
            report,
        })
    }
    /// Convert markdown content to patto AST
    fn convert_to_ast(
        &self,
        markdown: &str,
        report: &mut ConversionReport,
    ) -> Result<AstNode, ImportError> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_FOOTNOTES);

        let mut conversion = Conversion::new(&self.options, report);
        for event in Parser::new_ext(markdown, options) {
            conversion.handle(event)?;
        }
        Ok(conversion.finish())
    }

    /// Strip due date patterns from text
    #[allow(dead_code)]
    fn strip_due_date(&self, text: &str) -> String {
        let patterns = [
            r"📅\s*\d{4}-\d{2}-\d{2}",
            r"\(due:\s*\d{4}-\d{2}-\d{2}\)",
            r"\[due::\s*\d{4}-\d{2}-\d{2}\]",
            r"@\d{4}-\d{2}-\d{2}",
        ];

        let mut result = text.to_string();
        for pattern in patterns {
            let re = Regex::new(pattern).unwrap();
            result = re.replace_all(&result, "").to_string();
        }
        result
    }
}

/// Markdown patto has no way to express.
struct Unsupported<'a> {
    feature: &'a str,
    statistic: &'a str,
    strict_message: String,
    lossy_message: String,
    suggestion: &'a str,
}

/// A heading being collected; patto has no heading node, so the contents are
/// buffered and re-emitted as a line when the heading ends.
struct Heading {
    level: u8,
    contents: Vec<AstNode>,
}

struct Table {
    node: AstNode,
    row: Option<AstNode>,
    cell: Option<AstNode>,
}

struct Link {
    url: String,
    contents: Vec<AstNode>,
}

/// Nesting of the lists currently open.
#[derive(Default)]
struct Lists {
    /// One entry per open list, `true` when that list is ordered.
    stack: Vec<bool>,
    /// Line the outermost list hangs from.
    root: Option<AstNode>,
    /// Depth of the item being built, i.e. the stack size when it started.
    depth: usize,
    /// `Some` while building a task list item, holding its checkbox state.
    task_checked: Option<bool>,
}

#[derive(Default, Clone, Copy)]
struct Decoration {
    bold: bool,
    italic: bool,
    strikethrough: bool,
}

/// State of one markdown-to-patto conversion.
///
/// A markdown event stream is flat, so each block kind that patto nests keeps an
/// `Option` here: `Some` means that block is open and new content belongs to it.
struct Conversion<'a> {
    options: &'a ImportOptions,
    report: &'a mut ConversionReport,
    root: AstNode,
    /// 1-based line in the markdown source, used for AST locations and warnings.
    line: usize,

    heading: Option<Heading>,
    code: Option<AstNode>,
    quote: Option<AstNode>,
    table: Option<Table>,
    link: Option<Link>,
    lists: Lists,
    decoration: Decoration,

    /// Line being built, and the inline content collected for it so far.
    line_node: Option<AstNode>,
    pending: Vec<AstNode>,
}

impl<'a> Conversion<'a> {
    fn new(options: &'a ImportOptions, report: &'a mut ConversionReport) -> Self {
        Self {
            options,
            report,
            root: AstNode::new("", 0, None, Some(AstNodeKind::Dummy)),
            line: 1,
            heading: None,
            code: None,
            quote: None,
            table: None,
            link: None,
            lists: Lists::default(),
            decoration: Decoration::default(),
            line_node: None,
            pending: Vec::new(),
        }
    }

    fn finish(mut self) -> AstNode {
        if let Some(line_node) = self.line_node.take() {
            for content in self.pending.drain(..) {
                line_node.add_content(content);
            }
            self.root.add_child(line_node);
        }
        self.root
    }

    fn handle(&mut self, event: Event) -> Result<(), ImportError> {
        match event {
            Event::Start(tag) => self.start(tag)?,
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.inline_code(&code),
            Event::Html(html) => self.html(&html)?,
            Event::SoftBreak | Event::HardBreak => self.line += 1,
            Event::Rule => {
                self.root
                    .add_child(AstNode::horizontal_line("-----", self.line, None));
                self.report.statistics.increment_feature("horizontal_rules");
            }
            Event::TaskListMarker(checked) => self.lists.task_checked = Some(checked),
            Event::FootnoteReference(name) => self.footnote_reference(&name)?,
            _ => {}
        }
        Ok(())
    }

    fn start(&mut self, tag: Tag) -> Result<(), ImportError> {
        match tag {
            Tag::Heading { level, .. } => {
                self.heading = Some(Heading {
                    level: match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    },
                    contents: Vec::new(),
                });
                self.report.statistics.increment_feature("headings");
            }
            Tag::List(ordered) => self.start_list(ordered.is_some()),
            Tag::Item => {
                self.lists.depth = self.lists.stack.len();
                self.lists.task_checked = None;
                self.line_node = Some(AstNode::line("", self.line, None, None));
            }
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                self.code = Some(AstNode::code("", self.line, None, &lang, false));
                self.report.statistics.increment_feature("code_blocks");
            }
            Tag::BlockQuote(_) => {
                self.quote = Some(AstNode::quote("", self.line, None));
                self.report.statistics.increment_feature("blockquotes");
            }
            Tag::Table(_) => {
                self.table = Some(Table {
                    node: AstNode::table("", self.line, None, None),
                    row: None,
                    cell: None,
                });
                self.report.statistics.increment_feature("tables");
            }
            Tag::TableHead | Tag::TableRow => {
                let row = AstNode::tablerow("", self.line, None);
                if let Some(table) = self.table.as_mut() {
                    table.row = Some(row);
                }
            }
            Tag::TableCell => {
                let cell = AstNode::tablecolumn("", self.line, None);
                if let Some(table) = self.table.as_mut() {
                    table.cell = Some(cell);
                }
            }
            Tag::Emphasis => self.decoration.italic = true,
            Tag::Strong => self.decoration.bold = true,
            Tag::Strikethrough => self.decoration.strikethrough = true,
            Tag::Link { dest_url, .. } => {
                self.link = Some(Link {
                    url: dest_url.to_string(),
                    contents: Vec::new(),
                });
                self.report.statistics.increment_feature("links");
            }
            Tag::Image {
                dest_url, title, ..
            } => {
                let alt = (!title.is_empty()).then(|| title.as_ref());
                let image = AstNode::image("", self.line, None, &dest_url, alt);
                self.push_inline(image);
                self.report.statistics.increment_feature("images");
            }
            Tag::Paragraph => {
                if self.heading.is_none() && self.line_node.is_none() {
                    self.line_node = Some(AstNode::line("", self.line, None, None));
                }
            }
            Tag::FootnoteDefinition(_) => {
                self.handle_unsupported(Unsupported {
                    feature: "footnote",
                    statistic: "footnotes",
                    strict_message: "Footnotes are not supported by patto".to_string(),
                    lossy_message: "Dropped footnote definition".to_string(),
                    suggestion: "Move footnote content inline",
                })?;
            }
            _ => {}
        }
        Ok(())
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => self.end_heading(),
            TagEnd::List(_) => {
                self.lists.stack.pop();
                self.lists.depth = self.lists.stack.len();
                if self.lists.stack.is_empty() {
                    self.lists.root = None;
                }
            }
            TagEnd::Item => self.end_item(),
            TagEnd::CodeBlock => {
                if let Some(code) = self.code.take() {
                    self.add_block_line(code);
                }
            }
            TagEnd::BlockQuote(_) => {
                if let Some(quote) = self.quote.take() {
                    self.add_block_line(quote);
                }
            }
            TagEnd::Table => {
                if let Some(table) = self.table.take() {
                    self.add_block_line(table.node);
                }
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some(table) = self.table.as_mut() {
                    if let Some(row) = table.row.take() {
                        table.node.add_child(row);
                    }
                }
            }
            TagEnd::TableCell => {
                if let Some(table) = self.table.as_mut() {
                    if let (Some(cell), Some(row)) = (table.cell.take(), table.row.as_ref()) {
                        row.add_content(cell);
                    }
                }
            }
            TagEnd::Emphasis => self.decoration.italic = false,
            TagEnd::Strong => self.decoration.bold = false,
            TagEnd::Strikethrough => self.decoration.strikethrough = false,
            TagEnd::Link => {
                if let Some(link) = self.link.take() {
                    let node = link_node(&link.url, &link.contents, self.line);
                    self.push_inline(node);
                }
            }
            TagEnd::Paragraph => self.end_paragraph(),
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        self.line += text.matches('\n').count();

        if let Some(code) = self.code.as_ref() {
            for line in text.lines() {
                code.add_child(AstNode::codecontent(line, self.line, None));
            }
        } else if let Some(link) = self.link.as_mut() {
            // Link text is the link title; the finished link is routed on TagEnd::Link.
            link.contents.push(AstNode::text(text, self.line, None));
        } else {
            let node = decorated_text(text, self.line, self.decoration);
            self.push_inline(node);
        }
    }

    fn inline_code(&mut self, code: &str) {
        let node = AstNode::code(code, self.line, None, "", true);
        node.add_content(AstNode::codecontent(code, self.line, None));
        self.push_inline(node);
        self.report.statistics.increment_feature("inline_code");
    }

    fn html(&mut self, html: &str) -> Result<(), ImportError> {
        self.line += html.matches('\n').count();

        let preserve = self.handle_unsupported(Unsupported {
            feature: "html",
            statistic: "html",
            strict_message: format!("HTML is not supported by patto: {}", html.trim()),
            lossy_message: format!("Dropped HTML: {}", html.trim()),
            suggestion: "Use plain text or patto markup instead",
        })?;
        if !preserve {
            return Ok(());
        }

        let code = AstNode::code("", self.line, None, "html", false);
        for line in html.lines() {
            code.add_child(AstNode::codecontent(line, self.line, None));
        }
        self.add_block_line(code);
        self.report.add_warning(ImportWarning {
            line: self.line,
            column: None,
            kind: WarningKind::PreservedContent,
            feature: "html".to_string(),
            message: "Preserved HTML in code block for manual editing".to_string(),
            suggestion: None,
        });
        Ok(())
    }

    fn footnote_reference(&mut self, name: &str) -> Result<(), ImportError> {
        let preserve = self.handle_unsupported(Unsupported {
            feature: "footnote_ref",
            statistic: "footnotes",
            strict_message: format!("Footnote reference [^{}] is not supported", name),
            lossy_message: format!("Dropped footnote reference [^{}]", name),
            suggestion: "Move footnote content inline",
        })?;
        if preserve {
            let text = AstNode::text(&format!("[^{}]", name), self.line, None);
            self.push_inline(text);
        }
        Ok(())
    }

    /// Apply the configured import mode. Returns `true` when the caller should
    /// preserve the content itself.
    fn handle_unsupported(&mut self, unsupported: Unsupported) -> Result<bool, ImportError> {
        match self.options.mode {
            ImportMode::Strict => Err(ImportError {
                line: self.line,
                message: unsupported.strict_message,
            }),
            ImportMode::Lossy => {
                self.report.add_warning(ImportWarning {
                    line: self.line,
                    column: None,
                    kind: WarningKind::UnsupportedFeature,
                    feature: unsupported.feature.to_string(),
                    message: unsupported.lossy_message,
                    suggestion: Some(unsupported.suggestion.to_string()),
                });
                self.report
                    .statistics
                    .increment_unsupported(unsupported.statistic);
                Ok(false)
            }
            ImportMode::Preserve => Ok(true),
        }
    }

    /// Route inline content to whichever container is open.
    ///
    /// Table cells are checked first: content produced inside a table would
    /// otherwise leak into the next line.
    fn push_inline(&mut self, node: AstNode) {
        if let Some(table) = self.table.as_ref() {
            if let Some(cell) = table.cell.as_ref() {
                cell.add_content(node);
            }
        } else if let Some(heading) = self.heading.as_mut() {
            heading.contents.push(node);
        } else {
            self.pending.push(node);
        }
    }

    /// Patto blocks live inside a line, so wrap and append at the top level.
    fn add_block_line(&self, block: AstNode) {
        let line_node = AstNode::line("", self.line, None, None);
        line_node.add_content(block);
        self.root.add_child(line_node);
    }

    fn start_list(&mut self, ordered: bool) {
        // A nested list interrupts its parent item, so close that item's line first.
        if let Some(line_node) = self.line_node.take() {
            for content in self.pending.drain(..) {
                line_node.add_content(content);
            }
            if self.lists.task_checked.take().is_some() {
                self.report.statistics.increment_feature("tasks");
            }
            self.attach_list_item(line_node);
        }

        if self.lists.stack.is_empty() {
            let list_root = AstNode::line("", self.line, None, None);
            self.root.add_child(list_root.clone());
            self.lists.root = Some(list_root);
        }

        self.lists.stack.push(ordered);
        self.report.statistics.increment_feature("lists");
    }

    fn end_item(&mut self) {
        let checked = self.lists.task_checked.take();
        let properties = checked.map(|checked| self.task_property(checked));

        let line_node = AstNode::line("", self.line, None, properties);
        for content in self.pending.drain(..) {
            line_node.add_content(content);
        }

        // The line created on Tag::Item was only a placeholder.
        self.line_node = None;
        self.attach_list_item(line_node);
    }

    /// The task property for a checklist item, with the dates written in its text.
    fn task_property(&mut self, checked: bool) -> Vec<Property> {
        let text: String = self.pending.iter().map(|node| node.extract_str()).collect();
        let date = |value: Option<String>| {
            value
                .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                .map(Deadline::Date)
        };

        self.report.statistics.increment_feature("tasks");
        vec![Property::Task {
            status: if checked {
                TaskStatus::Done
            } else {
                TaskStatus::Todo
            },
            status_is_canonical: true,
            due: date(extract_due_date(&text)).unwrap_or(Deadline::Uninterpretable(String::new())),
            scheduled: date(extract_scheduled_date(&text)),
            completed_at: date(extract_completed_at_date(&text)),
            started_at: None,
            time_spent: None,
            location: crate::parser::Location::default(),
        }]
    }

    fn attach_list_item(&self, line_node: AstNode) {
        let parent = self.lists.root.as_ref().unwrap_or(&self.root);
        add_child_at_depth(parent, line_node, self.lists.depth);
    }

    fn end_heading(&mut self) {
        let Some(heading) = self.heading.take() else {
            return;
        };

        let line_node = AstNode::line("", self.line, None, None);
        if heading.level == 1 {
            for content in heading.contents {
                line_node.add_content(content);
            }
            self.root.add_child(line_node);
            self.root
                .add_child(AstNode::horizontal_line("-----", self.line, None));
        } else {
            let decoration = AstNode::decoration("", self.line, None, 1, false, false, false);
            for content in heading.contents {
                decoration.add_content(content);
            }
            line_node.add_content(decoration);
            self.root.add_child(line_node);
        }

        self.report.add_warning(ImportWarning {
            line: self.line,
            column: None,
            kind: WarningKind::LossyConversion,
            feature: "heading".to_string(),
            message: format!(
                "Converted h{} heading to {}",
                heading.level,
                if heading.level == 1 {
                    "text with horizontal line"
                } else {
                    "emphasized text"
                }
            ),
            suggestion: None,
        });
    }

    fn end_paragraph(&mut self) {
        let Some(line_node) = self.line_node.take() else {
            return;
        };
        for content in self.pending.drain(..) {
            line_node.add_content(content);
        }

        match self.quote.as_ref() {
            None => self.root.add_child(line_node),
            Some(quote) => {
                let quote_content = AstNode::quotecontent("", self.line, None, None);
                for content in line_node.contents().iter() {
                    quote_content.add_content(content.clone());
                }
                quote.add_child(quote_content);
            }
        }
    }
}

fn decorated_text(text: &str, line: usize, decoration: Decoration) -> AstNode {
    let Decoration {
        bold,
        italic,
        strikethrough,
    } = decoration;
    if !(bold || italic || strikethrough) {
        return AstNode::text(text, line, None);
    }

    let fontsize = if bold { 1 } else { 0 };
    let node = AstNode::decoration(text, line, None, fontsize, italic, false, strikethrough);
    node.add_content(AstNode::text(text, line, None));
    node
}

/// Convert a markdown link to the closest patto equivalent: a wiki link for
/// anchors and notes, a plain link otherwise.
fn link_node(url: &str, contents: &[AstNode], line: usize) -> AstNode {
    if let Some(anchor) = url.strip_prefix('#') {
        return AstNode::wikilink("", line, None, "", Some(anchor));
    }

    let is_note = url.ends_with(".md")
        || url.ends_with(".pn")
        || url.contains(".md#")
        || url.contains(".pn#");
    if is_note {
        return match url.split_once('#') {
            Some((file, anchor)) => {
                let note = file.trim_end_matches(".md").trim_end_matches(".pn");
                AstNode::wikilink("", line, None, note, Some(anchor))
            }
            None => {
                let note = url.trim_end_matches(".md").trim_end_matches(".pn");
                AstNode::wikilink("", line, None, note, None)
            }
        };
    }

    let text: String = contents.iter().map(|node| node.extract_str()).collect();
    let title = (!text.is_empty() && text != url).then_some(text.as_str());
    AstNode::link("", line, None, url, title)
}

/// Attach `child` under the item chain of `root`, `depth` levels down.
/// `depth <= 1` makes it a direct child.
fn add_child_at_depth(root: &AstNode, child: AstNode, depth: usize) {
    if depth <= 1 {
        root.add_child(child);
        return;
    }
    add_child_at_depth_recursive(root, child, depth - 1);
}

fn add_child_at_depth_recursive(node: &AstNode, child: AstNode, remaining_depth: usize) {
    let children = node.children();
    let Some(last_child) = children.last().cloned() else {
        drop(children);
        node.add_child(child);
        return;
    };
    drop(children);

    if remaining_depth == 1 {
        last_child.add_child(child);
    } else {
        add_child_at_depth_recursive(&last_child, child, remaining_depth - 1);
    }
}

/// Due date written as `📅 D`, `(due: D)`, `[due:: D]` or `@D`.
fn extract_due_date(text: &str) -> Option<String> {
    first_capture(
        text,
        &[
            r"📅\s*(\d{4}-\d{2}-\d{2})",
            r"\(due:\s*(\d{4}-\d{2}-\d{2})\)",
            r"\[due::\s*(\d{4}-\d{2}-\d{2})\]",
            r"@(\d{4}-\d{2}-\d{2})",
        ],
    )
}

/// Scheduled date written as `⏳ D`, `(scheduled: D)` or `[scheduled:: D]`.
fn extract_scheduled_date(text: &str) -> Option<String> {
    first_capture(
        text,
        &[
            r"⏳\s*(\d{4}-\d{2}-\d{2})",
            r"\(scheduled:\s*(\d{4}-\d{2}-\d{2})\)",
            r"\[scheduled::\s*(\d{4}-\d{2}-\d{2})\]",
        ],
    )
}

/// Completion date written as `✅ D`, `(completed: D)` or `[completed_at:: D]`.
fn extract_completed_at_date(text: &str) -> Option<String> {
    first_capture(
        text,
        &[
            r"✅\s*(\d{4}-\d{2}-\d{2})",
            r"\(completed:\s*(\d{4}-\d{2}-\d{2})\)",
            r"\[completed_at::\s*(\d{4}-\d{2}-\d{2})\]",
        ],
    )
}

/// First capture group matched by any of `patterns`, tried in order.
fn first_capture(text: &str, patterns: &[&str]) -> Option<String> {
    patterns.iter().find_map(|pattern| {
        Regex::new(pattern)
            .unwrap()
            .captures(text)?
            .get(1)
            .map(|m| m.as_str().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn import_lossy(md: &str) -> ImportResult {
        let importer = MarkdownImporter::new(ImportOptions::new(ImportMode::Lossy));
        importer.import(md, "test.md", "test.pn").unwrap()
    }

    fn import_strict(md: &str) -> Result<ImportResult, ImportError> {
        let importer = MarkdownImporter::new(ImportOptions::new(ImportMode::Strict));
        importer.import(md, "test.md", "test.pn")
    }

    fn import_preserve(md: &str) -> ImportResult {
        let importer = MarkdownImporter::new(ImportOptions::new(ImportMode::Preserve));
        importer.import(md, "test.md", "test.pn").unwrap()
    }

    #[test]
    fn test_plain_text() {
        let result = import_lossy("Hello world");
        assert_eq!(result.patto_content.trim(), "Hello world");
    }

    #[test]
    fn test_list_conversion() {
        let result = import_lossy("- item 1\n- item 2");
        assert!(result.patto_content.contains("item 1"));
        assert!(result.patto_content.contains("item 2"));
    }

    #[test]
    fn test_nested_list_conversion() {
        let result = import_lossy("- item 1\n  - nested");
        let lines: Vec<&str> = result.patto_content.lines().collect();
        // Check that nested item has indentation
        assert!(lines
            .iter()
            .any(|l| l.starts_with('\t') && l.contains("nested")));
    }

    #[test]
    fn test_code_block_conversion() {
        let result = import_lossy("```python\nprint('hello')\n```");
        assert!(result.patto_content.contains("[@code python]"));
        assert!(result.patto_content.contains("print('hello')"));
    }

    #[test]
    fn test_inline_code_conversion() {
        let result = import_lossy("Use `code` here");
        assert!(result.patto_content.contains("[` code `]"));
    }

    #[test]
    fn test_heading_conversion_h1() {
        let result = import_lossy("# Title");
        assert!(result.patto_content.contains("Title"));
        assert!(result.patto_content.contains("---"));
        assert_eq!(result.report.warnings.len(), 1);
        assert!(result.report.warnings[0].message.contains("h1"));
    }

    #[test]
    fn test_heading_conversion_h2() {
        let result = import_lossy("## Subtitle");
        assert!(result.patto_content.contains("[* Subtitle]"));
    }

    #[test]
    fn test_bold_conversion() {
        let result = import_lossy("This is **bold** text");
        assert!(result.patto_content.contains("[* bold]"));
    }

    #[test]
    fn test_italic_conversion() {
        let result = import_lossy("This is *italic* text");
        assert!(result.patto_content.contains("[/ italic]"));
    }

    #[test]
    fn test_bold_italic_conversion() {
        let result = import_lossy("This is ***bold italic*** text");
        assert!(result.patto_content.contains("[*/ bold italic]"));
    }

    #[test]
    fn test_link_internal() {
        let result = import_lossy("[link](note.md)");
        assert!(result.patto_content.contains("[note]"));
    }

    #[test]
    fn test_link_external() {
        let result = import_lossy("[Google](https://google.com)");
        assert!(result.patto_content.contains("[Google https://google.com]"));
    }

    #[test]
    fn test_link_anchor() {
        let result = import_lossy("[section](#anchor)");
        assert!(result.patto_content.contains("[#anchor]"));
    }

    #[test]
    fn test_blockquote_conversion() {
        let result = import_lossy("> This is a quote");
        assert!(result.patto_content.contains("[@quote]"));
        assert!(result.patto_content.contains("This is a quote"));
    }

    #[test]
    fn test_table_conversion() {
        let result = import_lossy("| h1 | h2 |\n|---|---|\n| a | b |");
        assert!(result.patto_content.contains("[@table]"));
        assert!(result.patto_content.contains("\th1\th2"));
        assert!(result.patto_content.contains("\ta\tb"));
    }

    #[test]
    fn test_table_with_inline_content() {
        let result = import_lossy(
            "| h1 | h2 | h3 |\n|---|---|---|\n| [Google](https://google.com) | **bold** | `code` |\n\nnext paragraph",
        );
        // Inline nodes must stay inside their cell...
        assert!(result
            .patto_content
            .contains("\t[Google https://google.com]\t[* bold]\t[` code `]"));
        // ...and must not leak into the following line
        assert!(result
            .patto_content
            .lines()
            .any(|l| l.trim() == "next paragraph"));
    }

    #[test]
    fn test_table_with_image() {
        let result = import_lossy("| h1 |\n|---|\n| ![alt](img.png) |");
        assert!(result.patto_content.contains("\t[@img img.png]"));
    }

    #[test]
    fn test_horizontal_rule() {
        let result = import_lossy("---");
        assert!(result.patto_content.contains("-----"));
    }

    #[test]
    fn test_task_list_unchecked() {
        let result = import_lossy("- [ ] todo task");
        assert!(result.patto_content.contains("{@task status=todo}"));
    }

    #[test]
    fn test_task_list_checked() {
        let result = import_lossy("- [x] done task");
        assert!(result.patto_content.contains("{@task status=done}"));
    }

    #[test]
    fn test_task_with_due_date_emoji() {
        let result = import_lossy("- [ ] task 📅 2024-12-31");
        assert!(result
            .patto_content
            .contains("{@task status=todo due=2024-12-31}"));
    }

    #[test]
    fn test_task_with_due_date_parentheses() {
        let result = import_lossy("- [ ] task (due: 2024-12-31)");
        assert!(result
            .patto_content
            .contains("{@task status=todo due=2024-12-31}"));
    }

    #[test]
    fn test_task_with_due_date_dataview() {
        let result = import_lossy("- [ ] task [due:: 2024-12-31]");
        assert!(result
            .patto_content
            .contains("{@task status=todo due=2024-12-31}"));
    }

    #[test]
    fn test_strict_mode_fails_on_html() {
        let result = import_strict("<div>html</div>");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("HTML is not supported"));
    }

    #[test]
    fn test_lossy_mode_drops_html() {
        let result = import_lossy("<div>html</div>");
        assert!(!result.report.warnings.is_empty());
        assert!(result.report.warnings.iter().any(|w| w.feature == "html"));
    }

    #[test]
    fn test_preserve_mode_wraps_html() {
        let result = import_preserve("<div>html</div>");
        assert!(result.patto_content.contains("[@code html]"));
        assert!(result.patto_content.contains("<div>html</div>"));
    }

    #[test]
    fn test_detect_flavor_obsidian() {
        assert_eq!(
            MarkdownImporter::detect_flavor("[[wikilink]]"),
            MarkdownInputFlavor::Obsidian
        );
        assert_eq!(
            MarkdownImporter::detect_flavor("task 📅 2024-12-31"),
            MarkdownInputFlavor::Obsidian
        );
        assert_eq!(
            MarkdownImporter::detect_flavor("[due:: 2024-12-31]"),
            MarkdownInputFlavor::Obsidian
        );
    }

    #[test]
    fn test_detect_flavor_github() {
        assert_eq!(
            MarkdownImporter::detect_flavor("cc @username"),
            MarkdownInputFlavor::GitHub
        );
    }

    #[test]
    fn test_detect_flavor_standard() {
        assert_eq!(
            MarkdownImporter::detect_flavor("Just normal text"),
            MarkdownInputFlavor::Standard
        );
    }

    #[test]
    fn test_report_generation() {
        let result = import_lossy("# Title\n- item\n- [ ] task 📅 2024-12-31");
        let report = &result.report;

        assert_eq!(report.mode, ImportMode::Lossy);
        assert!(report.statistics.feature_counts.contains_key("headings"));
        assert!(report.statistics.feature_counts.contains_key("lists"));
        assert!(report.statistics.feature_counts.contains_key("tasks"));
    }

    #[test]
    fn test_statistics_tracking() {
        let result = import_lossy("# Title\n## Subtitle\n- item 1\n- item 2\n```code\ntest\n```");
        let stats = &result.report.statistics;

        assert_eq!(stats.feature_counts.get("headings"), Some(&2));
        assert_eq!(stats.feature_counts.get("lists"), Some(&1)); // one list with 2 items
        assert_eq!(stats.feature_counts.get("code_blocks"), Some(&1));
    }

    #[test]
    fn test_lossy_mode_continues_on_error() {
        // pulldown-cmark treats inline HTML differently - text after/between HTML
        // may be parsed as more HTML events. Test with more realistic cases.
        let md = "Normal text\n\n<div>html content</div>\n\nAnother paragraph";
        let result = import_lossy(md);

        // Should have warnings for HTML
        assert!(
            !result.report.warnings.is_empty(),
            "Expected warnings for HTML"
        );
        assert!(
            result.report.warnings.iter().any(|w| w.feature == "html"),
            "Expected HTML warning"
        );
        // Should still produce output for valid text
        assert!(
            result.patto_content.contains("Normal text"),
            "Missing 'Normal text'"
        );
        // The paragraph after HTML should be captured
        assert!(
            result.patto_content.contains("Another paragraph"),
            "Missing 'Another paragraph', content: {}",
            result.patto_content
        );
    }

    #[test]
    fn test_image_conversion() {
        let result = import_lossy("![alt text](image.png)");
        assert!(result.patto_content.contains("[@img"));
        assert!(result.patto_content.contains("image.png"));
    }

    #[test]
    fn test_convert_link_wikilink() {
        // Test link conversion via actual markdown import
        // Internal links become wikilinks
        let result = import_lossy("[note](note.md)");
        assert!(
            result.patto_content.contains("[note]"),
            "Internal .md link should become wikilink"
        );

        let result = import_lossy("[text](note.md#anchor)");
        assert!(
            result.patto_content.contains("[note#anchor]"),
            "Link with anchor should preserve anchor"
        );

        let result = import_lossy("[section](#anchor)");
        assert!(
            result.patto_content.contains("[#anchor]"),
            "Self-anchor link"
        );

        let result = import_lossy("[Example](https://example.com)");
        assert!(
            result
                .patto_content
                .contains("[Example https://example.com]"),
            "External URL"
        );
    }

    #[test]
    fn test_extract_due_date() {
        assert_eq!(
            extract_due_date("task 📅 2024-12-31"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            extract_due_date("task (due: 2024-12-31)"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            extract_due_date("task [due:: 2024-12-31]"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            extract_due_date("task @2024-12-31"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(extract_due_date("task without date"), None);
    }

    #[test]
    fn test_strip_due_date() {
        let importer = MarkdownImporter::new(ImportOptions::default());

        assert_eq!(importer.strip_due_date("task 📅 2024-12-31").trim(), "task");
        assert_eq!(
            importer.strip_due_date("task (due: 2024-12-31)").trim(),
            "task"
        );
        assert_eq!(
            importer.strip_due_date("task [due:: 2024-12-31]").trim(),
            "task"
        );
    }
}
