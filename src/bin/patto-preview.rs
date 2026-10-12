use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use patto::preview::lsp_bridge::{self, BridgeOptions};
use patto::preview::server::{router, AppState};
use patto::repository::Repository;

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

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let dir = std::fs::canonicalize(PathBuf::from(&args.dir)).unwrap_or_else(|_| {
        eprintln!("Failed to canonicalize directory: {}", args.dir);
        std::process::exit(1);
    });
    if !dir.exists() {
        eprintln!("Directory does not exist: {}", dir.display());
        std::process::exit(1);
    }

    let repository = Arc::new(Repository::new(dir));
    repository.spawn_initial_scan();

    let watched = repository.clone();
    tokio::spawn(async move {
        if let Err(e) = watched.start_watcher().await {
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

    let app = router(AppState::new(repository));

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
