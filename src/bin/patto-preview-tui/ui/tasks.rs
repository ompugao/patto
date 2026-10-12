use patto::parser::TaskStatus;
use patto::tasks_view::PendingGroup;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::text::truncate_with_ellipsis;
use crate::app::App;
use crate::config::{TasksPanelConfig, TasksPanelPosition};
use crate::tasks::{
    fmt_timedelta, task_status_icon, total_elapsed, ReviewEntry, ReviewItem, TaskEntry, TaskItem,
    TasksView,
};

pub(super) fn draw_tasks_panel(frame: &mut Frame, app: &App) {
    let area = panel_area(frame.area(), &app.tui_config.tasks);
    frame.render_widget(Clear, area);

    let is_review = app.tasks.view == TasksView::Review;
    let title = if is_review {
        " Tasks Review  [R:upcoming] "
    } else {
        " Tasks  [R:review] "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if is_review {
        draw_review(frame, app, inner);
    } else {
        draw_upcoming(frame, app, inner);
    }
}

/// The panel floats in the content area, which is the terminal minus the
/// title and status bars.
fn panel_area(full: Rect, cfg: &TasksPanelConfig) -> Rect {
    let content = Rect {
        x: full.x,
        y: full.y + 1,
        width: full.width,
        height: full.height.saturating_sub(2),
    };
    let width = ((content.width as f64 * cfg.width.clamp(0.05, 1.0)) as u16).max(20);
    let height = ((content.height as f64 * cfg.height.clamp(0.05, 1.0)) as u16).max(3);
    let right = content.x + content.width.saturating_sub(width);
    let bottom = content.y + content.height.saturating_sub(height);
    let (x, y) = match cfg.position {
        TasksPanelPosition::BottomRight => (right, bottom),
        TasksPanelPosition::BottomLeft => (content.x, bottom),
        TasksPanelPosition::TopRight => (right, content.y),
        TasksPanelPosition::TopLeft => (content.x, content.y),
    };
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn draw_upcoming(frame: &mut Frame, app: &App, inner: Rect) {
    let panel = &app.tasks;
    if panel.entries.is_empty() {
        draw_empty_notice(frame, inner, "(no tasks)");
        return;
    }
    let width = inner.width as usize;
    let lines = visible_lines(
        &panel.entries,
        panel.list_state.selected,
        inner.height as usize,
        |entry, is_selected| match entry {
            TaskEntry::SectionHeader(title) => section_header_line(title),
            TaskEntry::Placeholder(msg) => placeholder_line(msg),
            TaskEntry::Item(item) => task_item_line(item, is_selected, width),
        },
    );
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_review(frame: &mut Frame, app: &App, inner: Rect) {
    let panel = &app.tasks;
    if panel.review_entries.is_empty() {
        draw_empty_notice(frame, inner, "(no completed tasks)");
        return;
    }
    let width = inner.width as usize;
    let lines = visible_lines(
        &panel.review_entries,
        panel.review_list_state.selected,
        inner.height as usize,
        |entry, is_selected| match entry {
            ReviewEntry::SectionHeader(title) => section_header_line(title),
            ReviewEntry::Placeholder(msg) => placeholder_line(msg),
            ReviewEntry::Item(item) => review_item_line(item, is_selected, width),
        },
    );
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_empty_notice(frame: &mut Frame, inner: Rect, notice: &'static str) {
    frame.render_widget(
        Paragraph::new(notice).style(Style::default().fg(Color::DarkGray)),
        inner,
    );
}

/// The `rows` entries to show, scrolled just far enough to keep the
/// selected one in view.
fn visible_lines<T>(
    entries: &[T],
    selected: Option<usize>,
    rows: usize,
    line_for: impl Fn(&T, bool) -> Line<'static>,
) -> Vec<Line<'static>> {
    let first = match selected {
        Some(sel) if sel >= rows => sel + 1 - rows,
        _ => 0,
    };
    entries
        .iter()
        .enumerate()
        .skip(first)
        .take(rows)
        .map(|(i, entry)| line_for(entry, selected == Some(i)))
        .collect()
}

fn section_header_line(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    ))
}

fn placeholder_line(msg: &str) -> Line<'static> {
    Line::from(Span::styled(
        msg.to_string(),
        Style::default().fg(Color::DarkGray),
    ))
}

/// `>○  due text ⏱ 1h ▶ 10:30  file.pn`, with the text cut to fit `width`.
fn task_item_line(item: &TaskItem, is_selected: bool, width: usize) -> Line<'static> {
    let text_fg = match item.group {
        PendingGroup::Overdue => Color::Red,
        PendingGroup::Today => Color::Yellow,
        _ => Color::White,
    };
    let (text_style, marker) = if is_selected {
        (
            Style::default()
                .fg(Color::Black)
                .bg(text_fg)
                .add_modifier(Modifier::BOLD),
            ">",
        )
    } else {
        (Style::default().fg(text_fg), " ")
    };
    let chip_style = |unselected: Style| if is_selected { text_style } else { unselected };

    let prefix = format!("{}{} ", marker, task_status_icon(&item.status));
    let due_chip = if item.due_str.is_empty() {
        String::new()
    } else {
        format!(" {}", item.due_str)
    };
    let (time_chip, started_chip) = timing_chips(item);
    let suffix = format!("  {}", item.file_name);
    let fixed_chars = prefix.chars().count()
        + due_chip.chars().count()
        + 1
        + time_chip.chars().count()
        + started_chip.chars().count()
        + suffix.chars().count();
    let text = truncate_with_ellipsis(&item.text, width.saturating_sub(fixed_chars));

    let mut spans = vec![Span::styled(prefix, text_style)];
    if !due_chip.is_empty() {
        spans.push(Span::styled(
            due_chip,
            chip_style(due_chip_style(item.group)),
        ));
    }
    spans.push(Span::styled(format!(" {}", text), text_style));
    if !time_chip.is_empty() {
        spans.push(Span::styled(
            time_chip,
            chip_style(Style::default().fg(Color::Blue)),
        ));
    }
    if !started_chip.is_empty() {
        spans.push(Span::styled(
            started_chip,
            chip_style(Style::default().fg(Color::Yellow)),
        ));
    }
    spans.push(Span::styled(suffix, Style::default().fg(Color::DarkGray)));
    Line::from(spans)
}

fn due_chip_style(group: PendingGroup) -> Style {
    match group {
        PendingGroup::Overdue => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        PendingGroup::Today => Style::default().fg(Color::Yellow),
        PendingGroup::Tomorrow => Style::default().fg(Color::Cyan),
        _ => Style::default().fg(Color::DarkGray),
    }
}

/// The elapsed-time chip of a task being done or paused, and the start-time
/// chip of one being done. Empty strings otherwise.
fn timing_chips(item: &TaskItem) -> (String, String) {
    let time_chip = match item.status {
        TaskStatus::Doing | TaskStatus::Paused => {
            let elapsed = total_elapsed(&item.status, item.base_time_spent, item.started_at_dt);
            fmt_timedelta(elapsed)
                .map(|s| format!(" ⏱ {}", s))
                .unwrap_or_default()
        }
        _ => String::new(),
    };
    let started_chip = match (&item.status, item.started_at_dt) {
        (TaskStatus::Doing, Some(dt)) => format!(" ▶ {}", dt.format("%H:%M")),
        _ => String::new(),
    };
    (time_chip, started_chip)
}

/// `> ✓ 2024-05-05 text ⏱1h  file.pn`, with the text cut to fit `width`.
fn review_item_line(item: &ReviewItem, is_selected: bool, width: usize) -> Line<'static> {
    let base_style = Style::default().fg(Color::Green);
    let row_style = if is_selected {
        base_style.add_modifier(Modifier::REVERSED)
    } else {
        base_style
    };
    let marker = if is_selected { "> " } else { "  " };
    let done_chip = format!("✓ {} ", item.completed_at);
    let time_chip = fmt_timedelta(item.time_spent)
        .map(|s| format!(" ⏱{}", s))
        .unwrap_or_default();
    let suffix = format!("  {}", item.file_name);
    let fixed_chars = marker.chars().count()
        + done_chip.chars().count()
        + time_chip.chars().count()
        + suffix.chars().count();
    let text = truncate_with_ellipsis(&item.text, width.saturating_sub(fixed_chars));
    Line::from(Span::styled(
        format!("{marker}{done_chip}{text}{time_chip}{suffix}"),
        row_style,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        app_showing, assert_screen_contains, assert_screen_lacks, screen,
    };
    use crate::app::App;
    use crate::config::TasksPanelPosition;
    use crate::tasks::{ReviewEntry, ReviewItem, TaskEntry, TaskItem, TasksView};
    use chrono::TimeDelta;
    use patto::parser::TaskStatus;
    use patto::tasks_view::PendingGroup;
    use tower_lsp::lsp_types::Url;

    fn task(text: &str, status: TaskStatus) -> TaskItem {
        TaskItem {
            text: text.to_string(),
            file_name: "a.pn".to_string(),
            uri: Url::parse("file:///notes/a.pn").unwrap(),
            line: 0,
            due_str: "2024-01-01".to_string(),
            group: PendingGroup::Overdue,
            status,
            base_time_spent: TimeDelta::minutes(90),
            started_at_dt: None,
        }
    }

    fn app_with_panel(entries: Vec<TaskEntry>) -> App {
        let mut app = app_showing("hello\n");
        app.tasks.visible = true;
        app.tasks.entries = entries;
        app.tui_config.tasks.width = 0.9;
        app
    }

    #[test]
    fn an_empty_panel_says_there_are_no_tasks() {
        let mut app = app_with_panel(Vec::new());
        let rows = screen(&mut app, 80, 24);
        assert_screen_contains(&rows, "Tasks  [R:review]");
        assert_screen_contains(&rows, "(no tasks)");
    }

    #[test]
    fn a_task_row_shows_icon_due_date_text_and_file() {
        let mut app = app_with_panel(vec![
            TaskEntry::SectionHeader("⚠  Overdue".to_string()),
            TaskEntry::Item(task("buy milk", TaskStatus::Todo)),
        ]);
        let rows = screen(&mut app, 80, 24);
        assert_screen_contains(&rows, "⚠  Overdue");
        assert_screen_contains(&rows, " ○  2024-01-01 buy milk  a.pn");
    }

    #[test]
    fn the_selected_row_is_marked_with_an_arrow() {
        let mut app = app_with_panel(vec![
            TaskEntry::SectionHeader("⚠  Overdue".to_string()),
            TaskEntry::Item(task("buy milk", TaskStatus::Todo)),
        ]);
        app.tasks.list_state.select(Some(1));
        let rows = screen(&mut app, 80, 24);
        assert_screen_contains(&rows, ">○  2024-01-01 buy milk  a.pn");
    }

    #[test]
    fn a_paused_task_shows_its_time_spent() {
        let mut app = app_with_panel(vec![TaskEntry::Item(task("paused", TaskStatus::Paused))]);
        let rows = screen(&mut app, 80, 24);
        assert_screen_contains(&rows, " ⏸  2024-01-01 paused ⏱ 1h30m  a.pn");
    }

    #[test]
    fn long_task_text_is_cut_with_an_ellipsis() {
        let mut app = app_with_panel(vec![TaskEntry::Item(task(
            "a very long task description that cannot fit in the panel",
            TaskStatus::Todo,
        ))]);
        let rows = screen(&mut app, 80, 24);
        assert_screen_contains(&rows, "…  a.pn");
    }

    #[test]
    fn the_review_view_lists_completed_tasks_with_their_date() {
        let mut app = app_with_panel(Vec::new());
        app.tasks.view = TasksView::Review;
        app.tasks.review_entries = vec![
            ReviewEntry::SectionHeader("✓ Today".to_string()),
            ReviewEntry::Item(ReviewItem {
                text: "shipped".to_string(),
                file_name: "a.pn".to_string(),
                uri: Url::parse("file:///notes/a.pn").unwrap(),
                line: 0,
                completed_at: "2024-05-05".to_string(),
                time_spent: TimeDelta::minutes(130),
            }),
        ];
        let rows = screen(&mut app, 80, 24);
        assert_screen_contains(&rows, "Tasks Review  [R:upcoming]");
        assert_screen_contains(&rows, "  ✓ 2024-05-05 shipped ⏱2h10m  a.pn");
    }

    #[test]
    fn the_panel_anchors_to_the_configured_corner() {
        let mut app = app_with_panel(Vec::new());
        app.tui_config.tasks.position = TasksPanelPosition::TopLeft;
        let rows = screen(&mut app, 80, 24);
        assert!(rows[1].starts_with('┌'), "{:?}", rows[1]);

        app.tui_config.tasks.position = TasksPanelPosition::BottomRight;
        let rows = screen(&mut app, 80, 24);
        assert!(rows[22].ends_with('┘'), "{:?}", rows[22]);
        assert_screen_lacks(&rows[..2], "┌");
    }
}
