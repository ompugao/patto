//! Filtering and ordering of the workspace task query.
#![cfg(feature = "repository")]

use chrono::NaiveDate;
use patto::ast_query::task_label;
use patto::repository::{DateRange, Repository, TaskQuery};

fn date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn labels(note: &str, query: TaskQuery) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let path = root.join("tasks.pn");
    std::fs::write(&path, note).unwrap();
    let repository = Repository::new(root);
    repository.add_file_to_graph(&path, note);
    repository
        .tasks(&query)
        .iter()
        .map(|(_, task)| task_label(&task.line))
        .collect()
}

const NOTE: &str = "\
later {@task status=todo due=2026-06-10}
soon {@task status=doing due=2026-06-01}
paused {@task status=paused}
done early {@task status=done completed_at=2026-05-02}
done late {@task status=done completed_at=2026-05-20T10:00}
done undated {@task status=done}
dropped {@task status=archived archived_at=2026-05-10}
";

fn in_may_first_half() -> DateRange {
    DateRange {
        from: Some(date("2026-05-01")),
        to: Some(date("2026-05-15")),
    }
}

#[test]
fn the_open_query_returns_open_tasks_by_deadline() {
    assert_eq!(labels(NOTE, TaskQuery::open()), ["soon", "later", "paused"]);
}

#[test]
fn done_tasks_match_by_completion_date_within_the_range() {
    let query = TaskQuery {
        done: Some(in_may_first_half()),
        ..TaskQuery::default()
    };
    assert_eq!(labels(NOTE, query), ["done early"]);
}

#[test]
fn an_open_range_matches_every_dated_done_task() {
    let query = TaskQuery {
        done: Some(DateRange::default()),
        ..TaskQuery::default()
    };
    assert_eq!(labels(NOTE, query), ["done early", "done late"]);
}

#[test]
fn archived_tasks_appear_only_when_asked_for() {
    let query = TaskQuery {
        done: Some(in_may_first_half()),
        archived: Some(in_may_first_half()),
        ..TaskQuery::default()
    };
    assert_eq!(labels(NOTE, query), ["done early", "dropped"]);
}

#[test]
fn open_tasks_come_before_resolved_ones() {
    let query = TaskQuery {
        open: true,
        done: Some(DateRange::default()),
        archived: Some(DateRange::default()),
    };
    assert_eq!(
        labels(NOTE, query),
        [
            "soon",
            "later",
            "paused",
            "done early",
            "dropped",
            "done late"
        ]
    );
}
