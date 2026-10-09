use patto::parser::TaskStatus;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::text::truncate_with_ellipsis;
use crate::app::App;
use crate::tasks::{fmt_timedelta, total_elapsed, TaskItem};

const MAX_ROWS: usize = 3;

/// Tasks being done or paused, pinned to the bottom-right corner of the
/// content area while the tasks panel is closed.
pub(super) fn draw_active_task_overlay(frame: &mut Frame, content_area: Rect, app: &App) {
    let max_width = (content_area.width * 60 / 100)
        .max(20)
        .min(content_area.width) as usize;
    let lines: Vec<Line> = app
        .tasks
        .active_tasks()
        .take(MAX_ROWS)
        .map(|item| overlay_line(item, max_width))
        .collect();
    if lines.is_empty() {
        return;
    }

    let width = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.chars().count())
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0)
        .min(max_width) as u16;
    let rows = lines.len() as u16;

    // One row above the bottom edge.
    let x = content_area.x + content_area.width.saturating_sub(width);
    let y = content_area
        .y
        .saturating_add(content_area.height)
        .saturating_sub(rows + 1);
    if y < content_area.y || content_area.height < rows + 1 {
        return;
    }

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Black)),
        Rect {
            x,
            y,
            width,
            height: rows,
        },
    );
}

fn overlay_line(item: &TaskItem, max_width: usize) -> Line<'static> {
    let (annotation, badge_style, text_style) = match item.status {
        TaskStatus::Doing => (
            "◑ doing",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Cyan).bg(Color::Black),
        ),
        _ => (
            "⏸ paused",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Yellow).bg(Color::Black),
        ),
    };

    let elapsed = total_elapsed(&item.status, item.base_time_spent, item.started_at_dt);
    let time_chip = fmt_timedelta(elapsed)
        .map(|s| format!(" ⏱ {}", s))
        .unwrap_or_default();

    let badge_chars = 1 + annotation.chars().count() + 2;
    let text = truncate_with_ellipsis(
        &item.text,
        max_width.saturating_sub(badge_chars + time_chip.chars().count()),
    );

    let mut spans = vec![
        Span::styled(format!(" {} ", annotation), badge_style),
        Span::styled(format!(" {}", text), text_style),
    ];
    if !time_chip.is_empty() {
        spans.push(Span::styled(
            time_chip,
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        app_showing, assert_screen_contains, assert_screen_lacks, screen,
    };
    use crate::app::App;
    use crate::tasks::{TaskEntry, TaskItem};
    use chrono::TimeDelta;
    use patto::parser::TaskStatus;
    use patto::tasks_view::PendingGroup;
    use tower_lsp::lsp_types::Url;

    fn app_with_tasks(statuses: &[TaskStatus]) -> App {
        let mut app = app_showing("hello\n");
        app.tasks.entries = statuses
            .iter()
            .enumerate()
            .map(|(i, status)| {
                TaskEntry::Item(TaskItem {
                    text: format!("task {i}"),
                    file_name: "a.pn".to_string(),
                    uri: Url::parse("file:///notes/a.pn").unwrap(),
                    line: 0,
                    due_str: String::new(),
                    group: PendingGroup::Later,
                    status: status.clone(),
                    base_time_spent: TimeDelta::minutes(5),
                    started_at_dt: None,
                })
            })
            .collect();
        app
    }

    #[test]
    fn tasks_being_done_or_paused_float_above_the_status_bar() {
        let mut app = app_with_tasks(&[TaskStatus::Todo, TaskStatus::Doing, TaskStatus::Paused]);
        let rows = screen(&mut app, 60, 10);
        assert_screen_lacks(&rows, "task 0");
        assert_eq!(rows[6].trim_start(), "◑ doing  task 1 ⏱ 5m");
        assert_eq!(rows[7].trim_start(), "⏸ paused  task 2 ⏱ 5m");
    }

    #[test]
    fn at_most_three_tasks_are_shown() {
        let doing = vec![
            TaskStatus::Doing,
            TaskStatus::Doing,
            TaskStatus::Doing,
            TaskStatus::Doing,
        ];
        let mut app = app_with_tasks(&doing);
        let rows = screen(&mut app, 60, 12);
        assert_screen_contains(&rows, "task 2");
        assert_screen_lacks(&rows, "task 3");
    }

    #[test]
    fn the_overlay_hides_while_the_tasks_panel_is_open() {
        let mut app = app_with_tasks(&[TaskStatus::Doing]);
        app.tasks.visible = true;
        let rows = screen(&mut app, 60, 10);
        assert_screen_lacks(&rows, "◑ doing");
    }

    #[test]
    fn nothing_is_drawn_without_active_tasks() {
        let mut app = app_with_tasks(&[TaskStatus::Todo]);
        let rows = screen(&mut app, 60, 10);
        assert_screen_lacks(&rows, "task 0");
    }
}
