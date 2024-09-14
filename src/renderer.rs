use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind};
use crate::parser::{Property, TaskStatus};

#[derive(Debug, Default)]
pub struct Options {}

pub trait Renderer {
    fn new(options: Options) -> Self;
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()>;
}

pub struct HtmlRenderer {
    options: Options,
}

impl Renderer for HtmlRenderer {
    fn new(options: Options) -> Self {
        HtmlRenderer { options }
    }

    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
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
    fn _format_impl(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        match &ast.value().kind {
            AstNodeKind::Dummy => {
                write!(output, "<ul>")?;
                for child in ast.value().children.borrow().iter() {
                    write!(output, "<li>")?;
                    self._format_impl(&child, output)?;
                    write!(output, "</li>")?;
                }
                write!(output, "</ul>")?;
            }
            AstNodeKind::Line { properties } => {
                for property in properties {
                    match property {
                        Property::Task { status, .. } => match status {
                            TaskStatus::Done => {
                                write!(output, "<del>")?;
                                write!(output, "<input type=\"checkbox\" checked disabled/>")?
                            }
                            _ => write!(output, "<input type=\"checkbox\" unchecked disabled/>")?,
                        },
                        _ => {}
                    }
                }
                for content in ast.value().contents.borrow().iter() {
                    self._format_impl(&content, output)?;
                }
                for property in properties {
                    match property {
                        Property::Anchor { name } => {
                            write!(output, "<a name=\"{}\">{}</a>", name, name)?;
                        }
                        Property::Task { status, until } => match status {
                            TaskStatus::Done => {
                                write!(output, "</del>")?;
                            }
                            _ => {
                                write!(output, "<code class=\"task-deadline\">{}</code>", until)?
                            }
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
            AstNodeKind::Quote => {
                write!(output, "<blockquote>")?;
                for child in ast.value().children.borrow().iter() {
                    self._format_impl(&child, output)?;
                    write!(output, "<br/>")?;
                }
                write!(output, "</blockquote>")?;
            }
            AstNodeKind::Math{inline} => {
                if *inline {
                    write!(output, "$$ ")?;
                    write!(output, "{}", ast.value().contents.borrow()[0].extract_str())?; //TODO html escape?
                    write!(output, " $$")?;
                } else {
                    write!(output, "[[ ")?;
                    for child in ast.value().children.borrow().iter() {
                        write!(output, "{}", child.extract_str())?;
                        write!(output, "\n")?;
                    }
                    write!(output, " ]]")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
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
            }
            AstNodeKind::Image { src, alt } => {
                write!(output, "<img alt=\"{}\" src=\"{}\"/>", alt, src)?;
            }
            AstNodeKind::WikiLink { link, anchor } => {
                if let Some(anchor) = anchor {
                    write!(
                        output,
                        "<a href=\"{}#{}\">{}#{}</a>",
                        link, anchor, link, anchor
                    )?;
                } else {
                    write!(output, "<a href=\"{}\">{}</a>", link, link)?;
                }
            }
            AstNodeKind::Decoration{ fontsize, italic, underline, deleted } => {
                let s = match fontsize {
                    isize::MIN..=-3 => "xx-small",
                    -2 => "x-small",
                    -1 => "small",
                    0 => "medium",
                    1 => "large",
                    2 => "x-large",
                    3..=isize::MAX => "xx-large",
                    _ => "",
                };
                write!(output, "<span style=\"font-size: {s}\">")?;
                if *italic {
                    write!(output, "<i>")?;
                }
                if *underline {
                    write!(output, "<u>")?;
                }
                if *deleted {
                    write!(output, "<del>")?;
                }
                for content in ast.value().contents.borrow().iter() {
                    self._format_impl(&content, output)?;
                }
                if *deleted {
                    write!(output, "</del>")?;
                }
                if *underline {
                    write!(output, "</u>")?;
                }
                if *italic {
                    write!(output, "</i>")?;
                }
                write!(output, "</span>")?;
            }
            AstNodeKind::Text => {
                write!(output, "{}", ast.extract_str())?;
            }
            AstNodeKind::Table => {todo!()}
            AstNodeKind::TableColumn => {
                for content in ast.value().contents.borrow().iter() {
                    self._format_impl(&content, output)?;
                }
            }
        }
        Ok(())
    }
}
