use pulldown_cmark::{CodeBlockKind, HeadingLevel};

use super::{Conversion, Unsupported};
use crate::importer::converter::ImportError;
use crate::importer::report::WarningKind;
use crate::parser::AstNode;

/// A heading being collected; patto has no heading node, so the contents are
/// buffered and re-emitted as a line when the heading ends.
pub(super) struct Heading {
    pub(super) level: u8,
    pub(super) contents: Vec<AstNode>,
}

impl Conversion<'_> {
    pub(super) fn start_heading(&mut self, level: HeadingLevel) {
        self.heading = Some(Heading {
            level: heading_level(level),
            contents: Vec::new(),
        });
        self.report.statistics.increment_feature("headings");
    }

    /// An h1 becomes a line followed by a rule; any other level becomes a
    /// line of emphasised text.
    pub(super) fn end_heading(&mut self) {
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

        let became = if heading.level == 1 {
            "text with horizontal line"
        } else {
            "emphasized text"
        };
        self.warn(
            WarningKind::LossyConversion,
            "heading",
            format!("Converted h{} heading to {}", heading.level, became),
            None,
        );
    }

    /// Headings and list items collect their paragraph's inline content
    /// themselves, so only a top-level paragraph opens a line.
    pub(super) fn start_paragraph(&mut self) {
        if self.heading.is_none() && self.line_node.is_none() && !self.in_list_item() {
            self.line_node = Some(AstNode::line("", self.line, None, None));
        }
    }

    pub(super) fn end_paragraph(&mut self) {
        if let Some(quote) = self.quote.clone() {
            let quote_content = AstNode::quotecontent("", self.line, None, None);
            self.flush_pending_into(&quote_content);
            quote.add_child(quote_content);
            self.line_node = None;
            return;
        }

        if self.in_list_item() {
            self.write_item_line();
            return;
        }

        let Some(line_node) = self.line_node.take() else {
            return;
        };
        self.flush_pending_into(&line_node);
        self.root.add_child(line_node);
    }

    pub(super) fn start_quote(&mut self) {
        self.quote = Some(AstNode::quote("", self.line, None));
        self.report.statistics.increment_feature("blockquotes");
    }

    pub(super) fn end_quote(&mut self) {
        if let Some(quote) = self.quote.take() {
            self.add_block_line(quote);
        }
    }

    pub(super) fn start_code_block(&mut self, kind: CodeBlockKind) {
        let lang = match kind {
            CodeBlockKind::Fenced(lang) => lang.to_string(),
            CodeBlockKind::Indented => String::new(),
        };
        self.code = Some(AstNode::code("", self.line, None, &lang, false));
        self.report.statistics.increment_feature("code_blocks");
    }

    pub(super) fn end_code_block(&mut self) {
        if let Some(code) = self.code.take() {
            self.add_block_line(code);
        }
    }

    pub(super) fn rule(&mut self) {
        self.root
            .add_child(AstNode::horizontal_line("-----", self.line, None));
        self.report.statistics.increment_feature("horizontal_rules");
    }

    pub(super) fn html(&mut self, html: &str) -> Result<(), ImportError> {
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
        add_code_lines(&code, html, self.line);
        self.add_block_line(code);
        self.warn(
            WarningKind::PreservedContent,
            "html",
            "Preserved HTML in code block for manual editing".to_string(),
            None,
        );
        Ok(())
    }

    pub(super) fn footnote_definition(&mut self) -> Result<(), ImportError> {
        self.handle_unsupported(Unsupported {
            feature: "footnote",
            statistic: "footnotes",
            strict_message: "Footnotes are not supported by patto".to_string(),
            lossy_message: "Dropped footnote definition".to_string(),
            suggestion: "Move footnote content inline",
        })?;
        Ok(())
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub(super) fn add_code_lines(code: &AstNode, text: &str, line: usize) {
    for content in text.lines() {
        code.add_child(AstNode::codecontent(content, line, None));
    }
}
