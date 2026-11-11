//! Pure queries over a parsed document.
//!
//! These helpers used to live in [`crate::repository`] and [`crate::lsp`], which
//! are gated behind the `repository` / `lsp` features. They carry no OS or
//! protocol dependency, so every client (LSP, TUI, preview, mobile) shares them.

use crate::parser::{AstNode, AstNodeKind, Deadline, Location, Property, TaskStatus};

/// Recursively collect wikilinks with their source locations.
pub fn gather_wikilinks(parent: &AstNode, wikilinks: &mut Vec<(String, Option<String>, Location)>) {
    if let AstNodeKind::WikiLink { link, anchor } = &parent.kind() {
        wikilinks.push((link.clone(), anchor.clone(), parent.location().clone()));
    }

    for content in parent.contents().iter() {
        gather_wikilinks(content, wikilinks);
    }

    for child in parent.children().iter() {
        gather_wikilinks(child, wikilinks);
    }
}

/// A task line together with the task fields that queries filter on.
#[derive(Debug, Clone)]
pub struct TaskLine {
    pub line: AstNode,
    pub status: TaskStatus,
    pub due: Deadline,
    /// Date of `completed_at` for a Done task, of `archived_at` for an Archived
    /// one; `None` when that stamp is missing or unreadable.
    pub resolved_on: Option<chrono::NaiveDate>,
}

/// Recursively collect every task line under `parent`.
pub fn gather_task_lines(parent: &AstNode, tasks: &mut Vec<TaskLine>) {
    if let AstNodeKind::Line { ref properties } = &parent.kind() {
        let task = properties.iter().find_map(|prop| match prop {
            Property::Task {
                status,
                due,
                completed_at,
                archived_at,
                ..
            } => {
                let stamp = match status {
                    TaskStatus::Done => completed_at.as_ref(),
                    TaskStatus::Archived => archived_at.as_ref(),
                    _ => None,
                };
                Some(TaskLine {
                    line: parent.clone(),
                    status: status.clone(),
                    due: due.clone(),
                    resolved_on: stamp.and_then(deadline_date),
                })
            }
            _ => None,
        });
        tasks.extend(task);
    }
    for child in parent.children().iter() {
        gather_task_lines(child, tasks);
    }
}

fn deadline_date(deadline: &Deadline) -> Option<chrono::NaiveDate> {
    match deadline {
        Deadline::Date(d) => Some(*d),
        Deadline::DateTime(dt) => Some(dt.date()),
        Deadline::Uninterpretable(_) => None,
    }
}

/// Find the line node carrying the given anchor name.
pub fn find_anchor(parent: &AstNode, anchor: &str) -> Option<AstNode> {
    if let AstNodeKind::Line { ref properties } = &parent.kind() {
        for prop in properties {
            if let Property::Anchor { name, .. } = prop {
                if name == anchor {
                    return Some(parent.clone());
                }
            }
        }
    }

    parent
        .children()
        .iter()
        .find_map(|child| find_anchor(child, anchor))
}

/// Visit every `Line` node depth-first, passing its nesting depth (root children
/// are depth 0).
pub fn walk_lines(parent: &AstNode, f: &mut impl FnMut(&AstNode, usize)) {
    fn inner(node: &AstNode, depth: usize, f: &mut impl FnMut(&AstNode, usize)) {
        if matches!(node.kind(), AstNodeKind::Line { .. }) {
            f(node, depth);
        }
        for child in node.children().iter() {
            inner(child, depth + 1, f);
        }
    }

    if matches!(parent.kind(), AstNodeKind::Dummy) {
        for child in parent.children().iter() {
            inner(child, 0, f);
        }
    } else {
        inner(parent, 0, f);
    }
}

/// Replace `[url title]` / `[title url]` with a link glyph so task labels stay short.
pub fn conceal_urls(text: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;

    static URL_TITLE_RE: OnceLock<Regex> = OnceLock::new();
    static TITLE_URL_RE: OnceLock<Regex> = OnceLock::new();

    let url_title_re =
        URL_TITLE_RE.get_or_init(|| Regex::new(r"\[\w+://[^\]\s]+\s+([^\]]+)\]").unwrap());
    let title_url_re =
        TITLE_URL_RE.get_or_init(|| Regex::new(r"\[([^\]]+?)\s+\w+://[^\]\s]+\]").unwrap());

    let text = url_title_re.replace_all(text, "[🔗$1]");
    title_url_re.replace_all(&text, "[$1🔗]").into_owned()
}

/// Return the line text with the task property token stripped and whitespace
/// trimmed, e.g. `buy milk {@task status=todo due=2026-06-01}` → `buy milk`.
pub fn task_label(line: &AstNode) -> String {
    let raw = line.extract_str();
    let task_location = line.properties().iter().find_map(|prop| match prop {
        Property::Task { location, .. } => Some(location),
        Property::Anchor { .. } => None,
    });
    let label = match task_location {
        Some(location) => {
            let before = raw[..location.span.0.min(raw.len())].trim();
            let after = raw[location.span.1.min(raw.len())..].trim_start();
            [before, after]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        }
        None => raw.trim_start().to_string(),
    };
    conceal_urls(&label)
}
