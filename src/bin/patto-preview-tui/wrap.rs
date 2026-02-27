//! Soft-wrap helpers for the patto-preview-tui.
//!
//! All wrap logic lives here so that the row-counting used for scroll math
//! and the row-building used for rendering share a single implementation.
//! This includes the canonical element-height API — `elem_height` and
//! `total_height` — which replaces `DocElement::height` / `RenderedDoc::total_height`
//! so that the data model stays free of rendering parameters.

use patto::tui_renderer::DocElement;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;

/// Parameters that control how lines are soft-wrapped.
#[derive(Clone)]
pub struct WrapConfig {
    /// Total display columns available (the terminal/pane width).
    pub col_width: usize,
    /// String prepended to every continuation row (showbreak), e.g. `"↪ "`.
    /// Empty string disables the prefix.
    pub showbreak: String,
}

impl WrapConfig {
    pub fn new(col_width: usize, showbreak: impl Into<String>) -> Self {
        Self {
            col_width,
            showbreak: showbreak.into(),
        }
    }

    /// Display columns available for content on continuation rows (rows 2, 3, …).
    ///
    /// Equals `col_width - showbreak_display_width`, or `col_width` if the
    /// showbreak would leave no room.
    pub fn cont_cols(&self) -> usize {
        use unicode_width::UnicodeWidthStr;
        let sw = self.showbreak.width();
        if sw > 0 && self.col_width > sw {
            self.col_width - sw
        } else {
            self.col_width
        }
    }

    /// Available columns on the given row (first row vs. continuation row).
    pub fn avail(&self, is_first_row: bool) -> usize {
        if is_first_row {
            self.col_width
        } else {
            self.cont_cols()
        }
    }

    /// Whether adding a character with display width `ch_w` to a row that has
    /// already used `col_used` columns would require a line break.
    ///
    /// Uses `>=` (not `>`) so the **last column is always reserved** for the
    /// `↩` wrap indicator — content never overwrites it.
    pub fn needs_break(&self, col_used: usize, ch_w: usize, is_first_row: bool) -> bool {
        let avail = self.avail(is_first_row);
        avail > 0 && col_used + ch_w >= avail
    }
}

// ---------------------------------------------------------------------------
// Row-counting (for scroll math)
// ---------------------------------------------------------------------------

/// Return the number of visual rows that `line` occupies with the given wrap config.
///
/// This mirrors `wrap_line` exactly — any change to the break logic must be
/// made in both functions (or only here, since `wrap_line` delegates to this
/// for counting).
pub fn count_wrap_rows(line: &Line<'_>, cfg: &WrapConfig) -> usize {
    if cfg.col_width == 0 {
        return 1;
    }
    let mut rows = 1usize;
    let mut col_used = 0usize;
    let mut is_first_row = true;
    for span in &line.spans {
        for ch in span.content.chars() {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if cfg.needs_break(col_used, ch_w, is_first_row) {
                rows += 1;
                is_first_row = false;
                col_used = 0;
            }
            col_used += ch_w;
        }
    }
    rows
}

/// Height of a single `DocElement` in terminal rows.
///
/// - Pass a `WrapConfig` to get soft-wrap–aware height for `TextLine` elements.
/// - Pass `None` (or a zero-width config) to get the unwarpped height (always 1
///   for `TextLine` / `Spacer`).
/// - `img_h` is the configured image height in terminal rows.
pub fn elem_height(elem: &DocElement, cfg: Option<&WrapConfig>, img_h: u16) -> usize {
    if let Some(cfg) = cfg {
        if cfg.col_width > 0 {
            if let DocElement::TextLine(line) = elem {
                return count_wrap_rows(line, cfg);
            }
        }
    }
    match elem {
        DocElement::TextLine(_) | DocElement::Spacer => 1,
        DocElement::Image { .. } | DocElement::ImageRow(_) => img_h as usize,
    }
}

/// Total height of a slice of elements in terminal rows.
pub fn total_height(elements: &[DocElement], cfg: Option<&WrapConfig>, img_h: u16) -> usize {
    elements.iter().map(|e| elem_height(e, cfg, img_h)).sum()
}

// ---------------------------------------------------------------------------
// Row-building (for rendering)
// ---------------------------------------------------------------------------

/// Accumulates spans for a single visual row, then finalises it into a `Line`.
struct RowBuilder {
    rows: Vec<Line<'static>>,
    cur_spans: Vec<Span<'static>>,
    cur_buf: String,
    showbreak: String,
    sb_style: Style,
}

impl RowBuilder {
    fn new(showbreak: &str) -> Self {
        let sb_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM);
        Self {
            rows: Vec::new(),
            cur_spans: Vec::new(),
            cur_buf: String::new(),
            showbreak: showbreak.to_string(),
            sb_style,
        }
    }

    /// Push any buffered chars as a span, then emit the current row and start
    /// a new one with the showbreak prefix.
    fn flush_row(&mut self, cur_style: Style) {
        if !self.cur_buf.is_empty() {
            self.cur_spans
                .push(Span::styled(self.cur_buf.clone(), cur_style));
            self.cur_buf.clear();
        }
        self.rows.push(Line::from(self.cur_spans.clone()));
        self.cur_spans.clear();
        if !self.showbreak.is_empty() {
            self.cur_spans
                .push(Span::styled(self.showbreak.clone(), self.sb_style));
        }
    }

    /// Finalise all remaining content as the last row and return all rows.
    fn finish(mut self, cur_style: Style) -> Vec<Line<'static>> {
        if !self.cur_buf.is_empty() {
            self.cur_spans
                .push(Span::styled(self.cur_buf, cur_style));
        }
        self.rows.push(Line::from(self.cur_spans));
        self.rows
    }
}

/// Split a styled `Line` into visual rows that each fit within `cfg.col_width`
/// display columns.
///
/// - Row 0 uses the full `col_width`.
/// - Rows 1+ are prefixed with a dim `showbreak` span; content uses
///   `cont_cols()` columns.
/// - The last column of each non-final row is **always left empty** (the `>=`
///   threshold in `needs_break`) so the `↩` indicator can be overlaid without
///   overwriting content.
pub fn wrap_line<'a>(line: &Line<'a>, cfg: &WrapConfig) -> Vec<Line<'static>> {
    if cfg.col_width == 0 {
        return vec![Line::default()];
    }

    let mut builder = RowBuilder::new(&cfg.showbreak);
    let mut col_used = 0usize;
    let mut is_first_row = true;
    let mut cur_style = Style::default();

    for span in &line.spans {
        let style = span.style;
        // Flush buffered chars when the style changes mid-span sequence.
        if style != cur_style && !builder.cur_buf.is_empty() {
            builder
                .cur_spans
                .push(Span::styled(builder.cur_buf.clone(), cur_style));
            builder.cur_buf.clear();
        }
        cur_style = style;

        for ch in span.content.chars() {
            let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
            if cfg.needs_break(col_used, ch_w, is_first_row) {
                builder.flush_row(cur_style);
                is_first_row = false;
                col_used = 0;
            }
            builder.cur_buf.push(ch);
            col_used += ch_w;
        }
    }

    builder.finish(cur_style)
}
