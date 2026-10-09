use patto::parser::TaskStatus;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::tasks::{fmt_timedelta, total_elapsed};

/// Fidget-style overlay showing active (Doing/Paused) tasks in the bottom-right corner
/// of the content area. Only drawn when the tasks panel is closed.
pub(super) fn draw_active_task_overlay(frame: &mut Frame, content_area: Rect, app: &App) {
    let active = app.tasks.active_tasks();
    if active.is_empty() {
        return;
    }

    // Show at most 3 tasks.
    let max_rows = 3usize;
    let tasks_to_show: Vec<_> = active.iter().take(max_rows).collect();
    let _num_rows = tasks_to_show.len() as u16;

    // Max width cap: 60% of content width, min 20 cols.
    let max_w = (content_area.width * 60 / 100)
        .max(20)
        .min(content_area.width) as usize;

    // Build lines first so we can measure actual rendered width.
    let mut lines: Vec<Line> = Vec::new();
    for (status, text, base_time_spent, started_at_dt) in &tasks_to_show {
        let (annotation, ann_style, text_style) = match status {
            TaskStatus::Doing => (
                "◑ doing",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Cyan).bg(Color::Black),
            ),
            TaskStatus::Paused => (
                "⏸ paused",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Yellow).bg(Color::Black),
            ),
            _ => continue,
        };

        let elapsed = total_elapsed(status, *base_time_spent, *started_at_dt);
        let time_chip = fmt_timedelta(elapsed)
            .map(|s| format!(" ⏱ {}", s))
            .unwrap_or_default();

        // " {annotation} " prefix + " " gap
        let prefix_len = 1 + annotation.chars().count() + 2;
        let text_max = max_w.saturating_sub(prefix_len + time_chip.chars().count());
        let truncated = if text.chars().count() > text_max {
            let s: String = text.chars().take(text_max.saturating_sub(1)).collect();
            format!("{}…", s)
        } else {
            text.clone()
        };

        let mut spans = vec![
            Span::styled(format!(" {} ", annotation), ann_style),
            Span::styled(format!(" {}", truncated), text_style),
        ];
        if !time_chip.is_empty() {
            spans.push(Span::styled(
                time_chip,
                Style::default().fg(Color::DarkGray).bg(Color::Black),
            ));
        }
        lines.push(Line::from(spans));
    }

    if lines.is_empty() {
        return;
    }

    // Measure actual width needed (sum of span char widths per line).
    let actual_w = lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.chars().count())
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0)
        .min(max_w) as u16;

    let num_rows = lines.len() as u16;

    // Position: bottom-right of content area, 1 row above the bottom edge.
    let x = content_area.x + content_area.width.saturating_sub(actual_w);
    let y = content_area
        .y
        .saturating_add(content_area.height)
        .saturating_sub(num_rows + 1);

    if y < content_area.y || content_area.height < num_rows + 1 {
        return;
    }

    let overlay_area = Rect {
        x,
        y,
        width: actual_w,
        height: num_rows,
    };

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Black)),
        overlay_area,
    );
}
