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
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use patto::{
    line_tracker::LineTracker,
    parser,
    renderer::{HtmlRenderer, HtmlRendererOptions, Renderer},
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::{fs, sync::broadcast, time::sleep};

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
}

// App state
#[derive(Clone)]
struct AppState {
    dir: PathBuf,
    tx: broadcast::Sender<Message>,
    line_trackers: Arc<Mutex<HashMap<PathBuf, LineTracker>>>,
    // Store file graphs for two-hop links: file -> files it links to
    graph: Arc<Mutex<HashMap<PathBuf, HashSet<PathBuf>>>>,
}

// Messages for the broadcast channel
#[derive(Clone)]
enum Message {
    FileChanged(PathBuf, String),
    #[allow(dead_code)]
    FileList(Vec<PathBuf>),
    FileAdded(PathBuf, FileMetadata),
    FileRemoved(PathBuf),
    BackLinksChanged(PathBuf, Vec<String>),
    TwoHopLinksChanged(PathBuf, Vec<(String, Vec<String>)>),
}

// File metadata for sorting
#[derive(Serialize, Deserialize, Clone)]
struct FileMetadata {
    modified: u64, // Unix timestamp
    created: u64,  // Unix timestamp
    #[serde(rename = "linkCount")]
    link_count: u32,
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
        back_links: Vec<String>,
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

    // Create broadcast channel for file changes
    let (tx, _) = broadcast::channel(100);

    // Create app state
    let state = AppState {
        dir: dir.clone(),
        tx: tx.clone(),
        line_trackers: Arc::new(Mutex::new(HashMap::new())),
        graph: Arc::new(Mutex::new(HashMap::new())),
    };

    // Start file watcher in a separate task
    let watcher_state = state.clone();
    tokio::spawn(async move {
        watch_files(watcher_state).await;
    });

    // Create router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .route("/api/twitter-embed", get(twitter_embed_handler))
        .route("/api/speakerdeck-embed", get(speakerdeck_embed_handler))
        .route("/api/files/*path", get(user_files_handler))
        .route("/_next/*path", get(nextjs_static_handler))
        .route("/js/*path", get(nextjs_public_handler))
        .route("/favicon.ico", get(favicon_handler))
        .fallback(get(index_handler)) // Serve SPA for all other routes
        .with_state(state);

    // Start server
    println!("Starting server at http://localhost:{}", args.port);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
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
    let file_path = state.dir.join(decoded_path.as_ref());

    // Security check - ensure the path doesn't escape the base directory
    let canonical_base = match std::fs::canonicalize(&state.dir) {
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
    println!("WebSocket client connected");

    // Subscribe to broadcast channel
    let mut rx = state.tx.subscribe();

    // Send initial file list immediately
    let dir = state.dir.clone();
    let mut file_paths = Vec::new();
    let mut file_metadata = HashMap::new();

    // Collect files synchronously to avoid spawning tasks
    if dir.is_dir() {
        collect_patto_files_with_metadata(&dir, &dir, &mut file_paths, &mut file_metadata);
    }

    // Send initial file list
    let message = WsMessage::FileList {
        files: file_paths,
        metadata: file_metadata,
    };

    if let Ok(json) = serde_json::to_string(&message) {
        if let Err(e) = socket.send(axum::extract::ws::Message::Text(json)).await {
            eprintln!("Error sending initial file list: {}", e);
            return;
        }
    }

    // Main loop - handle both broadcast messages and websocket messages
    loop {
        tokio::select! {
            // Handle broadcast messages
            msg = rx.recv() => {
                match msg {
                    Ok(msg) => {
                        let ws_msg = match msg {
                            Message::FileChanged(path, html) => {
                                WsMessage::FileChanged {
                                    path: path.to_string_lossy().to_string(),
                                    html,
                                }
                            },
                            Message::FileList(files) => {
                                WsMessage::FileList {
                                    files: files.iter().map(|p| p.to_string_lossy().to_string()).collect(),
                                    metadata: HashMap::new(), // Empty metadata for now since FileList isn't used
                                }
                            },
                            Message::FileAdded(path, metadata) => {
                                WsMessage::FileAdded {
                                    path: path.to_string_lossy().to_string(),
                                    metadata,
                                }
                            },
                            Message::FileRemoved(path) => {
                                WsMessage::FileRemoved {
                                    path: path.to_string_lossy().to_string(),
                                }
                            },
                            Message::BackLinksChanged(path, back_links) => {
                                WsMessage::BackLinksData {
                                    path: path.to_string_lossy().to_string(),
                                    back_links,
                                }
                            },
                            Message::TwoHopLinksChanged(path, two_hop_links) => {
                                WsMessage::TwoHopLinksData {
                                    path: path.to_string_lossy().to_string(),
                                    two_hop_links,
                                }
                            }
                        };

                        if let Ok(json) = serde_json::to_string(&ws_msg) {
                            if let Err(e) = socket.send(axum::extract::ws::Message::Text(json)).await {
                                eprintln!("Error sending WebSocket message: {}", e);
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Error receiving broadcast: {}", e);
                        break;
                    }
                }
            },

            // Handle WebSocket messages
            msg = socket.recv() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        if let Ok(WsMessage::SelectFile { path }) = serde_json::from_str(&text) {
                            println!("Client selected file: {}", path);

                            // Load and render the selected file
                            let file_path = state.dir.join(&path);
                            if let Ok(content) = fs::read_to_string(&file_path).await {
                                if let Ok(html) = render_patto_to_html(&content, &file_path.to_string_lossy(), &state).await {
                                    // Send the rendered HTML to the client
                                    let message = WsMessage::FileChanged {
                                        path: path.clone(),
                                        html,
                                    };

                                    if let Ok(json) = serde_json::to_string(&message) {
                                        if let Err(e) = socket.send(axum::extract::ws::Message::Text(json)).await {
                                            eprintln!("Error sending file content: {}", e);
                                        }
                                    }

                                    // Calculate and send back-links
                                    let back_links = calculate_back_links(&file_path, &state).await;
                                    let back_links_message = WsMessage::BackLinksData {
                                        path: path.clone(),
                                        back_links,
                                    };

                                    if let Ok(json) = serde_json::to_string(&back_links_message) {
                                        if let Err(e) = socket.send(axum::extract::ws::Message::Text(json)).await {
                                            eprintln!("Error sending back-links: {}", e);
                                        }
                                    }

                                    // Calculate and send two-hop links
                                    let two_hop_links = calculate_two_hop_links(&file_path, &state).await;
                                    let two_hop_message = WsMessage::TwoHopLinksData {
                                        path: path.clone(),
                                        two_hop_links,
                                    };

                                    if let Ok(json) = serde_json::to_string(&two_hop_message) {
                                        if let Err(e) = socket.send(axum::extract::ws::Message::Text(json)).await {
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
                        println!("WebSocket client disconnected");
                        break;
                    }
                }
            }
        }
    }
}

// Helper function to collect patto files with metadata
fn collect_patto_files_with_metadata(
    dir: &Path,
    base_dir: &Path,
    files: &mut Vec<String>,
    metadata: &mut HashMap<String, FileMetadata>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                collect_patto_files_with_metadata(&path, base_dir, files, metadata);
            } else if get_extension(&path) == "pn" {
                if let Ok(rel_path) = path.strip_prefix(base_dir) {
                    let rel_path_str = rel_path.to_string_lossy().to_string();
                    files.push(rel_path_str.clone());

                    // Collect file metadata
                    if let Ok(file_metadata) = std::fs::metadata(&path) {
                        let modified = file_metadata
                            .modified()
                            .unwrap_or(SystemTime::UNIX_EPOCH)
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        let created = file_metadata
                            .created()
                            .unwrap_or(SystemTime::UNIX_EPOCH)
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        // Count links by reading file content and parsing
                        let link_count = count_links_in_file(&path).unwrap_or(0);

                        metadata.insert(
                            rel_path_str,
                            FileMetadata {
                                modified,
                                created,
                                link_count,
                            },
                        );
                    }
                }
            }
        }
    }
}

// Count links in a patto file using the parser
fn count_links_in_file(path: &Path) -> std::io::Result<u32> {
    let content = std::fs::read_to_string(path)?;
    let result = parser::parse_text(&content);
    let mut wikilinks = vec![];
    gather_wikilinks(&result.ast, &mut wikilinks);
    Ok(wikilinks.len() as u32)
}

// Helper function to gather wikilinks from AST (from patto-lsp.rs)
fn gather_wikilinks(parent: &parser::AstNode, wikilinks: &mut Vec<(String, Option<String>)>) {
    if let parser::AstNodeKind::WikiLink { link, anchor } = &parent.kind() {
        wikilinks.push((link.clone(), anchor.clone()));
    }

    for content in parent.value().contents.lock().unwrap().iter() {
        gather_wikilinks(content, wikilinks);
    }

    for child in parent.value().children.lock().unwrap().iter() {
        gather_wikilinks(child, wikilinks);
    }
}

// Convert link name to file path
fn link_to_path(link: &str, base_dir: &Path) -> Option<PathBuf> {
    if !link.is_empty() {
        let file_path = base_dir.join(format!("{}.pn", link));
        if file_path.exists() {
            Some(file_path)
        } else {
            None
        }
    } else {
        None
    }
}

// Convert file path to link name
fn path_to_link(path: &Path, base_dir: &Path) -> Option<String> {
    if let Ok(rel_path) = path.strip_prefix(base_dir) {
        if let Some(stem) = rel_path.file_stem() {
            if let Some(stem_str) = stem.to_str() {
                return Some(stem_str.to_string());
            }
        }
        // if let Some(rel_path_str) = rel_path.to_str() {
        //     return Some(rel_path_str.to_string());
        // }
    }
    None
}

// Build file link graph for two-hop analysis (initial scan)
fn build_link_graph(state: &AppState) -> HashMap<PathBuf, HashSet<PathBuf>> {
    let mut graph = HashMap::new();

    // Scan all .pn files in the directory
    fn scan_dir_for_links(
        dir: &Path,
        base_dir: &Path,
        graph: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_dir_for_links(&path, base_dir, graph);
                } else if path.extension().and_then(|s| s.to_str()) == Some("pn") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let result = parser::parse_text(&content);
                        let mut wikilinks = vec![];
                        gather_wikilinks(&result.ast, &mut wikilinks);

                        let linked_paths: HashSet<PathBuf> = wikilinks
                            .into_iter()
                            .filter_map(|(link, _)| link_to_path(&link, base_dir))
                            .collect();

                        graph.insert(path, linked_paths);
                    }
                }
            }
        }
    }

    scan_dir_for_links(&state.dir, &state.dir, &mut graph);
    graph
}

// Update only the specific file's links in the graph
fn update_links_in_graph(
    file_path: &Path,
    content: &str,
    base_dir: &Path,
    graph: &mut HashMap<PathBuf, HashSet<PathBuf>>,
) {
    // Parse the file and extract its links
    let result = parser::parse_text(content);
    let mut wikilinks = vec![];
    gather_wikilinks(&result.ast, &mut wikilinks);

    let linked_paths: HashSet<PathBuf> = wikilinks
        .into_iter()
        .filter_map(|(link, _)| link_to_path(&link, base_dir))
        .collect();

    // Update the graph with this file's new links
    graph.insert(file_path.to_path_buf(), linked_paths);
}


// Calculate back-links for a specific file
async fn calculate_back_links(file_path: &Path, state: &AppState) -> Vec<String> {
    // Security check
    let canonical_base = match std::fs::canonicalize(&state.dir) {
        Ok(base) => base,
        Err(_) => return Vec::new(),
    };

    let canonical_file = match std::fs::canonicalize(file_path) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };

    if !canonical_file.starts_with(&canonical_base) {
        return Vec::new();
    }

    // Get the link name for the current file (what other files would reference it as)
    let _current_link_name = match path_to_link(&canonical_file, &state.dir) {
        Some(name) => name,
        None => return Vec::new(),
    };

    // Build or get cached file link graph
    let graph = {
        let mut graph = state.graph.lock().unwrap();
        if graph.is_empty() {
            *graph = build_link_graph(state);
        }
        graph.clone()
    };

    // Find all files that reference the current file
    let mut back_links: Vec<String> = Vec::new();

    for (other_file, linked_files) in &graph {
        // Check if this file links to our current file
        if linked_files.contains(&canonical_file) {
            if let Some(link_name) = path_to_link(other_file, &state.dir) {
                back_links.push(link_name);
            }
        }
    }

    // Sort alphabetically
    back_links.sort();
    back_links
}

// Calculate two-hop links for a specific file
async fn calculate_two_hop_links(file_path: &Path, state: &AppState) -> Vec<(String, Vec<String>)> {
    // Security check
    let canonical_base = match std::fs::canonicalize(&state.dir) {
        Ok(base) => base,
        Err(_) => return Vec::new(),
    };

    let canonical_file = match std::fs::canonicalize(file_path) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };

    if !canonical_file.starts_with(&canonical_base) {
        return Vec::new();
    }

    // Build or get cached file link graph
    let graph = {
        let mut graph = state.graph.lock().unwrap();
        if graph.is_empty() {
            *graph = build_link_graph(state);
        }
        graph.clone()
    };

    // Calculate two-hop links based on LSP algorithm
    let mut two_hop_links: Vec<(String, Vec<String>)> = Vec::new();

    if let Some(direct_links) = graph.get(&canonical_file) {
        for linked_file in direct_links {
            let mut connected_files = Vec::new();

            // Find files that link to the same file as our original file links to
            for (other_file, other_links) in &graph {
                if other_file != &canonical_file
                    && other_file != linked_file
                    && other_links.contains(linked_file)
                {
                    if let Some(link_name) = path_to_link(other_file, &state.dir) {
                        connected_files.push(link_name);
                    }
                }
            }

            if !connected_files.is_empty() {
                if let Some(bridge_link_name) = path_to_link(linked_file, &state.dir) {
                    connected_files.sort();
                    two_hop_links.push((bridge_link_name, connected_files));
                }
            }
        }
    }

    // Sort by number of connections (descending)
    two_hop_links.sort_by_key(|(_, connections)| -(connections.len() as i32));
    two_hop_links
}

// Watch directory for file changes
async fn watch_files(state: AppState) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    // Create a separate copy of the directory path to use in the watcher
    let watch_dir = state.dir.clone();
    let dir_display = watch_dir.display().to_string();
    let watcher_tx = tx.clone();

    // Spawn a blocking task for the file watcher WITHOUT moving state
    tokio::task::spawn_blocking(move || {
        let mut watcher = RecommendedWatcher::new(
            move |result| {
                if let Ok(event) = result {
                    let _ = watcher_tx.blocking_send(event);
                }
            },
            Config::default(),
        )
        .unwrap();

        // Use the cloned watch_dir instead of state.dir
        watcher.watch(&watch_dir, RecursiveMode::Recursive).unwrap();

        // Keep the watcher alive
        std::thread::park();
    });

    println!("Watching directory: {}", dir_display);

    // Debouncing: track last modification time for each file
    let pending_changes: Arc<Mutex<HashMap<PathBuf, Instant>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let debounce_duration = Duration::from_millis(100); // 300ms debounce

    // Process events from the channel
    while let Some(event) = rx.recv().await {
        if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
            for path in event.paths {
                let is_pn_file = get_extension(&path) == "pn";

                // Handle file creation
                if event.kind.is_create() && is_pn_file {
                    println!("File created: {}", path.display());
                    if let Ok(rel_path) = path.strip_prefix(&state.dir) {
                        // Generate metadata for the new file
                        if let Ok(file_metadata) = std::fs::metadata(&path) {
                            let modified = file_metadata
                                .modified()
                                .unwrap_or(SystemTime::UNIX_EPOCH)
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            let created = file_metadata
                                .created()
                                .unwrap_or(SystemTime::UNIX_EPOCH)
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            let link_count = count_links_in_file(&path).unwrap_or(0);

                            let metadata = FileMetadata {
                                modified,
                                created,
                                link_count,
                            };

                            let _ = state
                                .tx
                                .send(Message::FileAdded(rel_path.to_path_buf(), metadata));
                        }
                    }
                }

                // Handle file removal
                if event.kind.is_remove() && is_pn_file {
                    println!("File removed: {}", path.display());
                    if let Ok(rel_path) = path.strip_prefix(&state.dir) {
                        // Remove the file from the two-hop links graph
                        {
                            let mut graph = state.graph.lock().unwrap();
                            graph.remove(&path);
                        }

                        let _ = state.tx.send(Message::FileRemoved(rel_path.to_path_buf()));
                    }
                }

                // Handle file content changes (modify existing .pn files)
                if event.kind.is_modify() && path.is_file() && is_pn_file {
                    // Update pending changes with current time
                    {
                        let mut changes = pending_changes.lock().unwrap();
                        changes.insert(path.clone(), Instant::now());
                    }

                    // Spawn a debounced task for this file
                    let state_clone = state.clone();
                    let path_clone = path.clone();
                    let pending_changes_clone = Arc::clone(&pending_changes);

                    tokio::spawn(async move {
                        // Wait for debounce duration
                        sleep(debounce_duration).await;

                        // Check if this is still the latest change for this file
                        let should_process = {
                            let mut changes = pending_changes_clone.lock().unwrap();
                            if let Some(&last_change) = changes.get(&path_clone) {
                                let is_latest =
                                    Instant::now().duration_since(last_change) >= debounce_duration;
                                if is_latest {
                                    changes.remove(&path_clone);
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        };

                        if should_process {
                            println!("Processing debounced file change: {}", path_clone.display());
                            if let Err(e) = process_file_change(&state_clone, &path_clone).await {
                                eprintln!("Error processing file change: {}", e);
                            }
                        }
                    });
                }
            }
        }
    }
}

// Process a file change
async fn process_file_change(state: &AppState, path: &Path) -> std::io::Result<()> {
    println!("Processing file: {}", path.display());
    // Read file contents
    let content = fs::read_to_string(path).await?;

    // Parse and render to HTML
    let start = Instant::now();
    let html = render_patto_to_html(&content, &path.to_string_lossy(), state).await?;

    // Generate relative path
    let rel_path = path.strip_prefix(&state.dir).unwrap_or(path);

    // Track what files this file linked to before the change
    let old_links = {
        let graph = state.graph.lock().unwrap();
        graph.get(path).cloned().unwrap_or_default()
    };

    // Update only this file's links in the graph instead of clearing everything
    {
        let mut graph = state.graph.lock().unwrap();

        // If graph is empty, we'll build it lazily when needed
        // If graph exists, update only this file's entry
        if !graph.is_empty() {
            update_links_in_graph(path, &content, &state.dir, &mut graph);
        }
    }

    // Get the new links after the update
    let new_links = {
        let graph = state.graph.lock().unwrap();
        graph.get(path).cloned().unwrap_or_default()
    };

    // Find files that need back-links updates
    let mut files_to_update = HashSet::new();
    
    // Files that were previously linked to (might lose this as a back-link)
    for old_linked_file in &old_links {
        files_to_update.insert(old_linked_file.clone());
    }
    
    // Files that are now linked to (might gain this as a back-link)
    for new_linked_file in &new_links {
        files_to_update.insert(new_linked_file.clone());
    }

    // Send back-links updates for affected files
    for file_to_update in files_to_update {
        let back_links = calculate_back_links(&file_to_update, state).await;
        let _ = state.tx.send(Message::BackLinksChanged(file_to_update.clone(), back_links));
        
        // Also update two-hop links for affected files
        let two_hop_links = calculate_two_hop_links(&file_to_update, state).await;
        let _ = state.tx.send(Message::TwoHopLinksChanged(file_to_update, two_hop_links));
    }

    println!(
        "{} html Generated, taking {} msec in total. Sending via websocket...",
        path.display(),
        start.elapsed().as_millis()
    );

    // Broadcast change
    let _ = state
        .tx
        .send(Message::FileChanged(rel_path.to_path_buf(), html));
    println!("{} Sent", path.display());

    Ok(())
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

    let html_output = tokio::task::spawn_blocking(move || {
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

        let renderer = HtmlRenderer::new(HtmlRendererOptions {
            ..HtmlRendererOptions::default()
        });

        let _ = renderer.format(&result.ast, &mut html_output);
        html_output
    })
    .await;

    match html_output {
        Ok(output) => Ok(String::from_utf8(output)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}
