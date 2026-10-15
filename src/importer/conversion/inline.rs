use super::blocks::add_code_lines;
use super::{Conversion, Unsupported};
use crate::importer::converter::ImportError;
use crate::parser::{AstNode, AstNodeKind};

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
            let node = AstNode::text(text, self.line, None);
            self.push_decorated(node);
        }
    }

    pub(super) fn inline_code(&mut self, code: &str) {
        let node = AstNode::code(code, self.line, None, "", true);
        node.add_content(AstNode::codecontent(code, self.line, None));
        self.push_decorated(node);
        self.report.statistics.increment_feature("inline_code");
    }

    pub(super) fn inline_math(&mut self, math: &str) {
        self.line += math.matches('\n').count();
        let node = inline_math_node(math, self.line);
        if let Some(link) = self.link.as_mut() {
            link.contents.push(node);
        } else {
            self.push_decorated(node);
        }
        self.report.statistics.increment_feature("math");
    }

    /// Wrap `node` in the active decoration. Consecutive pieces with the same
    /// decoration share one node, so `**a $x$ b**` stays a single `[* ...]`.
    fn push_decorated(&mut self, node: AstNode) {
        if !self.decoration.any() {
            self.push_inline(node);
            return;
        }
        if let Some(last) = self.last_inline() {
            if self.decoration.matches(&last) {
                last.add_content(node);
                return;
            }
        }
        let Decoration {
            bold,
            italic,
            strikethrough,
        } = self.decoration;
        let fontsize = if bold { 1 } else { 0 };
        let decoration =
            AstNode::decoration("", self.line, None, fontsize, italic, false, strikethrough);
        decoration.add_content(node);
        self.push_inline(decoration);
    }

    /// `$$...$$` becomes a `[@math]` block. Patto blocks own a whole line, so
    /// the paragraph is split around it: text before stays on its line, the
    /// block follows, and text after starts a new line.
    ///
    /// Where a block cannot go (headings, table cells, quotes, link text) the
    /// formula falls back to inline math.
    pub(super) fn display_math(&mut self, math: &str) {
        self.report.statistics.increment_feature("math");
        if self.heading.is_some()
            || self.table.is_some()
            || self.quote.is_some()
            || self.link.is_some()
        {
            self.inline_math(math);
            return;
        }

        let block = AstNode::math("", self.line, None, false);
        for content in math.trim().lines() {
            block.add_child(AstNode::mathcontent(content.trim_end(), self.line, None));
        }
        self.line += math.matches('\n').count();

        if self.in_list_item() {
            self.write_item_line();
            let Some(item_line) = self.current_item_line() else {
                return;
            };
            let math_line = AstNode::line("", self.line, None, None);
            math_line.add_content(block);
            item_line.add_child(math_line);
            return;
        }

        if let Some(line_node) = self.line_node.take() {
            if !self.pending.is_empty() {
                self.flush_pending_into(&line_node);
                self.root.add_child(line_node);
            }
            self.line_node = Some(AstNode::line("", self.line, None, None));
        }
        self.add_block_line(block);
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

impl Decoration {
    fn any(self) -> bool {
        self.bold || self.italic || self.strikethrough
    }

    /// Whether `node` is a decoration with exactly this style.
    fn matches(self, node: &AstNode) -> bool {
        match node.kind() {
            AstNodeKind::Decoration {
                fontsize,
                italic,
                underline,
                deleted,
            } => {
                (*fontsize > 0) == self.bold
                    && *italic == self.italic
                    && !*underline
                    && *deleted == self.strikethrough
            }
            _ => false,
        }
    }
}

fn inline_math_node(math: &str, line: usize) -> AstNode {
    // Patto inline math is a single line.
    let math = math.split_whitespace().collect::<Vec<_>>().join(" ");
    let node = AstNode::math(&math, line, None, true);
    node.add_content(AstNode::mathcontent(&math, line, None));
    node
}
