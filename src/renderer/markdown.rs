use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind, Deadline, Property, TaskStatus};

use super::Renderer;
use crate::utils::{get_twitter_embed, get_youtube_id};

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
        self.write_node(ast, output, depth, false)?;
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
                let children = ast.children();
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
                    self.write_node(ast, output, depth, in_quote)?;
                } else if row < start_line {
                    // This line is before range, but check children
                    let children = ast.children();
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
                self.write_node(ast, output, depth, in_quote)?;
            }
        }
        Ok(())
    }

    fn write_node(
        &self,
        ast: &AstNode,
        output: &mut dyn Write,
        depth: usize,
        in_quote: bool,
    ) -> io::Result<()> {
        match ast.kind() {
            AstNodeKind::Dummy => {
                for child in ast.children().iter() {
                    self.write_node(child, output, depth, in_quote)?;
                }
                Ok(())
            }
            AstNodeKind::Line { properties } => {
                self.write_line(ast, properties, false, output, depth, in_quote)
            }
            AstNodeKind::QuoteContent { properties } => {
                self.write_line(ast, properties, true, output, depth, in_quote)
            }
            AstNodeKind::Quote => self.render_quote_children(ast, output, depth, 0),
            AstNodeKind::Math { inline } => self.write_math(ast, *inline, output),
            AstNodeKind::Code { lang, inline } => self.write_code(ast, lang, *inline, output),
            AstNodeKind::Image { src, alt } => {
                write!(output, "![{}]({})", alt.as_deref().unwrap_or(""), src)
            }
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
            } => self.write_decoration(
                ast, *fontsize, *italic, *underline, *deleted, output, depth, in_quote,
            ),
            AstNodeKind::Text | AstNodeKind::CodeContent | AstNodeKind::MathContent => {
                write!(output, "{}", ast.extract_str())
            }
            AstNodeKind::HorizontalLine => write!(output, "---"),
            AstNodeKind::Table { caption } => {
                self.write_table(ast, caption.as_deref(), output, depth, in_quote)
            }
            AstNodeKind::TableRow => {
                write!(output, "|")?;
                for content in ast.contents().iter() {
                    write!(output, " ")?;
                    self.write_node(content, output, depth, in_quote)?;
                    write!(output, " |")?;
                }
                writeln!(output)
            }
            AstNodeKind::TableColumn => {
                for content in ast.contents().iter() {
                    self.write_node(content, output, depth, in_quote)?;
                }
                Ok(())
            }
        }
    }

    fn write_line(
        &self,
        ast: &AstNode,
        properties: &[Property],
        is_quote_content: bool,
        output: &mut dyn Write,
        depth: usize,
        in_quote: bool,
    ) -> io::Result<()> {
        let has_children = !ast.children().is_empty();

        let (is_block_container, is_empty) = {
            let contents = ast.contents();
            // A line holding nothing but a block writes its own markers and
            // newlines, so the list marker and indent are skipped for it.
            let is_block_container = contents.len() == 1
                && matches!(
                    contents[0].kind(),
                    AstNodeKind::Quote
                        | AstNodeKind::Code { inline: false, .. }
                        | AstNodeKind::Math { inline: false }
                        | AstNodeKind::Table { .. }
                );
            let is_empty = contents.is_empty() && properties.is_empty() && !has_children;
            (is_block_container, is_empty)
        };

        if is_empty {
            return writeln!(output);
        }

        // Quote content is indented by the quote itself.
        if !in_quote && !is_block_container {
            for _ in 0..depth {
                write!(output, "  ")?;
            }
        }

        let task = line_task(properties);

        if !is_quote_content && !is_block_container && (depth > 0 || has_children) {
            write!(output, "- ")?;
        }
        if let Some(task) = &task {
            write!(output, "{}", if task.is_done { "[x] " } else { "[ ] " })?;
        }

        for content in ast.contents().iter() {
            self.write_node(content, output, depth, in_quote)?;
        }

        if let Some(task) = &task {
            if task.is_done {
                self.write_task_date(&COMPLETED, task.completed_at, output)?;
            } else {
                self.write_task_date(&DUE, Some(task.due), output)?;
                self.write_task_date(&SCHEDULED, task.scheduled, output)?;
            }
        }

        for property in properties {
            if let Property::Anchor { name, .. } = property {
                match self.options.anchor_format() {
                    AnchorFormat::HtmlAnchor => write!(output, " <a id=\"{}\"></a>", name)?,
                    AnchorFormat::HtmlComment => write!(output, " <!-- anchor: {} -->", name)?,
                    AnchorFormat::ObsidianBlock => write!(output, " ^{}", name)?,
                    AnchorFormat::Inline => write!(output, " #{}", name)?,
                }
            }
        }

        // Block containers handle their own newlines
        if !is_block_container {
            writeln!(output)?;
        }

        for child in ast.children().iter() {
            self.write_node(child, output, depth + 1, in_quote)?;
        }
        Ok(())
    }

    /// One of a task's dates, in whichever form the flavor uses.
    fn write_task_date(
        &self,
        format: &TaskDateFormat,
        deadline: Option<&Deadline>,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        let Some(deadline) = deadline else {
            return Ok(());
        };
        let date = deadline.to_string();
        if date.is_empty() {
            return Ok(());
        }

        match self.options.task_format() {
            TaskFormat::Checkbox => write!(output, " ({}: {})", format.label, date),
            TaskFormat::ObsidianEmoji => write!(output, " {} {}", format.emoji, date),
            TaskFormat::ObsidianDataview => write!(output, " [{}:: {}]", format.dataview_key, date),
        }
    }

    fn write_math(&self, ast: &AstNode, inline: bool, output: &mut dyn Write) -> io::Result<()> {
        if inline {
            write!(output, "$")?;
            if let Some(content) = ast.contents().first() {
                write!(output, "{}", content.extract_str())?;
            }
            return write!(output, "$");
        }

        writeln!(output, "$$")?;
        write_block_body(ast, output)?;
        writeln!(output, "$$")
    }

    fn write_code(
        &self,
        ast: &AstNode,
        lang: &str,
        inline: bool,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        if inline {
            write!(output, "`")?;
            if let Some(content) = ast.contents().first() {
                write!(output, "{}", content.extract_str())?;
            }
            return write!(output, "`");
        }

        writeln!(output, "```{}", lang)?;
        write_block_body(ast, output)?;
        writeln!(output, "```")
    }

    fn write_wikilink(
        &self,
        link: &str,
        anchor: Option<&str>,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        match self.options.wiki_link_format() {
            WikiLinkFormat::WikiStyle => match anchor {
                Some(anchor) if link.is_empty() => write!(output, "[[#{}]]", anchor),
                Some(anchor) => write!(output, "[[{}#{}]]", link, anchor),
                None => write!(output, "[[{}]]", link),
            },
            WikiLinkFormat::Markdown => {
                let ext = self.options.file_extension();
                match anchor {
                    Some(anchor) if link.is_empty() => {
                        write!(output, "[#{}](#{})", anchor, anchor)
                    }
                    Some(anchor) => {
                        write!(output, "[{}#{}]({}{}#{})", link, anchor, link, ext, anchor)
                    }
                    None => write!(output, "[{}]({}{})", link, link, ext),
                }
            }
        }
    }

    fn write_link(
        &self,
        link: &str,
        title: Option<&str>,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        write!(output, "[{}]({})", title.unwrap_or(link), link)
    }

    fn write_embed(
        &self,
        link: &str,
        title: Option<&str>,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        // Markdown has no iframe, so a YouTube embed becomes a thumbnail
        // linking to the video.
        if let Some(youtube_id) = get_youtube_id(link) {
            return write!(
                output,
                "[![YouTube](https://img.youtube.com/vi/{}/0.jpg)](https://www.youtube.com/watch?v={})",
                youtube_id, youtube_id
            );
        }
        // Without the `oembed` feature this is a no-op and the link is written
        // plainly. Note that it blocks on an HTTP request while rendering.
        if let Some(embed) = get_twitter_embed(link) {
            return write!(output, "{}", embed);
        }
        self.write_link(link, title, output)
    }

    #[allow(clippy::too_many_arguments)]
    fn write_decoration(
        &self,
        ast: &AstNode,
        fontsize: isize,
        italic: bool,
        underline: bool,
        deleted: bool,
        output: &mut dyn Write,
        depth: usize,
        in_quote: bool,
    ) -> io::Result<()> {
        let emphasis = emphasis_marker(fontsize, italic);

        write!(output, "{}", emphasis)?;
        if underline {
            write!(output, "<ins>")?;
        }
        if deleted {
            write!(output, "~~")?;
        }

        for content in ast.contents().iter() {
            self.write_node(content, output, depth, in_quote)?;
        }

        if deleted {
            write!(output, "~~")?;
        }
        if underline {
            write!(output, "</ins>")?;
        }
        write!(output, "{}", emphasis)
    }

    fn write_table(
        &self,
        ast: &AstNode,
        caption: Option<&str>,
        output: &mut dyn Write,
        depth: usize,
        in_quote: bool,
    ) -> io::Result<()> {
        // Markdown tables have no caption, so it becomes emphasised text above.
        if let Some(caption) = caption {
            writeln!(output, "*{}*", caption)?;
        }

        for (i, child) in ast.children().iter().enumerate() {
            self.write_node(child, output, depth, in_quote)?;

            // Markdown needs a separator row for the first row to be a header.
            if i == 0 {
                write!(output, "|")?;
                for _ in 0..child.contents().len() {
                    write!(output, " --- |")?;
                }
                writeln!(output)?;
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
        let children = quote.children();
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
                    self.write_node(child, output, depth, true)?;
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
        let contents = quote_content.contents();
        let has_nested_quote =
            contents.len() == 1 && matches!(contents[0].kind(), AstNodeKind::Quote);

        if has_nested_quote {
            // For nested quotes, we need to output with extra "> " markers
            drop(contents);
            let contents = quote_content.contents();
            for content in contents.iter() {
                if let AstNodeKind::Quote = content.kind() {
                    writeln!(output)?; // End the current line
                                       // Render nested quote with extra "> " marker
                    self.render_nested_quote(content, output, depth, inner_depth + 1)?;
                } else {
                    self.write_node(content, output, depth, true)?;
                }
            }
        } else {
            // Regular content
            for content in contents.iter() {
                self.write_node(content, output, depth, true)?;
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
                // The newline was already written by write_line for lines with properties
            }
            writeln!(output)?;
        }

        // Render children (nested QuoteContent) with increased inner_depth
        let children = quote_content.children();
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
                self.write_node(child, output, depth, true)?;
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
        let children = quote.children();
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
                self.write_node(child, output, depth, true)?;
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
        let contents = quote_content.contents();
        for content in contents.iter() {
            self.write_node(content, output, depth, true)?;
        }
        drop(contents);

        writeln!(output)?;

        // Render children
        let children = quote_content.children();
        for child in children.iter() {
            if let AstNodeKind::QuoteContent { .. } = child.kind() {
                self.render_nested_quote_content(child, output, depth, quote_level)?;
            }
        }

        Ok(())
    }
}

/// How one of a task's dates is written in each markdown flavor.
struct TaskDateFormat {
    /// `(label: date)` in the plain checkbox form.
    label: &'static str,
    /// The Obsidian Tasks emoji.
    emoji: &'static str,
    /// `[key:: date]` in the Dataview form.
    dataview_key: &'static str,
}

const DUE: TaskDateFormat = TaskDateFormat {
    label: "due",
    emoji: "\u{1F4C5}",
    dataview_key: "due",
};
const SCHEDULED: TaskDateFormat = TaskDateFormat {
    label: "scheduled",
    emoji: "\u{23F3}",
    dataview_key: "scheduled",
};
const COMPLETED: TaskDateFormat = TaskDateFormat {
    label: "completed",
    emoji: "\u{2705}",
    dataview_key: "completed_at",
};

/// The task recorded on a line, if it has one.
struct LineTask<'a> {
    is_done: bool,
    due: &'a Deadline,
    scheduled: Option<&'a Deadline>,
    completed_at: Option<&'a Deadline>,
}

fn line_task(properties: &[Property]) -> Option<LineTask<'_>> {
    properties.iter().find_map(|property| match property {
        Property::Task {
            status,
            due,
            scheduled,
            completed_at,
            ..
        } => Some(LineTask {
            is_done: matches!(status, TaskStatus::Done),
            due,
            scheduled: scheduled.as_ref(),
            completed_at: completed_at.as_ref(),
        }),
        _ => None,
    })
}

/// The `*`/`**`/`***` wrapper for a decoration, or nothing.
fn emphasis_marker(fontsize: isize, italic: bool) -> &'static str {
    match (fontsize > 0, italic) {
        (true, false) => "**",
        (false, true) => "*",
        (true, true) => "***",
        (false, false) => "",
    }
}

/// The body of a fenced code or math block, one line per child.
fn write_block_body(ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
    for child in ast.children().iter() {
        writeln!(output, "{}", child.extract_str())?;
    }
    Ok(())
}
