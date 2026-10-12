use super::blocks::add_code_lines;
use super::{Conversion, Unsupported};
use crate::importer::converter::ImportError;
use crate::parser::AstNode;

#[derive(Default, Clone, Copy)]
pub(super) struct Decoration {
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) strikethrough: bool,
}

impl Conversion<'_> {
    pub(super) fn text(&mut self, text: &str) {
        self.line += text.matches('\n').count();

        if let Some(code) = self.code.as_ref() {
            add_code_lines(code, text, self.line);
        } else if let Some(link) = self.link.as_mut() {
            // Link text is the link title; the finished link is routed on TagEnd::Link.
            link.contents.push(AstNode::text(text, self.line, None));
        } else {
            let node = decorated_text(text, self.line, self.decoration);
            self.push_inline(node);
        }
    }

    pub(super) fn inline_code(&mut self, code: &str) {
        let node = AstNode::code(code, self.line, None, "", true);
        node.add_content(AstNode::codecontent(code, self.line, None));
        self.push_inline(node);
        self.report.statistics.increment_feature("inline_code");
    }

    pub(super) fn image(&mut self, dest_url: &str, title: &str) {
        let alt = (!title.is_empty()).then_some(title);
        let image = AstNode::image("", self.line, None, dest_url, alt);
        self.push_inline(image);
        self.report.statistics.increment_feature("images");
    }

    pub(super) fn footnote_reference(&mut self, name: &str) -> Result<(), ImportError> {
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
