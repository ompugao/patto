use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind, Deadline, Property, TaskStatus};

use super::{wiki_target, write_lines, Renderer, WikiTarget};
use crate::utils::{get_twitter_embed, get_youtube_id};

use crate::markdown::{AnchorFormat, MarkdownRendererOptions, TaskFormat, WikiLinkFormat};

pub struct MarkdownRenderer {
    options: MarkdownRendererOptions,
}

impl Renderer for MarkdownRenderer {
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        if self.options.include_frontmatter() {
            writeln!(output, "---")?;
            writeln!(output, "patto_source: true")?;
            writeln!(output, "flavor: {}", self.options.flavor)?;
            writeln!(output, "---")?;
            writeln!(output)?;
        }

        self.write_node(ast, output, 0, false)
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
        self.write_range(ast, output, start_line, end_line)
    }

    /// A line inside the range is written whole, children included. A line
    /// before the range is only searched for children inside it.
    fn write_range(
        &self,
        ast: &AstNode,
        output: &mut dyn Write,
        start_line: usize,
        end_line: usize,
    ) -> io::Result<()> {
        match ast.kind() {
            AstNodeKind::Dummy => self.write_range_children(ast, output, start_line, end_line),
            AstNodeKind::Line { .. } | AstNodeKind::QuoteContent { .. } => {
                let row = ast.location().row;
                if row >= start_line && row <= end_line {
                    self.write_node(ast, output, 0, false)
                } else if row < start_line {
                    self.write_range_children(ast, output, start_line, end_line)
                } else {
                    Ok(())
                }
            }
            _ => self.write_node(ast, output, 0, false),
        }
    }

    fn write_range_children(
        &self,
        ast: &AstNode,
        output: &mut dyn Write,
        start_line: usize,
        end_line: usize,
    ) -> io::Result<()> {
        for child in ast.children().iter() {
            if child.location().row <= end_line {
                self.write_range(child, output, start_line, end_line)?;
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
            AstNodeKind::Quote => self.write_quote(ast, output, QuotePrefix::at(depth)),
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
        write_lines(ast, output, "")?;
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
        write_lines(ast, output, "")?;
        writeln!(output, "```")
    }

    fn write_wikilink(
        &self,
        link: &str,
        anchor: Option<&str>,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        let target = wiki_target(link, anchor);
        match self.options.wiki_link_format() {
            WikiLinkFormat::WikiStyle => match target {
                WikiTarget::SelfAnchor(anchor) => write!(output, "[[#{}]]", anchor),
                WikiTarget::NoteAnchor { note, anchor } => {
                    write!(output, "[[{}#{}]]", note, anchor)
                }
                WikiTarget::Note(note) => write!(output, "[[{}]]", note),
            },
            WikiLinkFormat::Markdown => {
                let ext = self.options.file_extension();
                match target {
                    WikiTarget::SelfAnchor(anchor) => {
                        write!(output, "[#{}](#{})", anchor, anchor)
                    }
                    WikiTarget::NoteAnchor { note, anchor } => {
                        write!(output, "[{}#{}]({}{}#{})", note, anchor, note, ext, anchor)
                    }
                    WikiTarget::Note(note) => write!(output, "[{}]({}{})", note, note, ext),
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

    fn write_quote(
        &self,
        quote: &AstNode,
        output: &mut dyn Write,
        prefix: QuotePrefix,
    ) -> io::Result<()> {
        for child in quote.children().iter() {
            if let AstNodeKind::QuoteContent { .. } = child.kind() {
                self.write_quote_content(child, output, prefix)?;
            } else {
                prefix.write(output)?;
                self.write_node(child, output, prefix.depth, true)?;
            }
        }
        Ok(())
    }

    fn write_quote_content(
        &self,
        quote_content: &AstNode,
        output: &mut dyn Write,
        prefix: QuotePrefix,
    ) -> io::Result<()> {
        prefix.write(output)?;

        let contents = quote_content.contents();
        let nested_quote = prefix.quote_level == 0
            && contents.len() == 1
            && matches!(contents[0].kind(), AstNodeKind::Quote);
        if nested_quote {
            writeln!(output)?;
            self.write_quote(&contents[0], output, prefix.nested_quote())?;
        } else {
            for content in contents.iter() {
                self.write_node(content, output, prefix.depth, true)?;
            }
            writeln!(output)?;
        }
        drop(contents);

        for child in quote_content.children().iter() {
            if let AstNodeKind::QuoteContent { .. } = child.kind() {
                self.write_quote_content(child, output, prefix.child())?;
            } else if prefix.quote_level == 0 {
                prefix.write(output)?;
                self.write_node(child, output, prefix.depth, true)?;
            }
        }
        Ok(())
    }
}

/// What goes in front of a quoted line: the list indent, one `> ` per quote
/// level, and four spaces per nesting level inside the quote.
#[derive(Clone, Copy)]
struct QuotePrefix {
    depth: usize,
    quote_level: usize,
    inner_depth: usize,
}

impl QuotePrefix {
    fn at(depth: usize) -> Self {
        Self {
            depth,
            quote_level: 0,
            inner_depth: 0,
        }
    }

    fn write(&self, output: &mut dyn Write) -> io::Result<()> {
        for _ in 0..self.depth {
            write!(output, "  ")?;
        }
        for _ in 0..=self.quote_level {
            write!(output, "> ")?;
        }
        for _ in 0..self.inner_depth {
            write!(output, "    ")?;
        }
        Ok(())
    }

    /// The prefix of a quote block that is the whole content of a quoted line.
    fn nested_quote(self) -> Self {
        Self {
            depth: self.depth,
            quote_level: self.inner_depth + 1,
            inner_depth: 0,
        }
    }

    /// The prefix of a line nested under a quoted line.
    fn child(self) -> Self {
        // FIXME: inside a nested quote the children keep their parent's prefix,
        // so their nesting is invisible. Kept so that the output does not change.
        if self.quote_level > 0 {
            return self;
        }
        Self {
            inner_depth: self.inner_depth + 1,
            ..self
        }
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
