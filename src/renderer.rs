use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind};
use crate::parser::{Property, TaskStatus};
use crate::utils::{get_gyazo_img_src, get_twitter_embed, get_youtube_id};
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
                    write!(
                        output,
                        "<li class=\"patto-line\" style=\"list-style-type: none; min-height: 1em;\"{}>",
                        id_attr
                    )?;
                    self._format_impl(child, output)?;
                    write!(output, "</li>")?;
                }
                write!(output, "</ul>")?;
            }
            AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } => {
                let mut isdone = false;
                for property in properties {
                    if let Property::Task { status, .. } = property {
                        match status {
                            TaskStatus::Done => {
                                isdone = true;
                                write!(output, "<input type=\"checkbox\" checked disabled/>")?
                            }
                            _ => write!(output, "<input type=\"checkbox\" unchecked disabled/>")?,
                        }
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
                    write!(
                        output,
                        "<aside style=\"float: right; width: 285px; text-align: right\">"
                    )?;
                    for property in properties {
                        match property {
                            Property::Anchor { name, .. } => {
                                write!(
                                    output,
                                    "<span id=\"{}\" class=\"anchor\">{}</span>",
                                    name, name
                                )?;
                            }
                            Property::Task { status, due, .. } => match status {
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
                        write!(
                            output,
                            "<li class=\"patto-item\" style=\"min-height: 1em;\"{}>",
                            id_attr
                        )?;
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
                    self.render_quote_content_html(child, output, 0)?;
                }
                write!(output, "</blockquote>")?;
            }
            AstNodeKind::Math { inline } => {
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
                            writeln!(output, "{}", encode_text(child.extract_str()))?;
                            // TODO encode all at once?
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
                    write!(
                        output,
                        "<img class=\"patto-image\" alt=\"{}\" src=\"{}\"/>",
                        alt, src_exported
                    )?;
                } else {
                    write!(
                        output,
                        "<img class=\"patto-image\" src=\"{}\"/>",
                        src_exported
                    )?;
                }
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
                    write!(output, "<a href=\"{}\">{}</a>", link, title)?;
                } else {
                    write!(output, "<a href=\"{}\">{}</a>", link, link)?;
                }
            }
            AstNodeKind::Embed { link, title } => {
                if let Some(youtube_id) = get_youtube_id(link) {
                    write!(
                        output,
                        "<div style=\"max-width: 90%; height: 30vw;\"><iframe class=\"videoContainer__video\" width=100% height=100% src=\"http://www.youtube.com/embed/{youtube_id}?modestbranding=1&autoplay=0&controls=1&fs=1&loop=0&rel=0&showinfo=0&disablekb=0\" frameborder=\"0\" allow=\"accelerometer; autoplay; encrypted-media; gyroscope; picture-in-picture; fullscreen\" allowfullscreen></iframe></div>")?;
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
                } else if let Some(title) = title {
                    write!(output, "<a href=\"{}\">{}</a>", link, title)?;
                } else {
                    write!(output, "<a href=\"{}\">{}</a>", link, link)?;
                }
            }
            AstNodeKind::Decoration {
                fontsize,
                italic,
                underline,
                deleted,
            } => {
                let font_pct = 100isize + fontsize * 20;
                let fontweight = if *fontsize > 0 {
                    " font-weight: bold;"
                } else {
                    ""
                };
                write!(output, "<span style=\"font-size: {font_pct}%;{fontweight}\">")?;
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
            AstNodeKind::Text | AstNodeKind::CodeContent | AstNodeKind::MathContent => {
                write!(output, "{}", ast.extract_str())?;
            }
            AstNodeKind::HorizontalLine => {
                write!(output, "<hr/>")?;
            }
            AstNodeKind::Table { caption } => {
                write!(output, "<table>")?;
                if let Some(caption) = caption {
                    write!(output, "<caption>{}</caption>", encode_text(caption))?;
                }
                write!(output, "<tbody>")?;
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    self._format_impl(child, output)?;
                }
                write!(output, "</tbody></table>")?;
            }
            AstNodeKind::TableRow => {
                write!(output, "<tr>")?;
                let contents = ast.value().contents.lock().unwrap();
                for content in contents.iter() {
                    self._format_impl(content, output)?;
                }
                write!(output, "</tr>")?;
            }
            AstNodeKind::TableColumn => {
                write!(output, "<td>")?;
                let contents = ast.value().contents.lock().unwrap();
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
        let contents = quote_content.value().contents.lock().unwrap();
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
        let children = quote_content.value().children.lock().unwrap();
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

use crate::markdown::{AnchorFormat, MarkdownRendererOptions, TaskFormat, WikiLinkFormat};

pub struct MarkdownRenderer {
    options: MarkdownRendererOptions,
}

impl Renderer for MarkdownRenderer {
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        // Add frontmatter if enabled
        if self.options.include_frontmatter() {
            writeln!(output, "---")?;
            writeln!(output, "patto_source: true")?;
            writeln!(output, "flavor: {}", self.options.flavor)?;
            writeln!(output, "---")?;
            writeln!(output)?;
        }

        let depth: usize = 0;
        self._format_impl(ast, output, depth, false)?;
        Ok(())
    }
}

impl MarkdownRenderer {
    pub fn new(options: MarkdownRendererOptions) -> Self {
        Self { options }
    }

    /// Format a range of lines from the AST to markdown
    /// start_line and end_line are 0-indexed, inclusive
    pub fn format_range(
        &self,
        ast: &AstNode,
        output: &mut dyn Write,
        start_line: usize,
        end_line: usize,
    ) -> io::Result<()> {
        let depth: usize = 0;
        self._format_range_impl(ast, output, depth, false, start_line, end_line)?;
        Ok(())
    }

    fn _format_range_impl(
        &self,
        ast: &AstNode,
        output: &mut dyn Write,
        depth: usize,
        in_quote: bool,
        start_line: usize,
        end_line: usize,
    ) -> io::Result<()> {
        match &ast.kind() {
            AstNodeKind::Dummy => {
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    let child_row = child.location().row;
                    // Check if this child or any of its descendants are in range
                    if child_row <= end_line {
                        self._format_range_impl(
                            child, output, depth, in_quote, start_line, end_line,
                        )?;
                    }
                }
            }
            AstNodeKind::Line { .. } | AstNodeKind::QuoteContent { .. } => {
                let row = ast.location().row;
                if row >= start_line && row <= end_line {
                    // This line is in range, render it normally
                    self._format_impl(ast, output, depth, in_quote)?;
                } else if row < start_line {
                    // This line is before range, but check children
                    let children = ast.value().children.lock().unwrap();
                    for child in children.iter() {
                        let child_row = child.location().row;
                        if child_row >= start_line && child_row <= end_line {
                            self._format_range_impl(
                                child, output, depth, in_quote, start_line, end_line,
                            )?;
                        } else if child_row < start_line {
                            // Recurse to check deeper children
                            self._format_range_impl(
                                child, output, depth, in_quote, start_line, end_line,
                            )?;
                        }
                    }
                }
                // If row > end_line, skip entirely
            }
            _ => {
                // For other node types, delegate to regular format
                self._format_impl(ast, output, depth, in_quote)?;
            }
        }
        Ok(())
    }

    fn _format_impl(
        &self,
        ast: &AstNode,
        output: &mut dyn Write,
        depth: usize,
        in_quote: bool,
    ) -> io::Result<()> {
        match &ast.kind() {
            AstNodeKind::Dummy => {
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    self._format_impl(child, output, depth, in_quote)?;
                }
            }
            AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } => {
                let has_children = !ast.value().children.lock().unwrap().is_empty();
                let is_quote_content = matches!(ast.kind(), AstNodeKind::QuoteContent { .. });

                // Check if this line only contains a block element (quote, code, math, table)
                let contents = ast.value().contents.lock().unwrap();
                let is_block_container = contents.len() == 1
                    && matches!(
                        contents[0].kind(),
                        AstNodeKind::Quote
                            | AstNodeKind::Code { inline: false, .. }
                            | AstNodeKind::Math { inline: false }
                            | AstNodeKind::Table { .. }
                    );

                // Check if this is an empty line (no contents, no properties, no children)
                let is_empty = contents.is_empty() && properties.is_empty() && !has_children;
                drop(contents);

                // For empty lines, just output a blank line
                if is_empty {
                    writeln!(output)?;
                    return Ok(());
                }

                // Indentation for nested items (skip for quote content - handled by Quote)
                if !in_quote && !is_block_container {
                    for _ in 0..depth {
                        write!(output, "  ")?;
                    }
                }

                // Determine if this is a task
                let mut task_due: Option<&crate::parser::Deadline> = None;
                let mut is_done = false;
                for property in properties {
                    if let Property::Task { status, due, .. } = property {
                        task_due = Some(due);
                        is_done = matches!(status, TaskStatus::Done);
                        break;
                    }
                }

                // List marker for nested items or items with children (not for quote content or block containers)
                if !is_quote_content && !is_block_container && (depth > 0 || has_children) {
                    write!(output, "- ")?;
                }

                // Task checkbox
                if task_due.is_some() {
                    if is_done {
                        write!(output, "[x] ")?;
                    } else {
                        write!(output, "[ ] ")?;
                    }
                }

                // Render contents
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(content, output, depth, in_quote)?;
                }

                // Append due date (only if not done and has non-empty due date)
                if let Some(due) = task_due {
                    if !is_done {
                        let due_str = due.to_string();
                        if !due_str.is_empty() {
                            match self.options.task_format() {
                                TaskFormat::Checkbox => write!(output, " (due: {})", due_str)?,
                                TaskFormat::ObsidianEmoji => write!(output, " 📅 {}", due_str)?,
                                TaskFormat::ObsidianDataview => {
                                    write!(output, " [due:: {}]", due_str)?
                                }
                            }
                        }
                    }
                }

                // Append anchors
                for property in properties {
                    if let Property::Anchor { name, .. } = property {
                        match self.options.anchor_format() {
                            AnchorFormat::HtmlAnchor => write!(output, " <a id=\"{}\"></a>", name)?,
                            AnchorFormat::HtmlComment => {
                                write!(output, " <!-- anchor: {} -->", name)?
                            }
                            AnchorFormat::ObsidianBlock => write!(output, " ^{}", name)?,
                            AnchorFormat::Inline => write!(output, " #{}", name)?,
                        }
                    }
                }

                // Block containers handle their own newlines
                if !is_block_container {
                    writeln!(output)?;
                }

                // Render children
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    self._format_impl(child, output, depth + 1, in_quote)?;
                }
            }
            AstNodeKind::Quote => {
                // Render quote children with a helper to track inner indentation
                self.render_quote_children(ast, output, depth, 0)?;
            }
            AstNodeKind::Math { inline } => {
                if *inline {
                    write!(output, "$")?;
                    let contents = ast.value().contents.lock().unwrap();
                    if !contents.is_empty() {
                        write!(output, "{}", contents[0].extract_str())?;
                    }
                    write!(output, "$")?;
                } else {
                    writeln!(output, "$$")?;
                    let children = ast.value().children.lock().unwrap();
                    for child in children.iter() {
                        writeln!(output, "{}", child.extract_str())?;
                    }
                    writeln!(output, "$$")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
                if *inline {
                    write!(output, "`")?;
                    let contents = ast.value().contents.lock().unwrap();
                    if !contents.is_empty() {
                        write!(output, "{}", contents[0].extract_str())?;
                    }
                    write!(output, "`")?;
                } else {
                    // Proper fenced code block (NOT nested in list)
                    writeln!(output, "```{}", lang)?;
                    let children = ast.value().children.lock().unwrap();
                    for child in children.iter() {
                        writeln!(output, "{}", child.extract_str())?;
                    }
                    writeln!(output, "```")?;
                }
            }
            AstNodeKind::Image { src, alt } => {
                if let Some(alt) = alt {
                    write!(output, "![{}]({})", alt, src)?;
                } else {
                    write!(output, "![]({})", src)?;
                }
            }
            AstNodeKind::WikiLink { link, anchor } => {
                match self.options.wiki_link_format() {
                    WikiLinkFormat::WikiStyle => {
                        if let Some(anchor) = anchor {
                            if link.is_empty() {
                                // Self-link to anchor
                                write!(output, "[[#{}]]", anchor)?;
                            } else {
                                write!(output, "[[{}#{}]]", link, anchor)?;
                            }
                        } else {
                            write!(output, "[[{}]]", link)?;
                        }
                    }
                    WikiLinkFormat::Markdown => {
                        let ext = self.options.file_extension();
                        if let Some(anchor) = anchor {
                            if link.is_empty() {
                                // Self-link to anchor
                                write!(output, "[#{}](#{})", anchor, anchor)?;
                            } else {
                                write!(
                                    output,
                                    "[{}#{}]({}{}#{})",
                                    link, anchor, link, ext, anchor
                                )?;
                            }
                        } else {
                            write!(output, "[{}]({}{})", link, link, ext)?;
                        }
                    }
                }
            }
            AstNodeKind::Link { link, title } => {
                if let Some(title) = title {
                    write!(output, "[{}]({})", title, link)?;
                } else {
                    write!(output, "[{}]({})", link, link)?;
                }
            }
            AstNodeKind::Embed { link, title } => {
                if let Some(youtube_id) = get_youtube_id(link) {
                    // YouTube embed as link (markdown doesn't support iframe)
                    write!(
                        output,
                        "[![YouTube](https://img.youtube.com/vi/{}/0.jpg)](https://www.youtube.com/watch?v={})",
                        youtube_id, youtube_id
                    )?;
                } else if let Some(embed) = get_twitter_embed(link) {
                    write!(output, "{}", embed)?;
                } else if let Some(title) = title {
                    write!(output, "[{}]({})", title, link)?;
                } else {
                    write!(output, "[{}]({})", link, link)?;
                }
            }
            AstNodeKind::Decoration {
                fontsize,
                italic,
                underline,
                deleted,
            } => {
                // Open tags
                if *fontsize > 0 && !*italic {
                    write!(output, "**")?; // bold
                } else if *italic && *fontsize <= 0 {
                    write!(output, "*")?; // italic
                } else if *italic && *fontsize > 0 {
                    write!(output, "***")?; // bold italic
                }
                if *underline {
                    write!(output, "<ins>")?;
                }
                if *deleted {
                    write!(output, "~~")?;
                }

                // Content
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(content, output, depth, in_quote)?;
                }

                // Close tags (reverse order)
                if *deleted {
                    write!(output, "~~")?;
                }
                if *underline {
                    write!(output, "</ins>")?;
                }
                if *fontsize > 0 && !*italic {
                    write!(output, "**")?;
                } else if *italic && *fontsize <= 0 {
                    write!(output, "*")?;
                } else if *italic && *fontsize > 0 {
                    write!(output, "***")?;
                }
            }
            AstNodeKind::Text | AstNodeKind::CodeContent | AstNodeKind::MathContent => {
                write!(output, "{}", ast.extract_str())?;
            }
            AstNodeKind::HorizontalLine => {
                write!(output, "---")?;
            }
            AstNodeKind::Table { caption } => {
                // Caption as emphasized text
                if let Some(caption) = caption {
                    writeln!(output, "*{}*", caption)?;
                }

                let children = ast.value().children.lock().unwrap();
                for (i, child) in children.iter().enumerate() {
                    self._format_impl(child, output, depth, in_quote)?;

                    // Add header separator after first row
                    if i == 0 {
                        let col_count = child.value().contents.lock().unwrap().len();
                        write!(output, "|")?;
                        for _ in 0..col_count {
                            write!(output, " --- |")?;
                        }
                        writeln!(output)?;
                    }
                }
            }
            AstNodeKind::TableRow => {
                write!(output, "|")?;
                let contents = ast.value().contents.lock().unwrap();
                for content in contents.iter() {
                    write!(output, " ")?;
                    self._format_impl(content, output, depth, in_quote)?;
                    write!(output, " |")?;
                }
                writeln!(output)?;
            }
            AstNodeKind::TableColumn => {
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(content, output, depth, in_quote)?;
                }
            }
        }
        Ok(())
    }

    /// Helper to render quote children with nested indentation levels
    /// `inner_depth` tracks nesting level inside the quote for visual indentation
    fn render_quote_children(
        &self,
        quote: &AstNode,
        output: &mut dyn Write,
        depth: usize,
        inner_depth: usize,
    ) -> io::Result<()> {
        let children = quote.value().children.lock().unwrap();
        for child in children.iter() {
            match child.kind() {
                AstNodeKind::QuoteContent { .. } => {
                    self.render_quote_content(child, output, depth, inner_depth)?;
                }
                _ => {
                    // Other children (shouldn't happen normally but handle gracefully)
                    for _ in 0..depth {
                        write!(output, "  ")?;
                    }
                    write!(output, "> ")?;
                    self._format_impl(child, output, depth, true)?;
                }
            }
        }
        Ok(())
    }

    /// Render a QuoteContent node with proper visual indentation
    fn render_quote_content(
        &self,
        quote_content: &AstNode,
        output: &mut dyn Write,
        depth: usize,
        inner_depth: usize,
    ) -> io::Result<()> {
        // Output the "> " prefix with outer depth indentation
        for _ in 0..depth {
            write!(output, "  ")?;
        }
        write!(output, "> ")?;

        // Add visual indentation for inner depth (spaces after ">")
        for _ in 0..inner_depth {
            write!(output, "    ")?; // 4 spaces per indent level
        }

        // Check if this is a nested Quote block
        let contents = quote_content.value().contents.lock().unwrap();
        let has_nested_quote =
            contents.len() == 1 && matches!(contents[0].kind(), AstNodeKind::Quote);

        if has_nested_quote {
            // For nested quotes, we need to output with extra "> " markers
            drop(contents);
            let contents = quote_content.value().contents.lock().unwrap();
            for content in contents.iter() {
                if let AstNodeKind::Quote = content.kind() {
                    writeln!(output)?; // End the current line
                                       // Render nested quote with extra "> " marker
                    self.render_nested_quote(content, output, depth, inner_depth + 1)?;
                } else {
                    self._format_impl(content, output, depth, true)?;
                }
            }
        } else {
            // Regular content
            for content in contents.iter() {
                self._format_impl(content, output, depth, true)?;
            }
            drop(contents);

            // End the line
            let properties = if let AstNodeKind::QuoteContent { properties } = quote_content.kind()
            {
                properties
            } else {
                &vec![]
            };

            if !properties.is_empty() {
                // We already output newline in _format_impl for lines with properties
            }
            writeln!(output)?;
        }

        // Render children (nested QuoteContent) with increased inner_depth
        let children = quote_content.value().children.lock().unwrap();
        for child in children.iter() {
            if let AstNodeKind::QuoteContent { .. } = child.kind() {
                self.render_quote_content(child, output, depth, inner_depth + 1)?;
            } else {
                // Other children
                for _ in 0..depth {
                    write!(output, "  ")?;
                }
                write!(output, "> ")?;
                for _ in 0..inner_depth {
                    write!(output, "    ")?;
                }
                self._format_impl(child, output, depth, true)?;
            }
        }

        Ok(())
    }

    /// Render a nested Quote block using nested blockquote syntax (> >)
    fn render_nested_quote(
        &self,
        quote: &AstNode,
        output: &mut dyn Write,
        depth: usize,
        quote_level: usize,
    ) -> io::Result<()> {
        let children = quote.value().children.lock().unwrap();
        for child in children.iter() {
            if let AstNodeKind::QuoteContent { .. } = child.kind() {
                self.render_nested_quote_content(child, output, depth, quote_level)?;
            } else {
                for _ in 0..depth {
                    write!(output, "  ")?;
                }
                for _ in 0..=quote_level {
                    write!(output, "> ")?;
                }
                self._format_impl(child, output, depth, true)?;
            }
        }
        Ok(())
    }

    /// Render QuoteContent in a nested quote context
    fn render_nested_quote_content(
        &self,
        quote_content: &AstNode,
        output: &mut dyn Write,
        depth: usize,
        quote_level: usize,
    ) -> io::Result<()> {
        // Output nested "> >" prefix
        for _ in 0..depth {
            write!(output, "  ")?;
        }
        for _ in 0..=quote_level {
            write!(output, "> ")?;
        }

        // Render contents
        let contents = quote_content.value().contents.lock().unwrap();
        for content in contents.iter() {
            self._format_impl(content, output, depth, true)?;
        }
        drop(contents);

        writeln!(output)?;

        // Render children
        let children = quote_content.value().children.lock().unwrap();
        for child in children.iter() {
            if let AstNodeKind::QuoteContent { .. } = child.kind() {
                self.render_nested_quote_content(child, output, depth, quote_level)?;
            }
        }

        Ok(())
    }
}

/// Renderer that outputs patto format (for round-trip conversion)
#[derive(Debug, Default)]
pub struct PattoRenderer {
    /// Starting indentation level (0 for root-level content)
    base_indent: usize,
}

impl PattoRenderer {
    pub fn new() -> Self {
        Self { base_indent: 0 }
    }

    pub fn with_base_indent(base_indent: usize) -> Self {
        Self { base_indent }
    }

    fn _format_impl(&self, ast: &AstNode, output: &mut dyn Write, depth: usize) -> io::Result<()> {
        match ast.kind() {
            AstNodeKind::Dummy => {
                // Render all children at base indent level
                // The tree structure provides proper nesting for list items
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    self._format_impl(child, output, self.base_indent)?;
                }
            }
            AstNodeKind::Line { properties } => {
                // Indentation
                for _ in 0..depth {
                    write!(output, "\t")?;
                }

                // Check for task property
                let mut task_prop: Option<(&TaskStatus, &crate::parser::Deadline)> = None;
                for property in properties {
                    if let Property::Task { status, due, .. } = property {
                        task_prop = Some((status, due));
                        break;
                    }
                }

                // Render contents
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(content, output, 0)?;
                }

                // Add task property if present
                if let Some((status, due)) = task_prop {
                    let status_str = match status {
                        TaskStatus::Todo => "todo",
                        TaskStatus::Doing => "doing",
                        TaskStatus::Done => "done",
                    };
                    let due_str = due.to_string();
                    if due_str.is_empty() {
                        write!(output, " {{@task status={}}}", status_str)?;
                    } else {
                        write!(output, " {{@task status={} due={}}}", status_str, due_str)?;
                    }
                }

                writeln!(output)?;

                // Children
                for child in ast.value().children.lock().unwrap().iter() {
                    self._format_impl(child, output, depth + 1)?;
                }
            }
            AstNodeKind::QuoteContent { properties } => {
                // Indentation based on structural depth
                for _ in 0..depth {
                    write!(output, "\t")?;
                }

                // Check for task property
                let mut task_prop: Option<(&TaskStatus, &crate::parser::Deadline)> = None;
                for property in properties {
                    if let Property::Task { status, due, .. } = property {
                        task_prop = Some((status, due));
                        break;
                    }
                }

                // Render contents (clean text, no embedded tabs)
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(content, output, 0)?;
                }

                // Add task property if present
                if let Some((status, due)) = task_prop {
                    let status_str = match status {
                        TaskStatus::Todo => "todo",
                        TaskStatus::Doing => "doing",
                        TaskStatus::Done => "done",
                    };
                    let due_str = due.to_string();
                    if due_str.is_empty() {
                        write!(output, " {{@task status={}}}", status_str)?;
                    } else {
                        write!(output, " {{@task status={} due={}}}", status_str, due_str)?;
                    }
                }

                writeln!(output)?;

                // Recursively render nested children at depth+1
                for child in ast.value().children.lock().unwrap().iter() {
                    self._format_impl(child, output, depth + 1)?;
                }
            }
            AstNodeKind::Text => {
                write!(output, "{}", ast.extract_str())?;
            }
            AstNodeKind::Decoration {
                fontsize,
                italic,
                underline,
                deleted,
            } => {
                // Determine decoration markers
                let mut markers = String::new();
                if *fontsize > 0 {
                    markers.push('*');
                }
                if *italic {
                    markers.push('/');
                }
                if *underline {
                    markers.push('_');
                }
                if *deleted {
                    markers.push('-');
                }

                if !markers.is_empty() {
                    write!(output, "[{} ", markers)?;
                }
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(content, output, 0)?;
                }
                if !markers.is_empty() {
                    write!(output, "]")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
                if *inline {
                    write!(output, "[` ")?;
                    for content in ast.value().contents.lock().unwrap().iter() {
                        write!(output, "{}", content.extract_str())?;
                    }
                    write!(output, " `]")?;
                } else {
                    if lang.is_empty() {
                        writeln!(output, "[@code]")?;
                    } else {
                        writeln!(output, "[@code {}]", lang)?;
                    }
                    for child in ast.value().children.lock().unwrap().iter() {
                        write!(output, "\t")?;
                        write!(output, "{}", child.extract_str())?;
                        writeln!(output)?;
                    }
                }
            }
            AstNodeKind::CodeContent | AstNodeKind::MathContent => {
                write!(output, "{}", ast.extract_str())?;
            }
            AstNodeKind::Math { inline } => {
                if *inline {
                    write!(output, "[$ ")?;
                    for content in ast.value().contents.lock().unwrap().iter() {
                        write!(output, "{}", content.extract_str())?;
                    }
                    write!(output, " $]")?;
                } else {
                    writeln!(output, "[@math]")?;
                    for child in ast.value().children.lock().unwrap().iter() {
                        write!(output, "\t")?;
                        write!(output, "{}", child.extract_str())?;
                        writeln!(output)?;
                    }
                }
            }
            AstNodeKind::Quote => {
                writeln!(output, "[@quote]")?;
                // Render children (QuoteContent and nested Line/Quote) with depth+1
                for child in ast.value().children.lock().unwrap().iter() {
                    self._format_impl(child, output, depth + 1)?;
                }
            }
            AstNodeKind::Table { caption } => {
                if let Some(cap) = caption {
                    writeln!(output, "[@table caption=\"{}\"]", cap)?;
                } else {
                    writeln!(output, "[@table]")?;
                }
                for child in ast.value().children.lock().unwrap().iter() {
                    self._format_impl(child, output, depth)?;
                }
            }
            AstNodeKind::TableRow => {
                write!(output, "\t")?;
                let contents = ast.value().contents.lock().unwrap();
                for (i, cell) in contents.iter().enumerate() {
                    if i > 0 {
                        write!(output, "\t")?;
                    }
                    for content in cell.value().contents.lock().unwrap().iter() {
                        self._format_impl(content, output, 0)?;
                    }
                }
                writeln!(output)?;
            }
            AstNodeKind::TableColumn => {
                for content in ast.value().contents.lock().unwrap().iter() {
                    self._format_impl(content, output, 0)?;
                }
            }
            AstNodeKind::WikiLink { link, anchor } => {
                if let Some(anc) = anchor {
                    if link.is_empty() {
                        write!(output, "[#{}]", anc)?;
                    } else {
                        write!(output, "[{}#{}]", link, anc)?;
                    }
                } else {
                    write!(output, "[{}]", link)?;
                }
            }
            AstNodeKind::Link { link, title } => {
                if let Some(t) = title {
                    write!(output, "[{} {}]", t, link)?;
                } else {
                    write!(output, "[{}]", link)?;
                }
            }
            AstNodeKind::Embed { link, title } => {
                if let Some(t) = title {
                    write!(output, "[@embed {} {}]", link, t)?;
                } else {
                    write!(output, "[@embed {}]", link)?;
                }
            }
            AstNodeKind::Image { src, alt } => {
                if let Some(a) = alt {
                    write!(output, "[@img {} \"{}\"]", src, a)?;
                } else {
                    write!(output, "[@img {}]", src)?;
                }
            }
            AstNodeKind::HorizontalLine => {
                writeln!(output, "---")?;
            }
        }
        Ok(())
    }
}

impl Renderer for PattoRenderer {
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        self._format_impl(ast, output, 0)
    }
}
