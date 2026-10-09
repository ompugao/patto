use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::path::{Path, PathBuf};

use crate::app::App;

pub(super) fn app_showing(content: &str) -> App {
    let mut app = App::new(
        PathBuf::from("/notes/note.pn"),
        PathBuf::from("/notes"),
        None,
    );
    app.viewport_width = 60;
    app.viewport_height = 18;
    app.re_render(content);
    app
}

/// Draw into an off-screen buffer and return it as one string per row.
pub(super) fn screen(app: &mut App, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| super::draw(frame, app, Path::new("/notes")))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

pub(super) fn assert_screen_contains(rows: &[String], needle: &str) {
    assert!(
        rows.iter().any(|row| row.contains(needle)),
        "expected {needle:?} on screen:\n{}",
        rows.join("\n")
    );
}

pub(super) fn assert_screen_lacks(rows: &[String], needle: &str) {
    assert!(
        !rows.iter().any(|row| row.contains(needle)),
        "did not expect {needle:?} on screen:\n{}",
        rows.join("\n")
    );
}
