//! Document synchronisation: every full-text change re-indexes the note,
//! applies the edits a task status transition implies, and publishes
//! diagnostics.

use std::collections::HashMap;

use serde_json::Value;
use tower_lsp::lsp_types::{TextEdit, Url, WorkspaceEdit};

use crate::lsp::backend::{Backend, PattoSettings};
use crate::lsp::diagnostics::diagnostics_for;
use crate::lsp::task_edits::{
    collect_task_snapshots, detect_task_transitions, generate_edits_for_transition,
};
use crate::parser::AstNode;
use crate::repository::Repository;
use crate::task::TaskSnapshot;

impl Backend {
    pub(super) async fn on_change(&self, uri: Url, text: String, version: i32) {
        let normalized = Repository::normalize_url_percent_encoding(&uri);

        if let Ok(file_path) = normalized.to_file_path() {
            if let Some(repository) = self.repository.lock().unwrap().as_ref() {
                repository.add_file_to_graph(&file_path, &text);
            }
            if let Some(ast) = self.document_ast(&normalized) {
                let edits = self.task_transition_edits(&normalized, &ast);
                if !edits.is_empty() {
                    let _ = self
                        .client
                        .apply_edit(WorkspaceEdit {
                            changes: Some(HashMap::from([(normalized.clone(), edits)])),
                            document_changes: None,
                            change_annotations: None,
                        })
                        .await;
                }
            }
        }

        self.client
            .publish_diagnostics(uri, diagnostics_for(&text), Some(version))
            .await;
    }

    /// Transitions are detected against the last snapshot whose status parsed
    /// cleanly, not the previous keystroke: while `status=doin` is being typed
    /// the task is unparseable, and the `Doing` state must survive until the
    /// edit lands so that clock-out can still compute the elapsed time.
    fn task_transition_edits(&self, uri: &Url, ast: &AstNode) -> Vec<TextEdit> {
        let new_snapshots = collect_task_snapshots(ast);
        let old_snapshots: HashMap<usize, TaskSnapshot> = self
            .last_valid_task_snapshots
            .get(uri)
            .map(|entry| entry.value().clone())
            .unwrap_or_default();

        let mut remembered = self
            .last_valid_task_snapshots
            .entry(uri.clone())
            .or_default();
        for (row, snapshot) in &new_snapshots {
            if snapshot.status_is_canonical {
                remembered.insert(*row, snapshot.clone());
            }
        }
        drop(remembered);

        let now = chrono::Local::now().naive_local();
        detect_task_transitions(&new_snapshots, &old_snapshots)
            .iter()
            .flat_map(|transition| generate_edits_for_transition(transition, now))
            .collect()
    }

    /// VS Code sends `{ "patto": { ... } }`; other clients send the section itself.
    pub(super) fn update_settings(&self, settings: Value) {
        let section = match settings.get("patto") {
            Some(patto) => patto.clone(),
            None => settings,
        };
        match serde_json::from_value::<PattoSettings>(section) {
            Ok(new_settings) => {
                log::info!("Updated patto settings: {:?}", new_settings);
                *self.settings.lock().unwrap() = new_settings;
            }
            Err(err) => log::warn!("Failed to parse patto settings: {:?}", err),
        }
    }
}
