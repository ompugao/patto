//! Aggregated task views and status changes.
//!
//! Status changes reuse the same pipeline the language server runs on save, so
//! a task marked done on the phone gets `completed_at` (and a flushed
//! `started_at` → `time_spent`) written exactly as it would on the desktop.

use std::collections::HashMap;

use chrono::NaiveDate;
use patto::parser::{self, TaskStatus as CoreTaskStatus};
use patto::task_edits;
use patto::tasks_view::{self, ReviewTimeframe};

use crate::api::error::{PattoError, PattoResult};
use crate::api::index::{self, TaskRecord};
use crate::api::store;
use crate::api::types::{TaskDate, TaskInfo, TaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingGroup {
    Overdue,
    Today,
    ThisWeek,
    Later,
    NoDue,
}

/// The app shows five buckets; `tasks_view` distinguishes seven.
fn pending_group_of(due: &parser::Deadline, today: NaiveDate) -> PendingGroup {
    use tasks_view::PendingGroup as Core;
    match tasks_view::pending_group(due, today) {
        Core::Overdue => PendingGroup::Overdue,
        Core::Today => PendingGroup::Today,
        Core::Tomorrow | Core::ThisWeek => PendingGroup::ThisWeek,
        Core::ThisMonth | Core::Later => PendingGroup::Later,
        Core::Uninterpretable => PendingGroup::NoDue,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskItem {
    pub rel_path: String,
    pub note_name: String,
    pub row: u32,
    pub text: String,
    pub info: TaskInfo,
    pub group: PendingGroup,
    /// Set only for completed tasks, `YYYY-MM-DD`.
    pub completed_on: Option<String>,
}

fn info_of(record: &TaskRecord) -> TaskInfo {
    TaskInfo {
        status: (&record.status).into(),
        due: TaskDate::from_deadline(&record.due),
        scheduled: record.scheduled.as_ref().and_then(TaskDate::from_deadline),
        completed_at: record.completed_at.map(|d| TaskDate {
            text: d.format("%Y-%m-%d").to_string(),
            kind: crate::api::types::DateKind::Date,
        }),
        started_at: record.started_at.as_ref().and_then(TaskDate::from_deadline),
        time_spent_minutes: record.time_spent_minutes,
        is_shorthand: record.is_shorthand,
    }
}

/// Every task that is not done, earliest deadline first.
pub fn pending_tasks(root: String, today: Option<String>) -> PattoResult<Vec<TaskItem>> {
    let today = parse_today(today);

    index::with_index(&root, |idx| {
        let mut out: Vec<TaskItem> = Vec::new();
        for note in idx.notes.values() {
            for task in &note.tasks {
                if matches!(task.status, CoreTaskStatus::Done) {
                    continue;
                }
                out.push(TaskItem {
                    rel_path: note.rel_path.clone(),
                    note_name: note.name.clone(),
                    row: task.row,
                    text: task.label.clone(),
                    info: info_of(task),
                    group: pending_group_of(&task.due, today),
                    completed_on: None,
                });
            }
        }

        out.sort_by(|a, b| {
            let (ka, kb) = (due_sort_key_of(a), due_sort_key_of(b));
            ka.cmp(&kb)
                .then(a.note_name.cmp(&b.note_name))
                .then(a.row.cmp(&b.row))
        });
        Ok(out)
    })
}

fn due_sort_key_of(item: &TaskItem) -> (u8, String) {
    match &item.info.due {
        Some(d) => (0, d.text.clone()),
        None => (1, String::new()),
    }
}

/// Completed tasks whose `completed_at` falls in the timeframe, newest first.
pub fn completed_tasks(
    root: String,
    timeframe: String,
    from: Option<String>,
    to: Option<String>,
    today: Option<String>,
) -> PattoResult<Vec<TaskItem>> {
    let today = parse_today(today);
    let parse = |s: Option<String>| s.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
    let timeframe = ReviewTimeframe::from_name(&timeframe, parse(from), parse(to));
    let (from, to) = tasks_view::timeframe_bounds(&timeframe, today);

    index::with_index(&root, |idx| {
        let mut out: Vec<TaskItem> = Vec::new();
        for note in idx.notes.values() {
            for task in &note.tasks {
                if !matches!(task.status, CoreTaskStatus::Done) {
                    continue;
                }
                let Some(date) = task.completed_at else {
                    continue;
                };
                if from.map(|f| date < f).unwrap_or(false) || to.map(|t| date > t).unwrap_or(false)
                {
                    continue;
                }
                out.push(TaskItem {
                    rel_path: note.rel_path.clone(),
                    note_name: note.name.clone(),
                    row: task.row,
                    text: task.label.clone(),
                    info: info_of(task),
                    group: PendingGroup::NoDue,
                    completed_on: Some(date.format("%Y-%m-%d").to_string()),
                });
            }
        }

        out.sort_by(|a, b| {
            b.completed_on
                .cmp(&a.completed_on)
                .then(a.note_name.cmp(&b.note_name))
                .then(a.row.cmp(&b.row))
        });
        Ok(out)
    })
}

fn parse_today(today: Option<String>) -> NaiveDate {
    today
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEditResult {
    pub row: u32,
    pub new_line: String,
}

/// Set the status of the task on `row`, rewriting the line the way the LSP does.
pub fn set_task_status(
    root: String,
    rel_path: String,
    row: u32,
    status: TaskStatus,
) -> PattoResult<TaskEditResult> {
    set_task_status_at(
        root,
        rel_path,
        row,
        status,
        chrono::Local::now().naive_local(),
    )
}

pub(crate) fn set_task_status_at(
    root: String,
    rel_path: String,
    row: u32,
    status: TaskStatus,
    now: chrono::NaiveDateTime,
) -> PattoResult<TaskEditResult> {
    let text = store::read_note(root.clone(), rel_path.clone())?;
    let ast = parser::parse_text(&text).ast;

    let snapshots = task_edits::collect_task_snapshots(&ast);
    let old = snapshots
        .get(&(row as usize))
        .cloned()
        .ok_or(PattoError::NoTaskAtRow {
            path: rel_path.clone(),
            row,
        })?;

    let core_status: CoreTaskStatus = status.into();
    let mut new = old.clone();
    new.status = core_status.clone();
    new.status_is_canonical = true;

    let mut new_map = HashMap::new();
    new_map.insert(row as usize, new.clone());
    let mut old_map = HashMap::new();
    old_map.insert(row as usize, old.clone());

    let transitions = task_edits::detect_task_transitions(&new_map, &old_map);

    let edits = if let Some(transition) = transitions.first() {
        // A timestamp is being recorded, so the token is rewritten in long form.
        task_edits::generate_edits_for_transition(transition, now)
    } else if old.is_shorthand && shorthand_symbol(&core_status).is_some() {
        // Status-only change on a shorthand token: swap the leading symbol and
        // keep the compact form the user wrote.
        let symbol = shorthand_symbol(&core_status).unwrap();
        vec![task_edits::TextEdit {
            row: row as usize,
            start_byte: old.prop_span.0,
            end_byte: old.prop_span.0 + 1,
            start_utf16: 0,
            end_utf16: 0,
            new_text: symbol.to_string(),
        }]
    } else {
        task_edits::rewrite_task_token(&new, &[("status", status_word(&core_status).to_string())])
    };

    if edits.is_empty() {
        return Err(PattoError::NoTaskAtRow {
            path: rel_path,
            row,
        });
    }

    let new_text = task_edits::apply_edits(&text, &edits);
    store::write_note(root.clone(), rel_path.clone(), new_text.clone())?;
    index::index_update_file(root, rel_path)?;

    let new_line = new_text
        .split('\n')
        .nth(row as usize)
        .unwrap_or_default()
        .to_string();

    Ok(TaskEditResult { row, new_line })
}

/// `paused` has no shorthand spelling, so pausing always converts to long form.
fn shorthand_symbol(status: &CoreTaskStatus) -> Option<char> {
    match status {
        CoreTaskStatus::Todo => Some('!'),
        CoreTaskStatus::Doing => Some('*'),
        CoreTaskStatus::Done => Some('-'),
        CoreTaskStatus::Paused => None,
    }
}

fn status_word(status: &CoreTaskStatus) -> &'static str {
    match status {
        CoreTaskStatus::Todo => "todo",
        CoreTaskStatus::Doing => "doing",
        CoreTaskStatus::Paused => "paused",
        CoreTaskStatus::Done => "done",
    }
}
