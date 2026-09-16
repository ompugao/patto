//! LSP adapter over the pure [`crate::task_edits`] pipeline.
//!
//! The detection and edit-generation logic is editor-agnostic; this module only
//! converts the resulting byte/UTF-16 spans into `lsp_types::TextEdit`.

use tower_lsp::lsp_types::{Position, Range, TextEdit};

use crate::task::TaskTransition;

pub use crate::task_edits::{
    apply_edits, collect_task_snapshots, detect_task_transitions, rewrite_task_token,
    walk_task_lines,
};

fn to_lsp_edit(edit: crate::task_edits::TextEdit) -> TextEdit {
    let line = edit.row as u32;
    TextEdit {
        range: Range {
            start: Position {
                line,
                character: edit.start_utf16 as u32,
            },
            end: Position {
                line,
                character: edit.end_utf16 as u32,
            },
        },
        new_text: edit.new_text,
    }
}

/// Same as [`crate::task_edits::generate_edits_for_transition`], with the edits
/// expressed in LSP coordinates.
pub fn generate_edits_for_transition(
    transition: &TaskTransition,
    now: chrono::NaiveDateTime,
) -> Vec<TextEdit> {
    crate::task_edits::generate_edits_for_transition(transition, now)
        .into_iter()
        .map(to_lsp_edit)
        .collect()
}
