use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind};
use crate::parser::{Property, TaskStatus};
use crate::utils::{get_twitter_embed, get_youtube_id, get_gyazo_img_src};
use html_escape::encode_text;

#[derive(Debug, Default)]
pub struct Options {
    // maybe deleted in the future
}

pub trait Renderer {
    fn new(options: Options) -> Self;
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()>;
}

pub struct HtmlRenderer {
    #[allow(dead_code)]
    options: Options,
}

impl Renderer for HtmlRenderer {
    fn new(options: Options) -> Self {
        HtmlRenderer { options }
    }

    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        self._format_impl(ast, output)?;
        Ok(())
    }
}

impl HtmlRenderer {

    fn _format_impl(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        match &ast.kind() {
            AstNodeKind::Dummy => {
                write!(output, "<ul style=\"margin-bottom: 1.5rem\">")?;
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    write!(output, "<li class=\"patto-line\" style=\"list-style-type: none; min-height: 1em;\">")?;
                    self._format_impl(&child, output)?;
                    write!(output, "</li>")?;
                }
                write!(output, "</ul>")?;
            }
            AstNodeKind::Line { properties } => {
                let mut isdone = false;
                for property in properties {
                    match property {
                        Property::Task { status, .. } => match status {
                            TaskStatus::Done => {
                                isdone = true;
                                write!(output, "<input type=\"checkbox\" checked disabled/>")?
                            }
                            _ => write!(output, "<input type=\"checkbox\" unchecked disabled/>")?,
                        },
                        _ => {}
                    }
                }

                if isdone {
                    write!(output, "<del>")?;
                }
                let contents = ast.value().contents.lock().unwrap();
                for content in contents.iter() {
                    self._format_impl(&content, output)?;
                }
                if isdone {
                    write!(output, "</del>")?;
                }
                if properties.len() > 0 {
                    write!(output, "<aside style=\"float: right; width: 285px; text-align: right\">")?;
                    for property in properties {
                        match property {
                            Property::Anchor { name } => {
                                write!(output, "<span id=\"{}\" class=\"anchor\">{}</span>", name, name)?;
                            }
                            Property::Task { status, due } => match status {
                                TaskStatus::Done => {
                                    // do nothing
                                }
                                _ => {
                                    write!(output, "<mark class=\"task-deadline\">{}</mark>", due)?
                                }
                            },
                        }
                    }
                    write!(output, "</aside>")?;
                }
                let children = ast.value().children.lock().unwrap();
                if !children.is_empty() {
                    write!(output, "<ul style=\"margin-bottom: 0.5rem\">")?;
                    for child in children.iter() {
                        write!(output, "<li class=\"patto-item\" style=\"min-height: 1em;\">")?;
                        self._format_impl(&child, output)?;
                        write!(output, "</li>")?;
                    }
                    write!(output, "</ul>")?;
                }
            }
            AstNodeKind::Quote => {
                write!(output, "<blockquote>")?;
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    self._format_impl(&child, output)?;
                    write!(output, "<br/>")?;
                }
                write!(output, "</blockquote>")?;
            }
            AstNodeKind::Math{inline} => {
                if *inline {
                    write!(output, "\\(")?;
                    let contents = ast.value().contents.lock().unwrap();
                    write!(output, "{}", contents[0].extract_str())?; //TODO html escape?
                    write!(output, "\\)")?;
                } else {
                    write!(output, "\\[")?;
                    let children = ast.value().children.lock().unwrap();
                    for child in children.iter() {
                        write!(output, "{}", child.extract_str())?;
                        write!(output, "\n")?;
                    }
                    write!(output, "\\]")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
                if *inline {
                    write!(output, "<code>")?;
                    let contents = ast.value().contents.lock().unwrap();
                    write!(output, "{}", encode_text(contents[0].extract_str()))?;
                    write!(output, "</code>")?;
                } else {
                    //TODO use syntext
                    if lang == "mermaid" {
                        write!(output, "<pre class={}>", lang)?;
                        let children = ast.value().children.lock().unwrap();
                        for child in children.iter() {
                            write!(output, "{}\n", child.extract_str())?;
                        }
                        write!(output, "</pre>")?;
                    } else {
                        write!(output, "<pre><code class={}>", lang)?;
                        let children = ast.value().children.lock().unwrap();
                        for child in children.iter() {
                            write!(output, "{}\n", encode_text(child.extract_str()))?;  // TODO encode all at once?
                            //write!(output, "<br/>")?;
                        }
                        write!(output, "</code></pre>")?;
                    }
                }
            }
            AstNodeKind::Image { src, alt } => {
                let mut src_exported = src.clone();
                if let Some(src) = get_gyazo_img_src(src) {
                    src_exported = src.clone();
                }
                if let Some(alt) = alt {
                    write!(output, "<img class=\"patto-image\" alt=\"{}\" src=\"{}\"/>", alt, src_exported)?;
                } else {
                    write!(output, "<img class=\"patto-image\" src=\"{}\"/>", src_exported)?;
                }
            }
            AstNodeKind::WikiLink { link, anchor } => {
                if let Some(anchor) = anchor {
                    // TODO eliminate the logic that self-link if link is empty
                    if link.is_empty() {
                        write!(output, "<a class=\"patto-selflink\" href=\"#{}\">#{}</a>", anchor, anchor)?;
                    } else {
                        write!(
                            output,
                            "<a class=\"patto-wikilink\" href=\"{}.pn#{}\">{}#{}</a>",
                            link, anchor, link, anchor
                        )?;
                    }
                } else {
                    write!(output, "<a class=\"patto-wikilink\" href=\"{}.pn\">{}</a>", link, link)?;
                }
            }
            AstNodeKind::Link { link, title } => {
                if let Some(youtube_id) = get_youtube_id(link) {
                    write!(
                        output,
                        "<iframe class=\"videoContainer__video\" width=\"640\" height=\"480\" src=\"http://www.youtube.com/embed/{youtube_id}?modestbranding=1&autoplay=0&controls=1&fs=1&loop=0&rel=0&showinfo=0&disablekb=0\" frameborder=\"0\"></iframe>")?;
                } else if link.contains("twitter.com") || link.contains("x.com") {
                    // Render as placeholder that can be enhanced client-side
                    write!(
                        output,
                        "<div class=\"twitter-placeholder\" data-url=\"{}\"><a href=\"{}\">{}</a></div>",
                        link, link, title.as_deref().unwrap_or(link)
                    )?;
                } else if let Some(title) = title {
                    write!(
                        output,
                        "<a href=\"{}\">{}</a>",
                        link, title
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
                write!(output, "<span style=\"font-size: {s}; font-weight: bold\">")?;
                if *italic {
                    write!(output, "<i>")?;
                }
                if *underline {
                    write!(output, "<u>")?;
                }
                if *deleted {
                    write!(output, "<del>")?;
                }
                let contents = ast.value().contents.lock().unwrap();
                for content in contents.iter() {
                    self._format_impl(content, output)?;
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
            AstNodeKind::HorizontalLine => {
                write!(output, "<hr/>")?;
            }
            AstNodeKind::Table => {todo!()}
            AstNodeKind::TableColumn => {
                let contents = ast.value().contents.lock().unwrap();
                for content in contents.iter() {
                    self._format_impl(content, output)?;
                }
            }
        }
        Ok(())
    }
}



pub struct MarkdownRenderer {
}

impl Renderer for MarkdownRenderer {
    fn new(_options: Options) -> Self {
        Self { }
    }

    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        let depth: usize = 0;
        self._format_impl(ast, output, depth)?;
        Ok(())
    }
}

impl MarkdownRenderer {
    fn _format_impl(&self, ast: &AstNode, output: &mut dyn Write, depth: usize) -> io::Result<()> {
        match &ast.kind() {
            AstNodeKind::Dummy => {
                for child in ast.value().children.lock().unwrap().iter() {
                    self._format_impl(child, output, depth)?;
                    //write!(output, "  ")?;  // more than two trailing spaces represent a newline
                }
            }
            AstNodeKind::Line { properties } => {
                for _ in 0..depth {
                    write!(output, "  ")?;
                }
                write!(output, "* ")?;
                for property in properties {
                    if let Property::Task { status, .. } = property {
                        if matches!(status, TaskStatus::Done) {
                            write!(output, "[-] ")?;
                        } else {
                            write!(output, "[ ] ")?;
                        }
                        break;
                    }
                }

                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(&content, output, depth)?;
                }
                if properties.len() > 0 {
                    write!(output, " ")?;
                    for property in properties {
                        match property {
                            Property::Anchor { name } => {
                                write!(output, "#{}", name)?;
                            }
                            Property::Task { status, due } => match status {
                                TaskStatus::Done => {
                                    // do nothing
                                }
                                _ => {
                                    write!(output, "  due: {}", due)?
                                }
                            },
                        }
                    }
                }
                write!(output, "\n")?;
                if ast.value().children.lock().unwrap().len() > 0 {
                    for child in ast.value().children.lock().unwrap().iter() {
                        self._format_impl(&child, output, depth + 1)?;
                    }
                }
            }
            AstNodeKind::Quote => {
                for child in ast.value().children.lock().unwrap().iter() {
                    for _ in 0..depth {
                        write!(output, "  ")?;
                    }
                    write!(output, "> ")?;
                    self._format_impl(&child, output, depth + 1)?;
                }
            }
            AstNodeKind::Math{inline} => {
                if *inline {
                    write!(output, "\\(")?;
                    write!(output, "{}", ast.value().contents.lock().unwrap()[0].extract_str())?; //TODO html escape?
                    write!(output, "\\)")?;
                } else {
                    write!(output, "\\[\n")?;
                    for child in ast.value().children.lock().unwrap().iter() {
                        write!(output, "{}\n", child.extract_str())?;
                    }
                    write!(output, "\\]\n")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
                if *inline {
                    write!(output, "`")?;
                    write!(output, "{}", ast.value().contents.lock().unwrap()[0].extract_str())?;
                    write!(output, "`")?;
                } else {
                    //TODO use syntext
                    write!(output, "```{}\n", lang)?;
                    for child in ast.value().children.lock().unwrap().iter() {
                        write!(output, "{}\n", child.extract_str())?;
                    }
                    write!(output, "```\n")?;
                }
            }
            AstNodeKind::Image { src, alt } => {
                if let Some(alt) = alt {
                    write!(output, "![{}]({})", alt, src)?;
                } else {
                    write!(output, "![{}]({})", src, src)?;
                }
            }
            AstNodeKind::WikiLink { link, anchor } => {
                if let Some(anchor) = anchor {
                    write!(output, "[[{}#{}]]", link, anchor)?;
                    //write!(
                    //    output,
                    //    "[{}#{}]({}.md#{})",
                    //    link, anchor, link, anchor
                    //)?;
                } else {
                    write!(output, "[[{}]]", link)?;
                }
            }
            AstNodeKind::Link { link, title } => {
                if let Some(youtube_id) = get_youtube_id(link) {
                    write!(
                        output,
                        "<iframe class=\"videoContainer__video\" width=\"640\" height=\"480\" src=\"http://www.youtube.com/embed/{youtube_id}?modestbranding=1&autoplay=0&controls=1&fs=1&loop=0&rel=0&showinfo=0&disablekb=0\" frameborder=\"0\"></iframe>")?;
                } else if let Some(embed) = get_twitter_embed(link) {
                    write!(output, "{}", embed)?;
                } else if let Some(title) = title {
                    write!(output, "[{}]({})", title, link)?;
                } else {
                    write!(output, "[{}]({})", link, link)?;
                }
            }
            AstNodeKind::Decoration{ fontsize, italic, underline, deleted } => {
                if *fontsize > 0 && !*italic {
                    // bold
                    write!(output, "**")?;
                } else if *italic && *fontsize <= 0 {
                    // italic
                    write!(output, "*")?;
                } else if *italic && *fontsize > 0 {
                    // bold italic
                    write!(output, "***")?;
                }
                if *underline {
                    write!(output, "<ins>")?;
                }
                if *deleted {
                    write!(output, "<del>")?;
                }
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(&content, output, depth)?;
                }
                if *deleted {
                    write!(output, "</del>")?;
                }
                if *underline {
                    write!(output, "</ins>")?;
                }
                if *fontsize > 0 && !*italic {
                    // bold
                    write!(output, "**")?;
                } else if *italic && *fontsize <= 0 {
                    // italic
                    write!(output, "*")?;
                } else if *italic && *fontsize > 0 {
                    // bold italic
                    write!(output, "***")?;
                }
            }
            AstNodeKind::Text => {
                write!(output, "{}", ast.extract_str())?;
            }
            AstNodeKind::HorizontalLine => {
                write!(output, "---")?;
            }
            AstNodeKind::Table => {todo!()}
            AstNodeKind::TableColumn => {
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(&content, output, depth)?;
                }
            }
        }
        Ok(())
    }
}
