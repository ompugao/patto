//! One WebSocket connection to the browser preview.
//!
//! The message enums are the wire protocol: `patto-preview-ui/src/protocol.ts`
//! mirrors their serde shape, so variants and field names cannot change.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::line_tracker::LineTracker;
use crate::parser::{self, AstNode};
use crate::repository::{BackLinkData, FileMetadata, RepositoryMessage};

use super::server::AppState;

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum WsServerMessage {
    FileList {
        files: Vec<String>,
        metadata: HashMap<String, FileMetadata>,
    },
    FileChanged {
        path: String,
        metadata: FileMetadata,
        ast: AstNode,
    },
    FileAdded {
        path: String,
        metadata: FileMetadata,
    },
    FileRemoved {
        path: String,
    },
    BackLinksData {
        path: String,
        back_links: Vec<BackLinkData>,
    },
    TwoHopLinksData {
        path: String,
        two_hop_links: Vec<(String, Vec<String>)>,
    },
    PinnedFiles {
        pinned: Vec<String>,
    },
}

// Variant names are the wire protocol shared with patto-preview-ui.
#[allow(clippy::enum_variant_names)]
#[derive(Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsClientMessage {
    SelectFile { path: String },
    PinFile { path: String },
    UnpinFile { path: String },
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move { PreviewSession { socket, state }.run().await })
}

/// One connected preview client: pushes repository changes to the browser and
/// serves what the browser asks for.
struct PreviewSession {
    socket: WebSocket,
    state: AppState,
}

impl PreviewSession {
    async fn run(mut self) {
        eprintln!("WebSocket client connected");
        let mut repository_messages = self.state.repository.subscribe();

        if !self.send_initial_state().await {
            return;
        }

        loop {
            tokio::select! {
                message = repository_messages.recv() => {
                    match message {
                        Ok(message) => {
                            if !self.forward_repository_message(message).await {
                                break;
                            }
                        }
                        Err(err) => eprintln!("Error receiving broadcast: {err}"),
                    }
                }
                message = self.socket.recv() => {
                    match message {
                        Some(Ok(Message::Text(text))) => self.handle_client_message(&text).await,
                        Some(Ok(_)) => {}
                        Some(Err(err)) => {
                            eprintln!("WebSocket error: {err}");
                            break;
                        }
                        None => {
                            eprintln!("WebSocket client disconnected");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Send one message. `false` once the connection can no longer be used.
    async fn send(&mut self, message: &WsServerMessage) -> bool {
        let Ok(json) = serde_json::to_string(message) else {
            eprintln!("Failed to serialize a WebSocket message");
            return true;
        };
        match self.socket.send(Message::Text(json.into())).await {
            Ok(()) => true,
            Err(err) => {
                eprintln!("Error sending WebSocket message: {err}");
                false
            }
        }
    }

    async fn send_initial_state(&mut self) -> bool {
        // Scanning a large notes directory is synchronous `std::fs` work, so it
        // runs off the async runtime.
        let repository = self.state.repository.clone();
        let (files, metadata) = tokio::task::spawn_blocking(move || {
            let mut files = Vec::new();
            let mut metadata = HashMap::new();
            if repository.root_dir.is_dir() {
                repository.collect_patto_files_with_metadata(
                    &repository.root_dir,
                    &mut files,
                    &mut metadata,
                );
            }
            (files, metadata)
        })
        .await
        .unwrap_or_default();

        if !self
            .send(&WsServerMessage::FileList { files, metadata })
            .await
        {
            return false;
        }

        let pinned = self
            .state
            .repository
            .workspace_config
            .lock()
            .unwrap()
            .pinned_files
            .clone();
        self.send(&WsServerMessage::PinnedFiles { pinned }).await
    }

    async fn forward_repository_message(&mut self, message: RepositoryMessage) -> bool {
        let Some(message) = to_client_message(&self.state, message).await else {
            return true;
        };
        self.send(&message).await
    }

    async fn handle_client_message(&mut self, text: &str) {
        let Ok(message) = serde_json::from_str::<WsClientMessage>(text) else {
            return;
        };
        match message {
            WsClientMessage::SelectFile { path } => self.select_file(&path).await,
            // The WorkspaceConfigChanged broadcast carries the update to every client.
            WsClientMessage::PinFile { path } => {
                if let Err(err) = self.state.repository.pin_file(&path) {
                    eprintln!("Error pinning file: {err}");
                }
            }
            WsClientMessage::UnpinFile { path } => {
                if let Err(err) = self.state.repository.unpin_file(&path) {
                    eprintln!("Error unpinning file: {err}");
                }
            }
        }
    }

    /// Send the note the client asked for, together with its link context.
    async fn select_file(&mut self, path: &str) {
        eprintln!("Client selected file: {}", path);
        let file_path = self.state.repository.root_dir.join(path);
        if !super::within_root(&self.state.repository.root_dir, &file_path) {
            eprintln!("Refusing a path outside the notes directory: {}", path);
            return;
        }

        let Ok(content) = tokio::fs::read_to_string(&file_path).await else {
            eprintln!("Error reading file: {}", file_path.display());
            return;
        };
        let Ok(metadata) = self.state.repository.collect_file_metadata(&file_path) else {
            eprintln!("Error reading metadata: {}", file_path.display());
            return;
        };
        let Ok(ast) = parse_patto_ast(&content, &file_path, &self.state).await else {
            eprintln!("Error rendering file: {}", path);
            return;
        };

        let back_links = self.state.repository.calculate_back_links(&file_path);
        let two_hop_links = self
            .state
            .repository
            .calculate_two_hop_links(&file_path)
            .await;

        let messages = [
            WsServerMessage::FileChanged {
                path: path.to_string(),
                metadata,
                ast,
            },
            WsServerMessage::BackLinksData {
                path: path.to_string(),
                back_links,
            },
            WsServerMessage::TwoHopLinksData {
                path: path.to_string(),
                two_hop_links,
            },
        ];
        for message in &messages {
            self.send(message).await;
        }
    }
}

/// Translate a repository event into what the browser expects, or `None` when
/// the browser has no use for it.
pub async fn to_client_message(
    state: &AppState,
    message: RepositoryMessage,
) -> Option<WsServerMessage> {
    let root_dir = &state.repository.root_dir;
    let relative = |path: &Path| {
        path.strip_prefix(root_dir)
            .ok()
            .map(|path| path.to_string_lossy().to_string())
    };

    match message {
        RepositoryMessage::FileChanged(path, metadata, content) => {
            let ast = parse_patto_ast(&content, &path, state).await.ok()?;
            Some(WsServerMessage::FileChanged {
                path: relative(&path)?,
                metadata,
                ast,
            })
        }
        RepositoryMessage::FileAdded(path, metadata) => Some(WsServerMessage::FileAdded {
            path: relative(&path)?,
            metadata,
        }),
        RepositoryMessage::FileRemoved(path) => Some(WsServerMessage::FileRemoved {
            path: relative(&path)?,
        }),
        RepositoryMessage::BackLinksChanged(path, back_links) => {
            Some(WsServerMessage::BackLinksData {
                path: relative(&path)?,
                back_links,
            })
        }
        RepositoryMessage::TwoHopLinksChanged(path, two_hop_links) => {
            Some(WsServerMessage::TwoHopLinksData {
                path: relative(&path)?,
                two_hop_links,
            })
        }
        RepositoryMessage::WorkspaceConfigChanged(config) => Some(WsServerMessage::PinnedFiles {
            pinned: config.pinned_files,
        }),
        RepositoryMessage::ScanStarted { .. }
        | RepositoryMessage::ScanProgress { .. }
        | RepositoryMessage::ScanCompleted { .. } => None,
    }
}

/// Parse with the file's own line tracker, so that unchanged lines keep their
/// stable ids across renders and the browser can patch them in place.
async fn parse_patto_ast(
    content: &str,
    file_path: &Path,
    state: &AppState,
) -> std::io::Result<AstNode> {
    let content = Arc::new(content.to_string());
    let file_path: PathBuf = file_path.to_path_buf();
    let line_trackers = Arc::clone(&state.line_trackers);

    tokio::task::spawn_blocking(move || {
        let mut trackers = line_trackers.lock().unwrap();
        let line_tracker = trackers
            .entry(file_path)
            .or_insert_with(|| LineTracker::new().expect("a fresh line tracker"));
        parser::parse_text_with_persistent_line_tracking(&content, line_tracker).ast
    })
    .await
    .map_err(std::io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{PattoWorkspaceConfig, Repository};
    use serde_json::{json, to_value};

    fn metadata() -> FileMetadata {
        FileMetadata {
            modified: 1,
            created: 2,
            link_count: 3,
        }
    }

    fn state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let state = AppState::new(Arc::new(Repository::new(root)));
        (dir, state)
    }

    #[test]
    fn server_messages_are_tagged_with_type_and_data() {
        let message = WsServerMessage::FileRemoved {
            path: "a.pn".to_string(),
        };
        assert_eq!(
            to_value(&message).unwrap(),
            json!({"type": "FileRemoved", "data": {"path": "a.pn"}})
        );
    }

    #[test]
    fn file_metadata_is_sent_with_a_camel_case_link_count() {
        let message = WsServerMessage::FileAdded {
            path: "a.pn".to_string(),
            metadata: metadata(),
        };
        assert_eq!(
            to_value(&message).unwrap(),
            json!({
                "type": "FileAdded",
                "data": {"path": "a.pn", "metadata": {"modified": 1, "created": 2, "linkCount": 3}}
            })
        );
    }

    #[test]
    fn file_list_and_pinned_files_keep_their_field_names() {
        let list = WsServerMessage::FileList {
            files: vec!["a.pn".to_string()],
            metadata: HashMap::from([("a.pn".to_string(), metadata())]),
        };
        let value = to_value(&list).unwrap();
        assert_eq!(value["type"], "FileList");
        assert_eq!(value["data"]["files"], json!(["a.pn"]));
        assert_eq!(value["data"]["metadata"]["a.pn"]["linkCount"], 3);

        let pinned = WsServerMessage::PinnedFiles {
            pinned: vec!["a.pn".to_string()],
        };
        assert_eq!(
            to_value(&pinned).unwrap(),
            json!({"type": "PinnedFiles", "data": {"pinned": ["a.pn"]}})
        );
    }

    #[test]
    fn link_messages_keep_their_field_names() {
        let back = WsServerMessage::BackLinksData {
            path: "a.pn".to_string(),
            back_links: vec![],
        };
        assert_eq!(
            to_value(&back).unwrap(),
            json!({"type": "BackLinksData", "data": {"path": "a.pn", "back_links": []}})
        );
        let two_hop = WsServerMessage::TwoHopLinksData {
            path: "a.pn".to_string(),
            two_hop_links: vec![("hub".to_string(), vec!["b".to_string()])],
        };
        assert_eq!(
            to_value(&two_hop).unwrap(),
            json!({"type": "TwoHopLinksData", "data": {"path": "a.pn", "two_hop_links": [["hub", ["b"]]]}})
        );
    }

    #[test]
    fn client_messages_parse_from_type_and_data() {
        let select: WsClientMessage =
            serde_json::from_str(r#"{"type":"SelectFile","data":{"path":"n.pn"}}"#).unwrap();
        assert!(matches!(select, WsClientMessage::SelectFile { path } if path == "n.pn"));
        let pin: WsClientMessage =
            serde_json::from_str(r#"{"type":"PinFile","data":{"path":"n.pn"}}"#).unwrap();
        assert!(matches!(pin, WsClientMessage::PinFile { path } if path == "n.pn"));
        let unpin: WsClientMessage =
            serde_json::from_str(r#"{"type":"UnpinFile","data":{"path":"n.pn"}}"#).unwrap();
        assert!(matches!(unpin, WsClientMessage::UnpinFile { path } if path == "n.pn"));
    }

    #[test]
    fn unknown_client_messages_are_rejected() {
        assert!(serde_json::from_str::<WsClientMessage>(r#"{"type":"Reload","data":{}}"#).is_err());
    }

    #[tokio::test]
    async fn repository_paths_are_sent_relative_to_the_root() {
        let (_dir, state) = state();
        let path = state.repository.root_dir.join("sub").join("a.pn");

        let message = to_client_message(&state, RepositoryMessage::FileAdded(path, metadata()))
            .await
            .unwrap();

        assert_eq!(
            to_value(&message).unwrap()["data"]["path"],
            json!(PathBuf::from("sub").join("a.pn").to_string_lossy())
        );
    }

    #[tokio::test]
    async fn files_outside_the_root_are_not_forwarded() {
        let (_dir, state) = state();

        let message = to_client_message(
            &state,
            RepositoryMessage::FileRemoved(PathBuf::from("/elsewhere/a.pn")),
        )
        .await;

        assert!(message.is_none());
    }

    #[tokio::test]
    async fn file_changes_are_forwarded_with_the_parsed_ast() {
        let (_dir, state) = state();
        let path = state.repository.root_dir.join("a.pn");

        let message = to_client_message(
            &state,
            RepositoryMessage::FileChanged(path, metadata(), "hello\n".to_string()),
        )
        .await
        .unwrap();

        let value = to_value(&message).unwrap();
        assert_eq!(value["type"], "FileChanged");
        assert_eq!(value["data"]["path"], "a.pn");
        assert!(value["data"]["ast"].is_object());
    }

    #[tokio::test]
    async fn scan_progress_stays_server_side() {
        let (_dir, state) = state();
        assert!(
            to_client_message(&state, RepositoryMessage::ScanStarted { total_files: 1 })
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn workspace_config_changes_become_the_pinned_list() {
        let (_dir, state) = state();
        let config = PattoWorkspaceConfig {
            version: 1,
            pinned_files: vec!["a.pn".to_string()],
        };

        let message = to_client_message(&state, RepositoryMessage::WorkspaceConfigChanged(config))
            .await
            .unwrap();

        assert_eq!(
            to_value(&message).unwrap(),
            json!({"type": "PinnedFiles", "data": {"pinned": ["a.pn"]}})
        );
    }
}
