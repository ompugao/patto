mod app;
mod backlinks;
mod config;
mod image_cache;
mod math_render;
mod search;
mod syntax_highlight;
mod tasks;
mod tui_renderer;
mod ui;
mod wrap;

use clap::Parser;
use crossterm::{
    event::{Event, EventStream, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use patto::preview::lsp_bridge::{self, BridgeOptions};
use patto::repository::{Repository, RepositoryMessage};
use ratatui::Terminal;
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;

use app::{App, AppAction};
use config::EditorAction;

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
    let (file_path, dir) = resolve_paths(&args);

    let repository = Arc::new(Repository::new(dir.clone()));
    // Subscribe before scanning, so no scan progress is missed.
    let repository_messages = repository.subscribe();
    repository.spawn_initial_scan();

    let watched = repository.clone();
    tokio::spawn(async move {
        if let Err(err) = watched.start_watcher().await {
            eprintln!("Failed to start file watcher: {err}");
        }
    });

    lsp_bridge::serve_tcp(repository.clone(), args.lsp_port, BridgeOptions::default()).await?;

    let mut app = build_app(&args, &file_path, &dir)?;
    app.backlinks.refresh(&repository, &app.file_path).await;
    app.tasks.refresh(&repository);

    let mut terminal = enter_terminal()?;
    let result = run(
        &mut terminal,
        &mut app,
        &repository,
        repository_messages,
        &dir,
    )
    .await;
    leave_terminal(&mut terminal)?;
    result?;

    // Exit rather than return, to stop the background watcher task.
    std::process::exit(0);
}

fn resolve_paths(args: &Args) -> (PathBuf, PathBuf) {
    let file_path = std::fs::canonicalize(PathBuf::from(&args.file)).unwrap_or_else(|_| {
        eprintln!("Cannot find file: {}", args.file);
        std::process::exit(1);
    });

    if !file_path.is_file() {
        eprintln!("Not a file: {}", file_path.display());
        std::process::exit(1);
    }

    let dir = match &args.dir {
        Some(dir) => std::fs::canonicalize(PathBuf::from(dir)).unwrap_or_else(|_| {
            eprintln!("Cannot find directory: {}", dir);
            std::process::exit(1);
        }),
        None => file_path
            .parent()
            .expect("File must have a parent directory")
            .to_path_buf(),
    };

    (file_path, dir)
}

fn build_app(args: &Args, file_path: &Path, dir: &Path) -> anyhow::Result<App> {
    let mut app = App::new(
        file_path.to_path_buf(),
        dir.to_path_buf(),
        args.protocol.as_deref(),
    );
    if args.no_wrap {
        app.wrap = false;
    }
    app.showbreak = args.showbreak.clone();

    let tui_config = config::TuiConfig::load();
    app.syntax_theme = tui_config.syntax_theme.clone();
    app.images.background_color = tui_config.image_background.to_rgb();
    app.tui_config = tui_config;

    app.re_render(&std::fs::read_to_string(file_path)?);

    // Ask the terminal for its size before raw mode (crossterm allows this), so
    // wrap-aware element heights are right for the initial scroll position.
    // Without it `viewport_width` is 0, wrapping is effectively off, and
    // `scroll_to_source_line` counts every element as one row — drifting by up
    // to ~30 lines when long wrapped lines sit above the target.
    if let Ok((cols, rows)) = crossterm::terminal::size() {
        app.viewport_width = cols;
        // The content area is the terminal minus the title and status bars.
        app.viewport_height = (rows as usize).saturating_sub(2);
    }

    if let Some(line) = args.goto_line {
        app.scroll_to_source_line(line);
    }

    Ok(app)
}

type Tui = Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>;

fn enter_terminal() -> anyhow::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(ratatui::backend::CrosstermBackend::new(
        stdout,
    ))?)
}

fn leave_terminal(terminal: &mut Tui) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn run(
    terminal: &mut Tui,
    app: &mut App,
    repository: &Arc<Repository>,
    mut repository_messages: broadcast::Receiver<RepositoryMessage>,
    dir: &Path,
) -> anyhow::Result<()> {
    let mut events = EventStream::new();

    // Live elapsed time on Doing tasks is recomputed while rendering, so waking
    // the loop once a minute is enough to keep it current. Mirrors
    // `display_interval` in current_task.lua.
    let mut display_tick = tokio::time::interval(std::time::Duration::from_secs(60));
    display_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        terminal.draw(|frame| ui::draw(frame, app, dir))?;

        tokio::select! {
            event = events.next() => {
                match event {
                    Some(Ok(Event::Key(KeyEvent { code, modifiers, .. }))) => {
                        let height = terminal.size()?.height as usize;
                        let action = app.handle_key(repository, code, modifiers, height).await;
                        if handle_action(terminal, action)? == ControlFlow::Break(()) {
                            return Ok(());
                        }
                    }
                    // A resize needs no work: the next loop iteration redraws.
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => return Ok(()),
                }
            }
            message = repository_messages.recv() => {
                if let Ok(message) = message {
                    apply_repository_message(app, repository, message).await;
                }
            }
            _ = display_tick.tick() => {}
        }
    }
}

fn handle_action(terminal: &mut Tui, action: AppAction) -> anyhow::Result<ControlFlow<()>> {
    match action {
        AppAction::None => {}
        AppAction::Quit => return Ok(ControlFlow::Break(())),
        // Suspend and Quit hand the terminal to the editor, so raw mode and the
        // alternate screen have to be given up first.
        AppAction::LaunchEditor { cmd, action } => match action {
            EditorAction::Suspend => {
                leave_terminal(terminal)?;
                run_shell(&cmd);
                enable_raw_mode()?;
                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                terminal.clear()?;
            }
            EditorAction::Quit => {
                leave_terminal(terminal)?;
                run_shell(&cmd);
                std::process::exit(0);
            }
            EditorAction::Background => {
                let _ = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .spawn();
            }
        },
    }
    Ok(ControlFlow::Continue(()))
}

/// Run `cmd` and wait for it, with the terminal handed back to it.
fn run_shell(cmd: &str) {
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .spawn()
        .and_then(|mut child| child.wait());
}

async fn apply_repository_message(
    app: &mut App,
    repository: &Arc<Repository>,
    message: RepositoryMessage,
) {
    match message {
        RepositoryMessage::FileChanged(path, _, content) if path == app.file_path => {
            app.re_render(&content);
            app.backlinks.refresh(repository, &app.file_path).await;
            app.tasks.refresh(repository);
        }
        // The initial scan finished, or a new note appeared: the active-task
        // overlay covers the whole workspace, so its cache needs rebuilding.
        RepositoryMessage::ScanCompleted { .. } | RepositoryMessage::FileAdded(..) => {
            app.tasks.refresh(repository);
        }
        _ => {}
    }
}
