//! The `patto-query` command line, as editor and terminal plugins drive it.
#![cfg(all(feature = "cli", feature = "repository"))]

use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn query(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_patto-query"))
        .args(args)
        .output()
        .expect("failed to run patto-query");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

fn notes(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).unwrap();
    }
    dir
}

fn arg(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn tasks_are_grouped_by_deadline_relative_to_today() {
    let dir = notes(&[(
        "a.pn",
        "late {@task status=todo due=2026-11-10}\n\
         now {@task status=doing due=2026-11-12}\n\
         someday {@task status=todo}\n",
    )]);

    let out = query(&["tasks", "--root", arg(dir.path()), "--today", "2026-11-12"]);

    let groups: Vec<(&str, &str, u64)> = out["open"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["text"].as_str().unwrap(),
                t["group"].as_str().unwrap(),
                t["line"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        groups,
        [
            ("late", "Overdue", 1),
            ("now", "Today", 2),
            ("someday", "No Deadline", 3)
        ]
    );
    assert_eq!(out["done"], Value::Array(vec![]));
}

#[test]
fn done_tasks_are_listed_only_when_a_timeframe_is_given() {
    let dir = notes(&[(
        "a.pn",
        "shipped {@task status=done completed_at=2026-11-11T10:00}\n",
    )]);

    let out = query(&[
        "tasks",
        "--root",
        arg(dir.path()),
        "--today",
        "2026-11-12",
        "--done",
        "this_week",
    ]);

    assert_eq!(out["done"][0]["text"], "shipped");
    assert_eq!(out["done"][0]["group"], "Yesterday");
}

#[test]
fn links_report_back_links_with_their_line_and_two_hop_notes() {
    let dir = notes(&[
        ("a.pn", "see [b]\n"),
        ("b.pn", "body\nback to [a]\n"),
        ("c.pn", "also [b]\n"),
    ]);

    let out = query(&[
        "links",
        "--root",
        arg(dir.path()),
        arg(&dir.path().join("a.pn")),
    ]);

    assert_eq!(out["backlinks"][0]["name"], "b");
    assert_eq!(out["backlinks"][0]["lines"][0]["line"], 2);
    assert_eq!(out["backlinks"][0]["lines"][0]["context"], "back to [a]");
    assert_eq!(out["two_hop"][0]["bridge"]["name"], "b");
    assert_eq!(out["two_hop"][0]["notes"][0]["name"], "c");
}

#[test]
fn capture_appends_a_task_line_at_the_end_of_a_note() {
    let dir = tempfile::tempdir().unwrap();
    let inbox = dir.path().join("inbox.pn");
    std::fs::write(&inbox, "existing").unwrap();

    let out = query(&[
        "capture",
        "--file",
        arg(&inbox),
        "--task",
        "--due",
        "2026-11-14",
        "call Bob",
    ]);

    assert_eq!(out["line"], 2);
    assert_eq!(
        std::fs::read_to_string(&inbox).unwrap(),
        "existing\ncall Bob {@task status=todo due=2026-11-14}\n"
    );
}

#[test]
fn set_status_marks_a_task_done_with_a_completion_stamp() {
    let dir = notes(&[("a.pn", "note\nwork {@task status=todo}\n")]);
    let file = dir.path().join("a.pn");

    let out = query(&["set-status", "--file", arg(&file), "--line", "2", "done"]);

    let line = out["text"].as_str().unwrap();
    assert!(
        line.starts_with("work {@task status=done completed_at="),
        "{line}"
    );
    assert!(std::fs::read_to_string(&file).unwrap().contains(line));
}
