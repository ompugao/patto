use std::collections::HashMap;

use chrono::NaiveDateTime;
use patto::parser::{self, TaskStatus};
use patto::task::{TaskSnapshot, TaskTransition};
use patto::task_edits::{
    apply_edits, collect_task_snapshots, detect_task_transitions, generate_edits_for_transition,
    rewrite_task_token, TextEdit,
};

fn snapshots(text: &str) -> HashMap<usize, TaskSnapshot> {
    let result = parser::parse_text(text);
    assert!(
        result.parse_errors.is_empty(),
        "unexpected parse errors: {:?}",
        result.parse_errors
    );
    collect_task_snapshots(&result.ast)
}

fn only_snapshot(text: &str) -> TaskSnapshot {
    let mut map = snapshots(text);
    assert_eq!(map.len(), 1, "expected one task in {text:?}");
    map.remove(&0).expect("task on the first row")
}

fn at(time: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(time, "%Y-%m-%dT%H:%M").unwrap()
}

fn transitions(old: &str, new: &str) -> Vec<TaskTransition> {
    detect_task_transitions(&snapshots(new), &snapshots(old))
}

fn rewritten(text: &str, edits: &[TextEdit]) -> String {
    apply_edits(text, edits)
}

#[test]
fn apply_edits_replaces_byte_ranges_right_to_left_within_a_row() {
    let edit = |start, end, new_text: &str| TextEdit {
        row: 1,
        start_byte: start,
        end_byte: end,
        start_utf16: start,
        end_utf16: end,
        new_text: new_text.to_string(),
    };
    let text = "first\nabcdef\nlast";
    let edits = [edit(0, 2, "XYZ"), edit(4, 6, "Q")];
    assert_eq!(apply_edits(text, &edits), "first\nXYZcdQ\nlast");
}

#[test]
fn apply_edits_ignores_edits_that_fall_outside_the_line() {
    let edits = [TextEdit {
        row: 0,
        start_byte: 2,
        end_byte: 10,
        start_utf16: 2,
        end_utf16: 10,
        new_text: "gone".to_string(),
    }];
    assert_eq!(apply_edits("short", &edits), "short");
}

#[test]
fn rewrite_task_token_turns_shorthand_into_the_long_form() {
    let text = "buy milk -2026-01-31";
    let snapshot = only_snapshot(text);
    let edits = rewrite_task_token(&snapshot, &[("status", "todo".to_string())]);
    assert_eq!(
        rewritten(text, &edits),
        "buy milk {@task status=todo due=2026-01-31}"
    );
}

#[test]
fn rewrite_task_token_keeps_fields_that_were_not_overridden() {
    let text = "{@task status=doing due=2026-01-31 scheduled=2026-01-20 started_at=2026-01-30T09:00 time_spent=1h}";
    let snapshot = only_snapshot(text);
    let edits = rewrite_task_token(&snapshot, &[("status", "paused".to_string())]);
    assert_eq!(
        rewritten(text, &edits),
        "{@task status=paused due=2026-01-31 scheduled=2026-01-20 started_at=2026-01-30T09:00 time_spent=1h}"
    );
}

#[test]
fn rewrite_task_token_removes_a_field_set_to_the_empty_string() {
    let text = "{@task status=doing started_at=2026-01-30T09:00}";
    let snapshot = only_snapshot(text);
    let edits = rewrite_task_token(&snapshot, &[("started_at", String::new())]);
    assert_eq!(rewritten(text, &edits), "{@task status=doing}");
}

#[test]
fn rewrite_task_token_reports_utf16_columns_for_wide_characters() {
    let text = "日本語 {@task status=todo}";
    let snapshot = only_snapshot(text);
    let edits = rewrite_task_token(&snapshot, &[("status", "done".to_string())]);
    let [edit] = edits.as_slice() else {
        panic!("expected one edit, got {edits:?}");
    };
    assert_eq!((edit.start_byte, edit.end_byte), (10, text.len()));
    assert_eq!((edit.start_utf16, edit.end_utf16), (4, 23));
}

#[test]
fn becoming_done_records_completed_at_and_flushes_the_running_clock() {
    let old = "{@task status=doing started_at=2026-01-30T09:00 time_spent=1h}";
    let new = "{@task status=done started_at=2026-01-30T09:00 time_spent=1h}";
    let found = transitions(old, new);
    let [transition @ TaskTransition::BecameDone { .. }] = found.as_slice() else {
        panic!("expected BecameDone, got {found:?}");
    };
    let edits = generate_edits_for_transition(transition, at("2026-01-30T10:30"));
    assert_eq!(
        rewritten(new, &edits),
        "{@task status=done completed_at=2026-01-30T10:30 time_spent=2h30m}"
    );
}

#[test]
fn becoming_doing_records_started_at() {
    let old = "task {@task status=todo due=2026-02-01}";
    let new = "task {@task status=doing due=2026-02-01}";
    let found = transitions(old, new);
    let [transition @ TaskTransition::BecameDoing { .. }] = found.as_slice() else {
        panic!("expected BecameDoing, got {found:?}");
    };
    let edits = generate_edits_for_transition(transition, at("2026-01-30T09:00"));
    assert_eq!(
        rewritten(new, &edits),
        "task {@task status=doing due=2026-02-01 started_at=2026-01-30T09:00}"
    );
}

#[test]
fn pausing_moves_the_elapsed_time_into_time_spent() {
    let old = "{@task status=doing started_at=2026-01-30T09:00}";
    let new = "{@task status=paused started_at=2026-01-30T09:00}";
    let found = transitions(old, new);
    let [transition @ TaskTransition::BecamePaused { .. }] = found.as_slice() else {
        panic!("expected BecamePaused, got {found:?}");
    };
    let edits = generate_edits_for_transition(transition, at("2026-01-30T09:45"));
    assert_eq!(
        rewritten(new, &edits),
        "{@task status=paused time_spent=45m}"
    );
}

#[test]
fn clocking_out_without_a_start_time_produces_no_edits() {
    let found = transitions("{@task status=doing}", "{@task status=todo}");
    let [transition @ TaskTransition::BecameTodo { .. }] = found.as_slice() else {
        panic!("expected BecameTodo, got {found:?}");
    };
    assert!(generate_edits_for_transition(transition, at("2026-01-30T09:45")).is_empty());
}

#[test]
fn a_brand_new_task_line_is_not_a_transition() {
    assert!(transitions("plain line", "plain line {@task status=done}").is_empty());
}

#[test]
fn a_non_canonical_status_is_not_a_transition() {
    assert!(transitions("{@task status=todo}", "{@task status=doin}").is_empty());
    assert!(transitions("{@task status=doin}", "{@task status=done}").is_empty());
}

#[test]
fn done_with_completed_at_already_present_is_not_a_transition() {
    assert!(transitions(
        "{@task status=todo}",
        "{@task status=done completed_at=2026-01-30T10:30}"
    )
    .is_empty());
}

#[test]
fn transition_accessors_point_at_the_new_and_old_rows() {
    let found = transitions("a\n{@task status=todo}", "a\n{@task status=done}");
    let [transition] = found.as_slice() else {
        panic!("expected one transition, got {found:?}");
    };
    assert_eq!(transition.row(), 1);
    assert_eq!(transition.new_snapshot().status, TaskStatus::Done);
    assert_eq!(transition.old_snapshot().status, TaskStatus::Todo);
}
