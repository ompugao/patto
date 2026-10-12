use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use ratatui_image::StatefulImage;
use std::path::Path;

use crate::app::App;
use crate::image_cache::{CachedImage, ImageCache};

/// The image, or a text placeholder while it loads or when it failed, inside
/// a focus border when `focused`.
pub(super) fn draw_image_cell(
    frame: &mut Frame,
    images: &mut ImageCache,
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

    let name = alt.unwrap_or(src);
    let (label, color) = match images.get_mut(src) {
        Some(CachedImage::Loaded(protocol)) => {
            frame.render_stateful_widget(StatefulImage::default(), render_area, protocol);
            return;
        }
        Some(CachedImage::Failed(err)) => (format!("[Image: {} — {}]", name, err), Color::Red),
        Some(CachedImage::Pending) => (format!("[Image: {} — loading…]", name), Color::DarkGray),
        None => (format!("[Image: {}]", name), Color::DarkGray),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            label,
            Style::default().fg(color),
        )])),
        render_area,
    );
}

pub(super) fn draw_fullscreen_image(frame: &mut Frame, app: &mut App, root_dir: &Path, src: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    app.images.load(src, root_dir);
    draw_image_cell(frame, &mut app.images, src, None, chunks[0], false);

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

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        app_showing, assert_screen_contains, assert_screen_lacks, screen,
    };

    // The tests have no terminal, so there is no image protocol and every
    // image is drawn as its placeholder.

    #[test]
    fn an_image_without_a_protocol_shows_its_placeholder() {
        let mut app = app_showing("[@img ./cat.png]\n");
        let rows = screen(&mut app, 60, 16);
        assert_screen_contains(&rows, "[Image: ./cat.png]");
    }

    #[test]
    fn a_focused_image_is_framed_with_the_fullscreen_hint() {
        let mut app = app_showing("[@img ./cat.png]\n");
        app.focus_next_item();
        let rows = screen(&mut app, 60, 16);
        assert_screen_contains(&rows, "Enter:fullscreen");
        assert_screen_contains(&rows, "│[Image: ./cat.png]");
    }

    #[test]
    fn the_fullscreen_view_replaces_the_frame_and_names_the_image() {
        let mut app = app_showing("text\n[@img ./cat.png]\n");
        app.images.fullscreen_src = Some("./cat.png".to_string());
        let rows = screen(&mut app, 60, 10);
        assert_eq!(rows[0], "[Image: ./cat.png]");
        assert_eq!(rows[9], " Esc:close ./cat.png");
        assert_screen_lacks(&rows, "note.pn");
    }
}
