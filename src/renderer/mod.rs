use std::io;
use std::io::Write;

use crate::parser::AstNode;

mod html;
mod markdown;
mod patto;

pub use html::{HtmlRenderer, HtmlRendererOptions};
pub use markdown::MarkdownRenderer;
pub use patto::PattoRenderer;

pub trait Renderer {
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()>;
}

/// What a wiki link points at.
enum WikiTarget<'a> {
    SelfAnchor(&'a str),
    NoteAnchor { note: &'a str, anchor: &'a str },
    Note(&'a str),
}

// FIXME: the parser encodes "anchor in this note" as an empty link name, so
// every renderer has to special-case it here instead of matching a node kind.
fn wiki_target<'a>(link: &'a str, anchor: Option<&'a str>) -> WikiTarget<'a> {
    match anchor {
        Some(anchor) if link.is_empty() => WikiTarget::SelfAnchor(anchor),
        Some(anchor) => WikiTarget::NoteAnchor { note: link, anchor },
        None => WikiTarget::Note(link),
    }
}

/// The body of a code or math block: one child per line, each after `prefix`.
fn write_lines(ast: &AstNode, output: &mut dyn Write, prefix: &str) -> io::Result<()> {
    for child in ast.children().iter() {
        writeln!(output, "{}{}", prefix, child.extract_str())?;
    }
    Ok(())
}
