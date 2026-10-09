use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::search::{SearchDirection, SearchState};
use crate::tui_renderer::LinkAction;

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

    if let Some((spans, _)) = right {
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Black)),
            right_area,
        );
    }

    match &app.search {
        Some(search) if search.typing => frame.render_widget(
            Paragraph::new(Line::from(search_prompt_spans(search)))
                .style(Style::default().bg(Color::Black)),
            left_area,
        ),
        _ => frame.render_widget(
            Paragraph::new(Line::from(key_hint_spans(app)))
                .style(Style::default().bg(Color::DarkGray)),
            left_area,
        ),
    }
}

fn direction_char(direction: SearchDirection) -> &'static str {
    match direction {
        SearchDirection::Forward => "/",
        SearchDirection::Backward => "?",
    }
}

fn match_count_text(search: &SearchState) -> String {
    if search.matches.is_empty() {
        " no match ".to_string()
    } else {
        let cur = search.match_idx.map(|i| i + 1).unwrap_or(0);
        format!(" {}/{} ", cur, search.matches.len())
    }
}

/// Right-aligned search status spans and their display width; `None` when
/// there is nothing to show.
fn search_right_status(app: &App) -> Option<(Vec<Span<'static>>, u16)> {
    let search = app.search.as_ref()?;
    if search.query.is_empty() {
        return None;
    }
    Some(if search.typing {
        typing_match_count(search)
    } else {
        confirmed_search_status(search)
    })
}

fn typing_match_count(search: &SearchState) -> (Vec<Span<'static>>, u16) {
    let count_text = match_count_text(search);
    let width = count_text.chars().count() as u16;
    (
        vec![Span::styled(
            count_text,
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        )],
        width,
    )
}

fn confirmed_search_status(search: &SearchState) -> (Vec<Span<'static>>, u16) {
    let count_text = match_count_text(search);
    let count_style = if search.matches.is_empty() {
        Style::default().fg(Color::Red).bg(Color::Black)
    } else {
        Style::default().fg(Color::DarkGray).bg(Color::Black)
    };
    let query_text = format!(" {} ", search.query);
    let separator_width = 1u16;
    let direction_width = 1u16;
    let width = separator_width
        + direction_width
        + query_text.chars().count() as u16
        + count_text.chars().count() as u16;
    (
        vec![
            Span::styled(
                "│",
                Style::default().fg(Color::DarkGray).bg(Color::DarkGray),
            ),
            Span::styled(
                direction_char(search.direction),
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
        width,
    )
}

/// The query being typed, with the character under the cursor inverted.
fn search_prompt_spans(search: &SearchState) -> Vec<Span<'static>> {
    let cursor_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let before = search.query[..search.cursor].to_string();
    let (cursor_span, after) = match search.query[search.cursor..].chars().next() {
        Some(c) => (
            Span::styled(c.to_string(), cursor_style),
            search.query[search.cursor + c.len_utf8()..].to_string(),
        ),
        None => (Span::styled(" ", cursor_style), String::new()),
    };

    let mut spans = vec![
        Span::styled(
            direction_char(search.direction),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(before, Style::default().fg(Color::White)),
        cursor_span,
    ];
    if !after.is_empty() {
        spans.push(Span::styled(after, Style::default().fg(Color::White)));
    }
    spans
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

fn hint(key: &str, desc: &str) -> [Span<'static>; 2] {
    [key_badge(key), hint_desc(desc)]
}

fn key_hint_spans(app: &App) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = hint("q", "quit").to_vec();
    for group in [
        scroll_hints(),
        search_hints(app),
        focus_hints(app),
        tool_hints(app),
    ] {
        spans.push(hint_sep());
        spans.extend(group);
    }
    if !app.nav_history.is_empty() {
        spans.push(hint_sep());
        spans.extend(hint("BS/^O", "back"));
    }
    spans
}

fn scroll_hints() -> Vec<Span<'static>> {
    [
        hint("j/k", "↕1"),
        hint("^F/^B", "page"),
        hint("^D/^U", "½pg"),
        hint("g/G", "top/end"),
    ]
    .concat()
}

fn search_hints(app: &App) -> Vec<Span<'static>> {
    let mut spans = hint("/", "search").to_vec();
    if app.search.as_ref().is_some_and(|s| !s.matches.is_empty()) {
        spans.extend(hint("n/N", "next"));
    }
    spans
}

fn focus_hints(app: &App) -> Vec<Span<'static>> {
    let mut spans = hint("Tab/S-Tab", "focus").to_vec();
    if let Some(item) = app.focused_item() {
        let desc = match item.action {
            LinkAction::OpenNote { .. } => "open note",
            LinkAction::JumpToAnchor { .. } => "jump",
            LinkAction::OpenUrl(_) => "open url",
            LinkAction::ViewImage(_) => "fullscreen",
        };
        spans.extend(hint("↵", desc));
    }
    spans
}

fn tool_hints(app: &App) -> Vec<Span<'static>> {
    [
        hint("b", "backlinks"),
        hint("T", "tasks"),
        hint("+/-", &format!("img({})", app.images.height_rows)),
        hint("w", if app.wrap { "wrap[on]" } else { "wrap[off]" }),
        hint("r/^L", "reload"),
    ]
    .concat()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        app_showing, assert_screen_contains, assert_screen_lacks, screen,
    };
    use crate::app::{App, NavigationEntry};
    use crate::search::{SearchDirection, SearchState};
    use std::path::PathBuf;

    fn status_row(app: &mut App) -> String {
        screen(app, 200, 8).pop().unwrap()
    }

    fn start_search(app: &mut App, query: &str, direction: SearchDirection) {
        let mut search = SearchState::new(direction);
        for c in query.chars() {
            search.insert_at_cursor(c);
        }
        let offsets: Vec<usize> = (0..app.rendered_doc.elements.len()).collect();
        search.update_matches(&app.rendered_doc.elements, 0, &offsets);
        app.search = Some(search);
    }

    #[test]
    fn normal_mode_shows_the_quit_and_search_hints() {
        let mut app = app_showing("hello\n");
        let row = status_row(&mut app);
        assert!(row.starts_with(" q  quit"), "{row:?}");
        assert!(row.contains(" /  search"), "{row:?}");
    }

    #[test]
    fn the_wrap_hint_reflects_the_current_setting() {
        let mut app = app_showing("hello\n");
        assert!(status_row(&mut app).contains("wrap[on]"));
        app.wrap = false;
        assert!(status_row(&mut app).contains("wrap[off]"));
    }

    #[test]
    fn the_back_hint_appears_only_with_navigation_history() {
        let mut app = app_showing("hello\n");
        assert!(!status_row(&mut app).contains("BS/^O"));
        app.nav_history.push(NavigationEntry {
            file_path: PathBuf::from("/notes/other.pn"),
            scroll_offset: 0,
        });
        assert!(status_row(&mut app).contains("BS/^O  back"));
    }

    #[test]
    fn a_focused_wikilink_offers_to_open_the_note() {
        let mut app = app_showing("see [other note]\n");
        app.focus_next_item();
        assert!(status_row(&mut app).contains("↵  open note"));
    }

    #[test]
    fn typing_a_search_replaces_the_hints_with_the_prompt() {
        let mut app = app_showing("hello world\n");
        start_search(&mut app, "wor", SearchDirection::Forward);
        let row = status_row(&mut app);
        assert!(row.starts_with("/wor"), "{row:?}");
        assert!(!row.contains("quit"), "{row:?}");
    }

    #[test]
    fn a_backward_search_prompt_starts_with_a_question_mark() {
        let mut app = app_showing("hello world\n");
        start_search(&mut app, "wor", SearchDirection::Backward);
        assert!(status_row(&mut app).starts_with("?wor"));
    }

    #[test]
    fn the_match_count_is_shown_while_typing() {
        let mut app = app_showing("hello world\nworld again\n");
        start_search(&mut app, "world", SearchDirection::Forward);
        assert!(status_row(&mut app).ends_with(" 1/2"));
    }

    #[test]
    fn a_query_without_matches_says_so() {
        let mut app = app_showing("hello\n");
        start_search(&mut app, "zzz", SearchDirection::Forward);
        assert!(status_row(&mut app).ends_with(" no match"));
    }

    #[test]
    fn a_confirmed_search_keeps_the_hints_and_shows_the_query_on_the_right() {
        let mut app = app_showing("hello world\n");
        start_search(&mut app, "world", SearchDirection::Forward);
        app.search.as_mut().unwrap().confirm();
        let row = status_row(&mut app);
        assert!(row.starts_with(" q  quit"), "{row:?}");
        assert!(row.contains(" n/N  next"), "{row:?}");
        assert!(row.ends_with("│/ world  1/1"), "{row:?}");
    }

    #[test]
    fn the_next_match_hint_needs_at_least_one_match() {
        let mut app = app_showing("hello\n");
        start_search(&mut app, "zzz", SearchDirection::Forward);
        app.search.as_mut().unwrap().confirm();
        let rows = screen(&mut app, 200, 8);
        assert_screen_lacks(&rows, "n/N");
        assert_screen_contains(&rows, "no match");
    }
}
