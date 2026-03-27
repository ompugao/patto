use std::collections::HashMap;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::parser::{AstNode, AstNodeKind, Property, TaskStatus};
use crate::utils::get_gyazo_img_src;

/// Action to perform when a focusable item is activated.
#[derive(Debug, Clone)]
pub enum LinkAction {
    /// Open a wiki-linked note, optionally jumping to an anchor.
    OpenNote {
        name: String,
        anchor: Option<String>,
    },
    /// Jump to an anchor within the current document (self-link).
    JumpToAnchor { anchor: String },
    /// Open a URL in the system browser.
    OpenUrl(String),
    /// View an image fullscreen.
    ViewImage(String),
}

/// A focusable item in the rendered document (link, image, etc.).
#[derive(Debug, Clone)]
pub struct FocusableItem {
    /// Index into `RenderedDoc.elements` that contains this item.
    pub elem_idx: usize,
    /// Character offset where the focusable span starts within the text line.
    /// For images, this is 0.
    pub char_start: usize,
    /// Character offset where the focusable span ends (exclusive).
    /// For images, this equals 0.
    pub char_end: usize,
    /// Action to perform on Enter.
    pub action: LinkAction,
}

/// A segment within a line that mixes text and inline math.
#[derive(Debug, Clone)]
pub enum InlineSegment {
    /// Styled text spans.
    Text(Vec<Span<'static>>),
    /// Inline math expression (raw LaTeX content).
    Math(String),
}

/// A single element in the rendered document.
#[derive(Debug, Clone)]
pub enum DocElement {
    /// A styled text line. The `usize` is the 0-indexed source row from the AST.
    TextLine(Line<'static>, usize),
    /// An image to render via kitty/sixel.
    Image {
        src: String,
        alt: Option<String>,
        indent: usize,
    },
    /// Multiple images on the same line, rendered side by side.
    /// The second field is the indentation level.
    ImageRow(Vec<(String, Option<String>)>, usize),
    /// A math block to render as an image (LaTeX source).
    Math { content: String, indent: usize },
    /// A line containing inline math segments (text interleaved with math images).
    InlineMathLine {
        segments: Vec<InlineSegment>,
        source_row: usize,
    },
    /// A blank line.
    Spacer,
}

impl DocElement {
    /// Whether this element is an image.
    pub fn is_image(&self) -> bool {
        matches!(self, DocElement::Image { .. } | DocElement::ImageRow(..))
    }
}

/// A fully rendered document ready for display.
pub struct RenderedDoc {
    pub elements: Vec<DocElement>,
    /// All focusable items (links, images) in document order.
    pub focusables: Vec<FocusableItem>,
    /// Map from anchor name to element index.
    pub anchors: HashMap<String, usize>,
}

impl RenderedDoc {}

/// Configuration for the TUI rendering pass.
///
/// Bundled into a struct so new options can be added without changing the
/// signatures of `render_ast`, `render_node`, and `render_inline`.
pub struct RenderConfig<'a> {
    /// Optional syntect theme name for code-block highlighting.
    pub syntax_theme: Option<&'a str>,
    /// Whether to emit `InlineMathLine` elements for inline math (`[$…$]`).
    /// When `false`, inline math is rendered as plain magenta text.
    pub inline_math: bool,
}

/// Render an AST root node into a flat list of DocElements.
pub fn render_ast(ast: &AstNode, cfg: &RenderConfig<'_>) -> RenderedDoc {
    let mut elements = Vec::new();
    let mut focusables = Vec::new();
    let mut anchors = HashMap::new();
    render_node(
        ast,
        &mut elements,
        &mut focusables,
        &mut anchors,
        0,
        cfg,
    );
    RenderedDoc {
        elements,
        focusables,
        anchors,
    }
}

/// Result of inline rendering — may contain image blocks that need to be
/// emitted between text line fragments.
enum InlineResult {
    /// Pure inline content (appended to current spans).
    Inline,
    /// An image block that must be emitted as a separate DocElement.
    ImageBlock { src: String, alt: Option<String> },
    /// An inline math expression (when inline_math rendering is enabled).
    InlineMath { content: String },
}

/// Returns true if `spans` contains any non-whitespace text.
fn spans_have_content(spans: &[Span<'_>]) -> bool {
    spans.iter().any(|s| !s.content.trim().is_empty())
}

/// Flush `buf` as a single `Image` (len == 1) or `ImageRow` (len > 1) element.
fn flush_image_row(
    buf: &mut Vec<(String, Option<String>)>,
    elements: &mut Vec<DocElement>,
    focusables: &mut Vec<FocusableItem>,
    indent: usize,
) {
    if buf.is_empty() {
        return;
    }
    if buf.len() == 1 {
        let (src, alt) = buf.remove(0);
        focusables.push(FocusableItem {
            elem_idx: elements.len(),
            char_start: 0,
            char_end: 0,
            action: LinkAction::ViewImage(src.clone()),
        });
        elements.push(DocElement::Image { src, alt, indent });
    } else {
        for (src, _alt) in buf.iter() {
            focusables.push(FocusableItem {
                elem_idx: elements.len(),
                char_start: 0,
                char_end: 0,
                action: LinkAction::ViewImage(src.clone()),
            });
        }
        elements.push(DocElement::ImageRow(std::mem::take(buf), indent));
    }
    buf.clear();
}

fn render_node(
    ast: &AstNode,
    elements: &mut Vec<DocElement>,
    focusables: &mut Vec<FocusableItem>,
    anchors: &mut HashMap<String, usize>,
    indent: usize,
    cfg: &RenderConfig<'_>,
) {
    match ast.kind() {
        AstNodeKind::Dummy => {
            let children = ast.value().children.lock().unwrap();
            for child in children.iter() {
                render_node(
                    child,
                    elements,
                    focusables,
                    anchors,
                    indent,
                    cfg,
                );
            }
        }
        AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } => {
            let is_quote = matches!(ast.kind(), AstNodeKind::QuoteContent { .. });

            // Check if this line is a block container (only content is a block element)
            let contents = ast.value().contents.lock().unwrap();
            let is_block_container = contents.len() == 1
                && matches!(
                    contents[0].kind(),
                    AstNodeKind::Quote
                        | AstNodeKind::Code { inline: false, .. }
                        | AstNodeKind::Math { inline: false }
                        | AstNodeKind::Table { .. }
                );

            if is_block_container {
                // Delegate to the block element renderer
                let block_node = contents[0].clone();
                drop(contents);
                render_node(
                    &block_node,
                    elements,
                    focusables,
                    anchors,
                    indent,
                    cfg,
                );
                // Still render children (nested lines after the block)
                let children = ast.value().children.lock().unwrap();
                for child in children.iter() {
                    render_node(
                        child,
                        elements,
                        focusables,
                        anchors,
                        indent + 1,
                        cfg,
                    );
                }
                return;
            }
            drop(contents);

            // Record anchors defined on this line
            let current_elem_idx = elements.len();
            for property in properties {
                if let Property::Anchor { name, .. } = property {
                    anchors.insert(name.to_lowercase(), current_elem_idx);
                }
            }

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
                prefix_spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
            }

            // Task icon / bullet
            if let Some(status) = task_status {
                let (icon, color) = match status {
                    TaskStatus::Done => ("✓ ", Color::Green),
                    TaskStatus::Doing => ("◑ ", Color::Yellow),
                    _ => ("○ ", Color::White),
                };
                prefix_spans.push(Span::styled(icon.to_string(), Style::default().fg(color)));
            } else if !is_quote && indent > 0 {
                let contents = ast.value().contents.lock().unwrap();
                let is_blank = contents.is_empty()
                    || contents.iter().all(|c| {
                        matches!(c.kind(), AstNodeKind::Text) && c.extract_str().trim().is_empty()
                    });
                drop(contents);
                if !is_blank {
                    prefix_spans.push(Span::raw("• "));
                }
            }

            // Inline contents — collect spans, grouping consecutive images into ImageRow
            let base_style = if is_done {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else if is_quote {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::ITALIC)
            } else {
                Style::default()
            };

            let mut spans = prefix_spans.clone();
            // Buffer for consecutive images (no non-whitespace text between them).
            let mut image_row_buf: Vec<(String, Option<String>)> = Vec::new();
            // Whether any InlineMath result was returned for this line.
            let mut has_inline_math = false;
            // Segments collected when inline math is present.
            let mut segments: Vec<InlineSegment> = Vec::new();

            let contents = ast.value().contents.lock().unwrap();
            for content in contents.iter() {
                let result = render_inline(
                    content,
                    &mut spans,
                    base_style,
                    focusables,
                    elements.len(),
                    cfg,
                );
                match result {
                    InlineResult::ImageBlock { src, alt } => {
                        // If spans have real text, flush them before starting an image group
                        if spans_have_content(&spans) {
                            elements.push(DocElement::TextLine(
                                Line::from(std::mem::take(&mut spans)),
                                ast.location().row,
                            ));
                            // Also flush any existing image row — text breaks the group
                            flush_image_row(&mut image_row_buf, elements, focusables, indent);
                        } else if !image_row_buf.is_empty() {
                            // Consecutive image — keep accumulating (spans are only whitespace/indent)
                            spans = vec![Span::raw("  ".repeat(indent + 1))];
                        } else {
                            spans = vec![Span::raw("  ".repeat(indent + 1))];
                        }
                        image_row_buf.push((src, alt));
                    }
                    InlineResult::InlineMath {
                        content: math_content,
                    } => {
                        has_inline_math = true;
                        // Flush any accumulated spans into a Text segment
                        if !spans.is_empty() {
                            segments.push(InlineSegment::Text(std::mem::take(&mut spans)));
                            spans = Vec::new();
                        }
                        segments.push(InlineSegment::Math(math_content));
                    }
                    InlineResult::Inline => {
                        // Non-image content — flush any pending image row first
                        if !image_row_buf.is_empty() {
                            flush_image_row(&mut image_row_buf, elements, focusables, indent);
                        }
                    }
                }
            }
            // Flush any trailing image row
            flush_image_row(&mut image_row_buf, elements, focusables, indent);

            // Deadline
            for property in properties {
                if let Property::Task { status, due, .. } = property {
                    if !matches!(status, TaskStatus::Done) {
                        let due_str = format!(" [{}]", due);
                        spans.push(Span::styled(due_str, Style::default().fg(Color::Red)));
                    }
                }
            }

            // Emit the line: InlineMathLine if inline math was found, else plain TextLine
            if has_inline_math {
                // Flush any remaining spans as a final Text segment
                if !spans.is_empty() {
                    segments.push(InlineSegment::Text(spans));
                }
                elements.push(DocElement::InlineMathLine {
                    segments,
                    source_row: ast.location().row,
                });
            } else {
                // Flush remaining spans (always emit to preserve blank lines)
                elements.push(DocElement::TextLine(Line::from(spans), ast.location().row));
            }

            // Children (nested lines)
            let children = ast.value().children.lock().unwrap();
            for child in children.iter() {
                render_node(
                    child,
                    elements,
                    focusables,
                    anchors,
                    indent + 1,
                    cfg,
                );
            }
        }
        AstNodeKind::Quote => {
            let children = ast.value().children.lock().unwrap();
            for child in children.iter() {
                render_node(
                    child,
                    elements,
                    focusables,
                    anchors,
                    indent,
                    cfg,
                );
            }
        }
        AstNodeKind::Math { inline } => {
            if *inline {
                // Handled as inline content in parent Line
            } else {
                let children = ast.value().children.lock().unwrap();
                let content: String = children
                    .iter()
                    .map(|c| c.extract_str().to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                drop(children);
                elements.push(DocElement::Math { content, indent });
            }
        }
        AstNodeKind::Code { lang, inline } => {
            if *inline {
                // Handled as inline content in parent Line
            } else {
                let prefix = if indent > 0 {
                    "  ".repeat(indent)
                } else {
                    "  ".to_string()
                };

                // Show language label if present
                if !lang.is_empty() {
                    elements.push(DocElement::TextLine(
                        Line::from(vec![
                            Span::raw(prefix.clone()),
                            Span::styled(
                                format!(" {} ", lang),
                                Style::default()
                                    .fg(Color::Cyan)
                                    .bg(Color::DarkGray)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        ast.location().row,
                    ));
                }

                let children = ast.value().children.lock().unwrap();
                let raw_lines: Vec<String> = children
                    .iter()
                    .map(|c| c.extract_str().replace('\t', "    "))
                    .collect();
                drop(children);
                let raw_refs: Vec<&str> = raw_lines.iter().map(|s| s.as_str()).collect();
                let highlighted =
                    crate::syntax_highlight::highlight_code(lang, &raw_refs, cfg.syntax_theme);
                for (line_spans, _raw) in highlighted.into_iter().zip(raw_lines.iter()) {
                    let mut spans = vec![Span::raw(prefix.clone())];
                    if line_spans.is_empty() {
                        // empty line — push a blank styled span to preserve height
                        spans.push(Span::raw(""));
                    } else {
                        spans.extend(line_spans);
                    }
                    elements.push(DocElement::TextLine(Line::from(spans), ast.location().row));
                }
            }
        }
        AstNodeKind::Image { src, alt } => {
            let mut src_resolved = src.clone();
            if let Some(gyazo_src) = get_gyazo_img_src(src) {
                src_resolved = gyazo_src;
            }
            focusables.push(FocusableItem {
                elem_idx: elements.len(),
                char_start: 0,
                char_end: 0,
                action: LinkAction::ViewImage(src_resolved.clone()),
            });
            elements.push(DocElement::Image {
                src: src_resolved,
                alt: alt.clone(),
                indent,
            });
            if let Some(alt_text) = alt {
                elements.push(DocElement::TextLine(
                    Line::from(vec![Span::styled(
                        format!("  {}", alt_text),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    )]),
                    ast.location().row,
                ));
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
            elements.push(DocElement::TextLine(
                Line::from(vec![Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )]),
                ast.location().row,
            ));
        }
        AstNodeKind::Table { caption } => {
            if let Some(cap) = caption {
                elements.push(DocElement::TextLine(
                    Line::from(vec![
                        Span::raw("  ".repeat(indent)),
                        Span::styled(cap.clone(), Style::default().add_modifier(Modifier::BOLD)),
                    ]),
                    ast.location().row,
                ));
            }
            let children = ast.value().children.lock().unwrap();
            for child in children.iter() {
                render_table_row(child, elements, focusables, indent, child.location().row);
            }
        }
        AstNodeKind::TableRow | AstNodeKind::TableColumn => {
            // Handled inside Table rendering
        }
    }
}

fn render_table_row(
    ast: &AstNode,
    elements: &mut Vec<DocElement>,
    focusables: &mut Vec<FocusableItem>,
    indent: usize,
    source_row: usize,
) {
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
            let no_inline = RenderConfig {
                syntax_theme: None,
                inline_math: false,
            };
            render_inline(
                c,
                &mut spans,
                Style::default(),
                focusables,
                elements.len(),
                &no_inline,
            );
        }
    }
    spans.push(Span::styled(" │", Style::default().fg(Color::DarkGray)));
    elements.push(DocElement::TextLine(Line::from(spans), source_row));
}

/// Count total display column width of accumulated spans (Unicode-aware).
/// CJK and other double-width characters each count as 2 columns.
fn spans_char_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|s| s.content.width()).sum()
}

fn render_inline(
    ast: &AstNode,
    spans: &mut Vec<Span<'static>>,
    base_style: Style,
    focusables: &mut Vec<FocusableItem>,
    current_elem_idx: usize,
    cfg: &RenderConfig<'_>,
) -> InlineResult {
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
            let text = format!("[{}]", display);
            let char_start = spans_char_width(spans);
            let char_end = char_start + text.chars().count();
            spans.push(Span::styled(
                text,
                base_style
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED),
            ));
            // Self-link: empty link name with anchor -> jump within current doc
            let action = if link.is_empty() {
                if let Some(anc) = anchor {
                    LinkAction::JumpToAnchor {
                        anchor: anc.clone(),
                    }
                } else {
                    // Edge case: empty link with no anchor (shouldn't happen normally)
                    LinkAction::OpenNote {
                        name: link.clone(),
                        anchor: anchor.clone(),
                    }
                }
            } else {
                LinkAction::OpenNote {
                    name: link.clone(),
                    anchor: anchor.clone(),
                }
            };
            focusables.push(FocusableItem {
                elem_idx: current_elem_idx,
                char_start,
                char_end,
                action,
            });
        }
        AstNodeKind::Link { link, title } => {
            let display = title.as_deref().unwrap_or(link.as_str());
            let char_start = spans_char_width(spans);
            let char_end = char_start + display.chars().count();
            spans.push(Span::styled(
                display.to_string(),
                base_style
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED),
            ));
            focusables.push(FocusableItem {
                elem_idx: current_elem_idx,
                char_start,
                char_end,
                action: LinkAction::OpenUrl(link.clone()),
            });
        }
        AstNodeKind::Embed { link, title } => {
            let is_pdf = link.to_lowercase().ends_with(".pdf");
            let display = title.as_deref().unwrap_or(link.as_str());
            let text = if is_pdf {
                format!("[PDF: {}]", display)
            } else {
                format!("[embed: {}]", display)
            };
            let char_start = spans_char_width(spans);
            let char_end = char_start + text.chars().count();
            let style = if is_pdf {
                base_style
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                base_style
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED)
            };
            spans.push(Span::styled(text, style));
            focusables.push(FocusableItem {
                elem_idx: current_elem_idx,
                char_start,
                char_end,
                action: LinkAction::OpenUrl(link.clone()),
            });
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
            if cfg.inline_math {
                // Return as InlineMath so the caller can render it as an image
                let content: String = contents
                    .iter()
                    .map(|c| c.extract_str().to_string())
                    .collect::<Vec<_>>()
                    .join("");
                return InlineResult::InlineMath { content };
            } else {
                // Fallback: display raw LaTeX as magenta text
                for content in contents.iter() {
                    spans.push(Span::styled(
                        content.extract_str().to_string(),
                        base_style.fg(Color::Magenta),
                    ));
                }
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
                let result = render_inline(
                    content,
                    spans,
                    style,
                    focusables,
                    current_elem_idx,
                    cfg,
                );
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
                render_inline(
                    child,
                    spans,
                    base_style.fg(Color::DarkGray),
                    focusables,
                    current_elem_idx,
                    cfg,
                );
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
