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

impl Renderer for PattoRenderer {
    fn format(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        self.write_node(ast, output, 0)
    }
}

impl PattoRenderer {
    pub fn new() -> Self {
        Self { base_indent: 0 }
    }

    pub fn with_base_indent(base_indent: usize) -> Self {
        Self { base_indent }
    }

    fn write_node(&self, ast: &AstNode, output: &mut dyn Write, depth: usize) -> io::Result<()> {
        match ast.kind() {
            // The tree structure carries the nesting, so children all start at
            // the base indent.
            AstNodeKind::Dummy => {
                for child in ast.children().iter() {
                    self.write_node(child, output, self.base_indent)?;
                }
                Ok(())
            }
            AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } => {
                self.write_line(ast, properties, output, depth)
            }
            AstNodeKind::Text | AstNodeKind::CodeContent | AstNodeKind::MathContent => {
                write!(output, "{}", ast.extract_str())
            }
            AstNodeKind::Decoration {
                fontsize,
                italic,
                underline,
                deleted,
            } => self.write_decoration(ast, *fontsize, *italic, *underline, *deleted, output),
            AstNodeKind::Code { lang, inline } => self.write_code(ast, lang, *inline, output),
            AstNodeKind::Math { inline } => self.write_math(ast, *inline, output),
            AstNodeKind::Quote => {
                writeln!(output, "[@quote]")?;
                for child in ast.children().iter() {
                    self.write_node(child, output, depth + 1)?;
                }
                Ok(())
            }
            AstNodeKind::Table { caption } => {
                self.write_table(ast, caption.as_deref(), output, depth)
            }
            AstNodeKind::TableRow => self.write_table_row(ast, output),
            AstNodeKind::TableColumn => self.write_contents(ast, output),
            AstNodeKind::WikiLink { link, anchor } => match anchor.as_deref() {
                Some(anchor) if link.is_empty() => write!(output, "[#{}]", anchor),
                Some(anchor) => write!(output, "[{}#{}]", link, anchor),
                None => write!(output, "[{}]", link),
            },
            AstNodeKind::Link { link, title } => match title {
                Some(title) => write!(output, "[{} {}]", title, link),
                None => write!(output, "[{}]", link),
            },
            AstNodeKind::Embed { link, title } => match title {
                Some(title) => write!(output, "[@embed {} {}]", link, title),
                None => write!(output, "[@embed {}]", link),
            },
            AstNodeKind::Image { src, alt } => match alt {
                Some(alt) => write!(output, "[@img {} \"{}\"]", src, alt),
                None => write!(output, "[@img {}]", src),
            },
            AstNodeKind::HorizontalLine => {
                // The grammar needs at least five dashes; fewer re-parse as text.
                writeln!(output, "-----")
            }
        }
    }

    fn write_indent(&self, output: &mut dyn Write, depth: usize) -> io::Result<()> {
        for _ in 0..depth {
            write!(output, "\t")?;
        }
        Ok(())
    }

    fn write_contents(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        for content in ast.contents().iter() {
            self.write_node(content, output, 0)?;
        }
        Ok(())
    }

    /// A line and a line of quote content are written the same way; only where
    /// they sit in the tree differs.
    fn write_line(
        &self,
        ast: &AstNode,
        properties: &[Property],
        output: &mut dyn Write,
        depth: usize,
    ) -> io::Result<()> {
        self.write_indent(output, depth)?;
        self.write_contents(ast, output)?;
        write_task_property(properties, output)?;
        writeln!(output)?;

        for child in ast.children().iter() {
            self.write_node(child, output, depth + 1)?;
        }
        Ok(())
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
        let mut markers = String::new();
        if fontsize > 0 {
            markers.push('*');
        }
        if italic {
            markers.push('/');
        }
        if underline {
            markers.push('_');
        }
        if deleted {
            markers.push('-');
        }

        if markers.is_empty() {
            return self.write_contents(ast, output);
        }
        write!(output, "[{} ", markers)?;
        self.write_contents(ast, output)?;
        write!(output, "]")
    }

    fn write_code(
        &self,
        ast: &AstNode,
        lang: &str,
        inline: bool,
        output: &mut dyn Write,
    ) -> io::Result<()> {
        if inline {
            write!(output, "[` ")?;
            write_raw_contents(ast, output)?;
            return write!(output, " `]");
        }

        if lang.is_empty() {
            writeln!(output, "[@code]")?;
        } else {
            writeln!(output, "[@code {}]", lang)?;
        }
        write_block_body(ast, output)
    }

    fn write_math(&self, ast: &AstNode, inline: bool, output: &mut dyn Write) -> io::Result<()> {
        if inline {
            write!(output, "[$ ")?;
            write_raw_contents(ast, output)?;
            return write!(output, " $]");
        }

        writeln!(output, "[@math]")?;
        write_block_body(ast, output)
    }

    fn write_table(
        &self,
        ast: &AstNode,
        caption: Option<&str>,
        output: &mut dyn Write,
        depth: usize,
    ) -> io::Result<()> {
        match caption {
            Some(caption) => writeln!(output, "[@table caption=\"{}\"]", caption)?,
            None => writeln!(output, "[@table]")?,
        }
        for child in ast.children().iter() {
            self.write_node(child, output, depth)?;
        }
        Ok(())
    }

    fn write_table_row(&self, ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
        write!(output, "\t")?;
        for (i, cell) in ast.contents().iter().enumerate() {
            if i > 0 {
                write!(output, "\t")?;
            }
            self.write_contents(cell, output)?;
        }
        writeln!(output)
    }
}

/// The `{@task ...}` suffix, written after the line's text.
fn write_task_property(properties: &[Property], output: &mut dyn Write) -> io::Result<()> {
    let Some(Property::Task {
        status,
        due,
        scheduled,
        completed_at,
        ..
    }) = properties
        .iter()
        .find(|property| matches!(property, Property::Task { .. }))
    else {
        return Ok(());
    };

    let status = match status {
        TaskStatus::Todo => "todo",
        TaskStatus::Doing => "doing",
        TaskStatus::Paused => "paused",
        TaskStatus::Done => "done",
    };
    write!(output, " {{@task status={}", status)?;

    let due = due.to_string();
    if !due.is_empty() {
        write!(output, " due={}", due)?;
    }
    if let Some(scheduled) = scheduled {
        write!(output, " scheduled={}", scheduled)?;
    }
    if let Some(completed_at) = completed_at {
        write!(output, " completed_at={}", completed_at)?;
    }
    write!(output, "}}")
}

/// Inline code and math keep their body exactly as written.
fn write_raw_contents(ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
    for content in ast.contents().iter() {
        write!(output, "{}", content.extract_str())?;
    }
    Ok(())
}

/// The indented body of a `[@code]` or `[@math]` block.
fn write_block_body(ast: &AstNode, output: &mut dyn Write) -> io::Result<()> {
    for child in ast.children().iter() {
        writeln!(output, "\t{}", child.extract_str())?;
    }
    Ok(())
}
