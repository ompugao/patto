mod app;
mod backlinks;
mod config;
mod image_cache;
mod math_render;
mod search;
mod tasks;
mod ui;
mod wrap;

use clap::Parser;
use crossterm::{
    event::{Event, EventStream, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use patto::repository::{Repository, RepositoryMessage};
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tower_lsp::{jsonrpc, Client, LanguageServer, LspService, Server};
use url::Url;

use app::App;

#[derive(Parser, Debug)]
#[command(author, version, about = "Terminal preview for .pn (patto) files")]
struct Args {
    /// Path to the .pn file to preview
    file: String,

    /// Workspace directory (defaults to file's parent directory)
    #[arg(short, long)]
    dir: Option<String>,

    /// Force a specific image protocol (kitty, iterm2, sixel, halfblocks).
    /// Overrides auto-detection. Useful when running inside tmux, over SSH,
    /// or when auto-detection silently falls back to halfblocks.
    #[arg(short = 'p', long, value_name = "PROTOCOL")]
    protocol: Option<String>,

    /// Start with word-wrap disabled (wrap is on by default; press 'w' to toggle)
    #[arg(long)]
    no_wrap: bool,

    /// String prepended to continuation rows when wrap is on (vim showbreak).
    /// Default: "↪ ". Set to "" to disable.
    #[arg(long, default_value = "↪ ")]
    showbreak: String,

    /// TCP port for the preview LSP bridge (enabled by default)
    #[arg(long, default_value_t = 9527)]
    lsp_port: u16,

    /// Jump to this line on startup (1-indexed). Useful when launched from an editor.
    #[arg(short = 'g', long, value_name = "LINE")]
    goto_line: Option<usize>,
}

struct PreviewLspBackend {
    client: Client,
    repository: Arc<Repository>,
}

impl PreviewLspBackend {
    fn new(client: Client, repository: Arc<Repository>) -> Self {
        Self { client, repository }
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

        let path = std::fs::canonicalize(&path).unwrap_or(path);

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

        self.repository.handle_live_file_change(path, text).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for PreviewLspBackend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(
                            tower_lsp::lsp_types::TextDocumentSyncSaveOptions::Supported(true),
                        ),
                    },
                )),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Preview TUI LSP bridge connected")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
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

async fn start_preview_lsp_server(repository: Arc<Repository>, port: u16) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    eprintln!("Preview TUI LSP server listening on 127.0.0.1:{}", port);

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let repo = repository.clone();
                    tokio::spawn(async move {
                        let (reader, writer) = tokio::io::split(stream);
                        let (service, socket) =
                            LspService::new(|client| PreviewLspBackend::new(client, repo.clone()));
                        Server::new(reader, writer, socket).serve(service).await;
                        //eprintln!("Preview TUI LSP connection {} closed", addr);
                    });
                }
                Err(_err) => {
                    //eprintln!("Preview TUI LSP accept error: {err}");
                }
            }
        }
    });

    Ok(())
}

/// Build the shell command string from the editor config, substituting `{file}`, `{line}`,
/// and `{top_line}`.
///
/// - `{line}`     — source line of the focused item (Tab-selected), or `top_line` if nothing focused.
/// - `{top_line}` — first visible source line of the viewport.
pub(crate) fn build_editor_cmd(
    editor: &config::EditorConfig,
    file: &str,
    line: usize,
    top_line: usize,
) -> String {
    let template = editor.cmd.as_deref().unwrap_or("");

    if template.is_empty() {
        // Fall back to $EDITOR or $VISUAL
        let editor_bin = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| "vi".to_string());
        return format!("{} +{} \"{}\"", editor_bin, line, file);
    }

    template
        .replace("{file}", file)
        .replace("{line}", &line.to_string())
        .replace("{top_line}", &top_line.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let file_path = std::fs::canonicalize(PathBuf::from(&args.file)).unwrap_or_else(|_| {
        eprintln!("Cannot find file: {}", args.file);
        std::process::exit(1);
    });

    if !file_path.exists() || !file_path.is_file() {
        eprintln!("Not a file: {}", file_path.display());
        std::process::exit(1);
    }

    let dir = if let Some(d) = &args.dir {
        std::fs::canonicalize(PathBuf::from(d)).unwrap_or_else(|_| {
            eprintln!("Cannot find directory: {}", d);
            std::process::exit(1);
        })
    } else {
        file_path
            .parent()
            .expect("File must have a parent directory")
            .to_path_buf()
    };

    // Create repository
    let repository = Arc::new(Repository::new(dir.clone()));
    let mut rx = repository.subscribe();

    // Start file watcher
    let repository_clone = repository.clone();
    tokio::spawn(async move {
        if let Err(e) = repository_clone.start_watcher().await {
            eprintln!("Failed to start file watcher: {}", e);
        }
    });

    // Start preview LSP server
    start_preview_lsp_server(repository.clone(), args.lsp_port).await?;

    // Read initial content
    let initial_content = std::fs::read_to_string(&file_path)?;

    // Set up app
    let mut app = App::new(file_path.clone(), dir.clone(), args.protocol.as_deref());
    if args.no_wrap {
        app.wrap = false;
    }
    app.showbreak = args.showbreak.clone();
    let tui_config = config::TuiConfig::load();
    app.syntax_theme = tui_config.syntax_theme.clone();
    app.images.background_color = tui_config.image_background.to_rgb();
    app.tui_config = tui_config;
    app.re_render(&initial_content);

    // Query the terminal size now (crossterm works without raw mode) so that
    // wrap-aware element heights are correct when computing the initial scroll
    // position. Without this, viewport_width = 0, wrap is effectively off, and
    // scroll_to_source_line counts every element as height 1 — causing up to
    // ~30-line drift when there are long wrapped lines above the target.
    if let Ok((cols, rows)) = crossterm::terminal::size() {
        app.viewport_width = cols;
        // Content area is rows minus title bar (1) and status bar (1).
        app.viewport_height = (rows as usize).saturating_sub(2);
    }

    // Jump to the requested line if --goto-line was supplied
    if let Some(line) = args.goto_line {
        app.scroll_to_source_line(line);
    }

    // Compute initial backlinks
    app.backlinks.refresh(&repository, &app.file_path).await;

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut event_stream = EventStream::new();

    // Main loop
    loop {
        terminal.draw(|f| ui::draw(f, &mut app, &dir))?;

        tokio::select! {
            event = event_stream.next() => {
                match event {
                    Some(Ok(Event::Key(KeyEvent { code, modifiers, .. }))) => {
                        let vh = terminal.size()?.height as usize;
                        let action = app
                            .handle_key(&repository, code, modifiers, vh)
                            .await;
                        use app::AppAction;
                        match action {
                            AppAction::Quit => break,
                            AppAction::LaunchEditor { cmd, action } => {
                                use config::EditorAction;
                                match action {
                                    EditorAction::Suspend => {
                                        disable_raw_mode()?;
                                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                        let _ = std::process::Command::new("sh")
                                            .arg("-c")
                                            .arg(&cmd)
                                            .spawn()
                                            .and_then(|mut c| c.wait());
                                        enable_raw_mode()?;
                                        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                        terminal.clear()?;
                                    }
                                    EditorAction::Quit => {
                                        disable_raw_mode()?;
                                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                        terminal.show_cursor()?;
                                        let _ = std::process::Command::new("sh")
                                            .arg("-c")
                                            .arg(&cmd)
                                            .spawn()
                                            .and_then(|mut c| c.wait());
                                        std::process::exit(0);
                                    }
                                    EditorAction::Background => {
                                        let _ = tokio::process::Command::new("sh")
                                            .arg("-c")
                                            .arg(&cmd)
                                            .spawn();
                                    }
                                }
                            }
                            AppAction::None => {}
                        }
                    }
                    Some(Ok(Event::Resize(_, _))) => {
                        // Terminal resized — redraw handled by the loop
                    }
                    Some(Err(_)) => break,
                    None => break,
                    _ => {}
                }
            }
            msg = rx.recv() => {
                match msg {
                    Ok(RepositoryMessage::FileChanged(path, _metadata, content)) => {
                        if path == app.file_path {
                            app.re_render(&content);
                            app.backlinks.refresh(&repository, &app.file_path).await;
                        }
                    }
                    Ok(_) => {
                        // Other messages: ignore for single-file mode
                    }
                    Err(_) => {
                        // Channel lagged or closed
                    }
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Force exit to stop background file watcher task
    std::process::exit(0);
}
