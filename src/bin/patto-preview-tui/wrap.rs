//! Soft-wrap helpers for the patto-preview-tui.
//!
//! All wrap logic lives here so that the row-counting used for scroll math
//! and the row-building used for rendering share a single implementation.
//! This includes the canonical element-height API — `elem_height` and
//! `total_height` — which replaces `DocElement::height` / `RenderedDoc::total_height`
//! so that the data model stays free of rendering parameters.

use patto::tui_renderer::{DocElement, InlineSegment};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::HashMap;
use unicode_width::UnicodeWidthChar;

/// Cached element sizes (heights and widths) used by scroll math and rendering.
///
/// Images and math blocks store their height (in terminal rows); inline math
/// images additionally store their width (in terminal columns).
#[derive(Clone, Default)]
pub struct ElemSizeCache {
    /// Cache key → height in terminal rows.
    pub heights: HashMap<String, u16>,
    /// Cache key → width in terminal columns (inline math only).
    pub widths: HashMap<String, u16>,
}

impl ElemSizeCache {
    pub fn clear(&mut self) {
        self.heights.clear();
        self.widths.clear();
    }
}

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
/// - Pass `None` (or a zero-width config) to get the unwrapped height (always 1
///   for `TextLine` / `Spacer`).
/// - `img_h` is the configured default height in terminal rows (fallback).
/// - `sizes` provides cached heights and widths for images and math elements.
pub fn elem_height(
    elem: &DocElement,
    cfg: Option<&WrapConfig>,
    img_h: u16,
    sizes: Option<&ElemSizeCache>,
) -> usize {
    if let Some(cfg) = cfg {
        if cfg.col_width > 0 {
            if let DocElement::TextLine(line, _) = elem {
                return count_wrap_rows(line, cfg);
            }
            if let DocElement::InlineMathLine { segments, .. } = elem {
                return inline_math_line_wrap_height(segments, cfg.col_width, sizes);
            }
        }
    }
    let heights = sizes.map(|s| &s.heights);
    match elem {
        DocElement::TextLine(_, _) | DocElement::Spacer => 1,
        DocElement::Image { src, .. } => heights
            .and_then(|m| m.get(src.as_str()).copied())
            .unwrap_or(img_h) as usize,
        DocElement::ImageRow(images, ..) => heights
            .map(|m| {
                images
                    .iter()
                    .filter_map(|(src, _)| m.get(src.as_str()).copied())
                    .max()
                    .unwrap_or(img_h)
            })
            .unwrap_or(img_h) as usize,
        DocElement::Math { content, .. } => heights
            .and_then(|m| m.get(content.as_str()).copied())
            .unwrap_or(img_h) as usize,
        DocElement::InlineMathLine { segments, .. } => {
            inline_math_line_wrap_height(segments, 0, sizes)
        }
    }
}

/// Compute the display height of an `InlineMathLine` given a maximum column width.
///
/// Text segments are wrapped character-by-character (mirroring the ui.rs render arm).
/// Math segments are treated atomically — if one doesn't fit, it moves to the next row.
/// Each row's height equals the tallest math image on that row (minimum 1).
/// When `col_width == 0` (no wrap) all segments share one row.
fn inline_math_line_wrap_height(
    segments: &[InlineSegment],
    col_width: usize,
    sizes: Option<&ElemSizeCache>,
) -> usize {
    let math_seg_h = |content: &str| -> usize {
        let key = format!("__inline__:{}", content);
        sizes
            .and_then(|s| s.heights.get(key.as_str()).copied())
            .unwrap_or(1) as usize
    };
    let math_seg_w = |content: &str| -> usize {
        let key = format!("__inline__:{}", content);
        sizes
            .and_then(|s| s.widths.get(key.as_str()).copied())
            .unwrap_or(4) as usize
    };

    // Each entry is the height of that visual row.
    let mut row_heights: Vec<usize> = vec![1];
    let mut x = 0usize;

    for seg in segments {
        match seg {
            InlineSegment::Text(spans) => {
                // Text wraps character-by-character (mirrors ui.rs render arm).
                for span in spans {
                    for ch in span.content.chars() {
                        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(0);
                        if ch_w == 0 {
                            continue;
                        }
                        if col_width > 0 && (x + ch_w) > col_width {
                            // Start a new text-only row (height 1).
                            row_heights.push(1);
                            x = ch_w;
                        } else {
                            x += ch_w;
                        }
                    }
                }
                // Text doesn't increase the current row's height.
            }
            InlineSegment::Math(c) => {
                let (w, h) = (math_seg_w(c), math_seg_h(c));
                if w == 0 {
                    continue;
                }
                if col_width > 0 && x > 0 && x + w > col_width {
                    // Math doesn't fit: start a new row.
                    row_heights.push(h.max(1));
                    x = w;
                } else {
                    // Math fits: update current row's height.
                    let last = row_heights.last_mut().unwrap();
                    *last = (*last).max(h);
                    x += w;
                }
            }
        }
    }
    row_heights.iter().sum()
}

/// Total height of a slice of elements in terminal rows.
pub fn total_height(
    elements: &[DocElement],
    cfg: Option<&WrapConfig>,
    img_h: u16,
    sizes: Option<&ElemSizeCache>,
) -> usize {
    elements
        .iter()
        .map(|e| elem_height(e, cfg, img_h, sizes))
        .sum()
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
            self.cur_spans.push(Span::styled(self.cur_buf, cur_style));
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
