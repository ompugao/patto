mod backlinks;
mod content;
mod highlight;
mod image;
mod status_bar;
mod task_overlay;
mod tasks;
mod title_bar;

#[cfg(test)]
mod test_support;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use std::path::Path;

use crate::app::App;

pub(crate) fn draw(frame: &mut Frame, app: &mut App, root_dir: &Path) {
    if let Some(ref src) = app.images.fullscreen_src.clone() {
        image::draw_fullscreen_image(frame, app, root_dir, src);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    title_bar::draw_title_bar(frame, chunks[0], app);
    content::draw_content(frame, chunks[1], app, root_dir);
    status_bar::draw_status_bar(frame, chunks[2], app);

    if !app.tasks.visible {
        task_overlay::draw_active_task_overlay(frame, chunks[1], app);
    }

    if app.backlinks.visible {
        backlinks::draw_backlinks_popup(frame, app);
    }

    if app.tasks.visible {
        tasks::draw_tasks_panel(frame, app);
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{app_showing, assert_screen_contains, screen};

    #[test]
    fn draws_a_title_bar_the_content_and_a_status_bar() {
        let mut app = app_showing("hello world\n\tnested line\n");
        let rows = screen(&mut app, 60, 12);

        assert_screen_contains(&rows, "note.pn");
        assert_screen_contains(&rows, "hello world");
        assert_screen_contains(&rows, "nested line");
        assert_screen_contains(&rows, "1:");
    }

    #[test]
    fn an_empty_document_still_draws_its_chrome() {
        let mut app = app_showing("");
        let rows = screen(&mut app, 60, 6);
        assert_screen_contains(&rows, "note.pn");
    }
}
