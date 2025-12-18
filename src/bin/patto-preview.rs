use axum::extract::ws::WebSocket;
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
    renderer::{HtmlRenderer, HtmlRendererOptions, Renderer},
    repository::{BackLinkData, FileMetadata, Repository, RepositoryMessage},
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::{sleep, Instant};
use tokio::{fs, sync::oneshot};
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    InitializedParams, MessageType, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

// Embed Next.js static files
#[derive(RustEmbed)]
#[folder = "patto-preview-next/out/_next/"]
struct NextJsAssets;

#[derive(RustEmbed)]
#[folder = "patto-preview-next/out/"]
#[include = "*.html"]
#[include = "*.ico"]
#[include = "*.svg"]
#[include = "*.txt"]
#[include = "js/*.js"]
struct NextJsRoot;

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
    debouncer: Arc<Mutex<debounce::ContentDebouncer>>,
}

// ============================================================================
// Content Debouncer - Batches content updates with adaptive timing
// ============================================================================
//
// Adjusts debounce delay based on previous parse/render time:
// - Slower files get longer debounce to avoid overwhelming the system
// - Faster files get shorter debounce for quicker feedback
// - Always flushes within max_wait to ensure responsiveness

mod debounce {
    use super::*;

    const DEBOUNCE_MIN_MS: u64 = 5;
    const DEBOUNCE_MAX_MS: u64 = 500;
    const DEBOUNCE_DEFAULT_MS: u64 = 20;
    const MAX_WAIT_MS: u64 = 2000;
    // Debounce = parse_time * multiplier (clamped to range)
    const PARSE_TIME_MULTIPLIER: f64 = 2.0;

    /// Tracks pending content for a single file
    struct PendingUpdate {
        text: String,
        first_pending_at: Instant,
        generation: u64,
    }

    /// Debounces content updates for multiple files based on parse time
    pub struct ContentDebouncer {
        pending: HashMap<PathBuf, PendingUpdate>,
        last_parse_time_ms: HashMap<PathBuf, u64>,
    }

    impl ContentDebouncer {
        pub fn new() -> Self {
            Self {
                pending: HashMap::new(),
                last_parse_time_ms: HashMap::new(),
            }
        }

        /// Record how long parsing took for a file (call after parse completes)
        pub fn record_parse_time(&mut self, path: &PathBuf, duration_ms: u64) {
            self.last_parse_time_ms.insert(path.clone(), duration_ms);
        }

        /// Compute debounce time based on last parse duration
        fn debounce_for(&self, path: &PathBuf) -> u64 {
            self.last_parse_time_ms
                .get(path)
                .map(|&ms| {
                    ((ms as f64 * PARSE_TIME_MULTIPLIER) as u64)
                        .clamp(DEBOUNCE_MIN_MS, DEBOUNCE_MAX_MS)
                })
                .unwrap_or(DEBOUNCE_DEFAULT_MS)
        }

        /// Queue a content update. Returns (text_to_flush_now, generation, debounce_ms).
        pub fn queue(&mut self, path: &PathBuf, text: String) -> (Option<String>, u64, u64) {
            let now = Instant::now();
            let debounce_ms = self.debounce_for(path);

            match self.pending.entry(path.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(PendingUpdate {
                        text,
                        first_pending_at: now,
                        generation: 0,
                    });
                    (None, 0, debounce_ms)
                }
                Entry::Occupied(mut entry) => {
                    let pending = entry.get_mut();
                    pending.text = text;
                    pending.generation = pending.generation.wrapping_add(1);

                    // Flush immediately if waiting too long
                    if pending.first_pending_at.elapsed() >= Duration::from_millis(MAX_WAIT_MS) {
                        let text = entry.remove().text;
                        (Some(text), 0, 0)
                    } else {
                        (None, pending.generation, debounce_ms)
                    }
                }
            }
        }

        /// Take pending text if generation matches (called after debounce delay).
        pub fn take_if_ready(&mut self, path: &PathBuf, generation: u64) -> Option<String> {
            if self.pending.get(path).map(|p| p.generation) == Some(generation) {
                self.pending.remove(path).map(|p| p.text)
            } else {
                None
            }
        }
    }
}

struct PreviewLspBackend {
    client: Client,
    repository: Arc<Repository>,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    debouncer: Arc<Mutex<debounce::ContentDebouncer>>,
}

impl PreviewLspBackend {
    fn new(
        client: Client,
        repository: Arc<Repository>,
        shutdown_tx: Option<oneshot::Sender<()>>,
        debouncer: Arc<Mutex<debounce::ContentDebouncer>>,
    ) -> Self {
        Self {
            client,
            repository,
            shutdown_tx: Mutex::new(shutdown_tx),
            debouncer,
        }
    }

    async fn handle_text_change(&self, uri: Url, text: String) {
        let normalized = Repository::normalize_url_percent_encoding(&uri);
        let Ok(path) = normalized.to_file_path() else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("Preview LSP ignoring non-file URI: {}", normalized),
                )
                .await;
            return;
        };

        if path.extension().and_then(|s| s.to_str()) != Some("pn") {
            return;
        }

        if !path.starts_with(&self.repository.root_dir) {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!(
                        "Preview LSP ignoring file outside workspace: {}",
                        path.display()
                    ),
                )
                .await;
            return;
        }

        self.queue_live_content_update(path, text).await;
    }

    async fn queue_live_content_update(&self, path: PathBuf, text: String) {
        let debouncer = self.debouncer.clone();
        let repository = self.repository.clone();

        let (flush_now, generation, debounce_ms) = debouncer.lock().unwrap().queue(&path, text);

        if let Some(text) = flush_now {
            // Max wait exceeded, flush immediately with lightweight update
            repository.handle_live_file_change_lightweight(path, text);
        } else {
            // Schedule delayed flush with lightweight update
            tokio::spawn(async move {
                sleep(Duration::from_millis(debounce_ms)).await;
                let text = debouncer.lock().unwrap().take_if_ready(&path, generation);
                if let Some(text) = text {
                    repository.handle_live_file_change_lightweight(path, text);
                }
            });
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for PreviewLspBackend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        ..Default::default()
                    },
                )),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Preview LSP bridge connected")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.handle_text_change(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.handle_text_change(params.text_document.uri, change.text)
                .await;
        }
    }
}

async fn start_preview_lsp_server(
    repository: Arc<Repository>,
    port: u16,
    debouncer: Arc<Mutex<debounce::ContentDebouncer>>,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    eprintln!("Preview LSP server listening on 127.0.0.1:{}", port);

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let repo = repository.clone();
                    let debouncer = debouncer.clone();
                    tokio::spawn(async move {
                        let (reader, writer) = tokio::io::split(stream);
                        let (service, socket) = LspService::new(|client| {
                            PreviewLspBackend::new(client, repo.clone(), None, debouncer.clone())
                        });
                        Server::new(reader, writer, socket).serve(service).await;
                        eprintln!("Preview LSP connection {} closed", addr);
                    });
                }
                Err(err) => {
                    eprintln!("Preview LSP accept error: {err}");
                }
            }
        }
    });

    Ok(())
}

fn start_preview_lsp_stdio(
    repository: Arc<Repository>,
    debouncer: Arc<Mutex<debounce::ContentDebouncer>>,
) -> oneshot::Receiver<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (tx, rx) = oneshot::channel();
    // PreviewLspBackend needs its own copy of the sender so it can fire it exactly once when Neovim calls the LSP shutdown() method; the stdio server
    // task also needs the same sender so it can notify the main process if the LSP loop exits on its own. If we stored Option<Arc<Mutex<_>>>, every
    // backend clone would hold the same Arc, so calling take() inside one backend wouldn’t remove the sender for others—they’d still see Some. By
    // storing the Option inside each backend (and cloning the Arc<Mutex<Option<_>>> wrapper instead), the sender itself lives only once in the shared
    // mutex and take() truly consumes it, preventing double-send/panic and ensuring whichever context notices shutdown first owns the signal.
    let shutdown_tx = Arc::new(Mutex::new(Some(tx)));

    let shutdown_tx_server = shutdown_tx.clone();

    tokio::spawn(async move {
        let (service, socket) = LspService::new(move |client| {
            let sender = shutdown_tx.lock().unwrap().take();
            PreviewLspBackend::new(client, repository.clone(), sender, debouncer.clone())
        });
        Server::new(stdin, stdout, socket).serve(service).await;
        if let Some(tx) = shutdown_tx_server.lock().unwrap().take() {
            let _ = tx.send(());
        }
    });

    rx
}

// WebSocket messages
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum WsMessage {
    FileList {
        files: Vec<String>,
        metadata: HashMap<String, FileMetadata>,
    },
    FileChanged {
        path: String,
        metadata: FileMetadata,
        html: String,
    },
    FileAdded {
        path: String,
        metadata: FileMetadata,
    },
    FileRemoved {
        path: String,
    },
    SelectFile {
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
}

// Helper function to get file extension
fn get_extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_string()
}

// Helper function to get MIME type based on file extension
fn get_mime_type(path: &Path) -> &str {
    match get_extension(path).as_str() {
        // Web formats
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",

        // Image formats
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        "ico" => "image/x-icon",
        "heic" => "image/heic",
        "avif" => "image/avif",

        // Video formats
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "mkv" => "video/x-matroska",
        "m4v" => "video/x-m4v",

        // Audio formats
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "wma" => "audio/x-ms-wma",

        // Document formats
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "pn" => "text/plain",
        "rtf" => "application/rtf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

        // Programming languages
        "py" => "text/x-python",

        // Archive formats
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",

        _ => "application/octet-stream",
    }
}

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

    // Create shared debouncer for adaptive timing based on parse speed
    let debouncer = Arc::new(Mutex::new(debounce::ContentDebouncer::new()));

    // Create repository and app state
    let repository = Arc::new(Repository::new(dir.clone()));
    let state = AppState {
        repository: repository.clone(),
        line_trackers: Arc::new(Mutex::new(HashMap::new())),
        debouncer: debouncer.clone(),
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
        shutdown_signal = Some(start_preview_lsp_stdio(
            repository.clone(),
            debouncer.clone(),
        ));
    } else if let Some(lsp_port) = args.preview_lsp_port {
        if let Err(e) =
            start_preview_lsp_server(repository.clone(), lsp_port, debouncer.clone()).await
        {
            eprintln!("Failed to start preview LSP server: {}", e);
        }
    }

    // Create router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .route("/api/twitter-embed", get(twitter_embed_handler))
        .route("/api/speakerdeck-embed", get(speakerdeck_embed_handler))
        .route("/api/files/{*path}", get(user_files_handler))
        .route("/_next/{*path}", get(nextjs_static_handler))
        .route("/js/{*path}", get(nextjs_public_handler))
        .route("/favicon.ico", get(favicon_handler))
        .fallback(get(index_handler)) // Serve SPA for all other routes
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

// Handler for the index page (Next.js app)
async fn index_handler() -> impl IntoResponse {
    match NextJsRoot::get("index.html") {
        Some(content) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Index file not found"))
            .unwrap(),
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
            let mime_type = get_mime_type(&canonical_file);
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

// Handler for Next.js static assets
async fn nextjs_static_handler(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
    match NextJsAssets::get(&path) {
        Some(content) => {
            let content_type = get_content_type_from_path(&path);

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(content.data))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Next.js asset not found"))
            .unwrap(),
    }
}

// Handler for Next.js public directory files (like /js/idiomorph.min.js)
async fn nextjs_public_handler(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
    // The path comes as "idiomorph.min.js" from "/js/idiomorph.min.js" route
    let full_path = format!("js/{}", path);

    // Try to get the file from the NextJsRoot embedded files
    match NextJsRoot::get(&full_path) {
        Some(content) => {
            let content_type = get_content_type_from_path(&path);

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(content.data))
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(format!("Public file not found: {}", full_path)))
            .unwrap(),
    }
}

// Handler for favicon
async fn favicon_handler() -> impl IntoResponse {
    match NextJsRoot::get("favicon.ico") {
        Some(content) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/x-icon")
            .body(Body::from(content.data))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Favicon not found"))
            .unwrap(),
    }
}

// Helper function to determine content type from path
fn get_content_type_from_path(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    }
}

// WebSocket handler
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        handle_socket(socket, state).await;
    })
}

// Handle WebSocket connection
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    eprintln!("WebSocket client connected");

    // Subscribe to broadcast channel
    let mut rx = state.repository.subscribe();

    // Send initial file list immediately
    let mut file_paths = Vec::new();
    let mut file_metadata = HashMap::new();

    // Collect files synchronously to avoid spawning tasks
    if state.repository.root_dir.is_dir() {
        state.repository.collect_patto_files_with_metadata(
            &state.repository.root_dir,
            &mut file_paths,
            &mut file_metadata,
        );
    }

    // Send initial file list
    let message = WsMessage::FileList {
        files: file_paths,
        metadata: file_metadata,
    };

    if let Ok(json) = serde_json::to_string(&message) {
        if let Err(e) = socket
            .send(axum::extract::ws::Message::Text(json.into()))
            .await
        {
            eprintln!("Error sending initial file list: {}", e);
            return;
        }
    }
    //let root_dir = state.repository.root_dir.clone();
    // Main loop - handle both broadcast messages and websocket messages
    loop {
        tokio::select! {
            // Handle broadcast messages
            msg = rx.recv() => {
                match msg {
                    Ok(msg) => {
                        let ws_msg = match msg {
                            RepositoryMessage::FileChanged(path, metadata, content) => {
                                let Ok(html) =
                                    render_patto_to_html(&content, &path.to_string_lossy(), &state).await else {
                                        continue;
                                };

                                let Ok(rel_path) = path.strip_prefix(&state.repository.root_dir) else {
                                    continue;
                                };
                                WsMessage::FileChanged {
                                    path: rel_path.to_string_lossy().to_string(),
                                    metadata,
                                    html,
                                }
                            },
                            //RepositoryMessage::FileList(files) => {
                            //    WsMessage::FileList {
                            //        files: files.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                            //        metadata: HashMap::new(), // Empty metadata for now since FileList isn't used
                            //    }
                            //},
                            RepositoryMessage::FileAdded(path, metadata) => {
                                let Ok(rel_path) = path.strip_prefix(&state.repository.root_dir) else {
                                    continue;
                                };
                                WsMessage::FileAdded {
                                    path: rel_path.to_string_lossy().to_string(),
                                    metadata,
                                }
                            },
                            RepositoryMessage::FileRemoved(path) => {
                                // Note: path is already relative (stripped in repository.rs)
                                WsMessage::FileRemoved {
                                    path: path.to_string_lossy().to_string(),
                                }
                            },
                            RepositoryMessage::BackLinksChanged(path, back_links) => {
                                let Ok(rel_path) = path.strip_prefix(&state.repository.root_dir) else {
                                    continue;
                                };
                                WsMessage::BackLinksData {
                                    path: rel_path.to_string_lossy().to_string(),
                                    back_links,
                                }
                            },
                            RepositoryMessage::TwoHopLinksChanged(path, two_hop_links) => {
                                let Ok(rel_path) = path.strip_prefix(&state.repository.root_dir) else {
                                    continue;
                                };
                                WsMessage::TwoHopLinksData {
                                    path: rel_path.to_string_lossy().to_string(),
                                    two_hop_links,
                                }
                            }
                            // Ignore scan progress messages in preview
                            RepositoryMessage::ScanStarted { .. } |
                            RepositoryMessage::ScanProgress { .. } |
                            RepositoryMessage::ScanCompleted { .. } => {
                                continue;
                            }
                        };

                        if let Ok(json) = serde_json::to_string(&ws_msg) {
                            if let Err(e) = socket.send(axum::extract::ws::Message::Text(json.into())).await {
                                eprintln!("Error sending WebSocket message: {e}");
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Error receiving broadcast: {e}");
                        continue;
                    }
                }
            },

            // Handle WebSocket messages
            msg = socket.recv() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        if let Ok(WsMessage::SelectFile { path }) = serde_json::from_str(&text) {
                            eprintln!("Client selected file: {}", path);

                            // Load and render the selected file
                            let file_path = state.repository.root_dir.join(&path);
                            if let Ok(content) = fs::read_to_string(&file_path).await {
                                //TODO add function to retrieve metadata in crate::Repository
                                let metadata = state.repository.collect_file_metadata(&file_path).unwrap();

                                if let Ok(html) = render_patto_to_html(&content, &file_path.to_string_lossy(), &state).await {
                                    // Send the rendered HTML to the client
                                    let message = WsMessage::FileChanged {
                                        path: path.clone(),
                                        metadata,
                                        html,
                                    };

                                    if let Ok(json) = serde_json::to_string(&message) {
                                        if let Err(e) = socket.send(axum::extract::ws::Message::Text(json.into())).await {
                                            eprintln!("Error sending file content: {}", e);
                                        }
                                    }

                                    // Calculate and send back-links
                                    let back_links = state.repository.calculate_back_links(&file_path);
                                    let back_links_message = WsMessage::BackLinksData {
                                        path: path.clone(),
                                        back_links,
                                    };

                                    if let Ok(json) = serde_json::to_string(&back_links_message) {
                                        if let Err(e) = socket.send(axum::extract::ws::Message::Text(json.into())).await {
                                            eprintln!("Error sending back-links: {}", e);
                                        }
                                    }

                                    // Calculate and send two-hop links
                                    let two_hop_links = state.repository.calculate_two_hop_links(&file_path).await;
                                    let two_hop_message = WsMessage::TwoHopLinksData {
                                        path: path.clone(),
                                        two_hop_links,
                                    };

                                    if let Ok(json) = serde_json::to_string(&two_hop_message) {
                                        if let Err(e) = socket.send(axum::extract::ws::Message::Text(json.into())).await {
                                            eprintln!("Error sending two-hop links: {}", e);
                                        }
                                    }
                                } else {
                                    eprintln!("Error rendering file: {}", path);
                                }
                            } else {
                                eprintln!("Error reading file: {}", file_path.display());
                            }
                        }
                    },
                    Some(Ok(_)) => { /* Ignore other message types */ },
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    },
                    None => {
                        eprintln!("WebSocket client disconnected");
                        break;
                    }
                }
            }
        }
    }
}

// Render patto content to HTML with persistent line tracking
async fn render_patto_to_html(
    content: &str,
    file_path: &str,
    state: &AppState,
) -> std::io::Result<String> {
    // Use Arc to avoid cloning large content
    let content = std::sync::Arc::new(content.to_string());
    let file_path_buf = PathBuf::from(file_path);

    // Get or create line tracker for this file
    let line_trackers = Arc::clone(&state.line_trackers);
    let debouncer = Arc::clone(&state.debouncer);
    let file_path_for_debounce = file_path_buf.clone();

    let html_output = tokio::task::spawn_blocking(move || {
        let start = Instant::now();

        // Get or create line tracker for this file
        let mut trackers = line_trackers.lock().unwrap();
        let line_tracker = trackers.entry(file_path_buf.clone()).or_insert_with(|| {
            LineTracker::new().unwrap_or_else(|_| {
                panic!();
            })
        });

        let result = parser::parse_text_with_persistent_line_tracking(&content, line_tracker);

        // Pre-allocate buffer with estimated size to reduce reallocations
        let estimated_size = content.len() * 2; // HTML is typically 2x larger than source
        let mut html_output = Vec::with_capacity(estimated_size);

        let renderer = HtmlRenderer::new(HtmlRendererOptions {});

        let _ = renderer.format(&result.ast, &mut html_output);

        // Record parse time for adaptive debounce
        let parse_time_ms = start.elapsed().as_millis() as u64;
        debouncer
            .lock()
            .unwrap()
            .record_parse_time(&file_path_for_debounce, parse_time_ms);

        html_output
    })
    .await;

    match html_output {
        Ok(output) => Ok(String::from_utf8(output)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?),
        Err(e) => Err(std::io::Error::other(e)),
    }
}
