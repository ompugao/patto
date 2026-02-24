use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::parser::{AstNode, AstNodeKind, Property, TaskStatus};
use crate::utils::get_gyazo_img_src;

/// A single element in the rendered document.
#[derive(Debug, Clone)]
pub enum DocElement {
    /// A styled text line.
    TextLine(Line<'static>),
    /// An image to render via kitty/sixel.
    Image {
        src: String,
        alt: Option<String>,
    },
    /// A blank line.
    Spacer,
}

impl DocElement {
    /// Height in terminal rows.
    pub fn height(&self, image_height_rows: u16) -> u16 {
        match self {
            DocElement::TextLine(_) | DocElement::Spacer => 1,
            DocElement::Image { .. } => image_height_rows,
        }
    }

    /// Whether this element is an image.
    pub fn is_image(&self) -> bool {
        matches!(self, DocElement::Image { .. })
    }
}

/// A fully rendered document ready for display.
pub struct RenderedDoc {
    pub elements: Vec<DocElement>,
}

impl RenderedDoc {
    /// Total height in terminal rows.
    pub fn total_height(&self, image_height_rows: u16) -> usize {
        self.elements
            .iter()
            .map(|e| e.height(image_height_rows) as usize)
            .sum()
    }
}

/// Render an AST root node into a flat list of DocElements.
pub fn render_ast(ast: &AstNode) -> RenderedDoc {
    let mut elements = Vec::new();
    render_node(ast, &mut elements, 0);
    RenderedDoc { elements }
}

/// Result of inline rendering — may contain image blocks that need to be
/// emitted between text line fragments.
enum InlineResult {
    /// Pure inline content (appended to current spans).
    Inline,
    /// An image block that must be emitted as a separate DocElement.
    ImageBlock { src: String, alt: Option<String> },
}

fn render_node(ast: &AstNode, elements: &mut Vec<DocElement>, indent: usize) {
    match ast.kind() {
        AstNodeKind::Dummy => {
            let children = ast.value().children.lock().unwrap();
            for child in children.iter() {
                render_node(child, elements, indent);
            }
        }
        AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } => {
            let is_quote = matches!(ast.kind(), AstNodeKind::QuoteContent { .. });
            let mut task_status: Option<&TaskStatus> = None;
            for property in properties {
                if let Property::Task { status, .. } = property {
                    task_status = Some(status);
                }
            }
            let is_done = matches!(task_status, Some(TaskStatus::Done));

            let mut prefix_spans: Vec<Span<'static>> = Vec::new();

            // Indent
            if indent > 0 {
                prefix_spans.push(Span::raw("  ".repeat(indent)));
            }

            // Quote prefix
            if is_quote {
                prefix_spans.push(Span::styled(
                    "│ ",
                    Style::default().fg(Color::DarkGray),
                ));
            }

            // Task icon
            if let Some(status) = task_status {
                let (icon, color) = match status {
                    TaskStatus::Done => ("✓ ", Color::Green),
                    TaskStatus::Doing => ("◑ ", Color::Yellow),
                    _ => ("○ ", Color::White),
                };
                prefix_spans.push(Span::styled(icon.to_string(), Style::default().fg(color)));
            } else {
                prefix_spans.push(Span::raw("• "));
            }

            // Inline contents — collect spans, breaking on images
            let base_style = if is_done {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default()
            };

            let mut spans = prefix_spans.clone();
            let contents = ast.value().contents.lock().unwrap();
            for content in contents.iter() {
                let result = render_inline(content, &mut spans, base_style);
                if let InlineResult::ImageBlock { src, alt } = result {
                    // Flush accumulated text before the image
                    if spans.iter().any(|s| !s.content.is_empty()) {
                        elements.push(DocElement::TextLine(Line::from(
                            std::mem::take(&mut spans),
                        )));
                    }
                    elements.push(DocElement::Image { src, alt });
                    // Reset spans with indent prefix for continuation
                    spans = vec![Span::raw("  ".repeat(indent + 1))];
                }
            }

            // Deadline
            for property in properties {
                if let Property::Task { status, due, .. } = property {
                    if !matches!(status, TaskStatus::Done) {
                        let due_str = format!(" [{}]", due);
                        spans.push(Span::styled(due_str, Style::default().fg(Color::Red)));
                    }
                }
            }

            // Flush remaining spans
            if spans.iter().any(|s| !s.content.is_empty()) {
                elements.push(DocElement::TextLine(Line::from(spans)));
            }

            // Children (nested lines)
            let children = ast.value().children.lock().unwrap();
            for child in children.iter() {
                render_node(child, elements, indent + 1);
            }
        }
        AstNodeKind::Quote => {
            let children = ast.value().children.lock().unwrap();
            for child in children.iter() {
                render_node(child, elements, indent);
            }
        }
        AstNodeKind::Math { inline } => {
            if *inline {
                // Handled as inline content in parent Line
            } else {
                let mut spans = vec![Span::raw("  ".repeat(indent))];
                spans.push(Span::styled(
                    "  [math block]  ",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::DIM),
                ));
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    spans.push(Span::styled(
                        child.extract_str().to_string(),
                        Style::default().fg(Color::Magenta),
                    ));
                }
                elements.push(DocElement::TextLine(Line::from(spans)));
            }
        }
        AstNodeKind::Code { lang, inline } => {
            if *inline {
                // Handled as inline content in parent Line
            } else {
                let header = if lang.is_empty() {
                    "```".to_string()
                } else {
                    format!("```{}", lang)
                };
                let code_style = Style::default().fg(Color::White).bg(Color::DarkGray);
                let prefix = "  ".repeat(indent);

                elements.push(DocElement::TextLine(Line::from(vec![
                    Span::raw(prefix.clone()),
                    Span::styled(header, code_style),
                ])));

                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    elements.push(DocElement::TextLine(Line::from(vec![
                        Span::raw(prefix.clone()),
                        Span::styled(child.extract_str().to_string(), code_style),
                    ])));
                }

                elements.push(DocElement::TextLine(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled("```", code_style),
                ])));
            }
        }
        AstNodeKind::Image { src, alt } => {
            let mut src_resolved = src.clone();
            if let Some(gyazo_src) = get_gyazo_img_src(src) {
                src_resolved = gyazo_src;
            }
            elements.push(DocElement::Image {
                src: src_resolved,
                alt: alt.clone(),
            });
            if let Some(alt_text) = alt {
                elements.push(DocElement::TextLine(Line::from(vec![Span::styled(
                    format!("  {}", alt_text),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )])));
            }
        }
        AstNodeKind::WikiLink { .. }
        | AstNodeKind::Link { .. }
        | AstNodeKind::Embed { .. }
        | AstNodeKind::Decoration { .. }
        | AstNodeKind::Text
        | AstNodeKind::MathContent
        | AstNodeKind::CodeContent => {
            // These are inline — rendered by render_inline when inside a Line
        }
        AstNodeKind::HorizontalLine => {
            elements.push(DocElement::TextLine(Line::from(vec![Span::styled(
                "─".repeat(40),
                Style::default().fg(Color::DarkGray),
            )])));
        }
        AstNodeKind::Table { caption } => {
            if let Some(cap) = caption {
                elements.push(DocElement::TextLine(Line::from(vec![
                    Span::raw("  ".repeat(indent)),
                    Span::styled(
                        cap.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ])));
            }
            let children = ast.value().children.lock().unwrap();
            for child in children.iter() {
                render_table_row(child, elements, indent);
            }
        }
        AstNodeKind::TableRow | AstNodeKind::TableColumn => {
            // Handled inside Table rendering
        }
    }
}

fn render_table_row(ast: &AstNode, elements: &mut Vec<DocElement>, indent: usize) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::raw("  ".repeat(indent)));
    spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));

    let contents = ast.value().contents.lock().unwrap();
    for (i, col) in contents.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        }
        let col_contents = col.value().contents.lock().unwrap();
        for c in col_contents.iter() {
            render_inline(c, &mut spans, Style::default());
        }
    }
    spans.push(Span::styled(" │", Style::default().fg(Color::DarkGray)));
    elements.push(DocElement::TextLine(Line::from(spans)));
}

fn render_inline(ast: &AstNode, spans: &mut Vec<Span<'static>>, base_style: Style) -> InlineResult {
    match ast.kind() {
        AstNodeKind::Text => {
            spans.push(Span::styled(ast.extract_str().to_string(), base_style));
        }
        AstNodeKind::WikiLink { link, anchor } => {
            let display = if let Some(anchor) = anchor {
                if link.is_empty() {
                    format!("#{}", anchor)
                } else {
                    format!("{}#{}", link, anchor)
                }
            } else {
                link.clone()
            };
            spans.push(Span::styled(
                format!("[[{}]]", display),
                base_style.fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
            ));
        }
        AstNodeKind::Link { link, title } => {
            let display = title.as_deref().unwrap_or(link.as_str());
            spans.push(Span::styled(
                display.to_string(),
                base_style.fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
            ));
        }
        AstNodeKind::Embed { link, title } => {
            let display = title.as_deref().unwrap_or(link.as_str());
            spans.push(Span::styled(
                format!("[embed: {}]", display),
                base_style.fg(Color::Blue),
            ));
        }
        AstNodeKind::Code { inline: true, .. } => {
            let contents = ast.value().contents.lock().unwrap();
            for content in contents.iter() {
                spans.push(Span::styled(
                    content.extract_str().to_string(),
                    base_style.fg(Color::Yellow).bg(Color::DarkGray),
                ));
            }
        }
        AstNodeKind::Math { inline: true } => {
            let contents = ast.value().contents.lock().unwrap();
            for content in contents.iter() {
                spans.push(Span::styled(
                    content.extract_str().to_string(),
                    base_style.fg(Color::Magenta),
                ));
            }
        }
        AstNodeKind::Decoration {
            fontsize,
            italic,
            underline,
            deleted,
        } => {
            let mut style = base_style;
            if *fontsize > 0 {
                style = style.add_modifier(Modifier::BOLD);
            }
            if *italic {
                style = style.add_modifier(Modifier::ITALIC);
            }
            if *underline {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            if *deleted {
                style = style.add_modifier(Modifier::CROSSED_OUT);
            }
            let contents = ast.value().contents.lock().unwrap();
            for content in contents.iter() {
                // Propagate image blocks from decorated content
                let result = render_inline(content, spans, style);
                if matches!(result, InlineResult::ImageBlock { .. }) {
                    return result;
                }
            }
        }
        AstNodeKind::Image { src, alt } => {
            let mut src_resolved = src.clone();
            if let Some(gyazo_src) = get_gyazo_img_src(src) {
                src_resolved = gyazo_src;
            }
            return InlineResult::ImageBlock {
                src: src_resolved,
                alt: alt.clone(),
            };
        }
        AstNodeKind::Quote => {
            let children = ast.value().children.lock().unwrap();
            for child in children.iter() {
                render_inline(child, spans, base_style.fg(Color::DarkGray));
            }
        }
        _ => {
            // Fallback: raw text
            let text = ast.extract_str();
            if !text.is_empty() {
                spans.push(Span::styled(text.to_string(), base_style));
            }
        }
    }
    InlineResult::Inline
}
