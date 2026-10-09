use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::search::SearchDirection;
use crate::tui_renderer::LinkAction;

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

pub(super) fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
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
