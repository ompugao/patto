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
    let area = frame.area();
    let popup_width = (area.width * 60 / 100).max(30).min(area.width - 4);
    let popup_height = (area.height * 60 / 100).max(10).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

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

    // Reserve the last row for the key-hint line.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    // Build the list view from flat entries.
    let entries = app.backlinks.entries.clone();
    let item_count = entries.len();

    let builder = ListBuilder::new(move |context| {
        let entry = &entries[context.index];
        let is_selected = context.is_selected;

        let line: Line<'static> = match entry {
            FlatEntry::SectionHeader(title) => {
                if title.is_empty() {
                    Line::from("")
                } else {
                    Line::from(Span::styled(
                        title.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                }
            }
            FlatEntry::BacklinkItem {
                source_file,
                line,
                context,
            } => {
                let ctx_text = context.as_deref().unwrap_or("");
                let (bullet_style, text_style, ctx_style) = if is_selected {
                    (
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    )
                } else {
                    (
                        Style::default().fg(Color::Yellow),
                        Style::default().fg(Color::White),
                        Style::default().fg(Color::DarkGray),
                    )
                };
                Line::from(vec![
                    Span::styled("  • ", bullet_style),
                    Span::styled(format!("{} (L{})", source_file, line + 1), text_style),
                    Span::styled(format!("  {}", ctx_text), ctx_style),
                ])
            }
            FlatEntry::ViaHeader(via) => Line::from(vec![
                Span::styled("  via ", Style::default().fg(Color::DarkGray)),
                Span::styled(via.clone(), Style::default().fg(Color::White)),
                Span::styled(":", Style::default().fg(Color::DarkGray)),
            ]),
            FlatEntry::TwoHopItem(name) => {
                let (arrow_style, name_style) = if is_selected {
                    (
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    )
                } else {
                    (
                        Style::default().fg(Color::Yellow),
                        Style::default().fg(Color::White),
                    )
                };
                Line::from(vec![
                    Span::styled("    → ", arrow_style),
                    Span::styled(name.clone(), name_style),
                ])
            }
            FlatEntry::Placeholder(msg) => Line::from(Span::styled(
                msg.clone(),
                Style::default().fg(Color::DarkGray),
            )),
        };

        // All entries are 1 row tall.
        let widget = EntryWidget { line };
        (widget, 1)
    });

    let list = ListView::new(builder, item_count);
    frame.render_stateful_widget(list, chunks[0], &mut app.backlinks.list_state);

    // Key hint
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " j/k:select  Enter:jump  b/Esc:close",
            Style::default().fg(Color::DarkGray),
        ))),
        chunks[1],
    );
}

/// A simple single-line widget used as a list item.
struct EntryWidget {
    line: Line<'static>,
}

impl Widget for EntryWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.line).render(area, buf);
    }
}
