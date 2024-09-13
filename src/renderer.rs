use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind};
use crate::parser::{Property, TaskStatus};

#[derive(Debug, Default)]
pub struct Options {
}

pub trait Renderer {
    fn new(options: Options) -> Self;
    fn format(&self, ast: &AstNode, output:&mut dyn Write) -> io::Result<()>;
} 

pub struct HtmlRenderer {
    options: Options,
}

impl Renderer for HtmlRenderer {
    fn new(options: Options) -> Self {
        HtmlRenderer { options }
    }

    fn format(&self, ast: &AstNode, output:&mut dyn Write) -> io::Result<()> {
        write!(output, "<html>\n")?;
        write!(output, "<head>\n")?;
        write!(output, "</head>\n")?;
        write!(output, "<body>\n")?;
        write!(output, "<div>\n")?;
        self._format_impl(ast, output)?;
        write!(output, "</div>\n")?;
        write!(output, "</body>\n")?;
        write!(output, "</html>\n")?;
        Ok(())
    }
}

impl HtmlRenderer {
    fn _format_impl(&self, ast: &AstNode, output:&mut dyn Write) -> io::Result<()> {
        match &ast.value().kind {
            AstNodeKind::Dummy => {
                write!(output, "<ul>")?;
                for child in ast.value().children.borrow().iter() {
                    write!(output, "<li>")?;
                    self._format_impl(&child, output)?;
                    write!(output, "</li>")?;
                }
                write!(output, "</ul>")?;
            },
            AstNodeKind::Line { properties } => {
                for property in properties {
                    match property {
                        Property::Task{status, until} => {
                            match status {
                                TaskStatus::Done => { write!(output, "<input type=\"checkbox\" checked disabled/>")? },
                                _ => { write!(output, "<input type=\"checkbox\" unchecked disabled/>")? },
                            }
                        },
                        _ => {
                        }
                    }
                }
                for content in ast.value().contents.borrow().iter() {
                    self._format_impl(&content, output)?;
                }
                for property in properties {
                    match property {
                        Property::Anchor{name} => {
                            write!(output, "<a name=\"{}\">{}</a>", name, name)?;
                        },
                        _ => {}
                    }
                }
                if ast.value().children.borrow().len() > 0 {
                    write!(output, "<ul>")?;
                    for child in ast.value().children.borrow().iter() {
                        // TODO handle empty line, needs to insert <br/>
                        write!(output, "<li>")?;
                        self._format_impl(&child, output)?;
                        write!(output, "</li>")?;
                    }
                    write!(output, "</ul>")?;
                }
            }
            AstNodeKind::Quote => {},
            AstNodeKind::Math => {},
            AstNodeKind::Code{lang, inline} => {
                if *inline {
                    write!(output, "<code>")?;
                    write!(output, "{}", ast.value().contents.borrow()[0].extract_str())?; //TODO html escape
                    write!(output, "</code>")?;
                } else {
                    //TODO use syntext
                    write!(output, "<pre><code class={}>", lang)?;
                    for child in ast.value().children.borrow().iter() {
                        write!(output, "{}", child.extract_str())?;
                        write!(output, "<br/>")?;
                    }
                    write!(output, "</code></pre>")?;
                }
            },
            AstNodeKind::Image{src, alt} => {
                write!(output, "<img alt=\"{}\" src=\"{}\"/>", alt, src)?;
            },
            AstNodeKind::WikiLink{link, anchor} => {
                if let Some(anchor) = anchor {
                    write!(output, "<a href=\"{}#{}\">{}#{}</a>", link, anchor, link, anchor)?;
                } else {
                    write!(output, "<a href=\"{}\">{}</a>", link, link)?;
                }
            },
            AstNodeKind::Text => {
                write!(output, "{}", ast.extract_str())?;
            },
        }
        Ok(())
    }
}
