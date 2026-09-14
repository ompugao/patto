use std::io;
use std::io::Write;

use crate::parser::{AstNode, AstNodeKind, Property, TaskStatus};

use super::Renderer;

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
                let children = ast.children();
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
                let mut task_prop: Option<(
                    &TaskStatus,
                    &crate::parser::Deadline,
                    Option<&crate::parser::Deadline>,
                    Option<&crate::parser::Deadline>,
                )> = None;
                for property in properties {
                    if let Property::Task {
                        status,
                        due,
                        scheduled,
                        completed_at,
                        ..
                    } = property
                    {
                        task_prop = Some((status, due, scheduled.as_ref(), completed_at.as_ref()));
                        break;
                    }
                }

                // Render contents
                for content in ast.contents().iter() {
                    self._format_impl(content, output, 0)?;
                }

                // Add task property if present
                if let Some((status, due, scheduled, completed_at)) = task_prop {
                    let status_str = match status {
                        TaskStatus::Todo => "todo",
                        TaskStatus::Doing => "doing",
                        TaskStatus::Paused => "paused",
                        TaskStatus::Done => "done",
                    };
                    let due_str = due.to_string();
                    write!(output, " {{@task status={}", status_str)?;
                    if !due_str.is_empty() {
                        write!(output, " due={}", due_str)?;
                    }
                    if let Some(s) = scheduled {
                        write!(output, " scheduled={}", s)?;
                    }
                    if let Some(c) = completed_at {
                        write!(output, " completed_at={}", c)?;
                    }
                    write!(output, "}}")?;
                }

                writeln!(output)?;

                // Children
                for child in ast.children().iter() {
                    self._format_impl(child, output, depth + 1)?;
                }
            }
            AstNodeKind::QuoteContent { properties } => {
                // Indentation based on structural depth
                for _ in 0..depth {
                    write!(output, "\t")?;
                }

                // Check for task property
                let mut task_prop: Option<(
                    &TaskStatus,
                    &crate::parser::Deadline,
                    Option<&crate::parser::Deadline>,
                    Option<&crate::parser::Deadline>,
                )> = None;
                for property in properties {
                    if let Property::Task {
                        status,
                        due,
                        scheduled,
                        completed_at,
                        ..
                    } = property
                    {
                        task_prop = Some((status, due, scheduled.as_ref(), completed_at.as_ref()));
                        break;
                    }
                }

                // Render contents (clean text, no embedded tabs)
                for content in ast.contents().iter() {
                    self._format_impl(content, output, 0)?;
                }

                // Add task property if present
                if let Some((status, due, scheduled, completed_at)) = task_prop {
                    let status_str = match status {
                        TaskStatus::Todo => "todo",
                        TaskStatus::Doing => "doing",
                        TaskStatus::Paused => "paused",
                        TaskStatus::Done => "done",
                    };
                    let due_str = due.to_string();
                    write!(output, " {{@task status={}", status_str)?;
                    if !due_str.is_empty() {
                        write!(output, " due={}", due_str)?;
                    }
                    if let Some(s) = scheduled {
                        write!(output, " scheduled={}", s)?;
                    }
                    if let Some(c) = completed_at {
                        write!(output, " completed_at={}", c)?;
                    }
                    write!(output, "}}")?;
                }

                writeln!(output)?;

                // Recursively render nested children at depth+1
                for child in ast.children().iter() {
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
                for content in ast.contents().iter() {
                    self._format_impl(content, output, 0)?;
                }
                if !markers.is_empty() {
                    write!(output, "]")?;
                }
            }
            AstNodeKind::Code { lang, inline } => {
                if *inline {
                    write!(output, "[` ")?;
                    for content in ast.contents().iter() {
                        write!(output, "{}", content.extract_str())?;
                    }
                    write!(output, " `]")?;
                } else {
                    if lang.is_empty() {
                        writeln!(output, "[@code]")?;
                    } else {
                        writeln!(output, "[@code {}]", lang)?;
                    }
                    for child in ast.children().iter() {
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
                    for content in ast.contents().iter() {
                        write!(output, "{}", content.extract_str())?;
                    }
                    write!(output, " $]")?;
                } else {
                    writeln!(output, "[@math]")?;
                    for child in ast.children().iter() {
                        write!(output, "\t")?;
                        write!(output, "{}", child.extract_str())?;
                        writeln!(output)?;
                    }
                }
            }
            AstNodeKind::Quote => {
                writeln!(output, "[@quote]")?;
                // Render children (QuoteContent and nested Line/Quote) with depth+1
                for child in ast.children().iter() {
                    self._format_impl(child, output, depth + 1)?;
                }
            }
            AstNodeKind::Table { caption } => {
                if let Some(cap) = caption {
                    writeln!(output, "[@table caption=\"{}\"]", cap)?;
                } else {
                    writeln!(output, "[@table]")?;
                }
                for child in ast.children().iter() {
                    self._format_impl(child, output, depth)?;
                }
            }
            AstNodeKind::TableRow => {
                write!(output, "\t")?;
                let contents = ast.contents();
                for (i, cell) in contents.iter().enumerate() {
                    if i > 0 {
                        write!(output, "\t")?;
                    }
                    for content in cell.contents().iter() {
                        self._format_impl(content, output, 0)?;
                    }
                }
                writeln!(output)?;
            }
            AstNodeKind::TableColumn => {
                for content in ast.contents().iter() {
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
