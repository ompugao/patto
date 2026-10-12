//! Minimal LSP server that lets an editor push unsaved buffer contents into a
//! preview process, so the preview follows the buffer instead of the file on disk.
//!
//! It implements only the text-synchronisation part of the protocol; every other
//! request is answered by `tower-lsp`'s defaults.

use std::sync::{Arc, Mutex};

use tokio::sync::oneshot;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    InitializedParams, MessageType, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions, Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::repository::Repository;

/// Whether the bridge may write connection lifecycle messages to stderr.
///
/// The TUI keeps this off: it owns the terminal, and stray stderr output
/// corrupts the display.
#[derive(Debug, Clone, Copy, Default)]
pub struct BridgeOptions {
    pub log_connections: bool,
}

pub struct PreviewLspBackend {
    client: Client,
    repository: Arc<Repository>,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl PreviewLspBackend {
    pub fn new(
        client: Client,
        repository: Arc<Repository>,
        shutdown_tx: Option<oneshot::Sender<()>>,
    ) -> Self {
        Self {
            client,
            repository,
            shutdown_tx: Mutex::new(shutdown_tx),
        }
    }

    async fn handle_text_change(&self, uri: Url, text: String) {
        let normalized = Repository::normalize_url_percent_encoding(&uri);
        let Ok(path) = normalized.to_file_path() else {
            self.warn(format!("Preview LSP ignoring non-file URI: {}", normalized))
                .await;
            return;
        };

        let path = std::fs::canonicalize(&path).unwrap_or(path);

        if path.extension().and_then(|s| s.to_str()) != Some("pn") {
            return;
        }

        if !path.starts_with(&self.repository.root_dir) {
            self.warn(format!(
                "Preview LSP ignoring file outside workspace: {}",
                path.display()
            ))
            .await;
            return;
        }

        self.repository.handle_live_file_change(path, text).await;
    }

    async fn warn(&self, message: String) {
        self.client.log_message(MessageType::WARNING, message).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for PreviewLspBackend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                    },
                )),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Preview LSP bridge connected")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
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

/// Accept bridge connections on `127.0.0.1:port` until the process exits.
pub async fn serve_tcp(
    repository: Arc<Repository>,
    port: u16,
    options: BridgeOptions,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    eprintln!("Preview LSP server listening on 127.0.0.1:{}", port);

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let repo = repository.clone();
                    tokio::spawn(async move {
                        let (reader, writer) = tokio::io::split(stream);
                        let (service, socket) =
                            LspService::new(|client| PreviewLspBackend::new(client, repo, None));
                        Server::new(reader, writer, socket).serve(service).await;
                        if options.log_connections {
                            eprintln!("Preview LSP connection {} closed", addr);
                        }
                    });
                }
                Err(err) => {
                    if options.log_connections {
                        eprintln!("Preview LSP accept error: {err}");
                    }
                }
            }
        }
    });

    Ok(())
}

/// Serve the bridge over stdio, for editors that spawn the preview as a child
/// process. The returned receiver fires once the editor disconnects.
pub fn serve_stdio(repository: Arc<Repository>) -> oneshot::Receiver<()> {
    let (tx, rx) = oneshot::channel();

    // Both the backend and the serving task must be able to fire the sender, but
    // only the first one to notice the shutdown may do so. Keeping the `Option`
    // inside the shared mutex (rather than one `Option` per backend clone) makes
    // `take()` consume it exactly once.
    let shutdown_tx = Arc::new(Mutex::new(Some(tx)));
    let shutdown_tx_server = shutdown_tx.clone();

    tokio::spawn(async move {
        let (service, socket) = LspService::new(move |client| {
            let sender = shutdown_tx.lock().unwrap().take();
            PreviewLspBackend::new(client, repository, sender)
        });
        Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
            .serve(service)
            .await;
        if let Some(tx) = shutdown_tx_server.lock().unwrap().take() {
            let _ = tx.send(());
        }
    });

    rx
}
