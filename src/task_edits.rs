//! Task transition detection and the edits that record them.
//!
//! The old and the new AST are reduced to row-keyed `TaskSnapshot`s, the
//! snapshots are diffed into `TaskTransition`s, and each transition becomes a
//! span-based `TextEdit` over the task token. Raw line text is never scanned.
use std::collections::HashMap;
use std::fmt;

use chrono::NaiveDateTime;
use str_indices::utf16::from_byte_idx as utf16_from_byte_idx;

use crate::parser::{parse_deadline, AstNode, Deadline, Property, Span, TaskStatus};
use crate::task::{Duration, TaskSnapshot, TaskTransition};

/// An editor-agnostic replacement of a byte range inside a single line.
///
/// Byte offsets address `line_text`; the UTF-16 columns are precomputed for LSP
/// clients, which address characters in UTF-16 code units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub row: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_utf16: usize,
    pub end_utf16: usize,
    pub new_text: String,
}

impl TextEdit {
    fn replacing(row: usize, line_text: &str, span: &Span, new_text: String) -> Self {
        Self {
            row,
            start_byte: span.0,
            end_byte: span.1,
            start_utf16: utf16_from_byte_idx(line_text, span.0),
            end_utf16: utf16_from_byte_idx(line_text, span.1),
            new_text,
        }
    }
}

pub fn apply_edits(text: &str, edits: &[TextEdit]) -> String {
    let mut lines: Vec<String> = text.split('\n').map(|l| l.to_string()).collect();

    let mut by_row: HashMap<usize, Vec<&TextEdit>> = HashMap::new();
    for edit in edits {
        by_row.entry(edit.row).or_default().push(edit);
    }

    for (row, mut row_edits) in by_row {
        let Some(line) = lines.get_mut(row) else {
            continue;
        };
        // Right to left, so earlier spans keep their offsets.
        row_edits.sort_by_key(|edit| std::cmp::Reverse(edit.start_byte));
        for edit in row_edits {
            if edit.start_byte > edit.end_byte || edit.end_byte > line.len() {
                continue;
            }
            line.replace_range(edit.start_byte..edit.end_byte, &edit.new_text);
        }
    }

    lines.join("\n")
}

/// Visit every node that carries a `Property::Task`, depth first.
pub fn walk_task_lines(node: &AstNode, f: &mut impl FnMut(&AstNode, &Property)) {
    let task = node
        .properties()
        .iter()
        .find(|prop| matches!(prop, Property::Task { .. }));
    if let Some(task) = task {
        f(node, task);
    }
    for child in node.children().iter() {
        walk_task_lines(child, f);
    }
}

pub fn collect_task_snapshots(root: &AstNode) -> HashMap<usize, TaskSnapshot> {
    let mut map = HashMap::new();
    walk_task_lines(root, &mut |node, prop| {
        let Property::Task {
            status,
            status_is_canonical,
            due,
            scheduled,
            completed_at,
            started_at,
            time_spent,
            location,
        } = prop
        else {
            return;
        };
        let line_text = node.extract_str().to_string();
        let token = &line_text[location.span.0..location.span.1.min(line_text.len())];
        let row = node.location().row;
        map.insert(
            row,
            TaskSnapshot {
                row,
                status: status.clone(),
                status_is_canonical: *status_is_canonical,
                due: due.clone(),
                scheduled: scheduled.clone(),
                completed_at: completed_at.clone(),
                started_at: started_at.clone(),
                time_spent: time_spent.clone(),
                prop_span: location.span.clone(),
                is_shorthand: !token.starts_with("{@"),
                line_text,
            },
        );
    });
    map
}

/// One transition per row at most; brand-new task lines never transition.
pub fn detect_task_transitions(
    new_snapshots: &HashMap<usize, TaskSnapshot>,
    old_snapshots: &HashMap<usize, TaskSnapshot>,
) -> Vec<TaskTransition> {
    new_snapshots
        .iter()
        .filter_map(|(row, new)| {
            let old = old_snapshots.get(row)?;
            // A half-typed status word such as `doin` must not clock the task
            // in or out, so both sides have to be canonical.
            if !new.status_is_canonical || !old.status_is_canonical || new.status == old.status {
                return None;
            }
            transition_between(old, new)
        })
        .collect()
}

fn transition_between(old: &TaskSnapshot, new: &TaskSnapshot) -> Option<TaskTransition> {
    let transition = match (&new.status, &old.status) {
        (TaskStatus::Done, _) if new.completed_at.is_none() => TaskTransition::BecameDone {
            old: old.clone(),
            new: new.clone(),
        },
        (TaskStatus::Doing, _) if new.started_at.is_none() => TaskTransition::BecameDoing {
            old: old.clone(),
            new: new.clone(),
        },
        (TaskStatus::Paused, TaskStatus::Doing) => TaskTransition::BecamePaused {
            old: old.clone(),
            new: new.clone(),
        },
        (TaskStatus::Todo, TaskStatus::Doing) => TaskTransition::BecameTodo {
            old: old.clone(),
            new: new.clone(),
        },
        _ => return None,
    };
    Some(transition)
}

/// `now` is passed in so a batch of edits shares one timestamp.
pub fn generate_edits_for_transition(
    transition: &TaskTransition,
    now: NaiveDateTime,
) -> Vec<TextEdit> {
    let stamp = now.format("%Y-%m-%dT%H:%M").to_string();
    match transition {
        TaskTransition::BecameDone { old, new } => {
            let mut fields = clock_out_fields(old, new, now);
            fields.push(("completed_at", stamp));
            rewrite_task_token(new, &fields)
        }
        TaskTransition::BecameDoing { new, .. } => {
            rewrite_task_token(new, &[("started_at", stamp)])
        }
        TaskTransition::BecameTodo { old, new } | TaskTransition::BecamePaused { old, new } => {
            let fields = clock_out_fields(old, new, now);
            if fields.is_empty() {
                vec![]
            } else {
                rewrite_task_token(new, &fields)
            }
        }
    }
}

/// Folds the running clock into `time_spent`. The new snapshot is preferred for
/// `started_at` and the larger `time_spent` wins, because the editor may or may
/// not have echoed back the previously injected fields yet.
fn clock_out_fields(
    old: &TaskSnapshot,
    new: &TaskSnapshot,
    now: NaiveDateTime,
) -> Vec<(&'static str, String)> {
    let started_at = new.started_at.as_ref().or(old.started_at.as_ref());
    let Some(elapsed) = elapsed_since(started_at, now) else {
        return vec![];
    };
    let carried = [&new.time_spent, &old.time_spent]
        .into_iter()
        .flatten()
        .max_by_key(|spent| spent.total_minutes())
        .cloned()
        .unwrap_or_default();
    vec![
        ("time_spent", (carried + elapsed).to_string()),
        ("started_at", String::new()),
    ]
}

fn elapsed_since(started_at: Option<&Deadline>, now: NaiveDateTime) -> Option<Duration> {
    let Deadline::DateTime(start) = started_at? else {
        return None;
    };
    let seconds = (now - *start).num_seconds();
    if seconds <= 0 {
        return None;
    }
    Some(Duration::from_minutes((seconds / 60) as u32))
}

/// Rewrite the task token with `(key, value)` overrides; an empty value removes
/// the key. The whole token is replaced, so a shorthand `-YYYY-MM-DD` comes
/// back in the long `{@task …}` form.
pub fn rewrite_task_token(snapshot: &TaskSnapshot, fields: &[(&str, String)]) -> Vec<TextEdit> {
    let mut token = TaskToken::from_snapshot(snapshot);
    for (key, value) in fields {
        token.set(key, value);
    }
    vec![TextEdit::replacing(
        snapshot.row,
        &snapshot.line_text,
        &snapshot.prop_span,
        token.to_string(),
    )]
}

struct TaskToken {
    status: TaskStatus,
    due: Deadline,
    scheduled: Option<Deadline>,
    completed_at: Option<Deadline>,
    started_at: Option<Deadline>,
    time_spent: Option<Duration>,
}

impl TaskToken {
    fn from_snapshot(snapshot: &TaskSnapshot) -> Self {
        Self {
            status: snapshot.status.clone(),
            due: snapshot.due.clone(),
            scheduled: snapshot.scheduled.clone(),
            completed_at: snapshot.completed_at.clone(),
            started_at: snapshot.started_at.clone(),
            time_spent: snapshot.time_spent.clone(),
        }
    }

    fn set(&mut self, key: &str, value: &str) {
        match key {
            "status" => {
                if let Some(status) = TaskStatus::from_keyword(value) {
                    self.status = status;
                }
            }
            "scheduled" => self.scheduled = optional_deadline(value),
            "completed_at" => self.completed_at = optional_deadline(value),
            "started_at" => self.started_at = optional_deadline(value),
            "time_spent" => self.time_spent = value.parse().ok(),
            _ => {}
        }
    }
}

fn optional_deadline(value: &str) -> Option<Deadline> {
    (!value.is_empty()).then(|| parse_deadline(value))
}

impl fmt::Display for TaskToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{@task status={}", self.status.keyword())?;
        if !matches!(&self.due, Deadline::Uninterpretable(text) if text.is_empty()) {
            write!(f, " due={}", self.due)?;
        }
        let deadlines = [
            ("scheduled", &self.scheduled),
            ("completed_at", &self.completed_at),
            ("started_at", &self.started_at),
        ];
        for (key, deadline) in deadlines {
            if let Some(deadline) = deadline {
                write!(f, " {key}={deadline}")?;
            }
        }
        if let Some(time_spent) = &self.time_spent {
            write!(f, " time_spent={time_spent}")?;
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Deadline;
    use crate::parser::TaskStatus;
    use crate::task::{TaskSnapshot, TaskTransition};

    fn make_snapshot(row: usize, status: TaskStatus, started_at: Option<&str>) -> TaskSnapshot {
        TaskSnapshot {
            row,
            status,
            status_is_canonical: true,
            due: Deadline::Uninterpretable("".to_string()),
            scheduled: None,
            completed_at: None,
            started_at: started_at.map(crate::parser::parse_deadline_pub),
            time_spent: None,
            prop_span: crate::parser::Span(0, 10),
            is_shorthand: false,
            line_text: "{@task status=todo due=}".to_string(),
        }
    }

    #[test]
    fn detect_todo_to_done() {
        let old = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Todo, None));
            m
        };
        let new = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Done, None));
            m
        };
        let transitions = detect_task_transitions(&new, &old);
        assert_eq!(transitions.len(), 1);
        assert!(matches!(transitions[0], TaskTransition::BecameDone { .. }));
    }

    #[test]
    fn detect_todo_to_doing() {
        let old = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Todo, None));
            m
        };
        let new = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Doing, None));
            m
        };
        let transitions = detect_task_transitions(&new, &old);
        assert_eq!(transitions.len(), 1);
        assert!(matches!(transitions[0], TaskTransition::BecameDoing { .. }));
    }

    #[test]
    fn detect_doing_to_todo() {
        let old = {
            let mut m = HashMap::new();
            m.insert(
                0,
                make_snapshot(0, TaskStatus::Doing, Some("2026-05-19T09:00")),
            );
            m
        };
        let new = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Todo, None));
            m
        };
        let transitions = detect_task_transitions(&new, &old);
        assert_eq!(transitions.len(), 1);
        assert!(matches!(transitions[0], TaskTransition::BecameTodo { .. }));
    }

    #[test]
    fn no_transition_when_done_already_has_completed_at() {
        let old = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Todo, None));
            m
        };
        let new = {
            let mut m = HashMap::new();
            let mut snap = make_snapshot(0, TaskStatus::Done, None);
            snap.completed_at = Some(Deadline::Date(
                chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            ));
            m.insert(0, snap);
            m
        };
        let transitions = detect_task_transitions(&new, &old);
        assert_eq!(transitions.len(), 0);
    }

    #[test]
    fn detect_doing_to_paused() {
        let old = {
            let mut m = HashMap::new();
            m.insert(
                0,
                make_snapshot(0, TaskStatus::Doing, Some("2026-05-19T09:00")),
            );
            m
        };
        let new = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Paused, None));
            m
        };
        let transitions = detect_task_transitions(&new, &old);
        assert_eq!(transitions.len(), 1);
        assert!(matches!(
            transitions[0],
            TaskTransition::BecamePaused { .. }
        ));
    }

    #[test]
    fn detect_paused_to_doing() {
        let old = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Paused, None));
            m
        };
        let new = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Doing, None));
            m
        };
        let transitions = detect_task_transitions(&new, &old);
        assert_eq!(transitions.len(), 1);
        assert!(matches!(transitions[0], TaskTransition::BecameDoing { .. }));
    }

    #[test]
    fn no_transition_paused_to_todo() {
        // Paused → Todo: time already accumulated; nothing to do.
        let old = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Paused, None));
            m
        };
        let new = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Todo, None));
            m
        };
        let transitions = detect_task_transitions(&new, &old);
        assert_eq!(transitions.len(), 0);
    }

    #[test]
    fn no_transition_todo_to_paused() {
        // Todo → Paused without ever clocking in: no started_at to flush.
        let old = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Todo, None));
            m
        };
        let new = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Paused, None));
            m
        };
        let transitions = detect_task_transitions(&new, &old);
        // No BecamePaused because old was not Doing.
        assert_eq!(transitions.len(), 0);
    }

    #[test]
    fn doing_to_paused_accumulates_time() {
        let now =
            chrono::NaiveDateTime::parse_from_str("2026-05-19T10:30", "%Y-%m-%dT%H:%M").unwrap();
        let old = {
            let mut m = HashMap::new();
            m.insert(
                0,
                make_snapshot(0, TaskStatus::Doing, Some("2026-05-19T09:00")),
            );
            m
        };
        let new = {
            let mut m = HashMap::new();
            m.insert(0, make_snapshot(0, TaskStatus::Paused, None));
            m
        };
        let transitions = detect_task_transitions(&new, &old);
        assert_eq!(transitions.len(), 1);
        let edits = generate_edits_for_transition(&transitions[0], now);
        // Should produce time_spent and clear started_at.
        assert!(!edits.is_empty());
        let combined = edits
            .iter()
            .map(|e| e.new_text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            combined.contains("time_spent=1h30m"),
            "expected time_spent=1h30m in: {combined}"
        );
        assert!(
            !combined.contains("started_at"),
            "started_at should be removed in: {combined}"
        );
    }
}
