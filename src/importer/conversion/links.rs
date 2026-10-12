use super::Conversion;
use crate::parser::AstNode;

pub(super) struct Link {
    pub(super) url: String,
    pub(super) contents: Vec<AstNode>,
}

impl Conversion<'_> {
    pub(super) fn start_link(&mut self, dest_url: &str) {
        self.link = Some(Link {
            url: dest_url.to_string(),
            contents: Vec::new(),
        });
        self.report.statistics.increment_feature("links");
    }

    pub(super) fn end_link(&mut self) {
        if let Some(link) = self.link.take() {
            let node = link_node(&link.url, &link.contents, self.line);
            self.push_inline(node);
        }
    }
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
        let (file, anchor) = match url.split_once('#') {
            Some((file, anchor)) => (file, Some(anchor)),
            None => (url, None),
        };
        let note = file.trim_end_matches(".md").trim_end_matches(".pn");
        return AstNode::wikilink("", line, None, note, anchor);
    }

    let text: String = contents.iter().map(|node| node.extract_str()).collect();
    let title = (!text.is_empty() && text != url).then_some(text.as_str());
    AstNode::link("", line, None, url, title)
}
