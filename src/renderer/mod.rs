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
