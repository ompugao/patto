use clap::Parser as ClapParser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use patto::parser::{self, ParserResult};
use patto::task::TaskSnapshot;
use patto::lsp::task_edits::{collect_task_snapshots, detect_task_transitions};
use str_indices::utf16::from_byte_idx as utf16_from_byte_idx;

#[derive(ClapParser)]
#[command(version, about = "Patto Notification LSP Server", long_about = None)]
struct Cli {
    #[arg(long)]
    debuglogfile: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ActionInfo {
    id: String,
    label: String,
    uri: String,
    line: u32,
    status: String,
}

struct NotificationBackend {
    client: Client,
    document_texts: Arc<dashmap::DashMap<Url, String>>,
    last_valid_task_snapshots: Arc<dashmap::DashMap<Url, HashMap<usize, TaskSnapshot>>>,
}

fn init_logger(logfile: Option<String>) {
    let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = vec![];
    if let Some(filename) = logfile {
        if let Ok(file) = File::create(filename) {
            loggers.push(simplelog::WriteLogger::new(
                log::LevelFilter::Info,
                simplelog::Config::default(),
                file,
            ) as Box<dyn simplelog::SharedLogger>);
        }
    }
    let _ = simplelog::CombinedLogger::init(loggers);
}

fn is_wsl() -> bool {
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        version.to_lowercase().contains("microsoft")
    } else {
        false
    }
}

fn get_task_label(snapshot: &TaskSnapshot) -> String {
    let raw = &snapshot.line_text;
    let span_0 = snapshot.prop_span.0;
    let span_1 = snapshot.prop_span.1;
    if !raw.is_char_boundary(span_0) || !raw.is_char_boundary(span_1) {
        return raw.trim().to_string();
    }
    let before = raw[..span_0].trim_end();
    let after = raw[span_1..].trim_start();
    match (before.is_empty(), after.is_empty()) {
        (true, true) => String::new(),
        (false, true) => before.trim_start().to_string(),
        (true, false) => after.trim_start().to_string(),
        (false, false) => format!("{} {}", before.trim_start(), after),
    }
}

impl NotificationBackend {
    fn show_notification(&self, title: &str, snapshot: &TaskSnapshot, actions: Vec<ActionInfo>) {
        let label = get_task_label(snapshot);
        if label.is_empty() {
            return;
        }

        let run_powershell = cfg!(target_os = "windows") || is_wsl();

        if run_powershell {
            let ps_script = format!(
                r#"[void] [System.Reflection.Assembly]::LoadWithPartialName("System.Windows.Forms");
                $n = New-Object System.Windows.Forms.NotifyIcon;
                $n.Icon = [System.Drawing.SystemIcons]::Information;
                $n.BalloonTipIcon = "Info";
                $n.BalloonTipTitle = "{}";
                $n.BalloonTipText = "{}";
                $n.Visible = $true;
                $n.ShowBalloonTip(5000);"#,
                title.replace('"', "\"\""),
                label.replace('"', "\"\"").replace('\n', " ").replace('\r', "")
            );
            let _ = std::process::Command::new("powershell.exe")
                .args(&["-NoProfile", "-Command", &ps_script])
                .spawn();
        } else {
            let mut notification = notify_rust::Notification::new();
            notification.summary(title).body(&label);

            for action in &actions {
                notification.action(&action.id, &action.label);
            }

            let client = self.client.clone();
            let backend_ref = self.clone_backend();
            tokio::spawn(async move {
                let handle = match notification.show_async().await {
                    Ok(h) => h,
                    Err(e) => {
                        log::error!("Failed to show notification: {:?}", e);
                        return;
                    }
                };

                handle.wait_for_action(move |action_id| {
                    log::info!("Notification action clicked: {}", action_id);
                    if let Some(act) = actions.iter().find(|a| a.id == action_id) {
                        if let Ok(uri) = Url::parse(&act.uri) {
                            if let Some(edit) = backend_ref.generate_status_change_edit(&uri, act.line, &act.status) {
                                let client_clone = client.clone();
                                tokio::spawn(async move {
                                    log::info!("Applying workspace edit to editor: {:?}", edit);
                                    if let Err(err) = client_clone.apply_edit(edit).await {
                                        log::error!("Failed to apply edit: {:?}", err);
                                    }
                                });
                            }
                        }
                    }
                });
            });
        }
    }

    fn clone_backend(&self) -> NotificationBackendClone {
        NotificationBackendClone {
            document_texts: self.document_texts.clone(),
        }
    }
}

struct NotificationBackendClone {
    document_texts: Arc<dashmap::DashMap<Url, String>>,
}

impl NotificationBackendClone {
    fn generate_status_change_edit(&self, uri: &Url, line_idx: u32, new_status: &str) -> Option<WorkspaceEdit> {
        let content = self.document_texts.get(uri)?;
        let lines: Vec<&str> = content.lines().collect();
        let line_text = lines.get(line_idx as usize)?;
        
        let status_prefix = "status=";
        if let Some(start_byte) = line_text.find(status_prefix) {
            let val_start = start_byte + status_prefix.len();
            let val_end = val_start + line_text[val_start..]
                .chars()
                .take_while(|c| c.is_alphabetic())
                .count();
            
            let range = Range {
                start: Position {
                    line: line_idx,
                    character: utf16_from_byte_idx(line_text, val_start) as u32,
                },
                end: Position {
                    line: line_idx,
                    character: utf16_from_byte_idx(line_text, val_end) as u32,
                },
            };
            
            let text_edit = TextEdit {
                range,
                new_text: new_status.to_string(),
            };
            
            let mut changes = std::collections::HashMap::new();
            changes.insert(uri.clone(), vec![text_edit]);
            
            Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            })
        } else {
            None
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for NotificationBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "patto-notification".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        log::info!("Patto Notification LSP initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        
        self.document_texts.insert(uri.clone(), text.clone());
        
        let ParserResult { ast, .. } = parser::parse_text(&text);
        let snapshots = collect_task_snapshots(&ast);
        
        let mut entry = self.last_valid_task_snapshots.entry(uri).or_default();
        for (row, snap) in snapshots {
            if snap.status_is_canonical {
                entry.insert(row, snap);
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.first() {
            let text = &change.text;
            self.document_texts.insert(uri.clone(), text.clone());

            let ParserResult { ast, .. } = parser::parse_text(text);
            let new_snapshots = collect_task_snapshots(&ast);
            
            let old_snapshots = self
                .last_valid_task_snapshots
                .get(&uri)
                .map(|e| e.value().clone())
                .unwrap_or_default();
                
            for transition in detect_task_transitions(&new_snapshots, &old_snapshots) {
                match transition {
                    patto::task::TaskTransition::BecameDoing { new, .. } => {
                        self.show_notification("Task Started", &new, vec![
                            ActionInfo {
                                id: "pause".to_string(),
                                label: "Pause Task".to_string(),
                                uri: uri.to_string(),
                                line: new.row as u32,
                                status: "paused".to_string(),
                            },
                            ActionInfo {
                                id: "done".to_string(),
                                label: "Complete Task".to_string(),
                                uri: uri.to_string(),
                                line: new.row as u32,
                                status: "done".to_string(),
                            }
                        ]);
                    }
                    patto::task::TaskTransition::BecamePaused { new, .. } => {
                        self.show_notification("Task Paused", &new, vec![
                            ActionInfo {
                                id: "doing".to_string(),
                                label: "Resume Task".to_string(),
                                uri: uri.to_string(),
                                line: new.row as u32,
                                status: "doing".to_string(),
                            },
                            ActionInfo {
                                id: "done".to_string(),
                                label: "Complete Task".to_string(),
                                uri: uri.to_string(),
                                line: new.row as u32,
                                status: "done".to_string(),
                            }
                        ]);
                    }
                    patto::task::TaskTransition::BecameDone { new, .. } => {
                        self.show_notification("Task Completed", &new, vec![]);
                    }
                    _ => {}
                }
            }
            
            // Update the cache
            {
                let mut entry = self.last_valid_task_snapshots.entry(uri).or_default();
                for (row, snap) in new_snapshots {
                    if snap.status_is_canonical {
                        entry.insert(row, snap);
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    init_logger(args.debuglogfile);

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| NotificationBackend {
        client,
        document_texts: Arc::new(dashmap::DashMap::new()),
        last_valid_task_snapshots: Arc::new(dashmap::DashMap::new()),
    });
    log::info!("Patto Notification Language Server started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Patto Notification Language Server exits");
}
