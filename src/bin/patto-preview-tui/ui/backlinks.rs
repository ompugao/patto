use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    Frame,
};
use tui_widget_list::{ListBuilder, ListView};

use crate::app::App;
use crate::backlinks::FlatEntry;

pub(super) fn draw_backlinks_popup(frame: &mut Frame, app: &mut App) {
    let popup_area = popup_area(frame.area());
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Backlinks & Two-hop Links ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let entries = app.backlinks.entries.clone();
    let item_count = entries.len();
    let builder = ListBuilder::new(move |context| {
        let line = entry_line(&entries[context.index], context.is_selected);
        (EntryWidget { line }, 1)
    });
    let list = ListView::new(builder, item_count);
    frame.render_stateful_widget(list, chunks[0], &mut app.backlinks.list_state);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " j/k:select  Enter:jump  b/Esc:close",
            Style::default().fg(Color::DarkGray),
        ))),
        chunks[1],
    );
}

/// 60% of the screen in each direction, at least 30x10, centred.
fn popup_area(area: Rect) -> Rect {
    let width = (area.width * 60 / 100).max(30).min(area.width - 4);
    let height = (area.height * 60 / 100).max(10).min(area.height - 4);
    Rect::new(
        (area.width - width) / 2,
        (area.height - height) / 2,
        width,
        height,
    )
}

fn entry_line(entry: &FlatEntry, is_selected: bool) -> Line<'static> {
    match entry {
        FlatEntry::SectionHeader(title) if title.is_empty() => Line::from(""),
        FlatEntry::SectionHeader(title) => Line::from(Span::styled(
            title.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        FlatEntry::BacklinkItem {
            source_file,
            line,
            context,
        } => Line::from(vec![
            Span::styled("  • ", selected_or(is_selected, Color::Yellow)),
            Span::styled(
                format!("{} (L{})", source_file, line + 1),
                selected_or(is_selected, Color::White),
            ),
            Span::styled(
                format!("  {}", context.as_deref().unwrap_or("")),
                selected_or(is_selected, Color::DarkGray),
            ),
        ]),
        FlatEntry::ViaHeader(via) => Line::from(vec![
            Span::styled("  via ", Style::default().fg(Color::DarkGray)),
            Span::styled(via.clone(), Style::default().fg(Color::White)),
            Span::styled(":", Style::default().fg(Color::DarkGray)),
        ]),
        FlatEntry::TwoHopItem(name) => Line::from(vec![
            Span::styled("    → ", selected_or(is_selected, Color::Yellow)),
            Span::styled(name.clone(), selected_or(is_selected, Color::White)),
        ]),
        FlatEntry::Placeholder(msg) => Line::from(Span::styled(
            msg.clone(),
            Style::default().fg(Color::DarkGray),
        )),
    }
}

fn selected_or(is_selected: bool, fg: Color) -> Style {
    if is_selected {
        Style::default().fg(Color::Black).bg(Color::Yellow)
    } else {
        Style::default().fg(fg)
    }
}

struct EntryWidget {
    line: Line<'static>,
}

impl Widget for EntryWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.line).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{app_showing, assert_screen_contains, screen};
    use crate::app::App;
    use crate::backlinks::FlatEntry;

    fn app_with_popup(entries: Vec<FlatEntry>) -> App {
        let mut app = app_showing("hello\n");
        app.backlinks.visible = true;
        app.backlinks.entries = entries;
        app
    }

    #[test]
    fn the_popup_is_titled_and_shows_its_key_hints() {
        let mut app = app_with_popup(vec![FlatEntry::Placeholder("  (none)".to_string())]);
        let rows = screen(&mut app, 80, 24);
        assert_screen_contains(&rows, "Backlinks & Two-hop Links");
        assert_screen_contains(&rows, "(none)");
        assert_screen_contains(&rows, "j/k:select  Enter:jump  b/Esc:close");
    }

    #[test]
    fn a_backlink_shows_its_file_one_based_line_and_context() {
        let mut app = app_with_popup(vec![
            FlatEntry::SectionHeader("Backlinks:".to_string()),
            FlatEntry::BacklinkItem {
                source_file: "x.pn".to_string(),
                line: 4,
                context: Some("some context".to_string()),
            },
        ]);
        let rows = screen(&mut app, 80, 24);
        assert_screen_contains(&rows, "Backlinks:");
        assert_screen_contains(&rows, "• x.pn (L5)  some context");
    }

    #[test]
    fn two_hop_links_are_listed_under_the_note_they_go_through() {
        let mut app = app_with_popup(vec![
            FlatEntry::ViaHeader("hub".to_string()),
            FlatEntry::TwoHopItem("target".to_string()),
        ]);
        let rows = screen(&mut app, 80, 24);
        assert_screen_contains(&rows, "via hub:");
        assert_screen_contains(&rows, "→ target");
    }

    #[test]
    fn the_popup_is_centred_over_the_content() {
        let mut app = app_with_popup(Vec::new());
        let rows = screen(&mut app, 80, 24);
        assert!(rows[5].trim_start().starts_with('┌'), "{:?}", rows[5]);
        assert!(rows[18].trim_start().starts_with('└'), "{:?}", rows[18]);
    }
}
