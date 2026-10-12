//! The browser preview's HTTP surface, assembled from the handlers in the
//! sibling modules so that `patto-preview` is only argument parsing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::{routing::get, Router};

use crate::line_tracker::LineTracker;
use crate::repository::Repository;

use super::{embeds, session, static_files, user_files};

#[derive(Clone)]
pub struct AppState {
    pub repository: Arc<Repository>,
    /// One tracker per file, so line identities survive between renders.
    pub line_trackers: Arc<Mutex<HashMap<PathBuf, LineTracker>>>,
}

impl AppState {
    pub fn new(repository: Arc<Repository>) -> Self {
        Self {
            repository,
            line_trackers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// The routes patto-preview-ui and the editor clients depend on: the
/// WebSocket, the oEmbed proxies, the notes directory under `/api/files/`,
/// and the bundled UI for every other path.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(session::ws_handler))
        .route("/api/twitter-embed", get(embeds::twitter))
        .route("/api/speakerdeck-embed", get(embeds::speakerdeck))
        .route("/api/slideshare-embed", get(embeds::slideshare))
        .route("/api/google-photos-embed", get(embeds::google_photos))
        .route("/api/files/{*path}", get(user_files::serve))
        .fallback(get(static_files::serve))
        .with_state(state)
}
