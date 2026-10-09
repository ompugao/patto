//! The notes directory: its documents, the links between them, and the
//! watcher that keeps both current.

mod config;
mod link_graph;
mod messages;
mod scan;
mod tasks;
mod watcher;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use gdsl::sync_digraph::Graph;
use tokio::sync::broadcast;
use url::Url;

use crate::parser::AstNode;

pub use crate::ast_query::{gather_completed_tasks, gather_tasks};
pub use config::{
    load_workspace_config, save_workspace_config, PattoWorkspaceConfig, WORKSPACE_CONFIG_FILENAME,
};
pub use messages::{
    BackLinkData, FileMetadata, LinkEdge, LinkLocation, LinkLocationData, RepositoryMessage,
};

/// Repository manages the collection of notes and their relationships
#[derive(Clone)]
pub struct Repository {
    /// Root directory of the repository
    pub root_dir: PathBuf,

    /// Broadcast channel for change notifications
    pub tx: broadcast::Sender<RepositoryMessage>,

    /// Graph structure for note relationships (used by LSP)
    pub document_graph: Arc<Mutex<Graph<Url, AstNode, LinkEdge>>>,

    /// AST cache for parsed documents
    pub ast_map: Arc<DashMap<Url, AstNode>>,

    /// Document content cache
    pub document_map: Arc<DashMap<Url, ropey::Rope>>,

    /// Workspace-level config (.patto.toml in notes directory)
    pub workspace_config: Arc<Mutex<PattoWorkspaceConfig>>,
}

impl Repository {
    /// Open the repository at `root_dir`.
    ///
    /// Nothing is scanned or watched yet: call `spawn_initial_scan` and
    /// `start_watcher` once there is a subscriber for the messages they emit.
    pub fn new(root_dir: PathBuf) -> Self {
        let (tx, _) = broadcast::channel(100);
        let workspace_config = load_workspace_config(&root_dir);

        Self {
            root_dir,
            tx,
            document_graph: Arc::new(Mutex::new(Graph::new())),
            ast_map: Arc::new(DashMap::new()),
            document_map: Arc::new(DashMap::new()),
            workspace_config: Arc::new(Mutex::new(workspace_config)),
        }
    }

    /// Subscribe to repository change notifications
    pub fn subscribe(&self) -> broadcast::Receiver<RepositoryMessage> {
        self.tx.subscribe()
    }
}
