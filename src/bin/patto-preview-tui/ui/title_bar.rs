use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;

pub(super) fn draw_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let (position, percent) = scroll_position(app);
    let right_text = format!("{}│{}", position, percent);
    let right_len = right_text.chars().count() as u16;
    let left_len = area.width.saturating_sub(right_len);

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
            format!(" {} ", file_name(app)),
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let right = Line::from(vec![
        Span::styled(
            position,
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled("│", Style::default().fg(Color::DarkGray).bg(Color::Black)),
        Span::styled(
            percent,
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

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

fn file_name(app: &App) -> String {
    app.file_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// `(" row:total ", " pct% ")` for the top of the viewport.
fn scroll_position(app: &App) -> (String, String) {
    let total = app.total_display_height();
    match ((app.scroll_offset + 1) * 100).checked_div(total) {
        Some(percent) => (
            format!(" {}:{} ", app.scroll_offset + 1, total),
            format!(" {}% ", percent.min(100)),
        ),
        None => (" 0:0 ".to_string(), " 0% ".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{app_showing, screen};

    #[test]
    fn the_title_bar_names_the_app_and_the_note() {
        let mut app = app_showing("one\ntwo\n");
        let rows = screen(&mut app, 60, 6);
        assert!(
            rows[0].starts_with(" ◉ patto   │   note.pn"),
            "{:?}",
            rows[0]
        );
    }

    #[test]
    fn the_right_side_shows_the_top_row_and_its_percentage() {
        let content: String = (0..10).map(|i| format!("line {i}\n")).collect();
        let mut app = app_showing(&content);
        let rows = screen(&mut app, 60, 6);
        assert!(rows[0].ends_with(" 1:10 │ 10%"), "{:?}", rows[0]);

        app.scroll_down(4);
        let rows = screen(&mut app, 60, 6);
        assert!(rows[0].ends_with(" 5:10 │ 50%"), "{:?}", rows[0]);
    }

    #[test]
    fn an_empty_note_reports_zero_rows() {
        let mut app = app_showing("");
        let rows = screen(&mut app, 60, 6);
        assert!(rows[0].ends_with(" 0:0 │ 0%"), "{:?}", rows[0]);
    }
}
