mod common;

use common::*;
use patto::lsp::task_edits::{collect_task_snapshots, detect_task_transitions, generate_edits_for_transition};
use patto::parser::{parse_text, TaskStatus};
use patto::task::TaskTransition;
// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A fixed "now" used in all tests so assertions on generated text are stable.
fn fixed_now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 5, 19)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

/// Parse two text snapshots and run the full pipeline, returning all generated
/// TextEdits.
fn edits_for(old_text: &str, new_text: &str) -> Vec<tower_lsp::lsp_types::TextEdit> {
    let old_ast = parse_text(old_text).ast;
    let new_ast = parse_text(new_text).ast;
    let old_snaps = collect_task_snapshots(&old_ast);
    let new_snaps = collect_task_snapshots(&new_ast);
    let transitions = detect_task_transitions(&new_snaps, &old_snaps);
    transitions
        .iter()
        .flat_map(|t| generate_edits_for_transition(t, fixed_now()))
        .collect()
}

/// Parse two text snapshots and return the detected transitions (without
/// generating edits) — useful for testing detection logic independently.
fn transitions_for(old_text: &str, new_text: &str) -> Vec<TaskTransition> {
    let old_ast = parse_text(old_text).ast;
    let new_ast = parse_text(new_text).ast;
    let old_snaps = collect_task_snapshots(&old_ast);
    let new_snaps = collect_task_snapshots(&new_ast);
    detect_task_transitions(&new_snaps, &old_snaps)
}

// ─── Detection tests ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_detect_todo_to_done() {
    let _workspace = TestWorkspace::new();
    let ts = transitions_for(
        "buy milk {@task status=todo due=2026-06-01}\n",
        "buy milk {@task status=done due=2026-06-01}\n",
    );
    assert_eq!(ts.len(), 1);
    assert!(matches!(ts[0], TaskTransition::BecameDone { .. }));
}

#[tokio::test]
async fn test_detect_todo_to_doing() {
    let _workspace = TestWorkspace::new();
    let ts = transitions_for(
        "buy milk {@task status=todo due=2026-06-01}\n",
        "buy milk {@task status=doing due=2026-06-01}\n",
    );
    assert_eq!(ts.len(), 1);
    assert!(matches!(ts[0], TaskTransition::BecameDoing { .. }));
}

#[tokio::test]
async fn test_detect_doing_to_todo() {
    let _workspace = TestWorkspace::new();
    let ts = transitions_for(
        "buy milk {@task status=doing due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=todo due=2026-06-01}\n",
    );
    assert_eq!(ts.len(), 1);
    assert!(matches!(ts[0], TaskTransition::BecameTodo { .. }));
}

#[tokio::test]
async fn test_detect_doing_to_done() {
    let _workspace = TestWorkspace::new();
    let ts = transitions_for(
        "buy milk {@task status=doing due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=done due=2026-06-01}\n",
    );
    assert_eq!(ts.len(), 1);
    assert!(matches!(ts[0], TaskTransition::BecameDone { .. }));
}

#[tokio::test]
async fn test_no_transition_for_already_done_with_completed_at() {
    let _workspace = TestWorkspace::new();
    // Task already has completed_at — should not produce a second edit.
    let ts = transitions_for(
        "buy milk {@task status=todo due=2026-06-01}\n",
        "buy milk {@task status=done due=2026-06-01 completed_at=2026-05-18T10:00}\n",
    );
    assert_eq!(ts.len(), 0, "No transition expected when completed_at already set");
}

#[tokio::test]
async fn test_no_transition_for_already_doing_with_started_at() {
    let _workspace = TestWorkspace::new();
    // Task already has started_at — should not produce a second clock-in edit.
    let ts = transitions_for(
        "buy milk {@task status=todo due=2026-06-01}\n",
        "buy milk {@task status=doing due=2026-06-01 started_at=2026-05-19T08:00}\n",
    );
    assert_eq!(ts.len(), 0, "No transition expected when started_at already set");
}

#[tokio::test]
async fn test_no_transition_for_brand_new_task() {
    let _workspace = TestWorkspace::new();
    // A task that appears in the new snapshot but not in the old one is skipped.
    let ts = transitions_for(
        "some line without a task\n",
        "buy milk {@task status=done due=2026-06-01}\n",
    );
    assert_eq!(ts.len(), 0, "Brand-new task lines should not trigger auto-edits");
}

// ─── Edit-generation tests ────────────────────────────────────────────────────

#[tokio::test]
async fn test_clock_in_inserts_started_at() {
    let _workspace = TestWorkspace::new();
    let edits = edits_for(
        "buy milk {@task status=todo due=2026-06-01}\n",
        "buy milk {@task status=doing due=2026-06-01}\n",
    );
    assert_eq!(edits.len(), 1, "Expected one edit for todo→doing");
    let edit = &edits[0];
    assert_eq!(edit.range.start.line, 0);
    assert!(
        edit.new_text.contains("started_at="),
        "Edit should contain started_at, got: {}",
        edit.new_text
    );
    assert!(
        edit.new_text.contains("2026-05-19T10:00"),
        "started_at should equal fixed_now, got: {}",
        edit.new_text
    );
}

#[tokio::test]
async fn test_clock_out_to_todo_updates_time_spent() {
    let _workspace = TestWorkspace::new();
    // started_at is 1 hour before fixed_now (09:00 → 10:00 = 60 min).
    // Old snapshot has started_at; new snapshot (user only changed status word) also retains it.
    let edits = edits_for(
        "buy milk {@task status=doing due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=todo due=2026-06-01}\n",
    );
    assert_eq!(edits.len(), 1, "Expected one edit for doing→todo");
    let edit = &edits[0];
    assert!(
        edit.new_text.contains("time_spent=1h"),
        "Edit should accumulate 1 hour of time_spent, got: {}",
        edit.new_text
    );
    assert!(
        !edit.new_text.contains("started_at="),
        "started_at should be removed on clock-out, got: {}",
        edit.new_text
    );
}

/// Simulates the real-world nvim case: the user only changed the status word
/// from `doing` to `todo`, leaving `started_at` intact in the line.
/// The server must still compute elapsed time from the in-line `started_at`.
#[tokio::test]
async fn test_clock_out_to_todo_started_at_retained_in_line() {
    let _workspace = TestWorkspace::new();
    // Both old and new snapshots have started_at (user only changed the status word).
    let edits = edits_for(
        "buy milk {@task status=doing due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=todo due=2026-06-01 started_at=2026-05-19T09:00}\n",
    );
    assert_eq!(edits.len(), 1, "Expected one edit for doing→todo");
    let edit = &edits[0];
    assert!(
        edit.new_text.contains("time_spent=1h"),
        "Edit should accumulate 1 hour of time_spent, got: {}",
        edit.new_text
    );
    assert!(
        !edit.new_text.contains("started_at="),
        "started_at should be removed on clock-out, got: {}",
        edit.new_text
    );
}

#[tokio::test]
async fn test_clock_out_to_done_updates_time_spent_and_completed_at() {
    let _workspace = TestWorkspace::new();
    // started_at=09:30, fixed_now=10:00 → 30 min elapsed.
    let edits = edits_for(
        "buy milk {@task status=doing due=2026-06-01 started_at=2026-05-19T09:30}\n",
        "buy milk {@task status=done due=2026-06-01}\n",
    );
    assert_eq!(edits.len(), 1, "Expected one edit for doing→done");
    let edit = &edits[0];
    assert!(
        edit.new_text.contains("completed_at="),
        "Edit should set completed_at, got: {}",
        edit.new_text
    );
    assert!(
        edit.new_text.contains("time_spent=30m"),
        "Edit should set time_spent=30m, got: {}",
        edit.new_text
    );
    assert!(
        !edit.new_text.contains("started_at="),
        "started_at should be removed, got: {}",
        edit.new_text
    );
}

#[tokio::test]
async fn test_accumulate_time_spent_across_sessions() {
    let _workspace = TestWorkspace::new();
    // First session already accumulated 1h, current session is 30 min → total 1h30m.
    let edits = edits_for(
        "buy milk {@task status=doing due=2026-06-01 started_at=2026-05-19T09:30 time_spent=1h}\n",
        "buy milk {@task status=todo due=2026-06-01 time_spent=1h}\n",
    );
    assert_eq!(edits.len(), 1);
    let edit = &edits[0];
    assert!(
        edit.new_text.contains("time_spent=1h30m"),
        "Edit should accumulate to 1h30m, got: {}",
        edit.new_text
    );
}

#[tokio::test]
async fn test_todo_to_done_without_doing_no_time_spent() {
    let _workspace = TestWorkspace::new();
    // Direct todo→done without a doing phase: no time_spent should be added.
    let edits = edits_for(
        "buy milk {@task status=todo due=2026-06-01}\n",
        "buy milk {@task status=done due=2026-06-01}\n",
    );
    assert_eq!(edits.len(), 1, "Expected one edit for todo→done");
    let edit = &edits[0];
    assert!(
        edit.new_text.contains("completed_at="),
        "Edit should set completed_at, got: {}",
        edit.new_text
    );
    assert!(
        !edit.new_text.contains("time_spent="),
        "No time_spent should be added without a doing phase, got: {}",
        edit.new_text
    );
}

#[tokio::test]
async fn test_shorthand_done_replaced_with_longform() {
    let _workspace = TestWorkspace::new();
    // Shorthand `-YYYY-MM-DD` (done) transitioning from `!` (todo).
    let edits = edits_for(
        "buy milk !2026-06-01\n",
        "buy milk -2026-06-01\n",
    );
    assert_eq!(edits.len(), 1, "Expected one edit for shorthand todo→done");
    let edit = &edits[0];
    assert_eq!(edit.range.start.line, 0);
    assert!(
        edit.new_text.contains("{@task"),
        "Shorthand should be replaced with longform block, got: {}",
        edit.new_text
    );
    assert!(
        edit.new_text.contains("status=done"),
        "Replacement block should carry status=done, got: {}",
        edit.new_text
    );
    assert!(
        edit.new_text.contains("completed_at="),
        "Replacement block should contain completed_at, got: {}",
        edit.new_text
    );
}

/// Simulate the keystroke-by-keystroke `doing` → `todo` transition as nvim
/// sends it: each character change fires a separate `did_change`.
///
/// Intermediate states where `status=` has no value produce no parseable task.
/// The sticky snapshot map must bridge across those gaps so the final
/// `doing → todo` transition still has access to the `Doing` state.
///
/// We test this by running the pipeline directly, step by step, maintaining
/// our own sticky map — mirroring exactly what `on_change` does internally.
#[test]
fn test_doing_to_todo_via_keystroke_simulation() {
    use patto::task::Duration;
    use std::collections::HashMap;

    let now = fixed_now();

    // Simulate the sequence of texts nvim would send
    let keystrokes: &[&str] = &[
        // Initial state (from did_open)
        "buy milk {@task status=doing due=2026-06-01 started_at=2026-05-19T09:00}\n",
        // User starts deleting "doing"
        "buy milk {@task status=doin due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=doi due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=do due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=d due=2026-06-01 started_at=2026-05-19T09:00}\n",
        // status= with no value — task fails to parse entirely
        "buy milk {@task status= due=2026-06-01 started_at=2026-05-19T09:00}\n",
        // User types "todo"
        "buy milk {@task status=t due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=to due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=tod due=2026-06-01 started_at=2026-05-19T09:00}\n",
        "buy milk {@task status=todo due=2026-06-01 started_at=2026-05-19T09:00}\n",
    ];

    // Sticky map starts empty (no prior state)
    let mut sticky: HashMap<usize, patto::task::TaskSnapshot> = HashMap::new();

    let mut became_todo_fired = false;
    let mut clock_out_edits: Vec<tower_lsp::lsp_types::TextEdit> = vec![];

    for text in keystrokes {
        let ast = parse_text(text);
        let new_snaps = collect_task_snapshots(&ast.ast);

        // Detect transitions using the current sticky map as old state
        let transitions = detect_task_transitions(&new_snaps, &sticky);
        let edits: Vec<_> = transitions
            .iter()
            .flat_map(|t| generate_edits_for_transition(t, now))
            .collect();

        if transitions.iter().any(|t| matches!(t, TaskTransition::BecameTodo { .. })) {
            became_todo_fired = true;
            clock_out_edits = edits;
        }

        // Update sticky: only canonical states overwrite
        for (row, snap) in &new_snaps {
            if snap.status_is_canonical {
                sticky.insert(*row, snap.clone());
            }
        }
    }

    assert!(
        became_todo_fired,
        "BecameTodo should have fired during the keystroke sequence"
    );
    assert_eq!(clock_out_edits.len(), 1, "Expected one clock-out edit");
    let edit = &clock_out_edits[0];
    assert!(
        edit.new_text.contains("time_spent=1h"),
        "Clock-out edit should accumulate 1h of time_spent, got: {}",
        edit.new_text
    );
    assert!(
        !edit.new_text.contains("started_at="),
        "Clock-out edit should remove started_at, got: {}",
        edit.new_text
    );
}
