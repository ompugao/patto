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

    for content in parent.value().contents.lock().unwrap().iter() {
        gather_wikilinks(content, wikilinks);
    }

    for child in parent.value().children.lock().unwrap().iter() {
        gather_wikilinks(child, wikilinks);
    }
}

/// Recursively collect non-Done task lines from an AST node.
pub fn gather_tasks(parent: &AstNode, tasklines: &mut Vec<(AstNode, Deadline)>) {
    if let AstNodeKind::Line { ref properties } = &parent.kind() {
        for prop in properties {
            if let Property::Task { status, due, .. } = prop {
                if !matches!(status, TaskStatus::Done) {
                    tasklines.push((parent.clone(), due.clone()));
                    break;
                }
            }
        }
    }
    for child in parent.value().children.lock().unwrap().iter() {
        gather_tasks(child, tasklines);
    }
}

/// Recursively collect Done tasks that have a `completed_at` date.
pub fn gather_completed_tasks(parent: &AstNode, tasklines: &mut Vec<(AstNode, chrono::NaiveDate)>) {
    if let AstNodeKind::Line { ref properties } = &parent.kind() {
        for prop in properties {
            if let Property::Task {
                status,
                completed_at: Some(completed_at),
                ..
            } = prop
            {
                if matches!(status, TaskStatus::Done) {
                    let date = match completed_at {
                        Deadline::Date(d) => Some(*d),
                        Deadline::DateTime(dt) => Some(dt.date()),
                        Deadline::Uninterpretable(_) => None,
                    };
                    if let Some(date) = date {
                        tasklines.push((parent.clone(), date));
                        break;
                    }
                }
            }
        }
    }
    for child in parent.value().children.lock().unwrap().iter() {
        gather_completed_tasks(child, tasklines);
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
        .value()
        .children
        .lock()
        .unwrap()
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
        for child in node.value().children.lock().unwrap().iter() {
            inner(child, depth + 1, f);
        }
    }

    if matches!(parent.kind(), AstNodeKind::Dummy) {
        for child in parent.value().children.lock().unwrap().iter() {
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
    let label = if let AstNodeKind::Line { properties } = &line.kind() {
        let task_prop = properties
            .iter()
            .find(|prop| matches!(prop, Property::Task { .. }));

        if let Some(Property::Task { location, .. }) = task_prop {
            let raw = line.extract_str();
            let before = raw[..location.span.0.min(raw.len())].trim_end();
            let after = raw[location.span.1.min(raw.len())..].trim_start();
            match (before.is_empty(), after.is_empty()) {
                (true, true) => String::new(),
                (false, true) => before.trim_start().to_string(),
                (true, false) => after.trim_start().to_string(),
                (false, false) => format!("{} {}", before.trim_start(), after),
            }
        } else {
            line.extract_str().trim_start().to_string()
        }
    } else {
        line.extract_str().trim_start().to_string()
    };
    conceal_urls(&label)
}
