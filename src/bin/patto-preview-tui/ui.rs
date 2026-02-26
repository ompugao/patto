use crate::backlinks::FlatEntry;
use patto::tui_renderer::{DocElement, LinkAction};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    Frame,
};
use ratatui_image::StatefulImage;
use std::path::Path;
use tui_widget_list::{ListBuilder, ListView};

use crate::app::App;
use crate::image_cache::CachedImage;

pub(crate) fn draw(frame: &mut Frame, app: &mut App, root_dir: &Path) {
    // Fullscreen image overlay
    if let Some(ref src) = app.images.fullscreen_src.clone() {
        draw_fullscreen_image(frame, app, root_dir, src);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(1),    // content
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    draw_title_bar(frame, chunks[0], app);
    draw_content(frame, chunks[1], app, root_dir);
    draw_status_bar(frame, chunks[2], app);

    if app.backlinks.visible {
        draw_backlinks_popup(frame, app);
    }
}

fn draw_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let file_name = app
        .file_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let total = app.rendered_doc.total_height(app.images.height_rows);
    let (pos, pct) = if total > 0 {
        let p = ((app.scroll_offset + 1) * 100 / total).min(100);
        (
            format!(" {}:{} ", app.scroll_offset + 1, total),
            format!(" {}% ", p),
        )
    } else {
        (" 0:0 ".to_string(), " 0% ".to_string())
    };

    let left = Line::from(vec![
        Span::styled(
            " ◉ patto ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  │  ",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled(
            format!(" {} ", file_name),
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    // Right-side: pos + percentage, right-aligned
    let right_text = format!("{}│{}", pos, pct);
    let right_len = right_text.chars().count() as u16;
    let left_len = area.width.saturating_sub(right_len);

    let right = Line::from(vec![
        Span::styled(pos, Style::default().fg(Color::DarkGray).bg(Color::Black)),
        Span::styled("│", Style::default().fg(Color::DarkGray).bg(Color::Black)),
        Span::styled(
            pct,
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    // Render left block then right-aligned block via two overlapping areas
    let left_area = Rect {
        x: area.x,
        y: area.y,
        width: left_len,
        height: 1,
    };
    let right_area = Rect {
        x: area.x + left_len,
        y: area.y,
        width: right_len,
        height: 1,
    };

    frame.render_widget(
        Paragraph::new(left).style(Style::default().bg(Color::Black)),
        left_area,
    );
    frame.render_widget(
        Paragraph::new(right).style(Style::default().bg(Color::Black)),
        right_area,
    );
}

/// Produce a new Line with chars in [char_start, char_end) highlighted with reverse video.
fn highlight_line_range(line: &Line<'static>, char_start: usize, char_end: usize) -> Line<'static> {
    let mut new_spans: Vec<Span<'static>> = Vec::new();
    let mut pos = 0usize;
    for span in line.spans.iter() {
        let span_len = span.content.chars().count();
        let span_start = pos;
        let span_end = pos + span_len;
        pos = span_end;

        if span_end <= char_start || span_start >= char_end {
            // Entirely outside highlight range
            new_spans.push(span.clone());
        } else if span_start >= char_start && span_end <= char_end {
            // Entirely inside highlight range
            new_spans.push(Span::styled(
                span.content.clone(),
                span.style.bg(Color::Yellow).fg(Color::Black),
            ));
        } else {
            // Partially overlapping — split the span
            let chars: Vec<char> = span.content.chars().collect();
            let hl_start = char_start.saturating_sub(span_start);
            let hl_end = (char_end - span_start).min(span_len);

            if hl_start > 0 {
                let before: String = chars[..hl_start].iter().collect();
                new_spans.push(Span::styled(before, span.style));
            }
            let mid: String = chars[hl_start..hl_end].iter().collect();
            new_spans.push(Span::styled(
                mid,
                span.style.bg(Color::Yellow).fg(Color::Black),
            ));
            if hl_end < span_len {
                let after: String = chars[hl_end..].iter().collect();
                new_spans.push(Span::styled(after, span.style));
            }
        }
    }
    Line::from(new_spans)
}

/// Render a single image cell into `area`.
///
/// If `focused` is true, draws a yellow border around the cell and renders the
/// image (or placeholder) inside the inner area.  Otherwise renders directly
/// into `area`.
fn draw_image_cell(
    frame: &mut Frame,
    images: &mut crate::image_cache::ImageCache,
    src: &str,
    alt: Option<&str>,
    area: Rect,
    focused: bool,
) {
    let render_area = if focused && area.height >= 3 {
        let border = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(
                " Enter:fullscreen ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = border.inner(area);
        frame.render_widget(border, area);
        inner
    } else {
        area
    };

    match images.get_mut(src) {
        Some(CachedImage::Loaded(protocol)) => {
            let image_widget = StatefulImage::default();
            frame.render_stateful_widget(image_widget, render_area, protocol);
        }
        Some(CachedImage::Failed(err)) => {
            let label = format!("[Image: {} — {}]", alt.unwrap_or(src), err);
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    label,
                    Style::default().fg(Color::Red),
                )])),
                render_area,
            );
        }
        None => {
            let label = format!("[Image: {}]", alt.unwrap_or(src));
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    label,
                    Style::default().fg(Color::DarkGray),
                )])),
                render_area,
            );
        }
    }
}

fn draw_content(frame: &mut Frame, area: Rect, app: &mut App, root_dir: &Path) {
    let height = area.height as usize;
    let img_h = app.images.height_rows;

    // Update viewport height for visible-image logic
    app.viewport_height = height;
    app.clear_stale_focus();

    // Skip elements until we reach scroll_offset rows
    let mut skip_rows = app.scroll_offset;
    let mut start_elem = 0usize;
    for (i, elem) in app.rendered_doc.elements.iter().enumerate() {
        let h = elem.height(img_h) as usize;
        if skip_rows >= h {
            skip_rows -= h;
            start_elem = i + 1;
        } else {
            start_elem = i;
            break;
        }
    }

    // Pre-load images that will be visible (only scan viewport-worth of elements)
    let mut scan_rows = 0usize;
    let image_srcs: Vec<String> = app
        .rendered_doc
        .elements
        .iter()
        .skip(start_elem)
        .take_while(|elem| {
            let h = elem.height(img_h) as usize;
            scan_rows += h;
            scan_rows <= height + img_h as usize
        })
        .filter_map(|elem| match elem {
            DocElement::Image { src, .. } => Some(vec![src.clone()]),
            DocElement::ImageRow(images) => Some(images.iter().map(|(s, _)| s.clone()).collect()),
            _ => None,
        })
        .flatten()
        .collect();
    for src in &image_srcs {
        app.images.load(src, root_dir);
    }

    // Render visible elements
    // Determine which element index is focused and get char range for text highlights
    let (focused_elem_idx, focused_char_range) = match app.focused_item() {
        Some(fi) => (Some(fi.elem_idx), Some((fi.char_start, fi.char_end))),
        None => (None, None),
    };
    let mut y = 0usize;
    for (elem_idx, elem) in app
        .rendered_doc
        .elements
        .iter()
        .enumerate()
        .skip(start_elem)
    {
        if y >= height {
            break;
        }
        let is_focused = focused_elem_idx == Some(elem_idx);
        match elem {
            DocElement::TextLine(line) => {
                let line_area = Rect::new(area.x, area.y + y as u16, area.width, 1);
                if is_focused {
                    if let Some((cs, ce)) = focused_char_range {
                        // Highlight the focused span within the line
                        let highlighted = highlight_line_range(line, cs, ce);
                        frame.render_widget(Paragraph::new(highlighted), line_area);
                    } else {
                        frame.render_widget(Paragraph::new(line.clone()), line_area);
                    }
                } else {
                    frame.render_widget(Paragraph::new(line.clone()), line_area);
                }
                y += 1;
            }
            DocElement::Spacer => {
                y += 1;
            }
            DocElement::Image { src, alt } => {
                let elem_h = elem.height(img_h).min((height - y) as u16);
                let img_area = Rect::new(area.x, area.y + y as u16, area.width, elem_h);
                draw_image_cell(
                    frame,
                    &mut app.images,
                    src,
                    alt.as_deref(),
                    img_area,
                    is_focused,
                );
                y += elem_h as usize;
            }
            DocElement::ImageRow(images) => {
                let n = images.len() as u16;
                let elem_h = elem.height(img_h).min((height - y) as u16);
                let col_w = area.width / n;
                let focused_src: Option<String> = if is_focused {
                    app.focused_item().and_then(|fi| {
                        if let LinkAction::ViewImage(s) = &fi.action {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };
                for (i, (src, alt)) in images.iter().enumerate() {
                    let x_off = area.x + i as u16 * col_w;
                    let w = if i as u16 == n - 1 {
                        area.width - i as u16 * col_w
                    } else {
                        col_w
                    };
                    let cell_area = Rect::new(x_off, area.y + y as u16, w, elem_h);
                    let this_focused = focused_src.as_deref() == Some(src.as_str());
                    draw_image_cell(
                        frame,
                        &mut app.images,
                        src,
                        alt.as_deref(),
                        cell_area,
                        this_focused,
                    );
                }
                y += elem_h as usize;
            }
        }
    }
}

fn key_badge(key: &str) -> Span<'static> {
    Span::styled(
        format!(" {} ", key),
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}

fn hint_desc(desc: &str) -> Span<'static> {
    Span::styled(format!(" {} ", desc), Style::default().fg(Color::White))
}

fn hint_sep() -> Span<'static> {
    Span::styled(" │ ", Style::default().fg(Color::DarkGray))
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let focused_action = app.focused_item().map(|fi| &fi.action);

    let mut spans: Vec<Span<'static>> = Vec::new();

    // Group 1: Quit
    spans.push(key_badge("q"));
    spans.push(hint_desc("quit"));

    spans.push(hint_sep());

    // Group 2: Scroll
    spans.push(key_badge("j/k"));
    spans.push(hint_desc("↕1"));
    spans.push(key_badge("^F/^B"));
    spans.push(hint_desc("page"));
    spans.push(key_badge("^D/^U"));
    spans.push(hint_desc("½pg"));
    spans.push(key_badge("g/G"));
    spans.push(hint_desc("top/end"));

    spans.push(hint_sep());

    // Group 3: Focus / Action
    spans.push(key_badge("Tab/S-Tab"));
    spans.push(hint_desc("focus"));
    if let Some(action) = focused_action {
        let (key, desc) = match action {
            LinkAction::OpenNote { .. } => ("↵", "open note"),
            LinkAction::OpenUrl(_) => ("↵", "open url"),
            LinkAction::ViewImage(_) => ("↵", "fullscreen"),
        };
        spans.push(key_badge(key));
        spans.push(hint_desc(desc));
    }

    spans.push(hint_sep());

    // Group 4: Tools
    spans.push(key_badge("b"));
    spans.push(hint_desc("backlinks"));
    spans.push(key_badge("+/-"));
    spans.push(hint_desc(&format!("img({})", app.images.height_rows)));
    spans.push(key_badge("r/^L"));
    spans.push(hint_desc("reload"));

    // Group 5: Back (conditional)
    if !app.nav_history.is_empty() {
        spans.push(hint_sep());
        spans.push(key_badge("BS/^O"));
        spans.push(hint_desc("back"));
    }

    let status = Line::from(spans);
    frame.render_widget(
        Paragraph::new(status).style(Style::default().bg(Color::DarkGray)),
        area,
    );
}

fn draw_backlinks_popup(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let popup_width = (area.width * 60 / 100).max(30).min(area.width - 4);
    let popup_height = (area.height * 60 / 100).max(10).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Backlinks & Two-hop Links ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Reserve the last row for the key-hint line.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    // Build the list view from flat entries.
    let entries = app.backlinks.entries.clone();
    let item_count = entries.len();

    let builder = ListBuilder::new(move |context| {
        let entry = &entries[context.index];
        let is_selected = context.is_selected;

        let line: Line<'static> = match entry {
            FlatEntry::SectionHeader(title) => {
                if title.is_empty() {
                    Line::from("")
                } else {
                    Line::from(Span::styled(
                        title.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                }
            }
            FlatEntry::BacklinkItem {
                source_file,
                line,
                context,
            } => {
                let ctx_text = context.as_deref().unwrap_or("");
                let (bullet_style, text_style, ctx_style) = if is_selected {
                    (
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    )
                } else {
                    (
                        Style::default().fg(Color::Yellow),
                        Style::default().fg(Color::White),
                        Style::default().fg(Color::DarkGray),
                    )
                };
                Line::from(vec![
                    Span::styled("  • ", bullet_style),
                    Span::styled(format!("{} (L{})", source_file, line + 1), text_style),
                    Span::styled(format!("  {}", ctx_text), ctx_style),
                ])
            }
            FlatEntry::ViaHeader(via) => Line::from(vec![
                Span::styled("  via ", Style::default().fg(Color::DarkGray)),
                Span::styled(via.clone(), Style::default().fg(Color::White)),
                Span::styled(":", Style::default().fg(Color::DarkGray)),
            ]),
            FlatEntry::TwoHopItem(name) => {
                let (arrow_style, name_style) = if is_selected {
                    (
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    )
                } else {
                    (
                        Style::default().fg(Color::Yellow),
                        Style::default().fg(Color::White),
                    )
                };
                Line::from(vec![
                    Span::styled("    → ", arrow_style),
                    Span::styled(name.clone(), name_style),
                ])
            }
            FlatEntry::Placeholder(msg) => Line::from(Span::styled(
                msg.clone(),
                Style::default().fg(Color::DarkGray),
            )),
        };

        // All entries are 1 row tall.
        let widget = EntryWidget { line };
        (widget, 1)
    });

    let list = ListView::new(builder, item_count);
    frame.render_stateful_widget(list, chunks[0], &mut app.backlinks.list_state);

    // Key hint
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " j/k:select  Enter:jump  b/Esc:close",
            Style::default().fg(Color::DarkGray),
        ))),
        chunks[1],
    );
}

/// A simple single-line widget used as a list item.
struct EntryWidget {
    line: Line<'static>,
}

impl Widget for EntryWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.line).render(area, buf);
    }
}

fn draw_fullscreen_image(frame: &mut Frame, app: &mut App, root_dir: &Path, src: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    // Load image if needed
    app.images.load(src, root_dir);

    draw_image_cell(frame, &mut app.images, src, None, chunks[0], false);

    // Status hint
    let hint = Line::from(vec![
        Span::styled(" Esc", Style::default().fg(Color::Yellow)),
        Span::styled(":close ", Style::default().fg(Color::DarkGray)),
        Span::styled(src.to_string(), Style::default().fg(Color::White)),
    ]);
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().bg(Color::DarkGray)),
        chunks[1],
    );
}
