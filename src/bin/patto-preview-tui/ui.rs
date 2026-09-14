use crate::backlinks::FlatEntry;
use crate::config::TasksPanelPosition;
use crate::tasks::TaskEntry;
use crate::tui_renderer::{DocElement, LinkAction};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    Frame,
};
use ratatui_image::StatefulImage;
use std::collections::HashMap;
use std::path::Path;
use tui_widget_list::{ListBuilder, ListView};

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

    // Fidget-style active task overlay (when tasks panel is closed)
    if !app.tasks.visible {
        draw_active_task_overlay(frame, chunks[1], app);
    }

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
    let (pos, pct) = if let Some(p) = ((app.scroll_offset + 1) * 100).checked_div(total) {
        let p = p.min(100);
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

/// The parts of the draw pass that do not change between elements.
struct ContentLayout {
    area: Rect,
    /// Rows available for content.
    height: usize,
    wrap: Option<WrapConfig>,
    showbreak: String,
    image_rows: u16,
    /// Copied from the image cache so the draw pass can measure elements while
    /// holding the cache mutably.
    elem_heights: HashMap<String, u16>,
}

impl ContentLayout {
    fn new(app: &App, area: Rect) -> Self {
        Self {
            area,
            height: area.height as usize,
            wrap: (app.wrap && area.width > 0)
                .then(|| WrapConfig::new(area.width as usize, &app.showbreak)),
            showbreak: app.showbreak.clone(),
            image_rows: app.images.height_rows,
            elem_heights: app.images.elem_heights.clone(),
        }
    }

    /// Display height of an element, in rows.
    fn height_of(&self, elem: &DocElement) -> usize {
        elem_height(
            elem,
            self.wrap.as_ref(),
            self.image_rows,
            Some(&self.elem_heights),
        )
    }

    /// Height of a media element, which soft-wrap does not apply to.
    fn media_height_at(&self, elem: &DocElement, y: usize) -> u16 {
        let rows = elem_height(elem, None, self.image_rows, Some(&self.elem_heights));
        (rows as u16).min((self.height - y) as u16)
    }

    fn row_area(&self, y: usize, rows: u16) -> Rect {
        Rect::new(self.area.x, self.area.y + y as u16, self.area.width, rows)
    }

    fn indented_area(&self, y: usize, indent: usize, rows: u16) -> Rect {
        let indent_width = (indent as u16) * 2;
        Rect::new(
            self.area.x + indent_width,
            self.area.y + y as u16,
            self.area.width.saturating_sub(indent_width),
            rows,
        )
    }

    /// Index of the first element the current scroll position shows, and how
    /// many of its rows are above the viewport.
    fn first_visible(&self, app: &App) -> usize {
        let mut remaining = app.scroll_offset;
        for (i, elem) in app.rendered_doc.elements.iter().enumerate() {
            let rows = self.height_of(elem);
            if remaining < rows {
                return i;
            }
            remaining -= rows;
        }
        app.rendered_doc.elements.len()
    }

    /// The elements from `start` that can appear in the viewport.
    fn visible<'a>(
        &'a self,
        app: &'a App,
        start: usize,
    ) -> impl Iterator<Item = &'a DocElement> + 'a {
        let mut rows = 0usize;
        let limit = self.height + self.image_rows as usize;
        app.rendered_doc
            .elements
            .iter()
            .skip(start)
            .take_while(move |elem| {
                rows += self.height_of(elem);
                rows <= limit
            })
    }
}

/// What the draw pass highlights: the focused item and the search matches.
struct Highlights {
    focused_elem: Option<usize>,
    focused_range: Option<(usize, usize)>,
    /// The image the focus is on, when it is on one.
    focused_image: Option<String>,
    /// `(element, char start, char end, is the current match)`
    search: Vec<(usize, usize, usize, bool)>,
}

impl Highlights {
    fn snapshot(app: &App) -> Self {
        let focused = app.focused_item();
        Self {
            focused_elem: focused.map(|item| item.elem_idx),
            focused_range: focused.map(|item| (item.char_start, item.char_end)),
            focused_image: focused.and_then(|item| match &item.action {
                LinkAction::ViewImage(src) => Some(src.clone()),
                _ => None,
            }),
            search: app
                .search
                .as_ref()
                .map(|search| {
                    search
                        .matches
                        .iter()
                        .enumerate()
                        .map(|(i, m)| {
                            (
                                m.elem_idx,
                                m.char_start,
                                m.char_end,
                                Some(i) == search.match_idx,
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn search_ranges(&self, elem_idx: usize) -> Vec<(usize, usize, bool)> {
        self.search
            .iter()
            .filter(|(idx, _, _, _)| *idx == elem_idx)
            .map(|(_, start, end, current)| (*start, *end, *current))
            .collect()
    }
}

fn draw_content(frame: &mut Frame, area: Rect, app: &mut App, root_dir: &Path) {
    // Wrap-aware scrolling reads these back.
    app.viewport_width = area.width;
    app.viewport_height = area.height as usize;
    app.clear_stale_focus();

    let layout = ContentLayout::new(app, area);
    let start = layout.first_visible(app);
    preload_visible_media(app, &layout, start, root_dir);

    let highlights = Highlights::snapshot(app);

    // `app.rendered_doc` and `app.images` are borrowed separately, so the draw
    // pass can read elements while loading images.
    let App {
        rendered_doc,
        images,
        ..
    } = app;

    let mut y = 0usize;
    for (elem_idx, elem) in rendered_doc.elements.iter().enumerate().skip(start) {
        if y >= layout.height {
            break;
        }
        let is_focused = highlights.focused_elem == Some(elem_idx);

        y += match elem {
            DocElement::TextLine(line, _) => {
                let ranges = highlights.search_ranges(elem_idx);
                let focused_range = is_focused.then_some(highlights.focused_range).flatten();
                draw_text_line(frame, &layout, y, elem, line, &ranges, focused_range)
            }
            DocElement::Image { src, alt, indent } => {
                let rows = layout.media_height_at(elem, y);
                let cell = layout.indented_area(y, *indent, rows);
                draw_image_cell(frame, images, src, alt.as_deref(), cell, is_focused);
                rows as usize
            }
            DocElement::ImageRow(row, indent) => {
                let rows = layout.media_height_at(elem, y);
                let focused_src = is_focused
                    .then_some(highlights.focused_image.as_deref())
                    .flatten();
                draw_image_row(frame, images, &layout, y, row, *indent, rows, focused_src);
                rows as usize
            }
            DocElement::Math { content, indent } => {
                let rows = layout.media_height_at(elem, y);
                let cell = layout.indented_area(y, *indent, rows);
                draw_math(frame, images, content, cell, rows);
                rows as usize
            }
        };
    }
}

/// Decode the images and math blocks about to come into view.
fn preload_visible_media(app: &mut App, layout: &ContentLayout, start: usize, root_dir: &Path) {
    let sources: Vec<String> = layout
        .visible(app, start)
        .filter_map(|elem| match elem {
            DocElement::Image { src, .. } => Some(vec![src.clone()]),
            DocElement::ImageRow(images, ..) => {
                Some(images.iter().map(|(src, _)| src.clone()).collect())
            }
            _ => None,
        })
        .flatten()
        .collect();

    let math: Vec<String> = layout
        .visible(app, start)
        .filter_map(|elem| match elem {
            DocElement::Math { content, .. } => Some(content.clone()),
            _ => None,
        })
        .collect();

    for src in &sources {
        app.images.load(src, root_dir);
    }
    for content in &math {
        app.images.load_math(content);
    }
}

/// Draw one text line, wrapped if wrapping is on, and return the rows used.
fn draw_text_line(
    frame: &mut Frame,
    layout: &ContentLayout,
    y: usize,
    elem: &DocElement,
    line: &Line<'static>,
    search_ranges: &[(usize, usize, bool)],
    focused_range: Option<(usize, usize)>,
) -> usize {
    let full_rows = layout.height_of(elem);
    let rows = full_rows.min(layout.height - y) as u16;

    // Search highlights first, then focus on top of them.
    let mut line = if search_ranges.is_empty() {
        line.clone()
    } else {
        highlight_line_multi(line, search_ranges)
    };
    if let Some((start, end)) = focused_range {
        line = highlight_line_range(&line, start, end);
    }

    match &layout.wrap {
        None => frame.render_widget(Paragraph::new(line), layout.row_area(y, rows)),
        Some(_) => {
            let wrap_cfg = WrapConfig::new(layout.area.width as usize, &layout.showbreak);
            for (i, row) in wrap_line(&line, &wrap_cfg)
                .iter()
                .enumerate()
                .take(rows as usize)
            {
                frame.render_widget(Paragraph::new(row.clone()), layout.row_area(y + i, 1));
            }
            draw_wrap_indicators(frame, layout, y, rows, full_rows);
        }
    }
    rows as usize
}

/// Mark each wrapped row that continues onto the next with `↩` at the right
/// edge. The last column is always free: `WrapConfig::needs_break` reserves it.
fn draw_wrap_indicators(
    frame: &mut Frame,
    layout: &ContentLayout,
    y: usize,
    rows: u16,
    full_rows: usize,
) {
    if rows <= 1 {
        return;
    }
    let count = if rows < full_rows as u16 {
        rows
    } else {
        rows.saturating_sub(1)
    };

    let style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM);
    let x = layout.area.x + layout.area.width - 1;
    for row in 0..count {
        if let Some(cell) = frame
            .buffer_mut()
            .cell_mut((x, layout.area.y + y as u16 + row))
        {
            cell.set_symbol("↩");
            cell.set_style(style);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_image_row(
    frame: &mut Frame,
    images: &mut crate::image_cache::ImageCache,
    layout: &ContentLayout,
    y: usize,
    row: &[(String, Option<String>)],
    indent: usize,
    rows: u16,
    focused_src: Option<&str>,
) {
    let count = row.len() as u16;
    let indent_width = (indent as u16) * 2;
    let row_width = layout.area.width.saturating_sub(indent_width);
    let column_width = row_width / count;

    for (i, (src, alt)) in row.iter().enumerate() {
        let i = i as u16;
        // The last column takes the remainder, so rounding leaves no gap.
        let width = if i == count - 1 {
            row_width - i * column_width
        } else {
            column_width
        };
        let cell = Rect::new(
            layout.area.x + indent_width + i * column_width,
            layout.area.y + y as u16,
            width,
            rows,
        );
        draw_image_cell(
            frame,
            images,
            src,
            alt.as_deref(),
            cell,
            focused_src == Some(src.as_str()),
        );
    }
}

/// A rendered math block, or its source as text when there is no image for it.
fn draw_math(
    frame: &mut Frame,
    images: &mut crate::image_cache::ImageCache,
    content: &str,
    area: Rect,
    rows: u16,
) {
    if images.get_mut(content).is_some() {
        draw_image_cell(frame, images, content, None, area, false);
        return;
    }

    let lines: Vec<Line<'static>> = std::iter::once(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "  [math]  ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::DIM),
        ),
    ]))
    .chain(content.lines().map(|line| {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(line.to_string(), Style::default().fg(Color::Magenta)),
        ])
    }))
    .take(rows as usize)
    .collect();
    frame.render_widget(Paragraph::new(lines), area);
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

/// Fidget-style overlay showing active (Doing/Paused) tasks in the bottom-right corner
/// of the content area. Only drawn when the tasks panel is closed.
fn draw_active_task_overlay(frame: &mut Frame, content_area: Rect, app: &App) {
    use patto::parser::TaskStatus;
    let active = app.tasks.active_tasks();
    if active.is_empty() {
        return;
    }

    // Show at most 3 tasks.
    let max_rows = 3usize;
    let tasks_to_show: Vec<_> = active.iter().take(max_rows).collect();
    let _num_rows = tasks_to_show.len() as u16;

    // Max width cap: 60% of content width, min 20 cols.
    let max_w = (content_area.width * 60 / 100)
        .max(20)
        .min(content_area.width) as usize;

    // Build lines first so we can measure actual rendered width.
    let mut lines: Vec<Line> = Vec::new();
    for (status, text, base_time_spent, started_at_dt) in &tasks_to_show {
        use crate::tasks::{fmt_timedelta, total_elapsed};
        let (annotation, ann_style, text_style) = match status {
            TaskStatus::Doing => (
                "◑ doing",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Cyan).bg(Color::Black),
            ),
            TaskStatus::Paused => (
                "⏸ paused",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Yellow).bg(Color::Black),
            ),
            _ => continue,
        };

        let elapsed = total_elapsed(status, *base_time_spent, *started_at_dt);
        let time_chip = fmt_timedelta(elapsed)
            .map(|s| format!(" ⏱ {}", s))
            .unwrap_or_default();

        // " {annotation} " prefix + " " gap
        let prefix_len = 1 + annotation.chars().count() + 2;
        let text_max = max_w.saturating_sub(prefix_len + time_chip.chars().count());
        let truncated = if text.chars().count() > text_max {
            let s: String = text.chars().take(text_max.saturating_sub(1)).collect();
            format!("{}…", s)
        } else {
            text.clone()
        };

        let mut spans = vec![
            Span::styled(format!(" {} ", annotation), ann_style),
            Span::styled(format!(" {}", truncated), text_style),
        ];
        if !time_chip.is_empty() {
            spans.push(Span::styled(
                time_chip,
                Style::default().fg(Color::DarkGray).bg(Color::Black),
            ));
        }
        lines.push(Line::from(spans));
    }

    if lines.is_empty() {
        return;
    }

    // Measure actual width needed (sum of span char widths per line).
    let actual_w = lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.chars().count())
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0)
        .min(max_w) as u16;

    let num_rows = lines.len() as u16;

    // Position: bottom-right of content area, 1 row above the bottom edge.
    let x = content_area.x + content_area.width.saturating_sub(actual_w);
    let y = content_area
        .y
        .saturating_add(content_area.height)
        .saturating_sub(num_rows + 1);

    if y < content_area.y || content_area.height < num_rows + 1 {
        return;
    }

    let overlay_area = Rect {
        x,
        y,
        width: actual_w,
        height: num_rows,
    };

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Black)),
        overlay_area,
    );
}

fn draw_tasks_panel(frame: &mut Frame, app: &mut App) {
    use crate::tasks::TasksView;

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

    let is_review = app.tasks.view == TasksView::Review;
    let title = if is_review {
        " Tasks Review  [R:upcoming] "
    } else {
        " Tasks  [R:review] "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let inner_h = inner.height as usize;

    if is_review {
        draw_tasks_review_content(frame, app, inner, inner_h, inner_width);
    } else {
        draw_tasks_upcoming_content(frame, app, inner, inner_h, inner_width);
    }
}

fn draw_tasks_upcoming_content(
    frame: &mut Frame,
    app: &App,
    inner: Rect,
    inner_h: usize,
    inner_width: usize,
) {
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
                status,
                base_time_spent,
                started_at_dt,
                ..
            } => {
                use crate::tasks::{
                    fmt_timedelta, task_status_icon, total_elapsed, DeadlineCategory,
                };
                use patto::parser::TaskStatus;

                // ── colours ───────────────────────────────────────────────
                let text_fg = match category {
                    DeadlineCategory::Overdue => Color::Red,
                    DeadlineCategory::Today => Color::Yellow,
                    _ => Color::White,
                };
                let (text_style, sel_prefix) = if is_sel {
                    (
                        Style::default()
                            .fg(Color::Black)
                            .bg(text_fg)
                            .add_modifier(Modifier::BOLD),
                        ">",
                    )
                } else {
                    (Style::default().fg(text_fg), " ")
                };

                // ── due-date chip (plain text, coloured fg, no brackets) ──
                let due_chip_style = if is_sel {
                    text_style
                } else {
                    match category {
                        DeadlineCategory::Overdue => {
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                        }
                        DeadlineCategory::Today => Style::default().fg(Color::Yellow),
                        DeadlineCategory::Tomorrow => Style::default().fg(Color::Cyan),
                        _ => Style::default().fg(Color::DarkGray),
                    }
                };
                let due_chip = if due_str.is_empty() {
                    String::new()
                } else {
                    format!(" {}", due_str)
                };

                // ── time chips (doing/paused only, computed at render time) ──
                let time_chip = match status {
                    TaskStatus::Doing | TaskStatus::Paused => {
                        let elapsed = total_elapsed(status, *base_time_spent, *started_at_dt);
                        fmt_timedelta(elapsed)
                            .map(|s| format!(" ⏱ {}", s))
                            .unwrap_or_default()
                    }
                    _ => String::new(),
                };
                let started_chip = if matches!(status, TaskStatus::Doing) {
                    started_at_dt
                        .map(|dt| format!(" ▶ {}", dt.format("%H:%M")))
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                // ── layout: prefix + icon + due + " " + text + chips + file ──
                let prefix_str = format!("{}{} ", sel_prefix, task_status_icon(status));
                let suffix_str = format!("  {}", file_name);
                let fixed_chars = prefix_str.chars().count()
                    + due_chip.chars().count()
                    + 1 // space between due and text
                    + time_chip.chars().count()
                    + started_chip.chars().count()
                    + suffix_str.chars().count();
                let text_max = inner_width.saturating_sub(fixed_chars);
                let truncated_text = if text.chars().count() > text_max {
                    let s: String = text.chars().take(text_max.saturating_sub(1)).collect();
                    format!("{}…", s)
                } else {
                    text.clone()
                };

                // ── build spans ───────────────────────────────────────────
                let mut spans = vec![Span::styled(prefix_str, text_style)];
                if !due_chip.is_empty() {
                    spans.push(Span::styled(due_chip, due_chip_style));
                }
                spans.push(Span::styled(format!(" {}", truncated_text), text_style));
                if !time_chip.is_empty() {
                    let chip_style = if is_sel {
                        text_style
                    } else {
                        Style::default().fg(Color::Blue)
                    };
                    spans.push(Span::styled(time_chip, chip_style));
                }
                if !started_chip.is_empty() {
                    let chip_style = if is_sel {
                        text_style
                    } else {
                        Style::default().fg(Color::Yellow)
                    };
                    spans.push(Span::styled(started_chip, chip_style));
                }
                spans.push(Span::styled(
                    suffix_str,
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(spans));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_tasks_review_content(
    frame: &mut Frame,
    app: &App,
    inner: Rect,
    inner_h: usize,
    inner_width: usize,
) {
    use crate::tasks::ReviewEntry;

    let selected = app.tasks.review_list_state.selected;
    let entries = &app.tasks.review_entries;
    let total = entries.len();

    if total == 0 {
        frame.render_widget(
            Paragraph::new("(no completed tasks)").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    // Simple manual scroll: ensure selected item is visible.
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
            ReviewEntry::SectionHeader(title) => {
                lines.push(Line::from(Span::styled(
                    title.clone(),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            ReviewEntry::Placeholder(msg) => {
                lines.push(Line::from(Span::styled(
                    msg.clone(),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            ReviewEntry::ReviewItem {
                text,
                file_name,
                completed_at,
                time_spent,
                ..
            } => {
                let base_style = Style::default().fg(Color::Green);
                let row_style = if is_sel {
                    base_style.add_modifier(Modifier::REVERSED)
                } else {
                    base_style
                };

                // Build row: "{>|space} ✓ {completed_at} {text} [⏱ ts]  {file}"
                let prefix = if is_sel { "> " } else { "  " };
                let done_chip = format!("✓ {} ", completed_at);
                let ts_part = {
                    use crate::tasks::fmt_timedelta;
                    fmt_timedelta(*time_spent)
                        .map(|s| format!(" ⏱{}", s))
                        .unwrap_or_default()
                };
                let suffix = format!("  {}", file_name);
                let fixed_len = prefix.chars().count()
                    + done_chip.chars().count()
                    + ts_part.chars().count()
                    + suffix.chars().count();
                let text_max = inner_width.saturating_sub(fixed_len);
                let truncated_text = if text.chars().count() > text_max {
                    let s: String = text.chars().take(text_max.saturating_sub(1)).collect();
                    format!("{}…", s)
                } else {
                    text.clone()
                };
                let row_text = format!(
                    "{}{}{}{}{}",
                    prefix, done_chip, truncated_text, ts_part, suffix
                );
                lines.push(Line::from(Span::styled(row_text, row_style)));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::path::PathBuf;

    fn app_showing(content: &str) -> App {
        let mut app = App::new(
            PathBuf::from("/notes/note.pn"),
            PathBuf::from("/notes"),
            None,
        );
        app.viewport_width = 60;
        app.viewport_height = 18;
        app.re_render(content);
        app
    }

    /// Draw into an off-screen buffer and return it as one string per row.
    fn screen(app: &mut App, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw(frame, app, Path::new("/notes")))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    fn assert_screen_contains(rows: &[String], needle: &str) {
        assert!(
            rows.iter().any(|row| row.contains(needle)),
            "expected {needle:?} on screen:\n{}",
            rows.join("\n")
        );
    }

    #[test]
    fn draws_a_title_bar_the_content_and_a_status_bar() {
        let mut app = app_showing("hello world\n\tnested line\n");
        let rows = screen(&mut app, 60, 12);

        // Title bar names the note.
        assert_screen_contains(&rows, "note.pn");
        assert_screen_contains(&rows, "hello world");
        assert_screen_contains(&rows, "nested line");
        // Status bar shows the position within the document.
        assert_screen_contains(&rows, "1:");
    }

    #[test]
    fn nested_lines_are_indented_and_bulleted() {
        let mut app = app_showing("parent\n\tchild\n");
        let rows = screen(&mut app, 60, 8);
        assert_screen_contains(&rows, "  • child");
    }

    #[test]
    fn scrolling_moves_the_visible_window() {
        let content: String = (0..40).map(|i| format!("line {i}\n")).collect();
        let mut app = app_showing(&content);

        let top = screen(&mut app, 60, 10);
        assert_screen_contains(&top, "line 0");

        app.scroll_down(20);
        let scrolled = screen(&mut app, 60, 10);
        assert!(
            !scrolled.iter().any(|row| row.contains("line 0")),
            "line 0 should have scrolled off:\n{}",
            scrolled.join("\n")
        );
        assert_screen_contains(&scrolled, "line 20");
    }

    #[test]
    fn a_code_block_is_drawn_with_its_language_label() {
        let mut app = app_showing("[@code python]\n\tprint(1)\n");
        let rows = screen(&mut app, 60, 10);
        assert_screen_contains(&rows, "python");
        assert_screen_contains(&rows, "print(1)");
    }

    #[test]
    fn a_task_line_shows_its_icon_and_deadline() {
        let mut app = app_showing("{@task status=todo due=2024-12-31} buy milk\n");
        let rows = screen(&mut app, 60, 8);
        assert_screen_contains(&rows, "○");
        assert_screen_contains(&rows, "buy milk");
        assert_screen_contains(&rows, "2024-12-31");
    }

    #[test]
    fn a_table_is_drawn_with_separators() {
        let mut app = app_showing("[@table cap]\n\ta\tb\n");
        let rows = screen(&mut app, 60, 10);
        assert_screen_contains(&rows, "cap");
        assert_screen_contains(&rows, "│");
    }

    #[test]
    fn long_lines_wrap_with_the_showbreak_marker() {
        let mut app = app_showing(&format!("{}\n", "word ".repeat(40)));
        let rows = screen(&mut app, 40, 12);
        assert_screen_contains(&rows, "↪");
    }

    #[test]
    fn wrapping_off_leaves_no_showbreak_marker() {
        let mut app = app_showing(&format!("{}\n", "word ".repeat(40)));
        app.wrap = false;
        let rows = screen(&mut app, 40, 12);
        assert!(
            !rows.iter().any(|row| row.contains("↪")),
            "no showbreak expected with wrap off:\n{}",
            rows.join("\n")
        );
    }

    #[test]
    fn an_empty_document_still_draws_its_chrome() {
        let mut app = app_showing("");
        let rows = screen(&mut app, 60, 6);
        assert_screen_contains(&rows, "note.pn");
    }
}
