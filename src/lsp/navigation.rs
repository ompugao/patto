//! `textDocument/definition` and `textDocument/references`, both resolved
//! through the repository's link graph.

use tower_lsp::lsp_types::{Location, Position, Range, Url};

use crate::ast_query::find_anchor;
use crate::lsp::backend::Backend;
use crate::lsp::locate::{cursor_byte, line_str, node_range, utf16_range, wiki_link_at};

impl Backend {
    /// The note a wiki link under the cursor points at; its anchored line when
    /// the link names an anchor the note defines.
    pub(super) fn definition_at(&self, uri: &Url, position: Position) -> Option<Location> {
        let repository = self.repository.lock().unwrap();
        let repo = repository.as_ref()?;
        let ast = repo.ast_map.get(uri)?;
        let rope = repo.document_map.get(uri)?;
        let line = line_str(rope.value(), position.line)?;

        let (link, anchor) =
            wiki_link_at(&ast, position.line as usize, cursor_byte(line, position))?;
        let root_uri = self.root_uri.lock().unwrap().clone()?;
        let link_uri = repo.link_to_uri(&link, &root_uri).unwrap_or(uri.clone());

        let start_of_file = Range::new(Position::new(0, 0), Position::new(0, 1));
        let range = anchor
            .and_then(|anchor| find_anchor(repo.ast_map.get(&link_uri)?.value(), &anchor))
            .map_or(start_of_file, |anchored_line| node_range(&anchored_line));
        Some(Location::new(link_uri, range))
    }

    pub(super) fn references_to(&self, uri: &Url) -> Option<Vec<Location>> {
        let repository = self.repository.lock().unwrap();
        let repo = repository.as_ref()?;
        let graph = repo.document_graph.lock().ok()?;
        let node = graph.get(uri)?;

        let mut references = Vec::new();
        for edge in node.iter_in() {
            let source_uri = edge.source().key();
            let Some(rope) = repo.document_map.get(source_uri) else {
                continue;
            };
            for link in &edge.value().locations {
                let Some(line) = line_str(rope.value(), link.source_line as u32) else {
                    continue;
                };
                let range = utf16_range(line, link.source_line as u32, link.source_col_range);
                references.push(Location::new(source_uri.clone(), range));
            }
        }
        log::debug!(
            "references retrieved from graph: {} locations",
            references.len()
        );
        Some(references)
    }
}
