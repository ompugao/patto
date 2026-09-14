//! `textDocument/rename` for notes and anchors.
//!
//! Both cases rewrite the same thing — every wiki link pointing at the renamed
//! note — so they share `retarget_link_edits` and differ only in which links
//! they touch and what they write.

use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::{
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    Position, Range, RenameFile, RenameFileOptions, ResourceOp, TextDocumentEdit, TextEdit, Url,
    WorkspaceEdit,
};

use str_indices::utf16::{from_byte_idx as utf16_from_byte_idx, to_byte_idx as utf16_to_byte_idx};

use crate::lsp::backend::{find_anchor_at_position, locate_node_route, Backend};
use crate::parser::AstNodeKind;
use crate::repository::{LinkLocation, Repository};

impl Backend {
    pub(super) fn rename_workspace_edit(
        &self,
        uri: &Url,
        position: Position,
        new_name: &str,
    ) -> Result<Option<WorkspaceEdit>> {
        if new_name.is_empty() {
            return Err(invalid_params("Name cannot be empty"));
        }

        if let Some(edit) = self.rename_anchor(uri, position, new_name) {
            return Ok(Some(edit));
        }

        if new_name.contains('/') || new_name.contains('\\') {
            return Err(invalid_params("Note name cannot contain path separators"));
        }
        if new_name.ends_with(".pn") {
            return Err(invalid_params("Note name should not include .pn extension"));
        }

        self.rename_note(uri, position, new_name)
            .map(Some)
            .ok_or_else(|| Error {
                code: ErrorCode::InternalError,
                message: "Failed to prepare rename operation".into(),
                data: None,
            })
    }

    /// Rename the anchor under the cursor, and every `[note#anchor]` that points
    /// at it. `None` when the cursor is not on an anchor definition.
    fn rename_anchor(
        &self,
        uri: &Url,
        position: Position,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        let repo_lock = self.repository.lock().unwrap();
        let repo = repo_lock.as_ref()?;
        let ast = repo.ast_map.get(uri)?;
        let rope = repo.document_map.get(uri)?;

        let line = rope.value().get_line(position.line as usize)?;
        let line_str = line.as_str()?;
        let position_byte = utf16_to_byte_idx(line_str, position.character as usize);

        let (old_name, anchor_loc) =
            find_anchor_at_position(&ast, position.line as usize, position_byte)?;

        // Anchors are a single path segment and `#` separates them from the note.
        let new_name = new_name.trim_start_matches('#');
        if new_name.is_empty()
            || new_name.contains('/')
            || new_name.contains('\\')
            || new_name.contains('#')
        {
            return None;
        }

        log::info!("Renaming anchor '{}' to '{}'", old_name, new_name);

        let note_link = repo.path_to_link(&uri.to_file_path().ok()?)?;

        // An anchor is written either as `#name` or as `{@anchor name}`; the
        // recorded span covers whichever form was used.
        let anchor_text = &line_str[anchor_loc.span.0..anchor_loc.span.1];
        let new_anchor_text = if anchor_text.starts_with("{@anchor") {
            format!("{{@anchor {}}}", new_name)
        } else {
            format!("#{}", new_name)
        };

        let definition_edit = TextEdit {
            range: Range::new(
                Position::new(
                    anchor_loc.row as u32,
                    utf16_from_byte_idx(line_str, anchor_loc.span.0) as u32,
                ),
                Position::new(
                    anchor_loc.row as u32,
                    utf16_from_byte_idx(line_str, anchor_loc.span.1) as u32,
                ),
            ),
            new_text: new_anchor_text,
        };

        let mut document_changes = vec![edit_operation(uri.clone(), vec![definition_edit])];
        document_changes.extend(retarget_link_edits(repo, uri, |link| {
            (link.target_anchor.as_ref() == Some(&old_name))
                .then(|| format!("[{}#{}]", note_link, new_name))
        }));

        Some(workspace_edit(document_changes))
    }

    /// Rename the note under the cursor — or the current note, when the cursor is
    /// not on a wiki link — and retarget every link to it.
    fn rename_note(&self, uri: &Url, position: Position, new_name: &str) -> Option<WorkspaceEdit> {
        let repo_lock = self.repository.lock().unwrap();
        let repo = repo_lock.as_ref()?;
        let ast = repo.ast_map.get(uri)?;
        let rope = repo.document_map.get(uri)?;

        let line = rope.value().get_line(position.line as usize)?;
        let line_str = line.as_str()?;
        let position_byte = utf16_to_byte_idx(line_str, position.character as usize);

        let link_at_cursor = locate_node_route(&ast, position.line as usize, position_byte)
            .and_then(|route| {
                route.iter().find_map(|node| match node.kind() {
                    AstNodeKind::WikiLink { link, .. } => Some(link.clone()),
                    _ => None,
                })
            });

        let old_name = match link_at_cursor {
            Some(name) => name,
            None => uri.to_file_path().ok()?.file_stem()?.to_str()?.to_string(),
        };

        log::info!("Renaming note '{}' to '{}'", old_name, new_name);

        let root_uri = self.root_uri.lock().unwrap().as_ref().cloned()?;
        let new_uri = repo.link_to_uri(new_name, &root_uri)?;
        if new_uri.to_file_path().is_ok_and(|path| path.exists()) {
            log::warn!("Target file already exists: {}", new_uri);
            return None;
        }

        let old_uri = repo.link_to_uri(&old_name, &root_uri)?;
        if old_uri.to_file_path().is_ok_and(|path| !path.exists()) {
            log::warn!("Target file does not exist: {}", old_uri);
            return None;
        }

        let mut document_changes = retarget_link_edits(repo, &old_uri, |link| {
            Some(match &link.target_anchor {
                Some(anchor) => format!("[{}#{}]", new_name, anchor),
                None => format!("[{}]", new_name),
            })
        });

        document_changes.push(DocumentChangeOperation::Op(ResourceOp::Rename(
            RenameFile {
                old_uri,
                new_uri,
                options: Some(RenameFileOptions {
                    overwrite: Some(false),
                    ignore_if_exists: Some(false),
                }),
                annotation_id: None,
            },
        )));

        Some(workspace_edit(document_changes))
    }
}

/// Rewrite the links pointing at `target_uri` for which `replacement` yields
/// new text, grouped one edit operation per source document.
fn retarget_link_edits(
    repo: &Repository,
    target_uri: &Url,
    replacement: impl Fn(&LinkLocation) -> Option<String>,
) -> Vec<DocumentChangeOperation> {
    let Ok(graph) = repo.document_graph.lock() else {
        return Vec::new();
    };
    let Some(target) = graph.get(target_uri) else {
        return Vec::new();
    };

    let mut operations = Vec::new();
    for edge in target.iter_in() {
        let source_uri = edge.source().key();
        let Some(source_rope) = repo.document_map.get(source_uri) else {
            continue;
        };

        let edits: Vec<TextEdit> = edge
            .value()
            .locations
            .iter()
            .filter_map(|link| {
                let new_text = replacement(link)?;
                let line = source_rope.value().get_line(link.source_line)?;
                let line_str = line.as_str()?;
                Some(TextEdit {
                    range: Range::new(
                        Position::new(
                            link.source_line as u32,
                            utf16_from_byte_idx(line_str, link.source_col_range.0) as u32,
                        ),
                        Position::new(
                            link.source_line as u32,
                            utf16_from_byte_idx(line_str, link.source_col_range.1) as u32,
                        ),
                    ),
                    new_text,
                })
            })
            .collect();

        if !edits.is_empty() {
            operations.push(edit_operation(source_uri.clone(), edits));
        }
    }
    operations
}

fn edit_operation(uri: Url, edits: Vec<TextEdit>) -> DocumentChangeOperation {
    DocumentChangeOperation::Edit(TextDocumentEdit {
        text_document: OptionalVersionedTextDocumentIdentifier { uri, version: None },
        edits: edits.into_iter().map(OneOf::Left).collect(),
    })
}

fn workspace_edit(document_changes: Vec<DocumentChangeOperation>) -> WorkspaceEdit {
    WorkspaceEdit {
        document_changes: Some(DocumentChanges::Operations(document_changes)),
        ..Default::default()
    }
}

fn invalid_params(message: &'static str) -> Error {
    Error {
        code: ErrorCode::InvalidParams,
        message: message.into(),
        data: None,
    }
}
