use crate::backlinks::FlatEntry;
use crate::config::TasksPanelPosition;
use crate::tasks::TaskEntry;
use patto::tui_renderer::{DocElement, InlineSegment, LinkAction};
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
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::image_cache::CachedImage;
use crate::search::SearchDirection;
use crate::wrap::{elem_height, wrap_line, WrapConfig};

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

    if app.tasks.visible {
        draw_tasks_panel(frame, app);
    }
}

fn draw_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let file_name = app
        .file_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let total = crate::wrap::total_height(
        &app.rendered_doc.elements,
        app.wrap_config().as_ref(),
        app.images.height_rows,
        Some(&app.images.elem_heights),
    );
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

/// Produce a new `Line` with multiple character ranges highlighted.
///
/// Each entry in `ranges` is `(char_start, char_end, is_current)`:
/// - `is_current = true`  → current search match: Yellow BG + Black FG
/// - `is_current = false` → other matches: Magenta BG + Black FG (subtle)
///
/// Ranges must not overlap. They are sorted by `char_start` before processing.
/// Handles syntect's many small fg-only spans correctly by splitting at boundaries.
fn highlight_line_multi(line: &Line<'static>, ranges: &[(usize, usize, bool)]) -> Line<'static> {
    if ranges.is_empty() {
        return line.clone();
    }

    let mut sorted_ranges = ranges.to_vec();
    sorted_ranges.sort_by_key(|(s, _, _)| *s);

    // Collect all span boundaries from the source line.
    let mut new_spans: Vec<Span<'static>> = Vec::new();
    let mut char_pos = 0usize;
    let mut range_iter = sorted_ranges.iter().peekable();

    for span in line.spans.iter() {
        let span_len = span.content.chars().count();
        let span_start = char_pos;
        let span_end = char_pos + span_len;
        char_pos = span_end;

        let mut cursor = span_start;
        let chars: Vec<char> = span.content.chars().collect();

        while cursor < span_end {
            // Skip ranges that end before cursor
            while range_iter.peek().map(|&&(_, e, _)| e <= cursor) == Some(true) {
                range_iter.next();
            }

            match range_iter.peek().map(|&&(s, e, c)| (s, e, c)) {
                None => {
                    // No more ranges — emit the rest of this span unstyled
                    let text: String = chars[cursor - span_start..].iter().collect();
                    if !text.is_empty() {
                        new_spans.push(Span::styled(text, span.style));
                    }
                    cursor = span_end;
                }
                Some((hl_start, hl_end, is_current)) => {
                    if hl_start >= span_end {
                        // Range starts after this span — emit rest of span unstyled
                        let text: String = chars[cursor - span_start..].iter().collect();
                        if !text.is_empty() {
                            new_spans.push(Span::styled(text, span.style));
                        }
                        cursor = span_end;
                    } else if hl_start > cursor {
                        // Unstyled section before range starts
                        let end = hl_start.min(span_end);
                        let text: String = chars[cursor - span_start..end - span_start]
                            .iter()
                            .collect();
                        if !text.is_empty() {
                            new_spans.push(Span::styled(text, span.style));
                        }
                        cursor = end;
                    } else {
                        // Highlighted section
                        let end = hl_end.min(span_end);
                        let text: String = chars[cursor - span_start..end - span_start]
                            .iter()
                            .collect();
                        if !text.is_empty() {
                            let hl_style = if is_current {
                                span.style.bg(Color::Yellow).fg(Color::Black)
                            } else {
                                span.style.bg(Color::Magenta).fg(Color::Black)
                            };
                            new_spans.push(Span::styled(text, hl_style));
                        }
                        cursor = end;
                        // If the range ends within this span, consume it
                        if hl_end <= span_end {
                            range_iter.next();
                        }
                    }
                }
            }
        }
    }

    Line::from(new_spans)
}
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
    let wrap = app.wrap;
    let showbreak = app.showbreak.clone();
    // Snapshot elem_heights so the closure doesn't hold a borrow on app.images
    // while we later call app.images.load() / load_math() mutably.
    let elem_heights = app.images.elem_heights.clone();
    // Update viewport dimensions (used by wrap-aware scroll calculations)
    app.viewport_width = area.width;
    app.viewport_height = height;
    app.clear_stale_focus();

    // Closure: display height of an element.
    // Pre-build WrapConfig so we don't rebuild it per element.
    let wrap_cfg = WrapConfig::new(area.width as usize, showbreak.as_str());
    let elem_h = |elem: &DocElement| -> usize {
        let cfg_opt = if wrap && area.width > 0 {
            Some(&wrap_cfg)
        } else {
            None
        };
        elem_height(elem, cfg_opt, img_h, Some(&elem_heights))
    };

    // Skip elements until we reach scroll_offset rows
    let mut skip_rows = app.scroll_offset;
    let mut start_elem = 0usize;
    for (i, elem) in app.rendered_doc.elements.iter().enumerate() {
        let h = elem_h(elem);
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
            let h = elem_h(elem);
            scan_rows += h;
            scan_rows <= height + img_h as usize
        })
        .filter_map(|elem| match elem {
            DocElement::Image { src, .. } => Some(vec![src.clone()]),
            DocElement::ImageRow(images, ..) => {
                Some(images.iter().map(|(s, _)| s.clone()).collect())
            }
            _ => None,
        })
        .flatten()
        .collect();
    for src in &image_srcs {
        app.images.load(src, root_dir);
    }

    // Pre-load math blocks in the viewport
    let math_contents: Vec<String> = app
        .rendered_doc
        .elements
        .iter()
        .skip(start_elem)
        .take_while({
            let mut rows = 0usize;
            move |elem| {
                rows += elem_h(elem);
                rows <= height + img_h as usize
            }
        })
        .filter_map(|elem| {
            if let DocElement::Math { content, .. } = elem {
                Some(content.clone())
            } else {
                None
            }
        })
        .collect();
    for content in &math_contents {
        app.images.load_math(content);
    }

    // Pre-load inline math segments in the viewport
    if app.inline_math_rendering {
        let inline_math_contents: Vec<String> = app
            .rendered_doc
            .elements
            .iter()
            .skip(start_elem)
            .take_while({
                let mut rows = 0usize;
                move |elem| {
                    rows += elem_h(elem);
                    rows <= height + img_h as usize
                }
            })
            .filter_map(|elem| {
                if let DocElement::InlineMathLine { segments, .. } = elem {
                    Some(
                        segments
                            .iter()
                            .filter_map(|s| {
                                if let InlineSegment::Math(c) = s {
                                    Some(c.clone())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            })
            .flatten()
            .collect();
        for content in &inline_math_contents {
            app.images.load_inline_math(content);
        }
    }

    // Render visible elements
    // Determine which element index is focused and get char range for text highlights
    let (focused_elem_idx, focused_char_range) = match app.focused_item() {
        Some(fi) => (Some(fi.elem_idx), Some((fi.char_start, fi.char_end))),
        None => (None, None),
    };
    // Snapshot search state for the render pass (avoid repeated borrows of app).
    let search_matches_snapshot: Vec<(usize, usize, usize, bool)> = app
        .search
        .as_ref()
        .map(|s| {
            s.matches
                .iter()
                .enumerate()
                .map(|(i, m)| (m.elem_idx, m.char_start, m.char_end, Some(i) == s.match_idx))
                .collect()
        })
        .unwrap_or_default();

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
            DocElement::TextLine(line, _) => {
                let true_lh = elem_h(elem);
                let lh = true_lh.min(height - y) as u16;
                let line_area = Rect::new(area.x, area.y + y as u16, area.width, lh);

                // Build search highlight ranges for this element.
                let search_ranges: Vec<(usize, usize, bool)> = search_matches_snapshot
                    .iter()
                    .filter(|(eidx, _, _, _)| *eidx == elem_idx)
                    .map(|(_, cs, ce, cur)| (*cs, *ce, *cur))
                    .collect();

                // Compose highlights: search first, then focus on top.
                let base_line = {
                    let after_search = if search_ranges.is_empty() {
                        line.clone()
                    } else {
                        highlight_line_multi(line, &search_ranges)
                    };
                    if is_focused {
                        if let Some((cs, ce)) = focused_char_range {
                            highlight_line_range(&after_search, cs, ce)
                        } else {
                            after_search
                        }
                    } else {
                        after_search
                    }
                };

                if wrap {
                    // Manual wrapping with showbreak prefix on continuation rows
                    let sub_rows = wrap_line(
                        &base_line,
                        &WrapConfig::new(area.width as usize, &showbreak),
                    );
                    for (row_i, sub_row) in sub_rows.iter().enumerate().take(lh as usize) {
                        let row_area =
                            Rect::new(area.x, area.y + y as u16 + row_i as u16, area.width, 1);
                        frame.render_widget(Paragraph::new(sub_row.clone()), row_area);
                    }
                } else {
                    frame.render_widget(Paragraph::new(base_line), line_area);
                }

                // Overlay ↩ at the right edge of every wrapped row that has more content.
                // The last column is always empty (WrapConfig::needs_break uses >= to reserve it).
                if wrap && lh > 1 {
                    let indicator_style = Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM);
                    let indicator_count = if lh < true_lh as u16 {
                        lh
                    } else {
                        lh.saturating_sub(1)
                    };
                    let ind_x = area.x + area.width - 1;
                    for row_i in 0..indicator_count {
                        let ind_y = area.y + y as u16 + row_i;
                        if let Some(c) = frame.buffer_mut().cell_mut((ind_x, ind_y)) {
                            c.set_symbol("↩");
                            c.set_style(indicator_style);
                        }
                    }
                }
                y += lh as usize;
            }
            DocElement::Spacer => {
                y += 1;
            }
            DocElement::Image { src, alt, indent } => {
                let elem_h = (elem_height(elem, None, img_h, None) as u16).min((height - y) as u16);
                let indent_w = (*indent as u16) * 2;
                let img_area = Rect::new(
                    area.x + indent_w,
                    area.y + y as u16,
                    area.width.saturating_sub(indent_w),
                    elem_h,
                );
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
            DocElement::ImageRow(images, indent) => {
                let n = images.len() as u16;
                let elem_h = (elem_height(elem, None, img_h, None) as u16).min((height - y) as u16);
                let indent_w = (*indent as u16) * 2;
                let row_width = area.width.saturating_sub(indent_w);
                let col_w = row_width / n;
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
                    let x_off = area.x + indent_w + i as u16 * col_w;
                    let w = if i as u16 == n - 1 {
                        row_width - i as u16 * col_w
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
            DocElement::Math { content, indent } => {
                let elem_h = (elem_height(elem, None, img_h, Some(&elem_heights)) as u16)
                    .min((height - y) as u16);
                let indent_w = (*indent as u16) * 2;
                let math_area = Rect::new(
                    area.x + indent_w,
                    area.y + y as u16,
                    area.width.saturating_sub(indent_w),
                    elem_h,
                );
                match app.images.get_mut(content) {
                    Some(_) => {
                        // Image already in cache (Loaded or Failed) — render as image cell
                        draw_image_cell(frame, &mut app.images, content, None, math_area, false);
                    }
                    None => {
                        // No picker or not yet loaded — text fallback
                        let prefix = "  ".to_string();
                        let lines: Vec<Line<'static>> = std::iter::once(Line::from(vec![
                            Span::raw(prefix.clone()),
                            Span::styled(
                                "  [math]  ",
                                Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(Modifier::DIM),
                            ),
                        ]))
                        .chain(content.lines().map(|l| {
                            Line::from(vec![
                                Span::raw(prefix.clone()),
                                Span::styled(l.to_string(), Style::default().fg(Color::Magenta)),
                            ])
                        }))
                        .take(elem_h as usize)
                        .collect();
                        frame.render_widget(Paragraph::new(lines), math_area);
                    }
                }
                y += elem_h as usize;
            }
            DocElement::InlineMathLine { segments, .. } => {
                // Height of this element (can be > 1 for tall expressions like fractions).
                let elem_h = elem_h(elem) as u16;
                // Text segments are vertically centred; math images anchor to top.
                let text_row_y = area.y + y as u16 + elem_h / 2;
                let math_row_y = area.y + y as u16;
                let mut x_cursor = area.x;
                let max_x = area.x + area.width;
                for segment in segments {
                    if x_cursor >= max_x {
                        break;
                    }
                    match segment {
                        InlineSegment::Text(spans) => {
                            // Use Unicode display width (double-width CJK chars count as 2 cols).
                            let text_width: u16 =
                                spans.iter().map(|s| s.content.width() as u16).sum();
                            let avail = max_x.saturating_sub(x_cursor);
                            let w = text_width.min(avail);
                            if w > 0 {
                                let text_area = Rect::new(x_cursor, text_row_y, w, 1);
                                frame.render_widget(
                                    Paragraph::new(Line::from(spans.clone())),
                                    text_area,
                                );
                            }
                            x_cursor = x_cursor.saturating_add(text_width).min(max_x);
                        }
                        InlineSegment::Math(content) => {
                            let key = crate::image_cache::ImageCache::inline_math_key(content);
                            let cols = app.images.inline_math_cols(content).unwrap_or(4);
                            let math_rows = app
                                .images
                                .elem_heights
                                .get(key.as_str())
                                .copied()
                                .unwrap_or(1);
                            let avail = max_x.saturating_sub(x_cursor);
                            let w = cols.min(avail);
                            let avail_rows = (height - y) as u16;
                            let h = math_rows.min(avail_rows).max(1);
                            if w > 0 {
                                let math_area = Rect::new(x_cursor, math_row_y, w, h);
                                match app.images.get_mut(&key) {
                                    Some(_) => {
                                        draw_image_cell(
                                            frame,
                                            &mut app.images,
                                            &key,
                                            None,
                                            math_area,
                                            false,
                                        );
                                    }
                                    None => {
                                        // Fallback: show raw LaTeX in magenta
                                        frame.render_widget(
                                            Paragraph::new(Line::from(vec![Span::styled(
                                                content.as_str().to_string(),
                                                Style::default().fg(Color::Magenta),
                                            )])),
                                            Rect::new(x_cursor, text_row_y, w, 1),
                                        );
                                    }
                                }
                            }
                            x_cursor = x_cursor.saturating_add(cols).min(max_x);
                        }
                    }
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

/// Build right-aligned search status spans and their display width.
///
/// Returns `None` when there is no active search to display.
fn search_right_status(app: &App) -> Option<(Vec<Span<'static>>, u16)> {
    let search = app.search.as_ref()?;

    let dir_char = match search.direction {
        SearchDirection::Forward => "/",
        SearchDirection::Backward => "?",
    };

    if search.typing {
        // While typing: show match count on the right only when there are results.
        if search.query.is_empty() {
            return None;
        }
        let count_text = if search.matches.is_empty() {
            " no match ".to_string()
        } else {
            let cur = search.match_idx.map(|i| i + 1).unwrap_or(0);
            format!(" {}/{} ", cur, search.matches.len())
        };
        let width = count_text.chars().count() as u16;
        Some((
            vec![Span::styled(
                count_text,
                Style::default().fg(Color::DarkGray).bg(Color::Black),
            )],
            width,
        ))
    } else {
        // Confirmed search: show  / query  cur/total  at the right.
        if search.query.is_empty() {
            return None;
        }
        let (count_text, count_style) = if search.matches.is_empty() {
            (
                " no match ".to_string(),
                Style::default().fg(Color::Red).bg(Color::Black),
            )
        } else {
            let cur = search.match_idx.map(|i| i + 1).unwrap_or(0);
            (
                format!(" {}/{} ", cur, search.matches.len()),
                Style::default().fg(Color::DarkGray).bg(Color::Black),
            )
        };
        let query_text = format!(" {} ", search.query);
        let dir_width = 1u16;
        let query_width = query_text.chars().count() as u16;
        let count_width = count_text.chars().count() as u16;
        let sep_width = 1u16;
        let total_width = sep_width + dir_width + query_width + count_width;
        Some((
            vec![
                Span::styled(
                    "│",
                    Style::default().fg(Color::DarkGray).bg(Color::DarkGray),
                ),
                Span::styled(
                    dir_char,
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    query_text,
                    Style::default().fg(Color::White).bg(Color::Black),
                ),
                Span::styled(count_text, count_style),
            ],
            total_width,
        ))
    }
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let right = search_right_status(app);
    let right_width = right.as_ref().map(|(_, w)| *w).unwrap_or(0);
    let left_width = area.width.saturating_sub(right_width);

    let left_area = Rect {
        width: left_width,
        ..area
    };
    let right_area = Rect {
        x: area.x + left_width,
        y: area.y,
        width: right_width,
        height: 1,
    };

    // Render right-side search status (if any).
    if let Some((spans, _)) = right {
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Black)),
            right_area,
        );
    }

    // Search input mode: show the search prompt on the left instead of hints.
    if let Some(search) = &app.search {
        if search.typing {
            let dir_char = match search.direction {
                SearchDirection::Forward => "/",
                SearchDirection::Backward => "?",
            };
            // Split query at cursor: before | cursor_char_or_block | after
            let before = search.query[..search.cursor].to_string();
            let cursor_style = Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD);
            let (cursor_span, after) = if search.cursor < search.query.len() {
                let c = search.query[search.cursor..].chars().next().unwrap();
                let after = search.query[search.cursor + c.len_utf8()..].to_string();
                (Span::styled(c.to_string(), cursor_style), after)
            } else {
                (Span::styled(" ", cursor_style), String::new())
            };
            let mut prompt_spans = vec![
                Span::styled(
                    dir_char,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(before, Style::default().fg(Color::White)),
                cursor_span,
            ];
            if !after.is_empty() {
                prompt_spans.push(Span::styled(after, Style::default().fg(Color::White)));
            }
            frame.render_widget(
                Paragraph::new(Line::from(prompt_spans)).style(Style::default().bg(Color::Black)),
                left_area,
            );
            return;
        }
    }

    // Normal mode: render hint bar on the left.
    let focused_action = app.focused_item().map(|fi| &fi.action);

    let mut spans: Vec<Span<'static>> = vec![
        // Group 1: Quit
        key_badge("q"),
        hint_desc("quit"),
        hint_sep(),
        // Group 2: Scroll
        key_badge("j/k"),
        hint_desc("↕1"),
        key_badge("^F/^B"),
        hint_desc("page"),
        key_badge("^D/^U"),
        hint_desc("½pg"),
        key_badge("g/G"),
        hint_desc("top/end"),
    ];

    spans.push(hint_sep());

    // Group 3: Search
    spans.push(key_badge("/"));
    spans.push(hint_desc("search"));
    if let Some(search) = &app.search {
        if !search.matches.is_empty() {
            spans.push(key_badge("n/N"));
            spans.push(hint_desc("next"));
        }
    }

    spans.push(hint_sep());

    // Group 4: Focus / Action
    spans.push(key_badge("Tab/S-Tab"));
    spans.push(hint_desc("focus"));
    if let Some(action) = focused_action {
        let (key, desc) = match action {
            LinkAction::OpenNote { .. } => ("↵", "open note"),
            LinkAction::JumpToAnchor { .. } => ("↵", "jump"),
            LinkAction::OpenUrl(_) => ("↵", "open url"),
            LinkAction::ViewImage(_) => ("↵", "fullscreen"),
        };
        spans.push(key_badge(key));
        spans.push(hint_desc(desc));
    }

    spans.push(hint_sep());

    // Group 5: Tools
    spans.push(key_badge("b"));
    spans.push(hint_desc("backlinks"));
    spans.push(key_badge("T"));
    spans.push(hint_desc("tasks"));
    spans.push(key_badge("+/-"));
    spans.push(hint_desc(&format!("img({})", app.images.height_rows)));
    spans.push(key_badge("w"));
    spans.push(hint_desc(if app.wrap { "wrap[on]" } else { "wrap[off]" }));
    spans.push(key_badge("r/^L"));
    spans.push(hint_desc("reload"));

    // Group 6: Back (conditional)
    if !app.nav_history.is_empty() {
        spans.push(hint_sep());
        spans.push(key_badge("BS/^O"));
        spans.push(hint_desc("back"));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray)),
        left_area,
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

fn draw_tasks_panel(frame: &mut Frame, app: &mut App) {
    let content_area = {
        let full = frame.area();
        // Reserve top title bar (1 row) and bottom status bar (1 row).
        Rect {
            x: full.x,
            y: full.y + 1,
            width: full.width,
            height: full.height.saturating_sub(2),
        }
    };

    let cfg = &app.tui_config.tasks;
    let panel_w = ((content_area.width as f64 * cfg.width.clamp(0.05, 1.0)) as u16).max(20);
    let panel_h = ((content_area.height as f64 * cfg.height.clamp(0.05, 1.0)) as u16).max(3);

    let (panel_x, panel_y) = match cfg.position {
        TasksPanelPosition::BottomRight => (
            content_area.x + content_area.width.saturating_sub(panel_w),
            content_area.y + content_area.height.saturating_sub(panel_h),
        ),
        TasksPanelPosition::BottomLeft => (
            content_area.x,
            content_area.y + content_area.height.saturating_sub(panel_h),
        ),
        TasksPanelPosition::TopRight => (
            content_area.x + content_area.width.saturating_sub(panel_w),
            content_area.y,
        ),
        TasksPanelPosition::TopLeft => (content_area.x, content_area.y),
    };

    let area = Rect {
        x: panel_x,
        y: panel_y,
        width: panel_w,
        height: panel_h,
    };

    // Clear the region behind the panel.
    frame.render_widget(Clear, area);

    let inner_width = area.width.saturating_sub(2) as usize; // inside borders

    let block = Block::default()
        .title(" Tasks ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let selected = app.tasks.list_state.selected;
    let entries = &app.tasks.entries;
    let total = entries.len();

    if total == 0 {
        frame.render_widget(
            Paragraph::new("(no tasks)").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    // Simple manual scroll: ensure selected item is visible.
    let inner_h = inner.height as usize;
    let scroll_offset = if let Some(sel) = selected {
        if sel >= inner_h {
            sel + 1 - inner_h
        } else {
            0
        }
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, entry) in entries.iter().enumerate().skip(scroll_offset) {
        if lines.len() >= inner_h {
            break;
        }
        let is_sel = selected == Some(i);
        match entry {
            TaskEntry::SectionHeader(title) => {
                lines.push(Line::from(Span::styled(
                    title.clone(),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            TaskEntry::Placeholder(msg) => {
                lines.push(Line::from(Span::styled(
                    msg.clone(),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            TaskEntry::TaskItem {
                text,
                file_name,
                due_str,
                category,
                ..
            } => {
                use crate::tasks::DeadlineCategory;
                let base_style = match category {
                    DeadlineCategory::Overdue => Style::default().fg(Color::Red),
                    DeadlineCategory::Today => Style::default().fg(Color::Yellow),
                    _ => Style::default().fg(Color::White),
                };
                let row_style = if is_sel {
                    base_style.add_modifier(Modifier::REVERSED)
                } else {
                    base_style
                };

                // Format: "> [due] text  file" truncated to fit inner width
                let prefix = if is_sel { "> " } else { "  " };
                let date_part = if due_str.is_empty() {
                    String::new()
                } else {
                    format!("[{}] ", due_str)
                };
                let suffix = format!(" {}", file_name);
                // Available space for task text
                let fixed_len = prefix.len() + date_part.len() + suffix.len();
                let text_max = inner_width.saturating_sub(fixed_len);
                let truncated_text = if text.chars().count() > text_max {
                    let s: String = text.chars().take(text_max.saturating_sub(1)).collect();
                    format!("{}…", s)
                } else {
                    text.clone()
                };
                let row_text = format!("{}{}{}{}", prefix, date_part, truncated_text, suffix);
                lines.push(Line::from(Span::styled(row_text, row_style)));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
