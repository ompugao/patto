use patto::parser::TaskStatus;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::config::TasksPanelPosition;
use crate::tasks::{
    fmt_timedelta, task_status_icon, total_elapsed, ReviewEntry, ReviewItem, TaskEntry, TaskItem,
    TasksView,
};
use patto::tasks_view::PendingGroup;

pub(super) fn draw_tasks_panel(frame: &mut Frame, app: &mut App) {
    let content_area = {
        let full = frame.area();
        // Reserve top title bar (1 row) and bottom status bar (1 row).
        Rect {
            x: full.x,
            y: full.y + 1,
            width: full.width,
            height: full.height.saturating_sub(2),
        }
    };

    let cfg = &app.tui_config.tasks;
    let panel_w = ((content_area.width as f64 * cfg.width.clamp(0.05, 1.0)) as u16).max(20);
    let panel_h = ((content_area.height as f64 * cfg.height.clamp(0.05, 1.0)) as u16).max(3);

    let (panel_x, panel_y) = match cfg.position {
        TasksPanelPosition::BottomRight => (
            content_area.x + content_area.width.saturating_sub(panel_w),
            content_area.y + content_area.height.saturating_sub(panel_h),
        ),
        TasksPanelPosition::BottomLeft => (
            content_area.x,
            content_area.y + content_area.height.saturating_sub(panel_h),
        ),
        TasksPanelPosition::TopRight => (
            content_area.x + content_area.width.saturating_sub(panel_w),
            content_area.y,
        ),
        TasksPanelPosition::TopLeft => (content_area.x, content_area.y),
    };

    let area = Rect {
        x: panel_x,
        y: panel_y,
        width: panel_w,
        height: panel_h,
    };

    // Clear the region behind the panel.
    frame.render_widget(Clear, area);

    let inner_width = area.width.saturating_sub(2) as usize; // inside borders

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

    let inner_h = inner.height as usize;

    if is_review {
        draw_tasks_review_content(frame, app, inner, inner_h, inner_width);
    } else {
        draw_tasks_upcoming_content(frame, app, inner, inner_h, inner_width);
    }
}

fn draw_tasks_upcoming_content(
    frame: &mut Frame,
    app: &App,
    inner: Rect,
    inner_h: usize,
    inner_width: usize,
) {
    let selected = app.tasks.list_state.selected;
    let entries = &app.tasks.entries;
    let total = entries.len();

    if total == 0 {
        frame.render_widget(
            Paragraph::new("(no tasks)").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    // Simple manual scroll: ensure selected item is visible.
    let scroll_offset = if let Some(sel) = selected {
        if sel >= inner_h {
            sel + 1 - inner_h
        } else {
            0
        }
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, entry) in entries.iter().enumerate().skip(scroll_offset) {
        if lines.len() >= inner_h {
            break;
        }
        let is_sel = selected == Some(i);
        match entry {
            TaskEntry::SectionHeader(title) => {
                lines.push(Line::from(Span::styled(
                    title.clone(),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            TaskEntry::Placeholder(msg) => {
                lines.push(Line::from(Span::styled(
                    msg.clone(),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            TaskEntry::Item(item) => {
                let TaskItem {
                    text,
                    file_name,
                    due_str,
                    group: category,
                    status,
                    base_time_spent,
                    started_at_dt,
                    ..
                } = item;
                // ── colours ───────────────────────────────────────────────
                let text_fg = match category {
                    PendingGroup::Overdue => Color::Red,
                    PendingGroup::Today => Color::Yellow,
                    _ => Color::White,
                };
                let (text_style, sel_prefix) = if is_sel {
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

                // ── due-date chip (plain text, coloured fg, no brackets) ──
                let due_chip_style = if is_sel {
                    text_style
                } else {
                    match category {
                        PendingGroup::Overdue => {
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                        }
                        PendingGroup::Today => Style::default().fg(Color::Yellow),
                        PendingGroup::Tomorrow => Style::default().fg(Color::Cyan),
                        _ => Style::default().fg(Color::DarkGray),
                    }
                };
                let due_chip = if due_str.is_empty() {
                    String::new()
                } else {
                    format!(" {}", due_str)
                };

                // ── time chips (doing/paused only, computed at render time) ──
                let time_chip = match status {
                    TaskStatus::Doing | TaskStatus::Paused => {
                        let elapsed = total_elapsed(status, *base_time_spent, *started_at_dt);
                        fmt_timedelta(elapsed)
                            .map(|s| format!(" ⏱ {}", s))
                            .unwrap_or_default()
                    }
                    _ => String::new(),
                };
                let started_chip = if matches!(status, TaskStatus::Doing) {
                    started_at_dt
                        .map(|dt| format!(" ▶ {}", dt.format("%H:%M")))
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                // ── layout: prefix + icon + due + " " + text + chips + file ──
                let prefix_str = format!("{}{} ", sel_prefix, task_status_icon(status));
                let suffix_str = format!("  {}", file_name);
                let fixed_chars = prefix_str.chars().count()
                    + due_chip.chars().count()
                    + 1 // space between due and text
                    + time_chip.chars().count()
                    + started_chip.chars().count()
                    + suffix_str.chars().count();
                let text_max = inner_width.saturating_sub(fixed_chars);
                let truncated_text = if text.chars().count() > text_max {
                    let s: String = text.chars().take(text_max.saturating_sub(1)).collect();
                    format!("{}…", s)
                } else {
                    text.clone()
                };

                // ── build spans ───────────────────────────────────────────
                let mut spans = vec![Span::styled(prefix_str, text_style)];
                if !due_chip.is_empty() {
                    spans.push(Span::styled(due_chip, due_chip_style));
                }
                spans.push(Span::styled(format!(" {}", truncated_text), text_style));
                if !time_chip.is_empty() {
                    let chip_style = if is_sel {
                        text_style
                    } else {
                        Style::default().fg(Color::Blue)
                    };
                    spans.push(Span::styled(time_chip, chip_style));
                }
                if !started_chip.is_empty() {
                    let chip_style = if is_sel {
                        text_style
                    } else {
                        Style::default().fg(Color::Yellow)
                    };
                    spans.push(Span::styled(started_chip, chip_style));
                }
                spans.push(Span::styled(
                    suffix_str,
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(spans));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_tasks_review_content(
    frame: &mut Frame,
    app: &App,
    inner: Rect,
    inner_h: usize,
    inner_width: usize,
) {
    let selected = app.tasks.review_list_state.selected;
    let entries = &app.tasks.review_entries;
    let total = entries.len();

    if total == 0 {
        frame.render_widget(
            Paragraph::new("(no completed tasks)").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    // Simple manual scroll: ensure selected item is visible.
    let scroll_offset = if let Some(sel) = selected {
        if sel >= inner_h {
            sel + 1 - inner_h
        } else {
            0
        }
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    for (i, entry) in entries.iter().enumerate().skip(scroll_offset) {
        if lines.len() >= inner_h {
            break;
        }
        let is_sel = selected == Some(i);
        match entry {
            ReviewEntry::SectionHeader(title) => {
                lines.push(Line::from(Span::styled(
                    title.clone(),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            ReviewEntry::Placeholder(msg) => {
                lines.push(Line::from(Span::styled(
                    msg.clone(),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            ReviewEntry::Item(item) => {
                let ReviewItem {
                    text,
                    file_name,
                    completed_at,
                    time_spent,
                    ..
                } = item;
                let base_style = Style::default().fg(Color::Green);
                let row_style = if is_sel {
                    base_style.add_modifier(Modifier::REVERSED)
                } else {
                    base_style
                };

                // Build row: "{>|space} ✓ {completed_at} {text} [⏱ ts]  {file}"
                let prefix = if is_sel { "> " } else { "  " };
                let done_chip = format!("✓ {} ", completed_at);
                let ts_part = {
                    fmt_timedelta(*time_spent)
                        .map(|s| format!(" ⏱{}", s))
                        .unwrap_or_default()
                };
                let suffix = format!("  {}", file_name);
                let fixed_len = prefix.chars().count()
                    + done_chip.chars().count()
                    + ts_part.chars().count()
                    + suffix.chars().count();
                let text_max = inner_width.saturating_sub(fixed_len);
                let truncated_text = if text.chars().count() > text_max {
                    let s: String = text.chars().take(text_max.saturating_sub(1)).collect();
                    format!("{}…", s)
                } else {
                    text.clone()
                };
                let row_text = format!(
                    "{}{}{}{}{}",
                    prefix, done_chip, truncated_text, ts_part, suffix
                );
                lines.push(Line::from(Span::styled(row_text, row_style)));
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}
