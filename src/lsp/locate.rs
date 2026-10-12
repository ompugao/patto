//! Finding AST nodes under a cursor, and converting their byte spans into the
//! UTF-16 ranges the LSP wire format uses.

use ropey::Rope;
use str_indices::utf16::{from_byte_idx as utf16_from_byte_idx, to_byte_idx as utf16_to_byte_idx};
use tower_lsp::lsp_types::{Position, Range};

use crate::parser::{self, AstNode, AstNodeKind, Property};

pub(super) fn line_str(rope: &Rope, row: u32) -> Option<&str> {
    rope.get_line(row as usize)?.as_str()
}

/// Byte offset into `line` of a cursor given in UTF-16 code units.
pub(super) fn cursor_byte(line: &str, position: Position) -> usize {
    utf16_to_byte_idx(line, position.character as usize)
}

/// Single-line range for a byte span of `line`.
pub(super) fn utf16_range(line: &str, row: u32, span: (usize, usize)) -> Range {
    Range::new(
        Position::new(row, utf16_from_byte_idx(line, span.0) as u32),
        Position::new(row, utf16_from_byte_idx(line, span.1) as u32),
    )
}

pub(super) fn node_range(node: &AstNode) -> Range {
    let location = node.location();
    utf16_range(
        node.extract_str(),
        location.row as u32,
        (location.span.0, location.span.1),
    )
}

pub(super) fn gather_anchors(parent: &AstNode, anchors: &mut Vec<(String, usize)>) {
    if let AstNodeKind::Line { properties } = parent.kind() {
        for prop in properties {
            if let Property::Anchor { name, location } = prop {
                anchors.push((name.to_string(), location.row));
            }
        }
    }
    for child in parent.children().iter() {
        gather_anchors(child, anchors);
    }
}

/// The anchor defined at `(row, col)`, as `(name, location)`.
pub(super) fn find_anchor_at_position(
    parent: &AstNode,
    row: usize,
    col: usize,
) -> Option<(String, parser::Location)> {
    if let AstNodeKind::Line { properties } = parent.kind() {
        if parent.location().row == row {
            for prop in properties {
                if let Property::Anchor { name, location } = prop {
                    if location.span.contains(col) {
                        return Some((name.clone(), location.clone()));
                    }
                }
            }
        }
    }
    for child in parent.children().iter() {
        if let Some(found) = find_anchor_at_position(child, row, col) {
            return Some(found);
        }
    }
    None
}

/// Nodes from the innermost node at `(row, col)` up to `parent`, innermost first.
pub(super) fn locate_node_route(parent: &AstNode, row: usize, col: usize) -> Option<Vec<AstNode>> {
    let parent_row = parent.location().row;
    if matches!(parent.kind(), AstNodeKind::Dummy) || parent_row < row {
        for child in parent.children().iter() {
            if let Some(mut route) = locate_node_route(child, row, col) {
                route.push(parent.clone());
                return Some(route);
            }
        }
    } else if parent_row == row {
        if parent.contents().is_empty() {
            return Some(vec![parent.clone()]);
        }
        for content in parent.contents().iter() {
            if !content.location().span.contains(col) {
                continue;
            }
            if let Some(mut route) = locate_node_route(content, row, col) {
                route.push(parent.clone());
                return Some(route);
            }
        }
    }
    None
}

/// The wiki link under the cursor, as `(link, anchor)`.
pub(super) fn wiki_link_at(
    ast: &AstNode,
    row: usize,
    col: usize,
) -> Option<(String, Option<String>)> {
    locate_node_route(ast, row, col)?
        .iter()
        .find_map(|node| match node.kind() {
            AstNodeKind::WikiLink { link, anchor } => Some((link.clone(), anchor.clone())),
            _ => None,
        })
}
