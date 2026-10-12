//! Workspace start-up: opening the repository at the client's root and
//! forwarding its scan progress as `$/progress`.

use tokio::sync::broadcast;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

use crate::lsp::backend::Backend;
use crate::repository::{Repository, RepositoryMessage};

impl Backend {
    /// vscode sets both `root_uri` and `workspace_folders`; `root_uri` is used
    /// because vim-lsp supports `workspace_folders` only experimentally.
    pub(super) async fn open_workspace(&self, root_uri: Url) {
        *self.root_uri.lock().unwrap() = Some(root_uri.clone());

        let Ok(path) = root_uri.to_file_path() else {
            return;
        };
        self.client
            .log_message(
                MessageType::INFO,
                &format!("LSP workspace root set to {:?}", path),
            )
            .await;

        let repository = Repository::new(path);
        *self.repository.lock().unwrap() = Some(repository.clone());

        // Subscribe before scanning, so no scan progress is missed.
        tokio::spawn(forward_scan_progress(
            self.client.clone(),
            repository.subscribe(),
        ));
        repository.spawn_initial_scan();
    }

    pub(super) fn announce_paper_provider(&self) {
        if !self.paper_catalog.is_configured() {
            return;
        }
        let client = self.client.clone();
        let catalog = self.paper_catalog.clone();
        let provider = catalog
            .provider_name()
            .unwrap_or("paper client")
            .to_string();
        tokio::spawn(async move {
            match catalog.health_check().await {
                Ok(()) => {
                    client
                        .show_message(MessageType::INFO, format!("Connected to {}", provider))
                        .await
                }
                Err(err) => {
                    client
                        .show_message(
                            MessageType::WARNING,
                            format!("Failed to connect to {}: {}", provider, err),
                        )
                        .await
                }
            }
        });
    }
}

async fn forward_scan_progress(client: Client, mut rx: broadcast::Receiver<RepositoryMessage>) {
    let mut progress = ScanProgress {
        client,
        active: false,
    };
    while let Ok(message) = rx.recv().await {
        match message {
            RepositoryMessage::ScanStarted { total_files } => progress.begin(total_files).await,
            RepositoryMessage::ScanProgress { scanned, total } => {
                progress.report(scanned, total).await
            }
            RepositoryMessage::ScanCompleted { total_files } => progress.end(total_files).await,
            _ => {}
        }
    }
}

struct ScanProgress {
    client: Client,
    active: bool,
}

impl ScanProgress {
    const TOKEN: &'static str = "patto-scan";

    async fn begin(&mut self, total_files: usize) {
        self.send(WorkDoneProgress::Begin(WorkDoneProgressBegin {
            title: "Scanning notes".to_string(),
            message: Some(format!("0/{} files", total_files)),
            percentage: Some(0),
            cancellable: Some(false),
        }))
        .await;
        self.active = true;
        self.client
            .log_message(
                MessageType::INFO,
                format!("Starting to scan {} patto files", total_files),
            )
            .await;
    }

    async fn report(&mut self, scanned: usize, total: usize) {
        if !self.active {
            return;
        }
        let percentage = (scanned * 100).checked_div(total).unwrap_or(0) as u32;
        self.send(WorkDoneProgress::Report(WorkDoneProgressReport {
            message: Some(format!("{}/{} files", scanned, total)),
            percentage: Some(percentage),
            cancellable: Some(false),
        }))
        .await;
    }

    async fn end(&mut self, total_files: usize) {
        if self.active {
            self.send(WorkDoneProgress::End(WorkDoneProgressEnd {
                message: Some("Complete".to_string()),
            }))
            .await;
            self.active = false;
        }
        self.client
            .log_message(
                MessageType::INFO,
                format!("Scan completed: {} files indexed", total_files),
            )
            .await;
    }

    async fn send(&self, value: WorkDoneProgress) {
        let _ = self
            .client
            .send_notification::<notification::Progress>(ProgressParams {
                token: NumberOrString::String(Self::TOKEN.to_string()),
                value: ProgressParamsValue::WorkDone(value),
            })
            .await;
    }
}
