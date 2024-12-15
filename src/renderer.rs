use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind};
use crate::parser::{Property, TaskStatus};
use crate::utils::{get_twitter_embed, get_youtube_id, get_gyazo_img_src};
use html_escape::encode_text;

#[derive(Debug, Default)]
pub struct Options {
    pub theme: String,
}

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

        //write!(output, "<link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/gh/yegor256/tacit@gh-pages/tacit-css-1.8.1.min.css\"/>\n")?;
        write!(output, "<link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/sakura.css/css/sakura.css\" type=\"text/css\" media=\"screen\">\n")?;
        //write!(output, "<link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/sakura.css/css/sakura-dark.css\" type=\"text/css\">\n");
        if self.options.theme == "dark" {
            write!(output, "<link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/sakura.css/css/sakura-vader.css\" type=\"text/css\" media=\"screen and (prefers-color-scheme: dark)\">\n")?;
        } else {
            write!(output, "<link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/npm/sakura.css/css/sakura-vader.css\" type=\"text/css\" media=\"screen and (prefers-color-scheme: light)\">\n")?;
        }


        if self.options.theme == "dark" {
            write!(output, "<link rel=\"stylesheet\" href=\"https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css\" type=\"text/css\" type=\"text/css\" >")?;
        } else {
            write!(output, "<link rel=\"stylesheet\" href=\"https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github.min.css\" type=\"text/css\" type=\"text/css\" >")?;
        }
        write!(output, "<script src=\"https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js\"></script>")?;
        write!(output, "<script>hljs.highlightAll();</script>")?;
        write!(output, "<body style=\"max-width: max-content\">\n")?;
        write!(output, "<script type=\"module\">")?;
        write!(output, "import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';")?;
        write!(output, "mermaid.initialize({{ startOnLoad: true, theme: 'forest' }});")?;
        write!(output, "</script>")?;
        write!(output, "<section style=\"width: 1920px; max-width: 100%;\">\n")?;
        write!(output, "<article>\n")?;
        self._format_impl(ast, output)?;
        write!(output, "</article>\n")?;
        write!(output, "</section>\n")?;
        write!(output, "</body>\n")?;
        write!(output, "</html>\n")?;
        Ok(())
    }
}

impl HtmlRenderer {
    fn _format_impl(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        match &ast.value().kind {
            AstNodeKind::Dummy => {
                write!(output, "<ul style=\"margin-bottom: 1.5rem\">")?;
                for child in ast.value().children.lock().unwrap().iter() {
                    write!(output, "<li style=\"list-style-type: none\">")?;
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
                for content in ast.value().contents.lock().unwrap().iter() {
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
                                write!(output, "<a name=\"{}\">{}</a>", name, name)?;
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
                if ast.value().children.lock().unwrap().len() > 0 {
                    write!(output, "<ul style=\"margin-bottom: 0.5rem\">")?;
                    for child in ast.value().children.lock().unwrap().iter() {
                        // TODO stealing the internal content, not efficient
                        // implement a trait that overloads `write' function that counts the
                        // written bytes
                        let mut bufcur = io::Cursor::new(Vec::<u8>::new());
                        self._format_impl(&child, &mut bufcur)?;
                        let s = unsafe {String::from_utf8_unchecked(bufcur.into_inner())};
                        if s.len() == 0 {
                            write!(output, "<li style=\"list-style-type: none\">")?;
                            write!(output, "{}<br/>", s)?;
                        } else {
                            write!(output, "<li>")?;
                            write!(output, "{}", s)?;
                        }
                        // no stealing:
                        // write!(output, "<li>")?;
                        // self._format_impl(&child, output)?;
                        write!(output, "</li>")?;
                    }
                    write!(output, "</ul>")?;
                }
            }
            AstNodeKind::Quote => {
                write!(output, "<blockquote>")?;
                for child in ast.value().children.lock().unwrap().iter() {
                    self._format_impl(&child, output)?;
                    write!(output, "<br/>")?;
                }
                write!(output, "</blockquote>")?;
            }
            AstNodeKind::Math{inline} => {
                if *inline {
                    write!(output, "$$ ")?;
                    write!(output, "{}", ast.value().contents.lock().unwrap()[0].extract_str())?; //TODO html escape?
                    write!(output, " $$")?;
                } else {
                    write!(output, "[[ ")?;
                    for child in ast.value().children.lock().unwrap().iter() {
                        write!(output, "{}", child.extract_str())?;
                        write!(output, "\n")?;
                    }
                    write!(output, " ]]")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
                if *inline {
                    write!(output, "<code>")?;
                    write!(output, "{}", ast.value().contents.lock().unwrap()[0].extract_str())?; //TODO html escape
                    write!(output, "</code>")?;
                } else {
                    //TODO use syntext
                    if lang == "mermaid" {
                        write!(output, "<pre class={}>", lang)?;
                        for child in ast.value().children.lock().unwrap().iter() {
                            write!(output, "{}\n", child.extract_str())?;
                        }
                        write!(output, "</pre>")?;
                    } else {
                        write!(output, "<pre><code class={}>", lang)?;
                        for child in ast.value().children.lock().unwrap().iter() {
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
                    write!(output, "<img alt=\"{}\" src=\"{}\"/>", alt, src_exported)?;
                } else {
                    write!(output, "<img src=\"{}\"/>", src_exported)?;
                }
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
            AstNodeKind::Link { link, title } => {
                if let Some(youtube_id) = get_youtube_id(link) {
                    write!(
                        output,
                        "<iframe class=\"videoContainer__video\" width=\"640\" height=\"480\" src=\"http://www.youtube.com/embed/{youtube_id}?modestbranding=1&autoplay=0&controls=1&fs=1&loop=0&rel=0&showinfo=0&disablekb=0\" frameborder=\"0\"></iframe>")?;
                } else if let Some(embed) = get_twitter_embed(link) {
                    write!(output, "{}", embed)?;
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
                for content in ast.value().contents.lock().unwrap().iter() {
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
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(&content, output)?;
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
        match &ast.value().kind {
            AstNodeKind::Dummy => {
                for child in ast.value().children.lock().unwrap().iter() {
                    self._format_impl(&child, output, depth)?;
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
                    for property in properties {
                        match property {
                            Property::Anchor { name } => {
                                write!(output, "#\"{}\"", name)?;
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
                    write!(output, "$$ ")?;
                    write!(output, "{}", ast.value().contents.lock().unwrap()[0].extract_str())?; //TODO html escape?
                    write!(output, " $$")?;
                } else {
                    write!(output, "[[ \n")?;
                    for child in ast.value().children.lock().unwrap().iter() {
                        write!(output, "{}\n", child.extract_str())?;
                    }
                    write!(output, " ]]\n")?;
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
                    write!(
                        output,
                        "[{}#{}]({}#{})",
                        link, anchor, link, anchor
                    )?;
                } else {
                    write!(output, "[{}]({})", link, link)?;
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
