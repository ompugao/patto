use chrono::NaiveDateTime;

use super::Workspace;
use crate::api::error::PattoError;
use crate::api::index::index_build;
use crate::api::tasks::*;
use crate::api::types::TaskStatus;

fn build(ws: &Workspace) {
    index_build(ws.root(), |_| {}).unwrap();
}

fn now() -> NaiveDateTime {
    NaiveDateTime::parse_from_str("2026-09-16T14:30", "%Y-%m-%dT%H:%M").unwrap()
}

fn set_status(ws: &Workspace, rel_path: &str, row: u32, status: TaskStatus) -> String {
    set_task_status_at(ws.root(), rel_path.to_string(), row, status, now()).unwrap();
    ws.read(rel_path)
}

#[test]
fn pending_tasks_are_grouped_by_deadline() {
    let ws = Workspace::new();
    ws.write(
        "a.pn",
        "late {@task status=todo due=2026-09-10}\n\
         now {@task status=todo due=2026-09-16}\n\
         soon {@task status=doing due=2026-09-18}\n\
         far {@task status=todo due=2027-01-01}\n\
         someday {@task status=todo}\n\
         finished {@task status=done completed_at=2026-09-16}\n",
    );
    build(&ws);

    let tasks = pending_tasks(ws.root(), Some("2026-09-16".to_string())).unwrap();
    let groups: Vec<(&str, PendingGroup)> =
        tasks.iter().map(|t| (t.text.as_str(), t.group)).collect();

    assert_eq!(
        groups,
        vec![
            ("late", PendingGroup::Overdue),
            ("now", PendingGroup::Today),
            ("soon", PendingGroup::ThisWeek),
            ("far", PendingGroup::Later),
            ("someday", PendingGroup::NoDue),
        ]
    );
}

#[test]
fn task_labels_drop_the_property_token() {
    let ws = Workspace::new();
    ws.write("a.pn", "buy milk {@task status=todo due=2026-09-20}\n");
    build(&ws);

    let tasks = pending_tasks(ws.root(), Some("2026-09-16".to_string())).unwrap();
    assert_eq!(tasks[0].text, "buy milk");
    assert_eq!(tasks[0].note_name, "a");
    assert_eq!(tasks[0].row, 0);
}

#[test]
fn completed_tasks_respect_the_timeframe() {
    let ws = Workspace::new();
    ws.write(
        "a.pn",
        "today {@task status=done completed_at=2026-09-16}\n\
         yesterday {@task status=done completed_at=2026-09-15}\n\
         last month {@task status=done completed_at=2026-08-01}\n",
    );
    build(&ws);

    let today = Some("2026-09-16".to_string());
    let names = |timeframe: &str| -> Vec<String> {
        completed_tasks(ws.root(), timeframe.to_string(), None, None, today.clone())
            .unwrap()
            .into_iter()
            .map(|t| t.text)
            .collect()
    };

    assert_eq!(names("today"), vec!["today".to_string()]);
    assert_eq!(names("yesterday"), vec!["yesterday".to_string()]);
    assert_eq!(
        names("this_week"),
        vec!["today".to_string(), "yesterday".to_string()]
    );
    assert_eq!(names("this_month").len(), 2);
}

#[test]
fn a_custom_timeframe_uses_the_given_bounds() {
    let ws = Workspace::new();
    ws.write(
        "a.pn",
        "old {@task status=done completed_at=2026-01-15}\n\
         new {@task status=done completed_at=2026-09-16}\n",
    );
    build(&ws);

    let found = completed_tasks(
        ws.root(),
        "custom".to_string(),
        Some("2026-01-01".to_string()),
        Some("2026-01-31".to_string()),
        Some("2026-09-16".to_string()),
    )
    .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].text, "old");
    assert_eq!(found[0].completed_on.as_deref(), Some("2026-01-15"));
}

#[test]
fn marking_a_task_done_records_when() {
    let ws = Workspace::new();
    ws.write("a.pn", "ship it {@task status=todo due=2026-09-20}\n");
    build(&ws);

    let after = set_status(&ws, "a.pn", 0, TaskStatus::Done);
    assert!(after.contains("status=done"), "{after}");
    assert!(after.contains("completed_at=2026-09-16T14:30"), "{after}");
    assert!(after.contains("due=2026-09-20"), "{after}");
}

#[test]
fn a_shorthand_task_becomes_long_form_once_a_timestamp_is_recorded() {
    let ws = Workspace::new();
    ws.write("a.pn", "ship it !2026-09-20\n");
    build(&ws);

    let after = set_status(&ws, "a.pn", 0, TaskStatus::Done);
    assert!(after.starts_with("ship it {@task "), "{after}");
    assert!(after.contains("status=done"), "{after}");
    assert!(after.contains("due=2026-09-20"), "{after}");
    assert!(after.contains("completed_at="), "{after}");
}

#[test]
fn starting_a_task_clocks_in() {
    let ws = Workspace::new();
    ws.write("a.pn", "work {@task status=todo}\n");
    build(&ws);

    let after = set_status(&ws, "a.pn", 0, TaskStatus::Doing);
    assert!(after.contains("status=doing"), "{after}");
    assert!(after.contains("started_at=2026-09-16T14:30"), "{after}");
}

#[test]
fn stopping_a_task_flushes_the_elapsed_time() {
    let ws = Workspace::new();
    ws.write(
        "a.pn",
        "work {@task status=doing started_at=2026-09-16T13:00}\n",
    );
    build(&ws);

    let after = set_status(&ws, "a.pn", 0, TaskStatus::Todo);
    assert!(after.contains("status=todo"), "{after}");
    assert!(after.contains("time_spent=1h30m"), "{after}");
    assert!(!after.contains("started_at="), "{after}");
}

#[test]
fn pausing_a_running_task_also_flushes_the_elapsed_time() {
    let ws = Workspace::new();
    ws.write(
        "a.pn",
        "work {@task status=doing started_at=2026-09-16T14:00 time_spent=30m}\n",
    );
    build(&ws);

    let after = set_status(&ws, "a.pn", 0, TaskStatus::Paused);
    assert!(after.contains("status=paused"), "{after}");
    assert!(after.contains("time_spent=1h"), "{after}");
}

#[test]
fn a_shorthand_status_change_without_a_timestamp_stays_shorthand() {
    let ws = Workspace::new();
    ws.write("a.pn", "ship it !2026-09-20\n");
    build(&ws);

    // Todo -> Doing records started_at, so go the other way: done -> todo.
    ws.write("a.pn", "ship it -2026-09-20\n");
    let after = set_status(&ws, "a.pn", 0, TaskStatus::Todo);
    assert_eq!(after.trim_end(), "ship it !2026-09-20");
}

#[test]
fn reopening_a_done_task_keeps_the_completion_date() {
    let ws = Workspace::new();
    ws.write(
        "a.pn",
        "ship it {@task status=done completed_at=2026-09-10}\n",
    );
    build(&ws);

    let after = set_status(&ws, "a.pn", 0, TaskStatus::Todo);
    assert!(after.contains("status=todo"), "{after}");
    assert!(after.contains("completed_at=2026-09-10"), "{after}");
}

#[test]
fn setting_a_status_updates_the_task_lists() {
    let ws = Workspace::new();
    ws.write("a.pn", "ship it {@task status=todo due=2026-09-20}\n");
    build(&ws);
    assert_eq!(
        pending_tasks(ws.root(), Some("2026-09-16".to_string()))
            .unwrap()
            .len(),
        1
    );

    set_status(&ws, "a.pn", 0, TaskStatus::Done);

    assert!(pending_tasks(ws.root(), Some("2026-09-16".to_string()))
        .unwrap()
        .is_empty());
    let done = completed_tasks(
        ws.root(),
        "today".to_string(),
        None,
        None,
        Some("2026-09-16".to_string()),
    )
    .unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].text, "ship it");
}

#[test]
fn a_row_without_a_task_is_reported() {
    let ws = Workspace::new();
    ws.write("a.pn", "just a line\n");
    build(&ws);

    assert!(matches!(
        set_task_status_at(ws.root(), "a.pn".to_string(), 0, TaskStatus::Done, now()),
        Err(PattoError::NoTaskAtRow { .. })
    ));
}

#[test]
fn other_lines_are_left_untouched() {
    let ws = Workspace::new();
    ws.write(
        "a.pn",
        "first line\nship it {@task status=todo}\nlast line\n",
    );
    build(&ws);

    let after = set_status(&ws, "a.pn", 1, TaskStatus::Done);
    let lines: Vec<&str> = after.split('\n').collect();
    assert_eq!(lines[0], "first line");
    assert_eq!(lines[2], "last line");
}
