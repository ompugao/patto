use log;
use std::fs::File;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

fn init_logger(){
    simplelog::CombinedLogger::init(vec![
        // simplelog::TermLogger::new(
        //     simplelog::LevelFilter::Warn,
        //     simplelog::Config::default(),
        //     simplelog::TerminalMode::Mixed,
        // ),
        simplelog::WriteLogger::new(
            simplelog::LevelFilter::Info,
            simplelog::Config::default(),
            File::create("tabton-lsp.log").unwrap(),
        ),
    ])
    .unwrap();
}

#[tokio::main]
async fn main() {
    init_logger();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    log::info!("Tabton Language Server Protocol started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Tabton Language Server Protocol stopped");
}
