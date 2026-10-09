use ratatui::{
    style::Color,
    text::{Line, Span},
};

/// Produce a new Line with chars in [char_start, char_end) highlighted with reverse video.
pub(super) fn highlight_line_range(
    line: &Line<'static>,
    char_start: usize,
    char_end: usize,
) -> Line<'static> {
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
pub(super) fn highlight_line_multi(
    line: &Line<'static>,
    ranges: &[(usize, usize, bool)],
) -> Line<'static> {
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
