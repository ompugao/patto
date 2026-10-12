//! `textDocument/foldingRange`: one fold per node that spans more than one row.

use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};

use crate::parser::{AstNode, AstNodeKind};

pub(super) fn collect_folding_ranges(root: &AstNode) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    for child in root.children().iter() {
        collect_node_folds(child, &mut ranges);
    }
    ranges
}

/// Inner folds are pushed before the fold of their container.
fn collect_node_folds(node: &AstNode, ranges: &mut Vec<FoldingRange>) {
    for child in node.children().iter() {
        collect_node_folds(child, ranges);
    }
    for content in node.contents().iter() {
        collect_node_folds(content, ranges);
    }

    let Some(kind) = fold_kind(node.kind()) else {
        return;
    };
    let start_row = node.location().row;
    let end_row = last_row_of(node);
    if end_row > start_row {
        ranges.push(FoldingRange {
            start_line: start_row as u32,
            start_character: None,
            end_line: end_row as u32,
            end_character: None,
            kind,
            collapsed_text: None,
        });
    }
}

/// `None` when the node is not a fold container; `Some(None)` for a plain fold.
fn fold_kind(kind: &AstNodeKind) -> Option<Option<FoldingRangeKind>> {
    match kind {
        AstNodeKind::Line { .. } | AstNodeKind::QuoteContent { .. } => Some(None),
        AstNodeKind::Code { inline: false, .. } | AstNodeKind::Math { inline: false } => {
            Some(Some(FoldingRangeKind::Region))
        }
        AstNodeKind::Quote | AstNodeKind::Table { .. } => Some(Some(FoldingRangeKind::Region)),
        _ => None,
    }
}

/// Block command nodes (`Code`, `Math`, `Quote`, `Table`) sit in the *contents*
/// of their `Line` while their body lines are *children*, so both are scanned.
fn last_row_of(node: &AstNode) -> usize {
    node.children()
        .iter()
        .chain(node.contents().iter())
        .map(last_row_of)
        .fold(node.location().row, usize::max)
}
