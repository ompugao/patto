/// Generic task-diff and edit-generation pipeline.
///
/// # Architecture
///
/// ```text
/// old AST  ──► collect_task_snapshots ──► HashMap<row, TaskSnapshot>
///                                                     │
/// new AST  ──► collect_task_snapshots ──► HashMap<row, TaskSnapshot>
///                                                     │
///                          detect_task_transitions ◄──┘
///                                     │
///                      Vec<TaskTransition>
///                                     │
///              generate_edits_for_transition  (per transition)
///                                     │
///                          Vec<TextEdit>  ──► workspace/applyEdit
/// ```
///
/// All edit-generation is span-based (using `Property::location.span` from the
/// parsed AST) and never scans raw text with `rfind` or `contains`.
use std::collections::HashMap;

use str_indices::utf16::from_byte_idx as utf16_from_byte_idx;
use tower_lsp::lsp_types::{Position, Range, TextEdit};

use crate::parser::{AstNode, AstNodeKind, Property, TaskStatus};
use crate::task::{Duration, TaskSnapshot, TaskTransition};

// ─── AST walk ────────────────────────────────────────────────────────────────

/// Walk every `(line_node, Property::Task)` pair in the AST tree, calling `f`
/// for each one found.  Children are visited recursively after the current node.
pub fn walk_task_lines(node: &AstNode, f: &mut impl FnMut(&AstNode, &Property)) {
    let props: Option<&Vec<Property>> = match node.kind() {
        AstNodeKind::Line { properties } => Some(properties),
        AstNodeKind::QuoteContent { properties } => Some(properties),
        _ => None,
    };

    if let Some(properties) = props {
        for prop in properties {
            if matches!(prop, Property::Task { .. }) {
                f(node, prop);
                break; // at most one Task property per line
            }
        }
    }

    for child in node.value().children.lock().unwrap().iter() {
        walk_task_lines(child, f);
    }
}

// ─── Snapshot collection ─────────────────────────────────────────────────────

/// Convert an entire AST into a row-keyed map of `TaskSnapshot`s.
///
/// This is the *only* place that pattern-matches on `Property::Task` fields —
/// all subsequent logic works on `TaskSnapshot` values.
pub fn collect_task_snapshots(root: &AstNode) -> HashMap<usize, TaskSnapshot> {
    let mut map = HashMap::new();
    walk_task_lines(root, &mut |node, prop| {
        if let Property::Task {
            status,
            status_is_canonical,
            due,
            scheduled,
            completed_at,
            started_at,
            time_spent,
            location,
        } = prop
        {
            let line_text = node.extract_str().to_string();
            // Determine whether the on-disk form is a shorthand token.
            // Shorthand tokens start with a single ASCII symbol (`-`/`*`/`!`)
            // followed immediately by a digit, not with `{@`.
            let prop_text = &line_text[location.span.0..location.span.1.min(line_text.len())];
            let is_shorthand = !prop_text.starts_with("{@");

            map.insert(
                node.location().row,
                TaskSnapshot {
                    row: node.location().row,
                    status: status.clone(),
                    status_is_canonical: *status_is_canonical,
                    due: due.clone(),
                    scheduled: scheduled.clone(),
                    completed_at: completed_at.clone(),
                    started_at: started_at.clone(),
                    time_spent: time_spent.clone(),
                    prop_span: location.span.clone(),
                    is_shorthand,
                    line_text,
                },
            );
        }
    });
    map
}

// ─── Transition detection ─────────────────────────────────────────────────────

/// Compare old and new snapshot maps and return every detected `TaskTransition`.
///
/// Rules:
/// - `* → Done` without a `completed_at` already set  → `BecameDone`
/// - `* → Doing` without a `started_at` already set   → `BecameDoing`
/// - `Doing → Todo`                                    → `BecameTodo`  (clock-out)
pub fn detect_task_transitions(
    new_snapshots: &HashMap<usize, TaskSnapshot>,
    old_snapshots: &HashMap<usize, TaskSnapshot>,
) -> Vec<TaskTransition> {
    let mut transitions = Vec::new();

    for (row, new) in new_snapshots {
        let old = match old_snapshots.get(row) {
            // Brand-new task line (no previous snapshot): skip auto-edits.
            None => continue,
            Some(o) => o,
        };

        // Skip transitions involving non-canonical status values (e.g. `status=doin`
        // during a mid-word edit). Both sides must be canonical to avoid spurious
        // clock-in/out events during keystroke-by-keystroke changes.
        if !new.status_is_canonical || !old.status_is_canonical {
            continue;
        }

        match (&new.status, &old.status) {
            // ── Any → Done ─────────────────────────────────────────────────
            (TaskStatus::Done, prev) if *prev != TaskStatus::Done => {
                // Only inject completed_at if it is not already present.
                if new.completed_at.is_none() {
                    transitions.push(TaskTransition::BecameDone {
                        old: old.clone(),
                        new: new.clone(),
                    });
                }
            }

            // ── Any → Doing ────────────────────────────────────────────────
            (TaskStatus::Doing, prev) if *prev != TaskStatus::Doing => {
                // Only inject started_at if it is not already present.
                if new.started_at.is_none() {
                    transitions.push(TaskTransition::BecameDoing {
                        old: old.clone(),
                        new: new.clone(),
                    });
                }
            }

            // ── Doing → Todo ───────────────────────────────────────────────
            (TaskStatus::Todo, TaskStatus::Doing) => {
                transitions.push(TaskTransition::BecameTodo {
                    old: old.clone(),
                    new: new.clone(),
                });
            }

            _ => {}
        }
    }

    transitions
}

// ─── Edit generation ─────────────────────────────────────────────────────────

/// Generate all `TextEdit`s required to record time-tracking data for one
/// `TaskTransition`.  The `now` timestamp is passed in so callers can use a
/// consistent timestamp for a batch of edits.
pub fn generate_edits_for_transition(
    transition: &TaskTransition,
    now: chrono::NaiveDateTime,
) -> Vec<TextEdit> {
    match transition {
        // ── BecameDone ────────────────────────────────────────────────────
        TaskTransition::BecameDone { old, new } => {
            // Fields to set: completed_at=<now>
            // Also flush started_at → time_spent if the task was clocked in.
            //
            // Prefer `new.started_at` over `old.started_at`: the editor will
            // have sent back a did_change that includes the injected started_at
            // field, so the new snapshot is more likely to have it.  Fall back
            // to old in case the applyEdit round-trip hasn't completed yet.
            let started_at = new.started_at.as_ref().or(old.started_at.as_ref());
            // Similarly, use the larger of new/old time_spent as the base so
            // we never lose accumulated time from a previous session.
            let base_time_spent = match (&new.time_spent, &old.time_spent) {
                (Some(n), Some(o)) => Some(if n.total_minutes() >= o.total_minutes() {
                    n.clone()
                } else {
                    o.clone()
                }),
                (Some(n), None) => Some(n.clone()),
                (None, Some(o)) => Some(o.clone()),
                (None, None) => None,
            };

            let mut fields: Vec<(&str, String)> = Vec::new();
            let elapsed = elapsed_since(&started_at.cloned(), now);

            if let Some(e) = elapsed {
                let total = base_time_spent.unwrap_or_default() + e;
                fields.push(("time_spent", total.to_string()));
                fields.push(("started_at", String::new())); // delete
            }

            fields.push(("completed_at", now.format("%Y-%m-%dT%H:%M").to_string()));

            build_edits(new, &fields)
        }

        // ── BecameDoing ───────────────────────────────────────────────────
        TaskTransition::BecameDoing { new, .. } => {
            let fields = vec![("started_at", now.format("%Y-%m-%dT%H:%M").to_string())];
            build_edits(new, &fields)
        }

        // ── BecameTodo (clock-out without Done) ───────────────────────────
        TaskTransition::BecameTodo { old, new } => {
            // Prefer started_at from the new snapshot (it may still be present
            // in the line text if the user only changed the status word).
            // Fall back to old snapshot in case the user also deleted it.
            let started_at = new.started_at.as_ref().or(old.started_at.as_ref());
            let base_time_spent = match (&new.time_spent, &old.time_spent) {
                (Some(n), Some(o)) => Some(if n.total_minutes() >= o.total_minutes() {
                    n.clone()
                } else {
                    o.clone()
                }),
                (Some(n), None) => Some(n.clone()),
                (None, Some(o)) => Some(o.clone()),
                (None, None) => None,
            };

            let elapsed = elapsed_since(&started_at.cloned(), now);
            if let Some(e) = elapsed {
                let total = base_time_spent.unwrap_or_default() + e;
                let fields = vec![
                    ("time_spent", total.to_string()),
                    ("started_at", String::new()), // delete
                ];
                build_edits(new, &fields)
            } else {
                // No started_at recorded anywhere — nothing to do
                vec![]
            }
        }
    }
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Compute elapsed time between `started_at` and `now`.
/// Returns `None` if `started_at` is absent or not a DateTime.
fn elapsed_since(
    started_at: &Option<crate::parser::Deadline>,
    now: chrono::NaiveDateTime,
) -> Option<Duration> {
    use crate::parser::Deadline;
    let start = match started_at.as_ref()? {
        Deadline::DateTime(dt) => *dt,
        Deadline::Date(d) => d.and_hms_opt(0, 0, 0)?,
        Deadline::Uninterpretable(_) => return None,
    };
    let secs = (now - start).num_seconds();
    if secs <= 0 {
        return None;
    }
    Some(Duration::from_minutes((secs / 60) as u32))
}

/// Build the actual `TextEdit` list that inserts / updates / deletes fields
/// inside (or replacing) the task property token.
///
/// `fields` is a list of `(key, value)` pairs.  An **empty value string**
/// means "delete this key" (i.e. remove the `key=value` pair from the block).
///
/// Two strategies:
/// - **Long-form** (`{@task …}`): each field is either inserted before `}` or
///   the existing `key=oldvalue` span is replaced in-place.
/// - **Shorthand** (`-YYYY-MM-DD`): the entire span is replaced with a full
///   `{@task …}` block that includes all existing fields plus the new ones.
fn build_edits(snapshot: &TaskSnapshot, fields: &[(&str, String)]) -> Vec<TextEdit> {
    if snapshot.is_shorthand {
        build_shorthand_replacement(snapshot, fields)
    } else {
        build_longform_edits(snapshot, fields)
    }
}

/// For a long-form `{@task …}` block, generate one `TextEdit` per field.
///
/// - If the field already exists, replace the value span.
/// - If the field is new (and value is non-empty), insert before the closing `}`.
/// - If the value is empty, delete the existing `key=value` (and any leading space).
fn build_longform_edits(snapshot: &TaskSnapshot, fields: &[(&str, String)]) -> Vec<TextEdit> {
    let line = snapshot.prop_span.0; // row (0-indexed)
                                     // We need the raw line text — extract from the snapshot's prop_span context.
                                     // The Location stores the full line input as `input`.
                                     // We reconstruct it from the AstNode indirectly via the span; however we
                                     // don't have the AstNode here.  Instead we locate the text via the
                                     // `prop_span` within the source string that we do not store in TaskSnapshot.
                                     //
                                     // To keep TaskSnapshot lean we store the raw line string in the snapshot
                                     // itself.  We add a `line_text` field to TaskSnapshot below, OR we accept
                                     // the text via a parameter.
                                     //
                                     // **Design decision**: accept `line_text` as an argument so we can stay pure.
                                     // This is called from `generate_edits_for_transition` which has no line text.
                                     //
                                     // ► We propagate line_text through TaskSnapshot instead.
                                     //   (See the `line_text` field added to TaskSnapshot in src/task.rs.)

    // For now, build a single replacement edit that rewrites the entire property
    // block.  This is simpler than per-field surgery and avoids offset
    // calculation complexity when multiple fields change simultaneously.
    build_longform_full_rewrite(snapshot, fields)
}

/// Rewrite the full `{@task …}` block in one edit, merging new field values.
fn build_longform_full_rewrite(
    snapshot: &TaskSnapshot,
    new_fields: &[(&str, String)],
) -> Vec<TextEdit> {
    use crate::parser::Deadline;

    // Start from the snapshot's current field values.
    let mut status = snapshot.status.clone();
    let mut due = snapshot.due.clone();
    let mut scheduled = snapshot.scheduled.clone();
    let mut completed_at = snapshot.completed_at.clone();
    let mut started_at = snapshot.started_at.clone();
    let mut time_spent = snapshot.time_spent.clone();

    // Apply overrides from `new_fields`.
    for (key, value) in new_fields {
        match *key {
            "status" => {
                status = match value.as_str() {
                    "todo" => TaskStatus::Todo,
                    "doing" => TaskStatus::Doing,
                    "done" => TaskStatus::Done,
                    _ => status,
                };
            }
            "completed_at" => {
                if value.is_empty() {
                    completed_at = None;
                } else {
                    completed_at = Some(crate::parser::parse_deadline_pub(value));
                }
            }
            "started_at" => {
                if value.is_empty() {
                    started_at = None;
                } else {
                    started_at = Some(crate::parser::parse_deadline_pub(value));
                }
            }
            "time_spent" => {
                if value.is_empty() {
                    time_spent = None;
                } else {
                    time_spent = value.parse().ok();
                }
            }
            "scheduled" => {
                if value.is_empty() {
                    scheduled = None;
                } else {
                    scheduled = Some(crate::parser::parse_deadline_pub(value));
                }
            }
            _ => {}
        }
    }

    let status_str = match status {
        TaskStatus::Todo => "todo",
        TaskStatus::Doing => "doing",
        TaskStatus::Done => "done",
    };

    let mut parts = vec![format!("status={}", status_str)];

    if !matches!(due, Deadline::Uninterpretable(ref s) if s.is_empty()) {
        parts.push(format!("due={}", due));
    }
    if let Some(s) = &scheduled {
        parts.push(format!("scheduled={}", s));
    }
    if let Some(c) = &completed_at {
        parts.push(format!("completed_at={}", c));
    }
    if let Some(sa) = &started_at {
        parts.push(format!("started_at={}", sa));
    }
    if let Some(ts) = &time_spent {
        parts.push(format!("time_spent={}", ts));
    }

    let new_text = format!("{{@task {}}}", parts.join(" "));

    let line_idx = snapshot.row as u32;
    let line_text = &snapshot.line_text;
    vec![TextEdit {
        range: Range {
            start: Position {
                line: line_idx,
                character: utf16_from_byte_idx(line_text, snapshot.prop_span.0) as u32,
            },
            end: Position {
                line: line_idx,
                character: utf16_from_byte_idx(line_text, snapshot.prop_span.1) as u32,
            },
        },
        new_text,
    }]
}

/// Replace the shorthand token with a full `{@task …}` block.
fn build_shorthand_replacement(
    snapshot: &TaskSnapshot,
    new_fields: &[(&str, String)],
) -> Vec<TextEdit> {
    // Delegate to the same full-rewrite logic — shorthand has no existing long
    // form, so rewriting the span (shorthand token) with `{@task …}` is correct.
    build_longform_full_rewrite(snapshot, new_fields)
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Deadline;
    use crate::parser::TaskStatus;
    use crate::task::{Duration, TaskSnapshot, TaskTransition};

    fn make_snapshot(row: usize, status: TaskStatus, started_at: Option<&str>) -> TaskSnapshot {
        TaskSnapshot {
            row,
            status,
            status_is_canonical: true,
            due: Deadline::Uninterpretable("".to_string()),
            scheduled: None,
            completed_at: None,
            started_at: started_at.map(|s| crate::parser::parse_deadline_pub(s)),
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
}
