use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind, Property, TaskStatus};

use super::Renderer;
use crate::utils::{get_gyazo_img_src, get_youtube_id};
use html_escape::encode_text;

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
        self.write_node(ast, output)
    }
}

impl HtmlRenderer {
    pub fn new(options: HtmlRendererOptions) -> Self {
        Self { options }
    }

    fn write_node(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        match ast.kind() {
            AstNodeKind::Dummy => self.write_document(ast, output),
            AstNodeKind::Line { properties } => self.write_line(ast, properties, false, output),
            AstNodeKind::QuoteContent { properties } => {
                self.write_line(ast, properties, true, output)
            }
            AstNodeKind::Quote => self.write_quote(ast, output),
            AstNodeKind::Math { inline } => self.write_math(ast, *inline, output),
            AstNodeKind::Code { lang, inline } => self.write_code(ast, lang, *inline, output),
            AstNodeKind::Image { src, alt } => self.write_image(src, alt.as_deref(), output),
            AstNodeKind::WikiLink { link, anchor } => {
                self.write_wikilink(link, anchor.as_deref(), output)
            }
            AstNodeKind::Link { link, title } => self.write_link(link, title.as_deref(), output),
            AstNodeKind::Embed { link, title } => self.write_embed(link, title.as_deref(), output),
            AstNodeKind::Decoration {
                fontsize,
                italic,
                underline,
                deleted,
            } => self.write_decoration(ast, *fontsize, *italic, *underline, *deleted, output),
            AstNodeKind::Table { caption } => self.write_table(ast, caption.as_deref(), output),
            AstNodeKind::TableRow => self.write_cells(ast, "tr", output),
            AstNodeKind::TableColumn => self.write_cells(ast, "td", output),
            AstNodeKind::Text | AstNodeKind::CodeContent | AstNodeKind::MathContent => {
                write!(output, "{}", ast.extract_str())
            }
            AstNodeKind::HorizontalLine => write!(output, "<hr class=\"patto-hr\"/>"),
        }
    }

    /// Identifies the line for the preview, which patches changed lines in place.
    fn stable_id_attr(&self, ast: &AstNode) -> String {
        match ast.stable_id() {
            Some(stable_id) => format!(" data-line-id=\"{}\"", stable_id),
            None => String::new(),
        }
    }

    fn write_document(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        write!(output, "<ul class=\"patto-document\">")?;
        for child in ast.children().iter() {
            self.write_list_item(child, "patto-line", output)?;
        }
        write!(output, "</ul>")
    }

    fn write_list_item(
        &self,
        ast: &AstNode,
        class: &str,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        write!(
            output,
            "<li class=\"{}\"{}>",
            class,
            self.stable_id_attr(ast)
        )?;
        self.write_node(ast, output)?;
        write!(output, "</li>")
    }

    fn write_line(
        &self,
        ast: &AstNode,
        properties: &[Property],
        is_quote: bool,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        let task_status = properties.iter().rev().find_map(|property| match property {
            Property::Task { status, .. } => Some(status),
            _ => None,
        });
        let is_done = matches!(task_status, Some(TaskStatus::Done));

        write!(output, "<div class=\"patto-task-row\">")?;
        if let Some(status) = task_status {
            let (icon, class) = match status {
                TaskStatus::Done => ("✓", "patto-task-icon-done"),
                TaskStatus::Doing => ("◑", "patto-task-icon-doing"),
                _ => ("○", "patto-task-icon-todo"),
            };
            write!(
                output,
                "<span class=\"patto-task-icon {}\">{}</span>",
                class, icon
            )?;
        }

        let done_class = if is_done { " patto-task-done-text" } else { "" };
        let quote_class = if is_quote { " text-quote" } else { "" };
        write!(
            output,
            "<div class=\"patto-task-content{}{}\">",
            done_class, quote_class
        )?;
        for content in ast.contents().iter() {
            self.write_node(content, output)?;
        }
        write!(output, "</div>")?;

        for property in properties {
            match property {
                Property::Anchor { name, .. } => {
                    write!(
                        output,
                        "<span id=\"{}\" class=\"anchor\">{}</span>",
                        name, name
                    )?;
                }
                // A done task's deadline is no longer interesting.
                Property::Task { status, due, .. } if !matches!(status, TaskStatus::Done) => {
                    write!(
                        output,
                        "<span class=\"patto-deadline patto-deadline-default\">{}</span>",
                        due
                    )?;
                }
                Property::Task { .. } => {}
            }
        }
        write!(output, "</div>")?;

        let children = ast.children();
        if !children.is_empty() {
            write!(output, "<ul class=\"patto-children\">")?;
            for child in children.iter() {
                self.write_list_item(child, "patto-item", output)?;
            }
            write!(output, "</ul>")?;
        }
        Ok(())
    }

    fn write_quote(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        write!(output, "<blockquote class=\"patto-quote\">")?;
        for child in ast.children().iter() {
            self.write_quote_content(child, output, 0)?;
        }
        write!(output, "</blockquote>")
    }

    /// Quote content is indented with a margin rather than nested blockquotes,
    /// so that one quote renders as one block.
    fn write_quote_content(
        &self,
        quote_content: &AstNode,
        output: &mut dyn Write,
        indent_level: usize,
    ) -> io::Result<()> {
        {
            let contents = quote_content.contents();
            let is_nested_quote =
                contents.len() == 1 && matches!(contents[0].kind(), AstNodeKind::Quote);

            if is_nested_quote {
                for content in contents.iter() {
                    self.write_node(content, output)?;
                }
            } else {
                self.open_indent(output, indent_level)?;
                for content in contents.iter() {
                    self.write_node(content, output)?;
                }
                write!(output, "</div>")?;
            }
        }

        for child in quote_content.children().iter() {
            if let AstNodeKind::QuoteContent { .. } = child.kind() {
                self.write_quote_content(child, output, indent_level + 1)?;
            } else if indent_level > 0 {
                self.open_indent(output, indent_level)?;
                self.write_node(child, output)?;
                write!(output, "</div>")?;
            } else {
                self.write_node(child, output)?;
            }
        }
        Ok(())
    }

    fn open_indent(&self, output: &mut dyn Write, indent_level: usize) -> io::Result<()> {
        if indent_level > 0 {
            write!(
                output,
                "<div style=\"margin-left: {}em\">",
                indent_level * 2
            )
        } else {
            write!(output, "<div>")
        }
    }

    fn write_math(&self, ast: &AstNode, inline: bool, output: &mut dyn Write) -> io::Result<()> {
        if inline {
            write!(output, "<span class=\"patto-math-inline\">\\(")?;
            write!(output, "{}", ast.contents()[0].extract_str())?;
            return write!(output, "\\)</span>");
        }

        write!(output, "<div class=\"patto-math-block\">")?;
        // see https://github.com/mathjax/MathJax/issues/2312
        write!(output, "\\[\\displaylines{{")?;
        for child in ast.children().iter() {
            write!(output, "{}", child.extract_str())?;
        }
        write!(output, "}}\\]")?;
        write!(output, "</div>")
    }

    fn write_code(
        &self,
        ast: &AstNode,
        lang: &str,
        inline: bool,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        if inline {
            write!(output, "<code class=\"patto-inline-code\">")?;
            write!(output, "{}", encode_text(ast.contents()[0].extract_str()))?;
            return write!(output, "</code>");
        }

        // mermaid ships its own renderer, which reads the source unescaped.
        if lang == "mermaid" {
            write!(output, "<pre class=\"mermaid\">")?;
            for child in ast.children().iter() {
                writeln!(output, "{}", child.extract_str())?;
            }
            return write!(output, "</pre>");
        }

        write!(
            output,
            "<pre class=\"hljs patto-code-block\"><code class=\"language-{}\">",
            lang
        )?;
        for child in ast.children().iter() {
            writeln!(output, "{}", encode_text(child.extract_str()))?;
        }
        write!(output, "</code></pre>")
    }

    fn write_image(&self, src: &str, alt: Option<&str>, output: &mut dyn Write) -> io::Result<()> {
        let src = get_gyazo_img_src(src).unwrap_or_else(|| src.to_string());

        write!(output, "<figure class=\"patto-figure\">")?;
        // The exported page has no lightbox, so images are capped in CSS
        // and the anchor is how a reader gets to the full-size original.
        write!(
            output,
            "<a class=\"patto-image-link\" href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
            src
        )?;
        match alt {
            Some(alt) => {
                write!(
                    output,
                    "<img class=\"patto-image\" alt=\"{}\" src=\"{}\"/>",
                    alt, src
                )?;
                write!(output, "</a>")?;
                write!(output, "<figcaption>{}</figcaption>", encode_text(alt))?;
            }
            None => {
                write!(output, "<img class=\"patto-image\" src=\"{}\"/>", src)?;
                write!(output, "</a>")?;
            }
        }
        write!(output, "</figure>")
    }

    fn write_wikilink(
        &self,
        link: &str,
        anchor: Option<&str>,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        match anchor {
            // TODO eliminate the logic that self-link if link is empty
            Some(anchor) if link.is_empty() => write!(
                output,
                "<a class=\"patto-selflink\" href=\"#{}\">#{}</a>",
                anchor, anchor
            ),
            Some(anchor) => write!(
                output,
                "<a class=\"patto-wikilink\" href=\"{}.pn#{}\">{}#{}</a>",
                link, anchor, link, anchor
            ),
            None => write!(
                output,
                "<a class=\"patto-wikilink\" href=\"{}.pn\">{}</a>",
                link, link
            ),
        }
    }

    fn write_link(
        &self,
        link: &str,
        title: Option<&str>,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        write!(
            output,
            "<a class=\"patto-link\" href=\"{}\">{}</a>",
            link,
            title.unwrap_or(link)
        )
    }

    /// Embeds the page cannot render server-side become placeholders carrying
    /// the source URL; the preview's client-side code fills them in.
    fn write_embed(
        &self,
        link: &str,
        title: Option<&str>,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        let label = title.unwrap_or(link);

        if link.to_lowercase().ends_with(".pdf") {
            let is_local = !link.contains("://");
            let (data_attr, href) = if is_local {
                (
                    format!("data-src=\"{}\"", link),
                    format!("/api/files/{}", link),
                )
            } else {
                (format!("data-url=\"{}\"", link), link.to_string())
            };
            return write!(
                output,
                "<div class=\"patto-embed-pdf\" {data_attr}><a href=\"{href}\">{label}</a></div>"
            );
        }

        if let Some(youtube_id) = get_youtube_id(link) {
            return write!(
                output,
                "<div class=\"patto-embed-youtube\"><iframe src=\"http://www.youtube.com/embed/{youtube_id}?modestbranding=1&autoplay=0&controls=1&fs=1&loop=0&rel=0&showinfo=0&disablekb=0\" frameborder=\"0\" allow=\"accelerometer; autoplay; encrypted-media; gyroscope; picture-in-picture; fullscreen\" allowfullscreen></iframe></div>");
        }

        let placeholder = if link.contains("twitter.com") || link.contains("x.com") {
            Some("twitter-placeholder")
        } else if link.contains("speakerdeck.com") {
            Some("speakerdeck-placeholder")
        } else if link.contains("slideshare.net") {
            Some("slideshare-placeholder")
        } else {
            None
        };

        match placeholder {
            Some(class) => write!(
                output,
                "<div class=\"{class}\" data-url=\"{link}\"><a href=\"{link}\">{label}</a></div>"
            ),
            None => self.write_link(link, title, output),
        }
    }

    fn write_decoration(
        &self,
        ast: &AstNode,
        fontsize: isize,
        italic: bool,
        underline: bool,
        deleted: bool,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        let font_pct = 100isize + (fontsize - 1).max(0) * 20;
        let font_weight = if fontsize > 0 {
            " font-weight: bold;"
        } else {
            ""
        };

        let mut class = String::new();
        if italic {
            class.push_str(" italic");
        }
        if underline {
            class.push_str(" underline");
        }
        if deleted {
            class.push_str(" patto-deleted");
        }

        write!(
            output,
            "<span class=\"{}\" style=\"font-size: {font_pct}%;{font_weight}\">",
            class.trim()
        )?;
        for content in ast.contents().iter() {
            self.write_node(content, output)?;
        }
        write!(output, "</span>")
    }

    fn write_table(
        &self,
        ast: &AstNode,
        caption: Option<&str>,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        write!(output, "<div class=\"patto-table-wrapper\">")?;
        if let Some(caption) = caption {
            write!(
                output,
                "<p class=\"patto-table-caption\">{}</p>",
                encode_text(caption)
            )?;
        }
        write!(output, "<table class=\"patto-table\">")?;
        write!(output, "<tbody>")?;
        for child in ast.children().iter() {
            self.write_node(child, output)?;
        }
        write!(output, "</tbody></table></div>")
    }

    fn write_cells(&self, ast: &AstNode, tag: &str, output: &mut dyn Write) -> io::Result<()> {
        write!(output, "<{}>", tag)?;
        for content in ast.contents().iter() {
            self.write_node(content, output)?;
        }
        write!(output, "</{}>", tag)
    }
}
