use clap::Parser as ClapParser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use std::path::PathBuf;
use tower_lsp::{LspService, Server};

use patto::cli::init_logger;
use patto::lsp::{lsp_config::load_config, paper::PaperCatalog, Backend};

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

    let (service, socket) =
        LspService::new(move |client| Backend::new(client, paper_catalog.clone()));
    log::info!("Patto Language Server Protocol started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Patto Language Server Protocol exits");
}
