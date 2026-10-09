use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;

pub(super) fn draw_title_bar(frame: &mut Frame, area: Rect, app: &App) {
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
