use ratatui::{
    style::Color,
    text::{Line, Span},
};

fn focused(style: ratatui::style::Style) -> ratatui::style::Style {
    style.bg(Color::Yellow).fg(Color::Black)
}

fn other_match(style: ratatui::style::Style) -> ratatui::style::Style {
    style.bg(Color::Magenta).fg(Color::Black)
}

/// `line` with the characters in `[char_start, char_end)` shown as focused.
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
            new_spans.push(span.clone());
        } else if span_start >= char_start && span_end <= char_end {
            new_spans.push(Span::styled(span.content.clone(), focused(span.style)));
        } else {
            let chars: Vec<char> = span.content.chars().collect();
            let hl_start = char_start.saturating_sub(span_start);
            let hl_end = (char_end - span_start).min(span_len);

            if hl_start > 0 {
                let before: String = chars[..hl_start].iter().collect();
                new_spans.push(Span::styled(before, span.style));
            }
            let mid: String = chars[hl_start..hl_end].iter().collect();
            new_spans.push(Span::styled(mid, focused(span.style)));
            if hl_end < span_len {
                let after: String = chars[hl_end..].iter().collect();
                new_spans.push(Span::styled(after, span.style));
            }
        }
    }
    Line::from(new_spans)
}

/// `line` with each `(char_start, char_end, is_current)` range highlighted:
/// the current search match as focused, the others more quietly.
///
/// Ranges must not overlap. Spans are split at range boundaries, so the many
/// small spans syntect produces keep their own foreground colours.
pub(super) fn highlight_line_multi(
    line: &Line<'static>,
    ranges: &[(usize, usize, bool)],
) -> Line<'static> {
    if ranges.is_empty() {
        return line.clone();
    }

    let mut sorted_ranges = ranges.to_vec();
    sorted_ranges.sort_by_key(|(s, _, _)| *s);

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
        let mut push = |from: usize, to: usize, style| {
            let text: String = chars[from - span_start..to - span_start].iter().collect();
            if !text.is_empty() {
                new_spans.push(Span::styled(text, style));
            }
        };

        while cursor < span_end {
            while range_iter.peek().map(|&&(_, e, _)| e <= cursor) == Some(true) {
                range_iter.next();
            }

            match range_iter.peek().map(|&&(s, e, c)| (s, e, c)) {
                None => {
                    push(cursor, span_end, span.style);
                    cursor = span_end;
                }
                Some((hl_start, _, _)) if hl_start >= span_end => {
                    push(cursor, span_end, span.style);
                    cursor = span_end;
                }
                Some((hl_start, _, _)) if hl_start > cursor => {
                    let end = hl_start.min(span_end);
                    push(cursor, end, span.style);
                    cursor = end;
                }
                Some((_, hl_end, is_current)) => {
                    let end = hl_end.min(span_end);
                    let style = if is_current {
                        focused(span.style)
                    } else {
                        other_match(span.style)
                    };
                    push(cursor, end, style);
                    cursor = end;
                    if hl_end <= span_end {
                        range_iter.next();
                    }
                }
            }
        }
    }

    Line::from(new_spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    fn texts(line: &Line<'static>) -> Vec<(String, Option<Color>)> {
        line.spans
            .iter()
            .map(|span| (span.content.to_string(), span.style.bg))
            .collect()
    }

    #[test]
    fn a_range_inside_one_span_splits_it_in_three() {
        let line = Line::from(vec![Span::raw("hello world")]);
        let highlighted = highlight_line_range(&line, 6, 11);
        assert_eq!(
            texts(&highlighted),
            vec![
                ("hello ".to_string(), None),
                ("world".to_string(), Some(Color::Yellow)),
            ]
        );
    }

    #[test]
    fn a_range_across_spans_keeps_each_spans_own_style() {
        let line = Line::from(vec![
            Span::styled("ab", Style::default().fg(Color::Red)),
            Span::styled("cd", Style::default().fg(Color::Blue)),
        ]);
        let highlighted = highlight_line_range(&line, 1, 3);
        let fgs: Vec<Option<Color>> = highlighted.spans.iter().map(|s| s.style.fg).collect();
        assert_eq!(
            texts(&highlighted),
            vec![
                ("a".to_string(), None),
                ("b".to_string(), Some(Color::Yellow)),
                ("c".to_string(), Some(Color::Yellow)),
                ("d".to_string(), None),
            ]
        );
        assert_eq!(fgs[1], Some(Color::Black));
        assert_eq!(fgs[3], Some(Color::Blue));
    }

    #[test]
    fn the_current_match_and_other_matches_get_different_backgrounds() {
        let line = Line::from(vec![Span::raw("aXbXc")]);
        let highlighted = highlight_line_multi(&line, &[(3, 4, true), (1, 2, false)]);
        assert_eq!(
            texts(&highlighted),
            vec![
                ("a".to_string(), None),
                ("X".to_string(), Some(Color::Magenta)),
                ("b".to_string(), None),
                ("X".to_string(), Some(Color::Yellow)),
                ("c".to_string(), None),
            ]
        );
    }

    #[test]
    fn no_ranges_leaves_the_line_untouched() {
        let line = Line::from(vec![Span::raw("abc")]);
        assert_eq!(highlight_line_multi(&line, &[]), line);
    }
}
