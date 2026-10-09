use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::collections::HashMap;
use std::path::Path;

use super::highlight::{highlight_line_multi, highlight_line_range};
use super::image::draw_image_cell;
use crate::app::App;
use crate::image_cache::ImageCache;
use crate::tui_renderer::{DocElement, LinkAction};
use crate::wrap::{elem_height, wrap_line, WrapConfig};

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

    /// Index of the first element the current scroll position shows.
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

pub(super) fn draw_content(frame: &mut Frame, area: Rect, app: &mut App, root_dir: &Path) {
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
    images: &mut ImageCache,
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
fn draw_math(frame: &mut Frame, images: &mut ImageCache, content: &str, area: Rect, rows: u16) {
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

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        app_showing, assert_screen_contains, assert_screen_lacks, screen,
    };

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
        assert_screen_lacks(&scrolled, "line 0");
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
        assert_screen_lacks(&rows, "↪");
    }
}
