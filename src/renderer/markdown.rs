use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind, Property, TaskStatus};

use super::Renderer;
use crate::utils::get_youtube_id;

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
                    self._format_impl(ast, output, depth, in_quote)?;
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
                let children = ast.children();
                for child in children.iter() {
                    self._format_impl(child, output, depth, in_quote)?;
                }
            }
            AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } => {
                let has_children = !ast.children().is_empty();
                let is_quote_content = matches!(ast.kind(), AstNodeKind::QuoteContent { .. });

                // Check if this line only contains a block element (quote, code, math, table)
                let contents = ast.contents();
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
                let mut task_scheduled: Option<&crate::parser::Deadline> = None;
                let mut task_completed_at: Option<&crate::parser::Deadline> = None;
                let mut is_done = false;
                for property in properties {
                    if let Property::Task {
                        status,
                        due,
                        scheduled,
                        completed_at,
                        ..
                    } = property
                    {
                        task_due = Some(due);
                        task_scheduled = scheduled.as_ref();
                        task_completed_at = completed_at.as_ref();
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
                for content in ast.contents().iter() {
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

                // Append scheduled date (only for non-done tasks)
                if !is_done {
                    if let Some(scheduled) = task_scheduled {
                        let s_str = scheduled.to_string();
                        if !s_str.is_empty() {
                            match self.options.task_format() {
                                TaskFormat::Checkbox => write!(output, " (scheduled: {})", s_str)?,
                                TaskFormat::ObsidianEmoji => write!(output, " ⏳ {}", s_str)?,
                                TaskFormat::ObsidianDataview => {
                                    write!(output, " [scheduled:: {}]", s_str)?
                                }
                            }
                        }
                    }
                }

                // Append completed_at date for done tasks
                if is_done {
                    if let Some(completed_at) = task_completed_at {
                        let c_str = completed_at.to_string();
                        if !c_str.is_empty() {
                            match self.options.task_format() {
                                TaskFormat::Checkbox => write!(output, " (completed: {})", c_str)?,
                                TaskFormat::ObsidianEmoji => write!(output, " ✅ {}", c_str)?,
                                TaskFormat::ObsidianDataview => {
                                    write!(output, " [completed_at:: {}]", c_str)?
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
                let children = ast.children();
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
                    let contents = ast.contents();
                    if !contents.is_empty() {
                        write!(output, "{}", contents[0].extract_str())?;
                    }
                    write!(output, "$")?;
                } else {
                    writeln!(output, "$$")?;
                    let children = ast.children();
                    for child in children.iter() {
                        writeln!(output, "{}", child.extract_str())?;
                    }
                    writeln!(output, "$$")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
                if *inline {
                    write!(output, "`")?;
                    let contents = ast.contents();
                    if !contents.is_empty() {
                        write!(output, "{}", contents[0].extract_str())?;
                    }
                    write!(output, "`")?;
                } else {
                    // Proper fenced code block (NOT nested in list)
                    writeln!(output, "```{}", lang)?;
                    let children = ast.children();
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
                } else if link.contains("slideshare.net") {
                    if let Some(title) = title {
                        write!(output, "[{}]({})", title, link)?;
                    } else {
                        write!(output, "[{}]({})", link, link)?;
                    }
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
                for content in ast.contents().iter() {
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

                let children = ast.children();
                for (i, child) in children.iter().enumerate() {
                    self._format_impl(child, output, depth, in_quote)?;

                    // Add header separator after first row
                    if i == 0 {
                        let col_count = child.contents().len();
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
                let contents = ast.contents();
                for content in contents.iter() {
                    write!(output, " ")?;
                    self._format_impl(content, output, depth, in_quote)?;
                    write!(output, " |")?;
                }
                writeln!(output)?;
            }
            AstNodeKind::TableColumn => {
                for content in ast.contents().iter() {
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
        let contents = quote_content.contents();
        for content in contents.iter() {
            self._format_impl(content, output, depth, true)?;
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
