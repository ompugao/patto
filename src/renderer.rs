use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind};
use crate::parser::{Property, TaskStatus};
use crate::utils::{get_twitter_embed, get_youtube_id, get_gyazo_img_src};
use html_escape::encode_text;

pub trait Renderer {
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()>;
}

#[derive(Debug, Default)]
pub struct HtmlRendererOptions {
    // maybe deleted in the future
}

pub struct HtmlRenderer {
    #[allow(dead_code)]
    options: HtmlRendererOptions,
}

impl Renderer for HtmlRenderer {
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        self._format_impl(ast, output)?;
        Ok(())
    }
}

impl HtmlRenderer {
    pub fn new(options: HtmlRendererOptions) -> Self {
        Self { options }
    }

    fn get_stable_id_attr(&self, ast: &AstNode) -> String {
        if let Some(stable_id) = *ast.value().stable_id.lock().unwrap() {
            format!(" data-line-id=\"{}\"", stable_id)
        } else {
            String::new()
        }
    }

    fn _format_impl(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        match &ast.kind() {
            AstNodeKind::Dummy => {
                write!(output, "<ul>")?;
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    let id_attr = self.get_stable_id_attr(child);
                    write!(output, "<li class=\"patto-line\" style=\"list-style-type: none; min-height: 1em;\"{}>", id_attr)?;
                    self._format_impl(child, output)?;
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
                    self._format_impl(content, output)?;
                }
                if isdone {
                    write!(output, "</del>")?;
                }
                if !properties.is_empty() {
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
                    write!(output, "<ul style=\"padding-left: 0rem;\">")?;
                    for child in children.iter() {
                        let id_attr = self.get_stable_id_attr(child);
                        write!(output, "<li class=\"patto-item\" style=\"min-height: 1em;\"{}>", id_attr)?;
                        self._format_impl(child, output)?;
                        write!(output, "</li>")?;
                    }
                    write!(output, "</ul>")?;
                }
            }
            AstNodeKind::Quote => {
                write!(output, "<blockquote>")?;
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    self._format_impl(child, output)?;
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
                    // see https://github.com/mathjax/MathJax/issues/2312
                    write!(output, "\\[\\displaylines{{")?;
                    let children = ast.value().children.lock().unwrap();
                    for child in children.iter() {
                        write!(output, "{}", child.extract_str())?;
                    }
                    write!(output, "}}\\]")?;
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
                            writeln!(output, "{}", child.extract_str())?;
                        }
                        write!(output, "</pre>")?;
                    } else {
                        write!(output, "<pre><code class={}>", lang)?;
                        let children = ast.value().children.lock().unwrap();
                        for child in children.iter() {
                            writeln!(output, "{}", encode_text(child.extract_str()))?;  // TODO encode all at once?
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
                        "<div style=\"max-width: 90%; height: 30vw;\"><iframe class=\"videoContainer__video\" width=100% height=100% src=\"http://www.youtube.com/embed/{youtube_id}?modestbranding=1&autoplay=0&controls=1&fs=1&loop=0&rel=0&showinfo=0&disablekb=0\" frameborder=\"0\" allow=\"accelerometer; autoplay; encrypted-media; gyroscope; picture-in-picture; fullscreen\" allowfullscreen></iframe></div>")?;
                } else if link.contains("twitter.com") || link.contains("x.com") {
                    // Render as placeholder that can be enhanced client-side
                    write!(
                        output,
                        "<div class=\"twitter-placeholder\" data-url=\"{}\"><a href=\"{}\">{}</a></div>",
                        link, link, title.as_deref().unwrap_or(link)
                    )?;
                } else if link.contains("speakerdeck.com") {
                    // Render as placeholder that can be enhanced client-side
                    write!(
                        output,
                        "<div class=\"speakerdeck-placeholder\" data-url=\"{}\"><a href=\"{}\">{}</a></div>",
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
                let fontweight = if *fontsize > 0 {
                    " font-weight: bold;"
                } else {
                    ""
                };
                write!(output, "<span style=\"font-size: {s};{fontweight}\">")?;
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


#[derive(Debug)]
pub struct MarkdownRendererOptions {
    pub use_hard_line_break: bool,
}

impl Default for MarkdownRendererOptions {
    fn default() -> Self {
        Self {
            use_hard_line_break: true,
        }
    }
}

pub struct MarkdownRenderer {
    options: MarkdownRendererOptions,
}

impl Renderer for MarkdownRenderer {
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        let depth: usize = 0;
        self._format_impl(ast, output, depth)?;
        Ok(())
    }
}

impl MarkdownRenderer {
    pub fn new(options: MarkdownRendererOptions) -> Self {
        Self { options }
    }

    fn _format_impl(&self, ast: &AstNode, output: &mut dyn Write, depth: usize) -> io::Result<()> {
        match &ast.kind() {
            AstNodeKind::Dummy => {
                for child in ast.value().children.lock().unwrap().iter() {
                    self._format_impl(child, output, depth)?;
                }
            }
            AstNodeKind::Line { properties } => {
                for _ in 0..depth {
                    write!(output, "  ")?;
                }
                if !self.options.use_hard_line_break || depth > 0 {
                    write!(output, "* ")?;
                }
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
                    self._format_impl(content, output, depth)?;
                }
                if !properties.is_empty() {
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
                let no_children = ast.value().children.lock().unwrap().is_empty();
                if no_children && depth == 0 && self.options.use_hard_line_break  {
                    writeln!(output, " \\")?;
                } else {
                    writeln!(output)?;
                }
                if !no_children {
                    for child in ast.value().children.lock().unwrap().iter() {
                        self._format_impl(child, output, depth + 1)?;
                    }
                }
            }
            AstNodeKind::Quote => {
                for (ichild, child) in ast.value().children.lock().unwrap().iter().enumerate() {
                    if ichild > 0 {
                        for _ in 0..depth {
                            write!(output, "  ")?;
                        }
                        write!(output, "  ")?;
                    }
                    write!(output, "> ")?;
                    self._format_impl(child, output, depth + 1)?;
                }
            }
            AstNodeKind::Math{inline} => {
                if *inline {
                    write!(output, "\\(")?;
                    write!(output, "{}", ast.value().contents.lock().unwrap()[0].extract_str())?; //TODO html escape?
                    write!(output, "\\)")?;
                } else {
                    // see https://github.com/mathjax/MathJax/issues/2312
                    write!(output, "\\[\\displaylines{{")?;
                    for child in ast.value().children.lock().unwrap().iter() {
                        write!(output, "{}", child.extract_str())?;
                    }
                    write!(output, "}}\\]")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
                if *inline {
                    write!(output, "`")?;
                    write!(output, "{}", ast.value().contents.lock().unwrap()[0].extract_str())?;
                    write!(output, "`")?;
                } else {
                    //TODO use syntext
                    writeln!(output, "```{}", lang)?;
                    for child in ast.value().children.lock().unwrap().iter() {
                        writeln!(output, "{}", child.extract_str())?;
                    }
                    writeln!(output, "```")?;
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
                        "<div style=\"max-width: 90%; height: 30vw;\"><iframe class=\"videoContainer__video\" width=100% height=100% src=\"http://www.youtube.com/embed/{youtube_id}?modestbranding=1&autoplay=0&controls=1&fs=1&loop=0&rel=0&showinfo=0&disablekb=0\" frameborder=\"0\" allow=\"accelerometer; autoplay; encrypted-media; gyroscope; picture-in-picture; fullscreen\" allowfullscreen></iframe></div>")?;
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
                    self._format_impl(content, output, depth)?;
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
                    self._format_impl(content, output, depth)?;
                }
            }
        }
        Ok(())
    }
}
