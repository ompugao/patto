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

///
/// If `focused` is true, draws a yellow border around the cell and renders the
/// image (or placeholder) inside the inner area.  Otherwise renders directly
/// into `area`.
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

    match images.get_mut(src) {
        Some(CachedImage::Loaded(protocol)) => {
            let image_widget = StatefulImage::default();
            frame.render_stateful_widget(image_widget, render_area, protocol);
        }
        Some(CachedImage::Failed(err)) => {
            let label = format!("[Image: {} — {}]", alt.unwrap_or(src), err);
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    label,
                    Style::default().fg(Color::Red),
                )])),
                render_area,
            );
        }
        Some(CachedImage::Pending) => {
            let label = format!("[Image: {} — loading…]", alt.unwrap_or(src));
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    label,
                    Style::default().fg(Color::DarkGray),
                )])),
                render_area,
            );
        }
        None => {
            let label = format!("[Image: {}]", alt.unwrap_or(src));
            frame.render_widget(
                Paragraph::new(Line::from(vec![Span::styled(
                    label,
                    Style::default().fg(Color::DarkGray),
                )])),
                render_area,
            );
        }
    }
}

pub(super) fn draw_fullscreen_image(frame: &mut Frame, app: &mut App, root_dir: &Path, src: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    // Load image if needed
    app.images.load(src, root_dir);

    draw_image_cell(frame, &mut app.images, src, None, chunks[0], false);

    // Status hint
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
