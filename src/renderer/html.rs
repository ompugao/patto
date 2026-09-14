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
        self._format_impl(ast, output)?;
        Ok(())
    }
}

impl HtmlRenderer {
    pub fn new(options: HtmlRendererOptions) -> Self {
        Self { options }
    }

    fn get_stable_id_attr(&self, ast: &AstNode) -> String {
        if let Some(stable_id) = ast.stable_id() {
            format!(" data-line-id=\"{}\"", stable_id)
        } else {
            String::new()
        }
    }

    fn _format_impl(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        match &ast.kind() {
            AstNodeKind::Dummy => {
                write!(output, "<ul class=\"patto-document\">")?;
                let children = ast.children();
                for child in children.iter() {
                    let id_attr = self.get_stable_id_attr(child);
                    write!(output, "<li class=\"patto-line\"{}>", id_attr)?;
                    self._format_impl(child, output)?;
                    write!(output, "</li>")?;
                }
                write!(output, "</ul>")?;
            }
            AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } => {
                let is_quote = matches!(ast.kind(), AstNodeKind::QuoteContent { .. });
                let mut task_status: Option<&TaskStatus> = None;
                for property in properties {
                    if let Property::Task { status, .. } = property {
                        task_status = Some(status);
                    }
                }
                let isdone = matches!(task_status, Some(TaskStatus::Done));

                write!(output, "<div class=\"patto-task-row\">")?;
                if let Some(status) = task_status {
                    let (icon, cls) = match status {
                        TaskStatus::Done => ("✓", "patto-task-icon-done"),
                        TaskStatus::Doing => ("◑", "patto-task-icon-doing"),
                        _ => ("○", "patto-task-icon-todo"),
                    };
                    write!(
                        output,
                        "<span class=\"patto-task-icon {}\">{}</span>",
                        cls, icon
                    )?;
                }

                let done_cls = if isdone { " patto-task-done-text" } else { "" };
                let quote_cls = if is_quote { " text-quote" } else { "" };
                write!(
                    output,
                    "<div class=\"patto-task-content{}{}\">",
                    done_cls, quote_cls
                )?;
                let contents = ast.contents();
                for content in contents.iter() {
                    self._format_impl(content, output)?;
                }
                write!(output, "</div>")?;

                // Deadline chip
                for property in properties {
                    match property {
                        Property::Anchor { name, .. } => {
                            write!(
                                output,
                                "<span id=\"{}\" class=\"anchor\">{}</span>",
                                name, name
                            )?;
                        }
                        Property::Task { status, due, .. } => {
                            if !matches!(status, TaskStatus::Done) {
                                write!(
                                    output,
                                    "<span class=\"patto-deadline patto-deadline-default\">{}</span>",
                                    due
                                )?;
                            }
                        }
                    }
                }
                write!(output, "</div>")?; // close patto-task-row

                let children = ast.children();
                if !children.is_empty() {
                    write!(output, "<ul class=\"patto-children\">")?;
                    for child in children.iter() {
                        let id_attr = self.get_stable_id_attr(child);
                        write!(output, "<li class=\"patto-item\"{}>", id_attr)?;
                        self._format_impl(child, output)?;
                        write!(output, "</li>")?;
                    }
                    write!(output, "</ul>")?;
                }
            }
            AstNodeKind::Quote => {
                write!(output, "<blockquote class=\"patto-quote\">")?;
                let children = ast.children();
                for child in children.iter() {
                    self.render_quote_content_html(child, output, 0)?;
                }
                write!(output, "</blockquote>")?;
            }
            AstNodeKind::Math { inline } => {
                if *inline {
                    write!(output, "<span class=\"patto-math-inline\">\\(")?;
                    let contents = ast.contents();
                    write!(output, "{}", contents[0].extract_str())?;
                    write!(output, "\\)</span>")?;
                } else {
                    write!(output, "<div class=\"patto-math-block\">")?;
                    // see https://github.com/mathjax/MathJax/issues/2312
                    write!(output, "\\[\\displaylines{{")?;
                    let children = ast.children();
                    for child in children.iter() {
                        write!(output, "{}", child.extract_str())?;
                    }
                    write!(output, "}}\\]")?;
                    write!(output, "</div>")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
                if *inline {
                    write!(output, "<code class=\"patto-inline-code\">")?;
                    let contents = ast.contents();
                    write!(output, "{}", encode_text(contents[0].extract_str()))?;
                    write!(output, "</code>")?;
                } else {
                    if lang == "mermaid" {
                        write!(output, "<pre class=\"mermaid\">")?;
                        let children = ast.children();
                        for child in children.iter() {
                            writeln!(output, "{}", child.extract_str())?;
                        }
                        write!(output, "</pre>")?;
                    } else {
                        write!(
                            output,
                            "<pre class=\"hljs patto-code-block\"><code class=\"language-{}\">",
                            lang
                        )?;
                        let children = ast.children();
                        for child in children.iter() {
                            writeln!(output, "{}", encode_text(child.extract_str()))?;
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
                write!(output, "<figure class=\"patto-figure\">")?;
                // The exported page has no lightbox, so images are capped in CSS
                // and the anchor is how a reader gets to the full-size original.
                write!(
                    output,
                    "<a class=\"patto-image-link\" href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
                    src_exported
                )?;
                if let Some(alt) = alt {
                    write!(
                        output,
                        "<img class=\"patto-image\" alt=\"{}\" src=\"{}\"/>",
                        alt, src_exported
                    )?;
                    write!(output, "</a>")?;
                    write!(output, "<figcaption>{}</figcaption>", encode_text(alt))?;
                } else {
                    write!(
                        output,
                        "<img class=\"patto-image\" src=\"{}\"/>",
                        src_exported
                    )?;
                    write!(output, "</a>")?;
                }
                write!(output, "</figure>")?;
            }
            AstNodeKind::WikiLink { link, anchor } => {
                if let Some(anchor) = anchor {
                    // TODO eliminate the logic that self-link if link is empty
                    if link.is_empty() {
                        write!(
                            output,
                            "<a class=\"patto-selflink\" href=\"#{}\">#{}</a>",
                            anchor, anchor
                        )?;
                    } else {
                        write!(
                            output,
                            "<a class=\"patto-wikilink\" href=\"{}.pn#{}\">{}#{}</a>",
                            link, anchor, link, anchor
                        )?;
                    }
                } else {
                    write!(
                        output,
                        "<a class=\"patto-wikilink\" href=\"{}.pn\">{}</a>",
                        link, link
                    )?;
                }
            }
            AstNodeKind::Link { link, title } => {
                if let Some(title) = title {
                    write!(
                        output,
                        "<a class=\"patto-link\" href=\"{}\">{}</a>",
                        link, title
                    )?;
                } else {
                    write!(
                        output,
                        "<a class=\"patto-link\" href=\"{}\">{}</a>",
                        link, link
                    )?;
                }
            }
            AstNodeKind::Embed { link, title } => {
                let is_pdf = link.to_lowercase().ends_with(".pdf");
                if is_pdf {
                    let is_local = !link.contains("://");
                    let (data_attr, href) = if is_local {
                        (
                            format!("data-src=\"{}\"", link),
                            format!("/api/files/{}", link),
                        )
                    } else {
                        (format!("data-url=\"{}\"", link), link.to_string())
                    };
                    write!(
                        output,
                        "<div class=\"patto-embed-pdf\" {data_attr}><a href=\"{href}\">{}</a></div>",
                        title.as_deref().unwrap_or(link)
                    )?;
                } else if let Some(youtube_id) = get_youtube_id(link) {
                    write!(
                        output,
                        "<div class=\"patto-embed-youtube\"><iframe src=\"http://www.youtube.com/embed/{youtube_id}?modestbranding=1&autoplay=0&controls=1&fs=1&loop=0&rel=0&showinfo=0&disablekb=0\" frameborder=\"0\" allow=\"accelerometer; autoplay; encrypted-media; gyroscope; picture-in-picture; fullscreen\" allowfullscreen></iframe></div>")?;
                } else if link.contains("twitter.com") || link.contains("x.com") {
                    // Render as placeholder that can be enhanced client-side
                    write!(
                        output,
                        "<div class=\"twitter-placeholder\" data-url=\"{}\"><a href=\"{}\">{}</a></div>",
                        link,
                        link,
                        title.as_deref().unwrap_or(link)
                    )?;
                } else if link.contains("speakerdeck.com") {
                    // Render as placeholder that can be enhanced client-side
                    write!(
                        output,
                        "<div class=\"speakerdeck-placeholder\" data-url=\"{}\"><a href=\"{}\">{}</a></div>",
                        link,
                        link,
                        title.as_deref().unwrap_or(link)
                    )?;
                } else if link.contains("slideshare.net") {
                    write!(
                        output,
                        "<div class=\"slideshare-placeholder\" data-url=\"{}\"><a href=\"{}\">{}</a></div>",
                        link,
                        link,
                        title.as_deref().unwrap_or(link)
                    )?;
                } else if let Some(title) = title {
                    write!(
                        output,
                        "<a class=\"patto-link\" href=\"{}\">{}</a>",
                        link, title
                    )?;
                } else {
                    write!(
                        output,
                        "<a class=\"patto-link\" href=\"{}\">{}</a>",
                        link, link
                    )?;
                }
            }
            AstNodeKind::Decoration {
                fontsize,
                italic,
                underline,
                deleted,
            } => {
                let font_pct = 100isize + (fontsize - 1).max(0) * 20;
                let fontweight = if *fontsize > 0 {
                    " font-weight: bold;"
                } else {
                    ""
                };
                let mut cls = String::new();
                if *italic {
                    cls.push_str(" italic");
                }
                if *underline {
                    cls.push_str(" underline");
                }
                if *deleted {
                    cls.push_str(" patto-deleted");
                }
                write!(
                    output,
                    "<span class=\"{}\" style=\"font-size: {font_pct}%;{fontweight}\">",
                    cls.trim()
                )?;
                let contents = ast.contents();
                for content in contents.iter() {
                    self._format_impl(content, output)?;
                }
                write!(output, "</span>")?;
            }
            AstNodeKind::Text | AstNodeKind::CodeContent | AstNodeKind::MathContent => {
                write!(output, "{}", ast.extract_str())?;
            }
            AstNodeKind::HorizontalLine => {
                write!(output, "<hr class=\"patto-hr\"/>")?;
            }
            AstNodeKind::Table { caption } => {
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
                let children = ast.children();
                for child in children.iter() {
                    self._format_impl(child, output)?;
                }
                write!(output, "</tbody></table></div>")?;
            }
            AstNodeKind::TableRow => {
                write!(output, "<tr>")?;
                let contents = ast.contents();
                for content in contents.iter() {
                    self._format_impl(content, output)?;
                }
                write!(output, "</tr>")?;
            }
            AstNodeKind::TableColumn => {
                write!(output, "<td>")?;
                let contents = ast.contents();
                for content in contents.iter() {
                    self._format_impl(content, output)?;
                }
                write!(output, "</td>")?;
            }
        }
        Ok(())
    }

    /// Render quote content with visual indentation using margin-left
    fn render_quote_content_html(
        &self,
        quote_content: &AstNode,
        output: &mut dyn Write,
        indent_level: usize,
    ) -> io::Result<()> {
        // Check if this contains a nested Quote block
        let contents = quote_content.contents();
        let has_nested_quote =
            contents.len() == 1 && matches!(contents[0].kind(), AstNodeKind::Quote);

        if has_nested_quote {
            // Render the nested quote as a nested blockquote
            for content in contents.iter() {
                if let AstNodeKind::Quote = content.kind() {
                    self._format_impl(content, output)?;
                } else {
                    self._format_impl(content, output)?;
                }
            }
            drop(contents);
        } else {
            // Render with indentation if needed
            if indent_level > 0 {
                write!(
                    output,
                    "<div style=\"margin-left: {}em\">",
                    indent_level * 2
                )?;
            } else {
                write!(output, "<div>")?;
            }

            for content in contents.iter() {
                self._format_impl(content, output)?;
            }
            drop(contents);

            write!(output, "</div>")?;
        }

        // Render children (nested QuoteContent) with increased indent
        let children = quote_content.children();
        for child in children.iter() {
            if let AstNodeKind::QuoteContent { .. } = child.kind() {
                self.render_quote_content_html(child, output, indent_level + 1)?;
            } else if indent_level > 0 {
                write!(
                    output,
                    "<div style=\"margin-left: {}em\">",
                    indent_level * 2
                )?;
                self._format_impl(child, output)?;
                write!(output, "</div>")?;
            } else {
                self._format_impl(child, output)?;
            }
        }

        Ok(())
    }
}
