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
        renderer.format(&ast, &mut patto_content)
            .map_err(|e| ImportError {
                line: 0,
                message: format!("Failed to render AST: {}", e),
            })?;
        let patto_content = String::from_utf8(patto_content)
            .map_err(|e| ImportError {
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
        let mut current_line: usize = 1;

        // Enable all markdown extensions
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_FOOTNOTES);

        let parser = Parser::new_ext(markdown, options);

        // Create root node (Dummy)
        let root = AstNode::new("", 0, None, Some(AstNodeKind::Dummy));

        // State tracking
        let mut indent_level: usize = 0;
        let mut in_code_block = false;
        let mut code_lang: String;
        let mut code_node: Option<AstNode> = None;
        let mut in_table = false;
        let mut table_node: Option<AstNode> = None;
        let mut current_row: Option<AstNode> = None;
        let mut current_cell: Option<AstNode> = None;
        let mut in_blockquote = false;
        let mut quote_node: Option<AstNode> = None;
        let mut list_stack: Vec<bool> = Vec::new();
        let mut list_root_node: Option<AstNode> = None; // Root node for top-level list items
        let mut current_task_status: Option<bool> = None;
        let mut in_heading = false;
        let mut heading_level: u8 = 0;
        let mut heading_contents: Vec<AstNode> = Vec::new();
        let mut current_line_node: Option<AstNode> = None;
        let mut pending_contents: Vec<AstNode> = Vec::new();
        let mut in_emphasis = false;
        let mut in_strong = false;
        let mut in_link = false;
        let mut link_url = String::new();
        let mut link_contents: Vec<AstNode> = Vec::new();

        for event in parser {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Heading { level, .. } => {
                            in_heading = true;
                            heading_level = match level {
                                HeadingLevel::H1 => 1,
                                HeadingLevel::H2 => 2,
                                HeadingLevel::H3 => 3,
                                HeadingLevel::H4 => 4,
                                HeadingLevel::H5 => 5,
                                HeadingLevel::H6 => 6,
                            };
                            heading_contents.clear();
                            report.statistics.increment_feature("headings");
                        }
                        Tag::List(ordered) => {
                            // Flush any pending line content before nested list
                            if let Some(line_node) = current_line_node.take() {
                                if !pending_contents.is_empty() {
                                    for content in pending_contents.drain(..) {
                                        line_node.add_content(content);
                                    }
                                }
                                // Add task property if applicable
                                if let Some(checked) = current_task_status.take() {
                                    self.add_task_property_to_line(&line_node, checked, report);
                                }
                                // Add to parent (use indent_level to find correct parent)
                                if let Some(ref list_root) = list_root_node {
                                    self.add_child_at_depth(list_root, line_node, indent_level);
                                } else {
                                    self.add_child_at_depth(&root, line_node, indent_level);
                                }
                            }
                            
                            // If this is a top-level list (no parent list), create a list root
                            if list_stack.is_empty() {
                                let list_root = AstNode::line("", current_line, None, None);
                                root.add_child(list_root.clone());
                                list_root_node = Some(list_root);
                            }
                            
                            list_stack.push(ordered.is_some());
                            report.statistics.increment_feature("lists");
                        }
                        Tag::Item => {
                            indent_level = list_stack.len();
                            current_task_status = None;
                            // Create new line node for this list item
                            let line_node = AstNode::line("", current_line, None, None);
                            current_line_node = Some(line_node);
                        }
                        Tag::CodeBlock(kind) => {
                            in_code_block = true;
                            code_lang = match kind {
                                pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                                pulldown_cmark::CodeBlockKind::Indented => String::new(),
                            };
                            code_node = Some(AstNode::code("", current_line, None, &code_lang, false));
                            report.statistics.increment_feature("code_blocks");
                        }
                        Tag::BlockQuote(_) => {
                            in_blockquote = true;
                            quote_node = Some(AstNode::quote("", current_line, None));
                            report.statistics.increment_feature("blockquotes");
                        }
                        Tag::Table(_) => {
                            in_table = true;
                            table_node = Some(AstNode::table("", current_line, None, None));
                            report.statistics.increment_feature("tables");
                        }
                        Tag::TableHead | Tag::TableRow => {
                            current_row = Some(AstNode::tablerow("", current_line, None));
                        }
                        Tag::TableCell => {
                            current_cell = Some(AstNode::tablecolumn("", current_line, None));
                        }
                        Tag::Emphasis => {
                            in_emphasis = true;
                        }
                        Tag::Strong => {
                            in_strong = true;
                        }
                        Tag::Link { dest_url, .. } => {
                            in_link = true;
                            link_url = dest_url.to_string();
                            link_contents.clear();
                            report.statistics.increment_feature("links");
                        }
                        Tag::Image { dest_url, title, .. } => {
                            let alt = if title.is_empty() {
                                None
                            } else {
                                Some(title.as_ref())
                            };
                            let img_node = AstNode::image("", current_line, None, &dest_url, alt);
                            pending_contents.push(img_node);
                            report.statistics.increment_feature("images");
                        }
                        Tag::Paragraph => {
                            if !in_heading && current_line_node.is_none() {
                                current_line_node = Some(AstNode::line("", current_line, None, None));
                            }
                        }
                        Tag::FootnoteDefinition(_) => {
                            match self.options.mode {
                                ImportMode::Strict => {
                                    return Err(ImportError {
                                        line: current_line,
                                        message: "Footnotes are not supported by patto".to_string(),
                                    });
                                }
                                ImportMode::Lossy => {
                                    report.add_warning(ImportWarning {
                                        line: current_line,
                                        column: None,
                                        kind: WarningKind::UnsupportedFeature,
                                        feature: "footnote".to_string(),
                                        message: "Dropped footnote definition".to_string(),
                                        suggestion: Some("Move footnote content inline".to_string()),
                                    });
                                    report.statistics.increment_unsupported("footnotes");
                                }
                                ImportMode::Preserve => {}
                            }
                        }
                        _ => {}
                    }
                }
                Event::End(tag_end) => {
                    match tag_end {
                        TagEnd::Heading(_) => {
                            in_heading = false;
                            // Create line node for heading
                            if heading_level == 1 {
                                // H1: plain text + horizontal line
                                let line_node = AstNode::line("", current_line, None, None);
                                for content in heading_contents.drain(..) {
                                    line_node.add_content(content);
                                }
                                root.add_child(line_node);
                                // Add horizontal line
                                let hr = AstNode::horizontal_line("---", current_line, None);
                                root.add_child(hr);
                            } else {
                                // H2-H6: bold decoration
                                let decoration = AstNode::decoration("", current_line, None, 1, false, false, false);
                                for content in heading_contents.drain(..) {
                                    decoration.add_content(content);
                                }
                                let line_node = AstNode::line("", current_line, None, None);
                                line_node.add_content(decoration);
                                root.add_child(line_node);
                            }
                            report.add_warning(ImportWarning {
                                line: current_line,
                                column: None,
                                kind: WarningKind::LossyConversion,
                                feature: "heading".to_string(),
                                message: format!(
                                    "Converted h{} heading to {}",
                                    heading_level,
                                    if heading_level == 1 { "text with horizontal line" } else { "emphasized text" }
                                ),
                                suggestion: None,
                            });
                        }
                        TagEnd::List(_) => {
                            list_stack.pop();
                            indent_level = list_stack.len();
                            // Clear list_root_node when exiting top-level list
                            if list_stack.is_empty() {
                                list_root_node = None;
                            }
                        }
                        TagEnd::Item => {
                            // Finalize the line node with task property if applicable
                            let properties = if let Some(checked) = current_task_status.take() {
                                // Extract text content to get due date
                                let text: String = pending_contents.iter()
                                    .map(|n| n.extract_str().to_string())
                                    .collect::<Vec<_>>()
                                    .join("");
                                
                                let status = if checked { TaskStatus::Done } else { TaskStatus::Todo };
                                let due = self.extract_due_date(&text)
                                    .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                                    .map(Deadline::Date)
                                    .unwrap_or(Deadline::Uninterpretable(String::new()));
                                
                                report.statistics.increment_feature("tasks");
                                
                                Some(vec![Property::Task {
                                    status,
                                    due,
                                    location: crate::parser::Location::default(),
                                }])
                            } else {
                                None
                            };
                            
                            // Create line node with properties
                            let line_node = AstNode::line("", current_line, None, properties);
                            for content in pending_contents.drain(..) {
                                line_node.add_content(content);
                            }
                            
                            // Discard the old line node if any and use the new one
                            current_line_node.take();
                            
                            // Add to list_root_node if in top-level list, otherwise use add_child_at_depth
                            if let Some(ref list_root) = list_root_node {
                                if indent_level == 1 {
                                    // First-level list item: add as child of list_root
                                    list_root.add_child(line_node);
                                } else {
                                    // Nested list item: add at proper depth within list_root
                                    // indent_level=2 means child of a first-level item
                                    self.add_child_at_depth(list_root, line_node, indent_level);
                                }
                            } else {
                                self.add_child_at_depth(&root, line_node, indent_level);
                            }
                        }
                        TagEnd::CodeBlock => {
                            in_code_block = false;
                            if let Some(code) = code_node.take() {
                                // Wrap in a line node
                                let line_node = AstNode::line("", current_line, None, None);
                                line_node.add_content(code);
                                root.add_child(line_node);
                            }
                        }
                        TagEnd::BlockQuote(_) => {
                            in_blockquote = false;
                            if let Some(quote) = quote_node.take() {
                                let line_node = AstNode::line("", current_line, None, None);
                                line_node.add_content(quote);
                                root.add_child(line_node);
                            }
                        }
                        TagEnd::Table => {
                            in_table = false;
                            if let Some(table) = table_node.take() {
                                let line_node = AstNode::line("", current_line, None, None);
                                line_node.add_content(table);
                                root.add_child(line_node);
                            }
                        }
                        TagEnd::TableHead | TagEnd::TableRow => {
                            if let (Some(row), Some(table)) = (current_row.take(), table_node.as_ref()) {
                                table.add_child(row);
                            }
                        }
                        TagEnd::TableCell => {
                            if let (Some(cell), Some(row)) = (current_cell.take(), current_row.as_ref()) {
                                row.add_content(cell);
                            }
                        }
                        TagEnd::Emphasis => {
                            in_emphasis = false;
                        }
                        TagEnd::Strong => {
                            in_strong = false;
                        }
                        TagEnd::Link => {
                            in_link = false;
                            // Convert link to patto format
                            let link_node = self.create_link_node(&link_url, &link_contents, current_line);
                            if in_heading {
                                heading_contents.push(link_node);
                            } else {
                                pending_contents.push(link_node);
                            }
                        }
                        TagEnd::Paragraph => {
                            // Finalize paragraph as line
                            if let Some(line_node) = current_line_node.take() {
                                for content in pending_contents.drain(..) {
                                    line_node.add_content(content);
                                }
                                if !in_blockquote {
                                    root.add_child(line_node);
                                } else if let Some(quote) = quote_node.as_ref() {
                                    // Add as quote content
                                    let quote_content = AstNode::quotecontent("", current_line, None, None);
                                    for content in line_node.value().contents.lock().unwrap().iter() {
                                        quote_content.add_content(content.clone());
                                    }
                                    quote.add_child(quote_content);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::Text(text) => {
                    let text_str = text.to_string();
                    current_line += text_str.matches('\n').count();

                    if in_code_block {
                        if let Some(code) = code_node.as_ref() {
                            for line in text_str.lines() {
                                let code_content = AstNode::codecontent(line, current_line, None);
                                code.add_child(code_content);
                            }
                        }
                    } else if in_table {
                        if let Some(cell) = current_cell.as_ref() {
                            let text_node = AstNode::text(&text_str, current_line, None);
                            cell.add_content(text_node);
                        }
                    } else if in_heading {
                        // Apply decorations if any
                        let content = self.create_text_with_decoration(&text_str, current_line, in_strong, in_emphasis);
                        heading_contents.push(content);
                    } else if in_link {
                        let text_node = AstNode::text(&text_str, current_line, None);
                        link_contents.push(text_node);
                    } else {
                        // Apply decorations
                        let content = self.create_text_with_decoration(&text_str, current_line, in_strong, in_emphasis);
                        pending_contents.push(content);
                    }
                }
                Event::Code(code) => {
                    // Inline code
                    let code_str = code.to_string();
                    let inline_code = AstNode::code(&code_str, current_line, None, "", true);
                    let code_content = AstNode::codecontent(&code_str, current_line, None);
                    inline_code.add_content(code_content);
                    
                    if in_heading {
                        heading_contents.push(inline_code);
                    } else {
                        pending_contents.push(inline_code);
                    }
                    report.statistics.increment_feature("inline_code");
                }
                Event::Html(html) => {
                    let html_str = html.to_string();
                    current_line += html_str.matches('\n').count();

                    match self.options.mode {
                        ImportMode::Strict => {
                            return Err(ImportError {
                                line: current_line,
                                message: format!("HTML is not supported by patto: {}", html_str.trim()),
                            });
                        }
                        ImportMode::Lossy => {
                            report.add_warning(ImportWarning {
                                line: current_line,
                                column: None,
                                kind: WarningKind::UnsupportedFeature,
                                feature: "html".to_string(),
                                message: format!("Dropped HTML: {}", html_str.trim()),
                                suggestion: Some("Use plain text or patto markup instead".to_string()),
                            });
                            report.statistics.increment_unsupported("html");
                        }
                        ImportMode::Preserve => {
                            // Wrap in code block
                            let code = AstNode::code("", current_line, None, "html", false);
                            for line in html_str.lines() {
                                let code_content = AstNode::codecontent(line, current_line, None);
                                code.add_child(code_content);
                            }
                            let line_node = AstNode::line("", current_line, None, None);
                            line_node.add_content(code);
                            root.add_child(line_node);
                            report.add_warning(ImportWarning {
                                line: current_line,
                                column: None,
                                kind: WarningKind::PreservedContent,
                                feature: "html".to_string(),
                                message: "Preserved HTML in code block for manual editing".to_string(),
                                suggestion: None,
                            });
                        }
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    current_line += 1;
                }
                Event::Rule => {
                    let hr = AstNode::horizontal_line("---", current_line, None);
                    root.add_child(hr);
                    report.statistics.increment_feature("horizontal_rules");
                }
                Event::TaskListMarker(checked) => {
                    current_task_status = Some(checked);
                }
                Event::FootnoteReference(name) => {
                    match self.options.mode {
                        ImportMode::Strict => {
                            return Err(ImportError {
                                line: current_line,
                                message: format!("Footnote reference [^{}] is not supported", name),
                            });
                        }
                        ImportMode::Lossy => {
                            report.add_warning(ImportWarning {
                                line: current_line,
                                column: None,
                                kind: WarningKind::UnsupportedFeature,
                                feature: "footnote_ref".to_string(),
                                message: format!("Dropped footnote reference [^{}]", name),
                                suggestion: Some("Move footnote content inline".to_string()),
                            });
                            report.statistics.increment_unsupported("footnotes");
                        }
                        ImportMode::Preserve => {
                            let text = AstNode::text(&format!("[^{}]", name), current_line, None);
                            pending_contents.push(text);
                        }
                    }
                }
                _ => {}
            }
        }

        // Flush any remaining content
        if let Some(line_node) = current_line_node.take() {
            for content in pending_contents.drain(..) {
                line_node.add_content(content);
            }
            root.add_child(line_node);
        }

        Ok(root)
    }

    /// Create a text node with optional decoration
    fn create_text_with_decoration(&self, text: &str, line: usize, bold: bool, italic: bool) -> AstNode {
        if bold || italic {
            let fontsize = if bold { 1 } else { 0 };
            let decoration = AstNode::decoration(text, line, None, fontsize, italic, false, false);
            let text_node = AstNode::text(text, line, None);
            decoration.add_content(text_node);
            decoration
        } else {
            AstNode::text(text, line, None)
        }
    }

    /// Create a link node from URL and content
    fn create_link_node(&self, url: &str, contents: &[AstNode], line: usize) -> AstNode {
        // Extract link text from contents
        let link_text: String = contents.iter()
            .map(|n| n.extract_str().to_string())
            .collect::<Vec<_>>()
            .join("");

        // Check if it's a self-anchor link
        if url.starts_with('#') {
            return AstNode::wikilink("", line, None, "", Some(&url[1..]));
        }

        // Check if it's an internal link (ends with .md or .pn)
        if url.ends_with(".md") || url.ends_with(".pn") || url.contains(".md#") || url.contains(".pn#") {
            if let Some(hash_pos) = url.find('#') {
                let (file_part, anchor) = url.split_at(hash_pos);
                let note_name = file_part.trim_end_matches(".md").trim_end_matches(".pn");
                return AstNode::wikilink("", line, None, note_name, Some(&anchor[1..]));
            }
            let note_name = url.trim_end_matches(".md").trim_end_matches(".pn");
            return AstNode::wikilink("", line, None, note_name, None);
        }

        // External URL
        let title = if link_text.is_empty() || link_text == url {
            None
        } else {
            Some(link_text.as_str())
        };
        AstNode::link("", line, None, url, title)
    }

    /// Add a child node at the specified depth
    /// depth=1 means it's a top-level list item (child of root)
    /// depth=2 means it's a nested item (child of the last depth=1 item)
    fn add_child_at_depth(&self, root: &AstNode, child: AstNode, depth: usize) {
        if depth <= 1 {
            // Top-level items go directly under root
            root.add_child(child);
            return;
        }

        // Find the parent at the right depth
        self.add_child_at_depth_recursive(root, child, depth - 1);
    }

    fn add_child_at_depth_recursive(&self, node: &AstNode, child: AstNode, remaining_depth: usize) {
        let children = node.value().children.lock().unwrap();
        if let Some(last_child) = children.last() {
            if remaining_depth == 1 {
                // Add as child of last_child
                last_child.add_child(child);
            } else {
                // Go deeper
                self.add_child_at_depth_recursive(last_child, child, remaining_depth - 1);
            }
        } else {
            // No children at this level, add here
            drop(children);
            node.add_child(child);
        }
    }

    /// Add task property to a line node
    fn add_task_property_to_line(&self, _line_node: &AstNode, checked: bool, report: &mut ConversionReport) {
        // Extract due date from line content
        let contents = _line_node.value().contents.lock().unwrap();
        let mut text = String::new();
        for content in contents.iter() {
            text.push_str(content.extract_str());
        }
        drop(contents);

        let status = if checked { TaskStatus::Done } else { TaskStatus::Todo };
        let due = self.extract_due_date(&text)
            .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
            .map(Deadline::Date);

        // Create new line node with task property
        // Note: Due to how AstNode works, we can't easily modify the kind after creation
        // The task property is tracked in the report for now
        report.statistics.increment_feature("tasks");
        
        // For now, we'll track this but the actual property setting
        // would require modifying how AstNode is created
        let _ = (status, due);
    }

    fn extract_due_date(&self, text: &str) -> Option<String> {
        // Pattern: 📅 2024-12-31
        if let Some(captures) = Regex::new(r"📅\s*(\d{4}-\d{2}-\d{2})")
            .unwrap()
            .captures(text)
        {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        // Pattern: (due: 2024-12-31)
        if let Some(captures) = Regex::new(r"\(due:\s*(\d{4}-\d{2}-\d{2})\)")
            .unwrap()
            .captures(text)
        {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        // Pattern: [due:: 2024-12-31]
        if let Some(captures) = Regex::new(r"\[due::\s*(\d{4}-\d{2}-\d{2})\]")
            .unwrap()
            .captures(text)
        {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        // Pattern: @2024-12-31
        if let Some(captures) = Regex::new(r"@(\d{4}-\d{2}-\d{2})").unwrap().captures(text) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        None
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
    fn test_horizontal_rule() {
        let result = import_lossy("---");
        assert!(result.patto_content.contains("---"));
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
        assert!(result.report.warnings.len() >= 1);
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
        assert!(result.patto_content.contains("[note]"), "Internal .md link should become wikilink");
        
        let result = import_lossy("[text](note.md#anchor)");
        assert!(result.patto_content.contains("[note#anchor]"), "Link with anchor should preserve anchor");
        
        let result = import_lossy("[section](#anchor)");
        assert!(result.patto_content.contains("[#anchor]"), "Self-anchor link");
        
        let result = import_lossy("[Example](https://example.com)");
        assert!(result.patto_content.contains("[Example https://example.com]"), "External URL");
    }

    #[test]
    fn test_extract_due_date() {
        let importer = MarkdownImporter::new(ImportOptions::default());

        assert_eq!(
            importer.extract_due_date("task 📅 2024-12-31"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            importer.extract_due_date("task (due: 2024-12-31)"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            importer.extract_due_date("task [due:: 2024-12-31]"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            importer.extract_due_date("task @2024-12-31"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(importer.extract_due_date("task without date"), None);
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
