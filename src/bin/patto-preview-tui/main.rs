mod app;
mod backlinks;
mod image_cache;
mod ui;

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

    // Read initial content
    let initial_content = std::fs::read_to_string(&file_path)?;

    // Set up app
    let mut app = App::new(file_path.clone(), dir.clone(), args.protocol.as_deref());
    app.re_render(&initial_content);

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
                        let quit = if app.backlinks.visible {
                            app.handle_backlinks_key(&repository, code, modifiers).await
                        } else {
                            app.handle_normal_key(&repository, code, modifiers, vh).await
                        };
                        if quit { break; }
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
