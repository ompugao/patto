use clap::Parser as ClapParser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_lsp::{LspService, Server};

use patto::cli::init_logger;
use patto::lsp::{lsp_config::load_config, paper::PaperCatalog, Backend, PattoSettings};

#[derive(ClapParser)]
#[command(version, about, long_about=None)]
struct Cli {
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,

    #[arg(long)]
    debuglogfile: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    init_logger(args.verbose.log_level_filter(), args.debuglogfile)
        .expect("failed to initialise the logger");

    let config = match load_config() {
        Ok(Some(result)) => {
            log::info!("Loaded patto-lsp config from {}", result.path.display());
            Some(result.config)
        }
        Ok(None) => None,
        Err(err) => {
            log::warn!("Failed to load patto-lsp config: {}", err);
            None
        }
    };

    let paper_catalog = match PaperCatalog::from_config(config.as_ref()) {
        Ok(manager) => manager,
        Err(err) => {
            log::warn!("Paper provider configuration error: {}", err);
            PaperCatalog::default()
        }
    };

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let shared_catalog = paper_catalog.clone();
    let (service, socket) = LspService::new(move |client| {
        let repository = Arc::new(Mutex::new(None)); // Root will be set in initialize
        Backend {
            client,
            repository,
            root_uri: Arc::new(Mutex::new(None)),
            paper_catalog: shared_catalog.clone(),
            settings: Arc::new(Mutex::new(PattoSettings::default())),
            last_valid_task_snapshots: Arc::new(dashmap::DashMap::new()),
        }
    });
    log::info!("Patto Language Server Protocol started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Patto Language Server Protocol exits");
}
