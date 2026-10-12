//! One markdown event stream turned into a patto AST.
//!
//! pulldown-cmark emits a flat stream of start/end events, while patto nests
//! blocks inside lines. `Conversion` keeps one `Option` per block kind that
//! can be open, and each submodule handles the events of one kind of node.

mod blocks;
mod inline;
mod links;
mod lists;
mod tables;
mod tasks;

use pulldown_cmark::{Event, Tag, TagEnd};

use super::converter::ImportError;
use super::options::{ImportMode, ImportOptions};
use super::report::{ConversionReport, ImportWarning, WarningKind};
use crate::parser::{AstNode, AstNodeKind};

use blocks::Heading;
use inline::Decoration;
use links::Link;
use lists::Lists;
use tables::Table;

/// Markdown patto has no way to express.
struct Unsupported<'a> {
    feature: &'a str,
    statistic: &'a str,
    strict_message: String,
    lossy_message: String,
    suggestion: &'a str,
}

pub(super) struct Conversion<'a> {
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
    pub(super) fn new(options: &'a ImportOptions, report: &'a mut ConversionReport) -> Self {
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

    pub(super) fn finish(mut self) -> AstNode {
        if let Some(line_node) = self.line_node.take() {
            self.flush_pending_into(&line_node);
            self.root.add_child(line_node);
        }
        self.root
    }

    pub(super) fn handle(&mut self, event: Event) -> Result<(), ImportError> {
        match event {
            Event::Start(tag) => self.start(tag)?,
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.inline_code(&code),
            Event::Html(html) => self.html(&html)?,
            Event::SoftBreak | Event::HardBreak => self.line += 1,
            Event::Rule => self.rule(),
            Event::TaskListMarker(checked) => self.set_task_checked(checked),
            Event::FootnoteReference(name) => self.footnote_reference(&name)?,
            _ => {}
        }
        Ok(())
    }

    fn start(&mut self, tag: Tag) -> Result<(), ImportError> {
        match tag {
            Tag::Heading { level, .. } => self.start_heading(level),
            Tag::List(_) => self.start_list(),
            Tag::Item => self.start_item(),
            Tag::CodeBlock(kind) => self.start_code_block(kind),
            Tag::BlockQuote(_) => self.start_quote(),
            Tag::Table(_) => self.start_table(),
            Tag::TableHead | Tag::TableRow => self.start_table_row(),
            Tag::TableCell => self.start_table_cell(),
            Tag::Emphasis => self.decoration.italic = true,
            Tag::Strong => self.decoration.bold = true,
            Tag::Strikethrough => self.decoration.strikethrough = true,
            Tag::Link { dest_url, .. } => self.start_link(&dest_url),
            Tag::Image {
                dest_url, title, ..
            } => self.image(&dest_url, &title),
            Tag::Paragraph => self.start_paragraph(),
            Tag::FootnoteDefinition(_) => self.footnote_definition()?,
            _ => {}
        }
        Ok(())
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => self.end_heading(),
            TagEnd::List(_) => self.end_list(),
            TagEnd::Item => self.end_item(),
            TagEnd::CodeBlock => self.end_code_block(),
            TagEnd::BlockQuote(_) => self.end_quote(),
            TagEnd::Table => self.end_table(),
            TagEnd::TableHead | TagEnd::TableRow => self.end_table_row(),
            TagEnd::TableCell => self.end_table_cell(),
            TagEnd::Emphasis => self.decoration.italic = false,
            TagEnd::Strong => self.decoration.bold = false,
            TagEnd::Strikethrough => self.decoration.strikethrough = false,
            TagEnd::Link => self.end_link(),
            TagEnd::Paragraph => self.end_paragraph(),
            _ => {}
        }
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
                self.warn(
                    WarningKind::UnsupportedFeature,
                    unsupported.feature,
                    unsupported.lossy_message,
                    Some(unsupported.suggestion),
                );
                self.report
                    .statistics
                    .increment_unsupported(unsupported.statistic);
                Ok(false)
            }
            ImportMode::Preserve => Ok(true),
        }
    }

    fn warn(
        &mut self,
        kind: WarningKind,
        feature: &str,
        message: String,
        suggestion: Option<&str>,
    ) {
        self.report.add_warning(ImportWarning {
            line: self.line,
            column: None,
            kind,
            feature: feature.to_string(),
            message,
            suggestion: suggestion.map(str::to_string),
        });
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

    fn flush_pending_into(&mut self, line_node: &AstNode) {
        for content in self.pending.drain(..) {
            line_node.add_content(content);
        }
    }

    /// Patto blocks live inside a line, so wrap and append at the top level.
    fn add_block_line(&self, block: AstNode) {
        let line_node = AstNode::line("", self.line, None, None);
        line_node.add_content(block);
        self.root.add_child(line_node);
    }
}
