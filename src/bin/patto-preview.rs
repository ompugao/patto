use axum::{
    extract::{State, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use axum::extract::ws::WebSocket;
use clap::Parser;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use patto::{
    parser,
    renderer::{HtmlRenderer, Options, Renderer}
};
use std::path::{Path, PathBuf};
use tokio::{
    fs,
    sync::broadcast,
};
use serde::{Serialize, Deserialize};

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
}

// Messages for the broadcast channel
#[derive(Clone)]
enum Message {
    FileChanged(PathBuf, String),
    FileList(Vec<PathBuf>),
}

// WebSocket messages
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum WsMessage {
    FileList { files: Vec<String> },
    FileChanged { path: String, html: String },
    SelectFile { path: String },
}

// Helper function to get file extension
fn get_extension(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_string()
}

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args = Args::parse();
    let dir = PathBuf::from(&args.dir);

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
        .with_state(state);

    // Start server
    println!("Starting server at http://localhost:{}", args.port);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handler for the index page
async fn index_handler() -> Html<String> {
    Html(include_str!("../../static/index.html").to_string())
}

// WebSocket handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
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

    // Collect files synchronously to avoid spawning tasks
    if dir.is_dir() {
        collect_patto_files(&dir, &dir, &mut file_paths);
    }

    // Send initial file list
    let message = WsMessage::FileList {
        files: file_paths,
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
                                if let Ok(html) = render_patto_to_html(&content) {
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

// Helper function to collect patto files
fn collect_patto_files(dir: &Path, base_dir: &Path, files: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                collect_patto_files(&path, base_dir, files);
            } else if get_extension(&path) == "pn" {
                if let Ok(rel_path) = path.strip_prefix(base_dir) {
                    files.push(rel_path.to_string_lossy().to_string());
                }
            }
        }
    }
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
        watcher
            .watch(&watch_dir, RecursiveMode::Recursive)
            .unwrap();

        // Keep the watcher alive
        std::thread::park();
    });

    println!("Watching directory: {}", dir_display);

    // Process events from the channel
    while let Some(event) = rx.recv().await {
        for path in event.paths {
            if !path.is_file() || get_extension(&path) != "pn" {
                continue;
            }

            // Process file change using the state that wasn't moved
            if let Err(e) = process_file_change(&state, &path).await {
                eprintln!("Error processing file change: {}", e);
            }
        }
    }
}

// Process a file change
async fn process_file_change(state: &AppState, path: &Path) -> std::io::Result<()> {
    println!("File changed: {}", path.display());

    // Read file contents
    let content = fs::read_to_string(path).await?;

    // Parse and render to HTML
    let html = render_patto_to_html(&content)?;

    // Generate relative path
    let rel_path = path.strip_prefix(&state.dir).unwrap_or(path);

    // Broadcast change
    let _ = state.tx.send(Message::FileChanged(rel_path.to_path_buf(), html));

    Ok(())
}

// Render patto content to HTML
fn render_patto_to_html(content: &str) -> std::io::Result<String> {
    let result = parser::parse_text(content);
    let mut html_output = Vec::new();

    let renderer = HtmlRenderer::new(Options {
        theme: "light".to_string(),
    });

    renderer.format(&result.ast, &mut html_output)?;

    Ok(String::from_utf8(html_output).unwrap())
}