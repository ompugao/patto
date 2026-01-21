use clap::Parser as ClapParser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use simplelog;
use std::fs::File;
use std::sync::{Arc, Mutex};
use tower_lsp::{LspService, Server};

use patto::lsp::{lsp_config::load_config, paper::PaperCatalog, Backend, PattoSettings};

#[derive(ClapParser)]
#[command(version, about, long_about=None)]
struct Cli {
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,

    #[arg(long)]
    debuglogfile: Option<String>,
}

fn init_logger(filter_level: log::LevelFilter, logfile: Option<String>) {
    let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = vec![];

    if let Some(filename) = logfile {
        loggers.push(simplelog::WriteLogger::new(
            filter_level,
            simplelog::Config::default(),
            File::create(filename).unwrap(),
        ) as Box<dyn simplelog::SharedLogger>)
    }
    simplelog::CombinedLogger::init(loggers).unwrap();
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    init_logger(args.verbose.log_level_filter(), args.debuglogfile);

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
        }
    });
    log::info!("Patto Language Server Protocol started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Patto Language Server Protocol exits");
}
