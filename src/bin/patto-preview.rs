use axum::extract::ws::{Message, WebSocket};
use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State, WebSocketUpgrade},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use clap::Parser;
use patto::{
    line_tracker::LineTracker,
    parser,
    preview::lsp_bridge::{self, BridgeOptions},
    repository::{BackLinkData, FileMetadata, Repository, RepositoryMessage},
    utils::mime_type_for_path,
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::fs;

// Embed the new Vite/React frontend (built with `npm run build` in patto-preview-ui/)
#[derive(RustEmbed)]
#[folder = "patto-preview-ui/dist/"]
struct ViteAssets;

// CLI argument parsing
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory to watch for .pn files
    #[arg(default_value = ".")]
    dir: String,

    /// Port to run the server on
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Optional TCP port for the preview LSP bridge
    #[arg(long)]
    preview_lsp_port: Option<u16>,

    /// Serve the preview LSP bridge over stdio (overrides preview_lsp_port)
    #[arg(long, default_value_t = false)]
    preview_lsp_stdio: bool,
}

// App state
#[derive(Clone)]
struct AppState {
    repository: Arc<Repository>,
    line_trackers: Arc<Mutex<HashMap<PathBuf, LineTracker>>>,
}

// WebSocket messages sent to client
#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
enum WsServerMessage {
    FileList {
        files: Vec<String>,
        metadata: HashMap<String, FileMetadata>,
    },
    FileChanged {
        path: String,
        metadata: FileMetadata,
        ast: patto::parser::AstNode,
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

// WebSocket messages received from client
#[derive(Deserialize)]
#[serde(tag = "type", content = "data")]
enum WsClientMessage {
    SelectFile { path: String },
    PinFile { path: String },
    UnpinFile { path: String },
}

// Helper function to get file extension

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args = Args::parse();
    let dir = std::fs::canonicalize(PathBuf::from(&args.dir)).unwrap_or_else(|_| {
        eprintln!("Failed to canonicalize directory: {}", args.dir);
        std::process::exit(1);
    });

    if !dir.exists() {
        eprintln!("Directory does not exist: {}", dir.display());
        std::process::exit(1);
    }

    // Create repository and app state
    let repository = Arc::new(Repository::new(dir.clone()));
    let state = AppState {
        repository: repository.clone(),
        line_trackers: Arc::new(Mutex::new(HashMap::new())),
    };

    // Start file watcher in a separate task
    let repository_clone = repository.clone();
    tokio::spawn(async move {
        if let Err(e) = repository_clone.start_watcher().await {
            eprintln!("Failed to start file watcher: {}", e);
        }
    });

    let mut shutdown_signal = None;
    if args.preview_lsp_stdio {
        shutdown_signal = Some(lsp_bridge::serve_stdio(repository.clone()));
    } else if let Some(lsp_port) = args.preview_lsp_port {
        let options = BridgeOptions {
            log_connections: true,
        };
        if let Err(e) = lsp_bridge::serve_tcp(repository.clone(), lsp_port, options).await {
            eprintln!("Failed to start preview LSP server: {}", e);
        }
    }

    // Create router
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/twitter-embed", get(twitter_embed_handler))
        .route("/api/speakerdeck-embed", get(speakerdeck_embed_handler))
        .route("/api/slideshare-embed", get(slideshare_embed_handler))
        .route("/api/files/{*path}", get(user_files_handler))
        .fallback(get(vite_static_handler)) // Serve Vite SPA for all other routes
        .with_state(state);

    // Start server
    eprintln!("Starting server at http://localhost:{}", args.port);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();

    let server = axum::serve(listener, app);
    if let Some(mut rx) = shutdown_signal {
        tokio::select! {
            result = server => {
                if let Err(err) = result {
                    eprintln!("Preview server error: {err}");
                }
            }
            _ = &mut rx => {
                eprintln!("Preview LSP connection closed; terminating preview server");
            }
        }

        std::process::exit(0);
    } else if let Err(err) = server.await {
        eprintln!("Preview server error: {err}");
    }
}

// Handler for Vite static assets — serves files from dist/, falls back to index.html for SPA
async fn vite_static_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Helper to build a response
    let serve_file = |data: Vec<u8>, content_type: &'static str| {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(data))
            .unwrap()
    };

    // Try to serve the exact path; if not found (or empty path), serve index.html
    if path.is_empty() {
        // Root: always serve index.html
        return match ViteAssets::get("index.html") {
            Some(f) => serve_file(f.data.to_vec(), "text/html; charset=utf-8"),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(
                    "UI not built — run `npm run build` in patto-preview-ui/",
                ))
                .unwrap(),
        };
    }

    match ViteAssets::get(path) {
        Some(f) => {
            // Serve the exact asset with the correct content-type
            let ct = if path.ends_with(".html") {
                "text/html; charset=utf-8"
            } else {
                mime_type_for_path(Path::new(path))
            };
            serve_file(f.data.to_vec(), ct)
        }
        None => {
            // SPA fallback: return index.html for client-side routing
            match ViteAssets::get("index.html") {
                Some(f) => serve_file(f.data.to_vec(), "text/html; charset=utf-8"),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Not found"))
                    .unwrap(),
            }
        }
    }
}

// Handler for Twitter embed proxy
async fn twitter_embed_handler(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let url = match params.get("url") {
        Some(url) => url,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing url parameter"})),
            )
        }
    };

    // Validate that this is actually a Twitter/X URL
    if !url.contains("twitter.com") && !url.contains("x.com") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid Twitter URL"})),
        );
    }

    let api_url = format!(
        "https://publish.twitter.com/oembed?url={}",
        urlencoding::encode(url)
    );

    match reqwest::get(&api_url).await {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(json) => (StatusCode::OK, Json(json)),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to parse Twitter response"})),
            ),
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to fetch Twitter embed"})),
        ),
    }
}

// Handler for SpeakerDeck embed proxy
async fn speakerdeck_embed_handler(
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let url = match params.get("url") {
        Some(url) => url,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing url parameter"})),
            )
        }
    };

    // Validate that this is actually a SpeakerDeck URL
    if !url.contains("speakerdeck.com") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid SpeakerDeck URL"})),
        );
    }

    let api_url = format!(
        "https://speakerdeck.com/oembed.json?url={}",
        urlencoding::encode(url)
    );

    match reqwest::get(&api_url).await {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(json) => (StatusCode::OK, Json(json)),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to parse SpeakerDeck response"})),
            ),
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to fetch SpeakerDeck embed"})),
        ),
    }
}

// Handler for SlideShare embed proxy
async fn slideshare_embed_handler(
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let url = match params.get("url") {
        Some(url) => url,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing url parameter"})),
            )
        }
    };

    if !url.contains("slideshare.net") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid SlideShare URL"})),
        );
    }

    let api_url = format!(
        "https://www.slideshare.net/api/oembed/2?url={}&format=json",
        urlencoding::encode(url)
    );

    match reqwest::get(&api_url).await {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(json) => (StatusCode::OK, Json(json)),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to parse SlideShare response"})),
            ),
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to fetch SlideShare embed"})),
        ),
    }
}

// Handler for user files (images, videos, etc.) from note directory
async fn user_files_handler(
    AxumPath(path): AxumPath<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Decode the URL-encoded path
    let path_cloned = path.clone();
    let decoded_path = urlencoding::decode(&path).unwrap_or_else(|_| path_cloned.into());
    let file_path = state.repository.root_dir.join(decoded_path.as_ref());

    // Security check - ensure the path doesn't escape the base directory
    let canonical_base = match std::fs::canonicalize(&state.repository.root_dir) {
        Ok(base) => base,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Base directory error"))
                .unwrap();
        }
    };

    let canonical_file = match std::fs::canonicalize(&file_path) {
        Ok(file) => file,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("File not found"))
                .unwrap();
        }
    };

    // Ensure the file is within the base directory (prevent directory traversal)
    if !canonical_file.starts_with(&canonical_base) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Access denied"))
            .unwrap();
    }

    // Check if the file exists and is actually a file (not a directory)
    if !canonical_file.exists() || !canonical_file.is_file() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("File not found"))
            .unwrap();
    }

    // Read and serve the file
    match fs::read(&canonical_file).await {
        Ok(contents) => {
            let mime_type = mime_type_for_path(&canonical_file);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime_type)
                .header(header::CACHE_CONTROL, "public, max-age=3600") // Cache for 1 hour
                .body(Body::from(contents))
                .unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Error reading file"))
            .unwrap(),
    }
}

// Helper function to determine content type from path
// WebSocket handler
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move { PreviewSession { socket, state }.run().await })
}

// Handle WebSocket connection
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
            WsClientMessage::PinFile { path } => {
                if let Err(err) = self.state.repository.pin_file(&path) {
                    eprintln!("Error pinning file: {err}");
                }
                // The WorkspaceConfigChanged broadcast carries the update to every client.
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

        let Ok(content) = fs::read_to_string(&file_path).await else {
            eprintln!("Error reading file: {}", file_path.display());
            return;
        };
        let Ok(metadata) = self.state.repository.collect_file_metadata(&file_path) else {
            eprintln!("Error reading metadata: {}", file_path.display());
            return;
        };
        let Ok(ast) = parse_patto_ast(&content, &file_path.to_string_lossy(), &self.state).await
        else {
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
async fn to_client_message(
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
            let ast = parse_patto_ast(&content, &path.to_string_lossy(), state)
                .await
                .ok()?;
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

// Parse patto content to AST with persistent line tracking
async fn parse_patto_ast(
    content: &str,
    file_path: &str,
    state: &AppState,
) -> std::io::Result<patto::parser::AstNode> {
    // Use Arc to avoid cloning large content
    let content = std::sync::Arc::new(content.to_string());
    let file_path_buf = PathBuf::from(file_path);

    // Get or create line tracker for this file
    let line_trackers = Arc::clone(&state.line_trackers);

    let ast_output = tokio::task::spawn_blocking(move || {
        // Get or create line tracker for this file
        let mut trackers = line_trackers.lock().unwrap();
        let line_tracker = trackers.entry(file_path_buf.clone()).or_insert_with(|| {
            LineTracker::new().unwrap_or_else(|_| {
                panic!();
            })
        });

        let result = parser::parse_text_with_persistent_line_tracking(&content, line_tracker);
        result.ast
    })
    .await;

    match ast_output {
        Ok(ast) => Ok(ast),
        Err(e) => Err(std::io::Error::other(e)),
    }
}
