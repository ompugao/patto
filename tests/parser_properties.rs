use chrono::{NaiveDate, NaiveDateTime};
use patto::parser::{self, AstNode, AstNodeKind, Deadline, Property, TaskStatus};
use patto::task::{Duration, TaskSnapshot};
use patto::task_edits;

fn parse(text: &str) -> AstNode {
    let result = parser::parse_text(text);
    assert!(
        result.parse_errors.is_empty(),
        "unexpected parse errors: {:?}",
        result.parse_errors
    );
    result.ast
}

fn first_line(text: &str) -> AstNode {
    parse(text).children()[0].clone()
}

fn parse_task(text: &str) -> TaskSnapshot {
    let snapshots = task_edits::collect_task_snapshots(&parse(text));
    assert_eq!(snapshots.len(), 1, "expected exactly one task in {text:?}");
    snapshots.into_values().next().unwrap()
}

fn anchor_names(line: &AstNode) -> Vec<String> {
    let AstNodeKind::Line { properties } = line.kind() else {
        panic!("not a line: {:?}", line.kind());
    };
    properties
        .iter()
        .filter_map(|prop| match prop {
            Property::Anchor { name, .. } => Some(name.clone()),
            Property::Task { .. } => None,
        })
        .collect()
}

fn date(s: &str) -> Deadline {
    Deadline::Date(NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap())
}

fn datetime(s: &str) -> Deadline {
    Deadline::DateTime(NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap())
}

#[test]
fn task_property_fields_are_parsed() {
    let task = parse_task(
        "write report {@task status=done due=2026-01-31 scheduled=2026-01-20 completed_at=2026-01-30T18:00 started_at=2026-01-30T16:30 time_spent=1h30m}",
    );
    assert_eq!(task.status, TaskStatus::Done);
    assert!(task.status_is_canonical);
    assert_eq!(task.due, date("2026-01-31"));
    assert_eq!(task.scheduled, Some(date("2026-01-20")));
    assert_eq!(task.completed_at, Some(datetime("2026-01-30T18:00")));
    assert_eq!(task.started_at, Some(datetime("2026-01-30T16:30")));
    assert_eq!(task.time_spent, Some(Duration::new(1, 30)));
    assert!(!task.is_shorthand);
}

#[test]
fn task_without_due_has_an_empty_uninterpretable_deadline() {
    let task = parse_task("{@task status=todo}");
    assert_eq!(task.due, Deadline::Uninterpretable(String::new()));
    assert_eq!(task.scheduled, None);
    assert_eq!(task.completed_at, None);
}

#[test]
fn task_status_aliases_map_to_doing() {
    for alias in ["doing", "inprogress", "wip"] {
        let task = parse_task(&format!("{{@task status={alias}}}"));
        assert_eq!(task.status, TaskStatus::Doing, "alias {alias}");
        assert!(task.status_is_canonical, "alias {alias}");
    }
}

#[test]
fn paused_status_is_canonical() {
    let task = parse_task("{@task status=paused}");
    assert_eq!(task.status, TaskStatus::Paused);
    assert!(task.status_is_canonical);
}

#[test]
fn unknown_task_status_falls_back_to_todo_and_is_not_canonical() {
    let task = parse_task("{@task status=doin}");
    assert_eq!(task.status, TaskStatus::Todo);
    assert!(!task.status_is_canonical);
}

#[test]
fn task_without_status_is_todo_and_not_canonical() {
    let task = parse_task("{@task due=2026-01-31}");
    assert_eq!(task.status, TaskStatus::Todo);
    assert!(!task.status_is_canonical);
}

#[test]
fn unknown_task_keys_are_ignored() {
    let task = parse_task("{@task status=done priority=high}");
    assert_eq!(task.status, TaskStatus::Done);
}

#[test]
fn unparsable_time_spent_is_dropped() {
    let task = parse_task("{@task status=todo time_spent=soon}");
    assert_eq!(task.time_spent, None);
}

#[test]
fn unparsable_deadline_is_kept_as_text() {
    let task = parse_task("{@task status=todo due=next-week}");
    assert_eq!(task.due, Deadline::Uninterpretable("next-week".to_string()));
}

#[test]
fn shorthand_task_symbols_map_to_a_status() {
    for (text, status) in [
        ("!2026-01-31", TaskStatus::Todo),
        ("*2026-01-31", TaskStatus::Doing),
        ("-2026-01-31", TaskStatus::Done),
    ] {
        let task = parse_task(&format!("buy milk {text}"));
        assert_eq!(task.status, status, "{text}");
        assert!(task.status_is_canonical, "{text}");
        assert!(task.is_shorthand, "{text}");
        assert_eq!(task.due, date("2026-01-31"), "{text}");
    }
}

#[test]
fn shorthand_task_accepts_a_time_of_day() {
    let task = parse_task("-2026-01-31T20:00");
    assert_eq!(task.due, datetime("2026-01-31T20:00"));
}

#[test]
fn task_property_span_covers_the_whole_token() {
    let line = "text {@task status=todo}";
    let task = parse_task(line);
    assert_eq!(
        &line[task.prop_span.0..task.prop_span.1],
        "{@task status=todo}"
    );
}

#[test]
fn short_and_long_anchor_forms_both_name_the_line() {
    let line = first_line("heading #short {@anchor long}");
    assert_eq!(anchor_names(&line), ["short", "long"]);
}

#[test]
fn long_anchor_without_a_name_is_dropped() {
    let line = first_line("heading {@anchor}");
    assert!(anchor_names(&line).is_empty());
}

#[test]
fn unknown_property_names_are_dropped() {
    let line = first_line("heading {@color red}");
    let AstNodeKind::Line { properties } = line.kind() else {
        panic!("not a line");
    };
    assert!(properties.is_empty());
}
