#[path = "patto-lsp/lsp_config.rs"]
mod lsp_config;
#[path = "patto-lsp/paper.rs"]
mod paper;

use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use urlencoding::decode;

use str_indices::utf16::{from_byte_idx as utf16_from_byte_idx, to_byte_idx as utf16_to_byte_idx};

use lsp_config::load_config;
use paper::{PaperCatalog, PaperProviderError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use patto::diagnostic_translator::{DiagnosticTranslator, FriendlyDiagnostic};
use patto::markdown::{MarkdownFlavor, MarkdownRendererOptions};
use patto::parser::{self, AstNode, AstNodeKind, Deadline, ParserResult, Property, TaskStatus};
use patto::renderer::{MarkdownRenderer, Renderer};
use patto::repository::{Repository, RepositoryMessage};
use patto::semantic_token::{
    compute_semantic_tokens_delta, get_semantic_tokens, get_semantic_tokens_range, LEGEND_TYPE,
};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// LSP settings that can be configured by clients
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PattoSettings {
    /// Markdown export settings
    #[serde(default)]
    markdown: MarkdownSettings,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MarkdownSettings {
    /// Default markdown flavor for export (standard, obsidian, github)
    #[serde(default)]
    default_flavor: Option<String>,
}

//#[derive(Debug)]
struct Backend {
    client: Client,
    repository: Arc<Mutex<Option<Repository>>>,
    root_uri: Arc<Mutex<Option<Url>>>,
    paper_catalog: PaperCatalog,
    settings: Arc<Mutex<PattoSettings>>,
    token_cache: Arc<Mutex<std::collections::HashMap<Url, CachedSemanticTokens>>>,
    //semantic_token_map: DashMap<String, Vec<ImCompleteSemanticToken>>,
}

fn get_node_range(from: &AstNode) -> Range {
    let row = from.location().row as u32;
    let s = utf16_from_byte_idx(from.extract_str(), from.location().span.0) as u32;
    let e = utf16_from_byte_idx(from.extract_str(), from.location().span.1) as u32;
    Range::new(Position::new(row, s), Position::new(row, e))
}

// fn uri_to_link(uri: &Url, base: &Url) -> Option<String> {
//     if base.scheme() != uri.scheme() {
//         log::debug!("Different scheme, cannot subtract: {}, {}", uri, base);
//         return None;
//     }
//
//     let base_path = base.path_segments().map(|c| c.map(|segment| decode(segment).unwrap()).collect::<Vec<_>>()).unwrap_or_default();
//     let uri_path = uri.path_segments().map(|c| c.map(|segment| decode(segment).unwrap()).collect::<Vec<_>>()).unwrap_or_default();
//
//     if !uri_path.starts_with(&base_path) {
//         log::debug!("uri is not inside base: {}, {}", uri, base);
//         return None; // uri is not inside base
//     }
//
//     // Extract the remainder after the base path
//     let relative_path = &uri_path[base_path.len()..];
//     Some(relative_path.join("/"))
// }

fn parse_text(text: &str) -> (AstNode, Vec<Diagnostic>) {
    let ParserResult { ast, parse_errors } = parser::parse_text(text);
    let translator = DiagnosticTranslator::default();
    let diagnostics: Vec<Diagnostic> = parse_errors
        .into_iter()
        .map(|error| {
            let location = error.location().clone();
            let FriendlyDiagnostic {
                message,
                code,
                code_description_uri,
            } = translator.translate(&error);

            let code_value = code.map(NumberOrString::String);
            let code_description = code_description_uri
                .and_then(|href| Url::parse(&href).ok())
                .map(|href| CodeDescription { href });

            Diagnostic {
                range: Range::new(
                    Position::new(location.row as u32, location.span.0 as u32),
                    Position::new(location.row as u32, location.span.1 as u32),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                code: code_value,
                code_description,
                source: Some("patto".into()),
                message,
                ..Diagnostic::default()
            }
        })
        .collect();
    (ast, diagnostics)
}

fn gather_anchors(parent: &AstNode, anchors: &mut Vec<String>) {
    if let AstNodeKind::Line { ref properties } = &parent.kind() {
        for prop in properties {
            if let Property::Anchor { name, .. } = prop {
                anchors.push(name.to_string());
            }
        }
    }

    for child in parent.value().children.lock().unwrap().iter() {
        gather_anchors(child, anchors);
    }
}

fn gather_tasks(parent: &AstNode, tasklines: &mut Vec<(AstNode, Deadline)>) {
    if let AstNodeKind::Line { ref properties } = &parent.kind() {
        for prop in properties {
            if let Property::Task { status, due, .. } = prop {
                if !matches!(status, TaskStatus::Done) {
                    tasklines.push((parent.clone(), due.clone()));
                    break;
                }
            }
        }
    }

    for child in parent.value().children.lock().unwrap().iter() {
        gather_tasks(child, tasklines);
    }
}

/// Cached semantic tokens for delta computation
#[derive(Debug, Clone)]
struct CachedSemanticTokens {
    result_id: String,
    tokens: Vec<SemanticToken>,
    document_version: i32,
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct TaskInformation {
    /// The location of this task
    pub location: Location,

    /// The text of this task
    pub text: String,

    pub message: String,

    /// The deadline of this task
    pub due: Deadline,
}

impl TaskInformation {
    pub fn new(location: Location, text: String, message: String, due: Deadline) -> Self {
        Self {
            location,
            text,
            message,
            due,
        }
    }
}

fn find_anchor(parent: &AstNode, anchor: &str) -> Option<AstNode> {
    if let AstNodeKind::Line { ref properties } = &parent.kind() {
        for prop in properties {
            if let Property::Anchor { name, .. } = prop {
                if name == anchor {
                    return Some(parent.clone());
                }
            }
        }
    }

    #[allow(clippy::map_clone)]
    return parent
        .value()
        .children
        .lock()
        .unwrap()
        .iter()
        .find_map(|child| find_anchor(child, anchor))
        .map(|x| x.clone());
}

/// Find anchor definition at the given row and column position
/// Returns (anchor_name, anchor_location) if cursor is on an anchor definition
fn find_anchor_at_position(
    parent: &AstNode,
    row: usize,
    col: usize,
) -> Option<(String, parser::Location)> {
    if let AstNodeKind::Line { ref properties } = &parent.kind() {
        if parent.location().row == row {
            for prop in properties {
                if let Property::Anchor { name, location } = prop {
                    if location.span.contains(col) {
                        return Some((name.clone(), location.clone()));
                    }
                }
            }
        }
    }

    for child in parent.value().children.lock().unwrap().iter() {
        if let Some(result) = find_anchor_at_position(child, row, col) {
            return Some(result);
        }
    }
    None
}

fn locate_node_route(parent: &AstNode, row: usize, col: usize) -> Option<Vec<AstNode>> {
    if let Some(route) = locate_node_route_impl(parent, row, col) {
        //route.reverse();
        return Some(route);
    }
    None
}

fn locate_node_route_impl(parent: &AstNode, row: usize, col: usize) -> Option<Vec<AstNode>> {
    let parentrow = parent.location().row;
    log::debug!(
        "finding row, col ({}, {}), scanning row: {}",
        row,
        col,
        parentrow
    );
    if matches!(parent.kind(), AstNodeKind::Dummy) || parentrow < row {
        for child in parent.value().children.lock().unwrap().iter() {
            if let Some(mut route) = locate_node_route_impl(child, row, col) {
                route.push(parent.clone());
                return Some(route);
            }
        }
    } else if parentrow == row {
        if parent.value().contents.lock().unwrap().is_empty() {
            log::debug!("{:?} must be leaf", parent.extract_str());
            return Some(vec![parent.clone()]);
        }
        for content in parent.value().contents.lock().unwrap().iter() {
            if content.location().span.contains(col) {
                log::debug!(
                    "in content: {:?}, spanning ({}, {})",
                    content.extract_str(),
                    content.location().span.0,
                    content.location().span.1
                );
                if let Some(mut route) = locate_node_route_impl(content, row, col) {
                    route.push(parent.clone());
                    return Some(route);
                }
            }
        }
    }
    None
}

/// Generate a result_id for semantic tokens based on URI and version
fn generate_result_id(uri: &Url, version: i32) -> String {
    format!("{}:v{}", uri, version)
}

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        let uri = Repository::normalize_url_percent_encoding(&params.uri);

        // Invalidate cached semantic tokens for this document
        self.token_cache.lock().unwrap().remove(&uri);

        // Use repository's add_file_to_graph method which handles everything
        if let Ok(file_path) = uri.to_file_path() {
            if let Some(repo) = self.repository.lock().unwrap().as_ref() {
                repo.add_file_to_graph(&file_path, &params.text);
            }
        }

        // Parse for diagnostics (this is LSP-specific, not handled by repository)
        let (_, diagnostics) = parse_text(&params.text);

        // Publish diagnostics to the client
        self.client
            .publish_diagnostics(params.uri, diagnostics, Some(params.version))
            .await;
    }

    async fn start_repository_listener(&self) {
        let repo_guard = self.repository.lock().unwrap();
        if let Some(repo) = repo_guard.as_ref() {
            let mut rx = repo.subscribe();
            drop(repo_guard); // Release lock before async loop

            let client = self.client.clone();

            tokio::spawn(async move {
                let token = NumberOrString::String("patto-scan".to_string());
                let mut progress_active = false;

                while let Ok(msg) = rx.recv().await {
                    match msg {
                        RepositoryMessage::ScanStarted { total_files } => {
                            let _ = client
                                .send_notification::<notification::Progress>(ProgressParams {
                                    token: token.clone(),
                                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                                        WorkDoneProgressBegin {
                                            title: "Scanning notes".to_string(),
                                            message: Some(format!("0/{} files", total_files)),
                                            percentage: Some(0),
                                            cancellable: Some(false),
                                        },
                                    )),
                                })
                                .await;
                            progress_active = true;

                            client
                                .log_message(
                                    MessageType::INFO,
                                    format!("Starting to scan {} patto files", total_files),
                                )
                                .await;
                        }

                        RepositoryMessage::ScanProgress { scanned, total } => {
                            if progress_active {
                                let percentage = if total > 0 {
                                    ((scanned * 100) / total) as u32
                                } else {
                                    0
                                };

                                let _ = client
                                    .send_notification::<notification::Progress>(ProgressParams {
                                        token: token.clone(),
                                        value: ProgressParamsValue::WorkDone(
                                            WorkDoneProgress::Report(WorkDoneProgressReport {
                                                message: Some(format!(
                                                    "{}/{} files",
                                                    scanned, total
                                                )),
                                                percentage: Some(percentage),
                                                cancellable: Some(false),
                                            }),
                                        ),
                                    })
                                    .await;
                            }
                        }

                        RepositoryMessage::ScanCompleted { total_files } => {
                            if progress_active {
                                let _ = client
                                    .send_notification::<notification::Progress>(ProgressParams {
                                        token: token.clone(),
                                        value: ProgressParamsValue::WorkDone(
                                            WorkDoneProgress::End(WorkDoneProgressEnd {
                                                message: Some("Complete".to_string()),
                                            }),
                                        ),
                                    })
                                    .await;
                                progress_active = false;
                            }

                            client
                                .log_message(
                                    MessageType::INFO,
                                    format!("Scan completed: {} files indexed", total_files),
                                )
                                .await;
                        }

                        _ => {}
                    }
                }
            });
        }
    }

    async fn gather_completion_items(
        &self,
        uri: &Url,
        position: Position,
    ) -> Option<Vec<CompletionItem>> {
        let mut deferred: Option<(Vec<CompletionItem>, Range, String)> = None;

        {
            let repo_guard = self.repository.lock().unwrap();
            let repo = repo_guard.as_ref()?;
            let rope = repo.document_map.get(uri)?;
            let line = rope.value().get_line(position.line as usize)?;
            let line_str = line.as_str()?;

            let cur_col =
                line.byte_to_char(utf16_to_byte_idx(line_str, position.character as usize));
            let prev_col = cur_col.saturating_sub(1);
            let c = line.char(prev_col);
            if c == '#' {
                let slice = line.slice(..cur_col);
                if let Some(foundbracket) =
                    slice.chars_at(cur_col).reversed().position(|c| c == '[')
                {
                    let maybelink = slice.len_chars().saturating_sub(foundbracket);
                    let s = line.slice(maybelink..prev_col).as_str()?;
                    log::debug!("link? {}, from {}, found at {}", s, maybelink, foundbracket);
                    let Some(root_uri) = self.root_uri.lock().unwrap().as_ref().cloned() else {
                        log::debug!("root_uri is not set");
                        return None;
                    };
                    let linkuri = repo.link_to_uri(s, &root_uri).unwrap_or(uri.clone());
                    log::debug!("linkuri: {}", linkuri);
                    if let Some(ast) = repo.ast_map.get(&linkuri) {
                        let mut anchors = vec![];
                        gather_anchors(ast.value(), &mut anchors);
                        return Some(
                            anchors
                                .iter()
                                .map(|anchor| CompletionItem {
                                    label: format!("#{}", anchor),
                                    kind: Some(CompletionItemKind::REFERENCE),
                                    filter_text: Some(anchor.to_string()),
                                    insert_text: Some(anchor.to_string()),
                                    ..Default::default()
                                })
                                .collect(),
                        );
                    }
                }
            }

            let slice = line.slice(..cur_col);
            let slicelen = slice.len_chars();
            if let Some(foundbracket) = slice.chars_at(cur_col).reversed().position(|c| c == '[') {
                let maybelink = slicelen.saturating_sub(foundbracket);
                let s = line.slice(maybelink..cur_col).as_str()?;
                log::debug!(
                    "matching {}, from {}, found at {}",
                    s,
                    maybelink,
                    foundbracket
                );

                if let Some(root_uri_str) = self
                    .root_uri
                    .lock()
                    .unwrap()
                    .as_ref()
                    .and_then(|root_uri| root_uri.to_file_path().ok())
                {
                    let baselen = root_uri_str.to_string_lossy().len();
                    let matcher = SkimMatcherV2::default();
                    let start_char =
                        utf16_from_byte_idx(line_str, line.char_to_byte(maybelink)) as u32;
                    let replacement_range = Range {
                        start: Position {
                            line: position.line,
                            character: start_char,
                        },
                        end: position,
                    };

                    let files: Vec<CompletionItem> = repo
                        .document_map
                        .iter()
                        .filter_map(|e| {
                            let mut path = decode(
                                &e.key().to_file_path().unwrap().to_string_lossy()[baselen + 1..],
                            )
                            .unwrap()
                            .to_string();
                            if path.ends_with(".pn") {
                                path = path.strip_suffix(".pn").unwrap().to_string();
                            }
                            if matcher.fuzzy_match(&path, s).is_some() {
                                return Some(CompletionItem {
                                    label: path.clone(),
                                    detail: Some(path.clone()),
                                    kind: Some(CompletionItemKind::FILE),
                                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                        new_text: path.clone(),
                                        range: replacement_range,
                                    })),
                                    ..Default::default()
                                });
                            }
                            None
                        })
                        .collect();

                    deferred = Some((files, replacement_range, s.to_string()));
                }
            }

            let slice = line.slice(..cur_col);
            let slicelen = slice.len_chars();
            if let Some(foundat) = slice.chars_at(cur_col).reversed().position(|c| c == '@') {
                let maybecommand = slicelen.saturating_sub(foundat);
                let s = line.slice(maybecommand..cur_col).as_str()?;
                log::debug!(
                    "command? {}, from {}, found at {}",
                    s,
                    maybecommand,
                    foundat
                );
                match s {
                    "@code" => {
                        let item = CompletionItem {
                            label: "@code".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("code command".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                new_text: "[@code ${1:lang}]$0".to_string(),
                                range: Range {
                                    start: Position {
                                        line: position.line,
                                        character: utf16_from_byte_idx(
                                            line_str,
                                            line.char_to_byte(maybecommand),
                                        ) as u32,
                                    },
                                    end: position,
                                },
                            })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    }
                    "@math" => {
                        let item = CompletionItem {
                            label: "@math".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("math command".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                new_text: "[@math]$0".to_string(),
                                range: Range {
                                    start: Position {
                                        line: position.line,
                                        character: utf16_from_byte_idx(
                                            line_str,
                                            line.char_to_byte(maybecommand),
                                        ) as u32,
                                    },
                                    end: position,
                                },
                            })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    }
                    "@quote" => {
                        let item = CompletionItem {
                            label: "@quote".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("quote command".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                new_text: "[@quote]$0".to_string(),
                                range: Range {
                                    start: Position {
                                        line: position.line,
                                        character: utf16_from_byte_idx(
                                            line_str,
                                            line.char_to_byte(maybecommand),
                                        ) as u32,
                                    },
                                    end: position,
                                },
                            })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    }
                    "@img" => {
                        let item = CompletionItem {
                            label: "@img".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("img command".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                new_text: "[@img ${1:path} \"${2:alt_text}\"]$0".to_string(),
                                range: Range {
                                    start: Position {
                                        line: position.line,
                                        character: utf16_from_byte_idx(
                                            line_str,
                                            line.char_to_byte(maybecommand),
                                        ) as u32,
                                    },
                                    end: position,
                                },
                            })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    }
                    "@task" => {
                        let item = CompletionItem {
                            label: "@task".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("task property".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                new_text: format!(
                                    "{{@task status=${{1:todo}} due=${{2:{}}}}}$0",
                                    chrono::Local::now().format("%Y-%m-%d")
                                ),
                                range: Range {
                                    start: Position {
                                        line: position.line,
                                        character: utf16_from_byte_idx(
                                            line_str,
                                            line.char_to_byte(maybecommand),
                                        ) as u32,
                                    },
                                    end: position,
                                },
                            })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    }
                    &_ => {}
                }
            }
        }

        if let Some((mut files, replacement_range, query)) = deferred {
            let mut papers = self
                .paper_completion_items(&query, &replacement_range)
                .await;
            files.append(&mut papers);
            return Some(files);
        }

        None
    }

    async fn paper_completion_items(&self, query: &str, range: &Range) -> Vec<CompletionItem> {
        match self.paper_catalog.search(query).await {
            Ok(papers) => papers
                .into_iter()
                .map(|paper| CompletionItem {
                    label: paper.title.clone(),
                    detail: Some(format!("Zotero · {}", paper.title)),
                    kind: Some(CompletionItemKind::REFERENCE),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        new_text: format!("{} {}", paper.title, paper.link),
                        range: range.clone(),
                    })),
                    ..Default::default()
                })
                .collect(),
            Err(PaperProviderError::NotConfigured) => Vec::new(),
            Err(err) => {
                log::warn!("paper completion failed: {}", err);
                self.client
                    .log_message(
                        MessageType::WARNING,
                        &format!("paper completion failed: {}", err),
                    )
                    .await;
                Vec::new()
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri {
            {
                let mut backend_root_uri = self.root_uri.lock().unwrap();
                *backend_root_uri = Some(root_uri.clone());
            } // Drop backend_root_uri here

            if let Ok(path) = root_uri.to_file_path() {
                self.client
                    .log_message(
                        MessageType::INFO,
                        &format!("LSP workspace root set to {:?}", path),
                    )
                    .await;

                // Create repository (scanning happens in background)
                {
                    let mut repo = self.repository.lock().unwrap();
                    *repo = Some(Repository::new(path));
                } // Drop repo here

                // Start listening to repository messages (including scan progress)
                self.start_repository_listener().await;
            }
        }

        // vscode sets both root_uri and workspace_folders.
        // Using root_uri for now, since vim-lsp experimentally support workspace_folers.
        //
        // if let Some(workspace_folders) = params.workspace_folders {
        //     for folder in workspace_folders {
        //         self.client.log_message(MessageType::INFO, &format!("scanning folder {:?}", folder)).await;
        //         let path = folder.uri.to_file_path();
        //         if path.is_ok() {
        //             let client = self.client.clone();
        //             let ast_map = Arc::clone(&self.ast_map);
        //             tokio::spawn(async move {
        //                 if let Err(e) = scan_workspace(client, path.unwrap(), ast_map).await {
        //                     log::warn!("Failed to scan workspace: {:?}", e);
        //                 }
        //             });
        //         }
        //     }
        // }

        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF16), // vscode only supports utf-16 ;(
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(
                        vec!["[", "#", "@img", "@math", "@quote", "@table", "@task"]
                            .into_iter()
                            .map(ToString::to_string)
                            .collect(),
                    ),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "experimental/aggregate_tasks".to_string(),
                        "experimental/retrieve_two_hop_notes".to_string(),
                        "experimental/scan_workspace".to_string(),
                        "patto/snapshotPapers".to_string(),
                        "patto/renderAsMarkdown".to_string(),
                    ],
                    work_done_progress_options: Default::default(),
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                        SemanticTokensRegistrationOptions {
                            text_document_registration_options: {
                                TextDocumentRegistrationOptions {
                                    document_selector: Some(vec![DocumentFilter {
                                        language: Some("patto".to_string()),
                                        scheme: Some("file".to_string()),
                                        pattern: None,
                                    }]),
                                }
                            },
                            semantic_tokens_options: SemanticTokensOptions {
                                work_done_progress_options: WorkDoneProgressOptions::default(),
                                legend: SemanticTokensLegend {
                                    token_types: LEGEND_TYPE.into(),
                                    token_modifiers: vec![],
                                },
                                range: Some(true),
                                full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                            },
                            static_registration_options: StaticRegistrationOptions::default(),
                        },
                    ),
                ),
                // definition: Some(GotoCapability::default()),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                ..ServerCapabilities::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "patto-lsp server initialized!")
            .await;

        if self.paper_catalog.is_configured() {
            let client = self.client.clone();
            let manager = self.paper_catalog.clone();
            let provider_label = manager
                .provider_name()
                .unwrap_or("paper client")
                .to_string();
            tokio::spawn(async move {
                match manager.health_check().await {
                    Ok(_) => {
                        client
                            .show_message(
                                MessageType::INFO,
                                format!("Connected to {}", provider_label),
                            )
                            .await;
                    }
                    Err(err) => {
                        client
                            .show_message(
                                MessageType::WARNING,
                                format!("Failed to connect to {}: {}", provider_label, err),
                            )
                            .await;
                    }
                }
            });
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        // Try to extract patto settings from the configuration
        // VSCode sends: { "patto": { "markdown": { "defaultFlavor": "obsidian" } } }
        // or just the patto section depending on client
        let settings_value = if let Some(patto) = params.settings.get("patto") {
            patto.clone()
        } else {
            params.settings
        };

        match serde_json::from_value::<PattoSettings>(settings_value) {
            Ok(new_settings) => {
                log::info!("Updated patto settings: {:?}", new_settings);
                let mut settings = self.settings.lock().unwrap();
                *settings = new_settings;
            }
            Err(e) => {
                log::warn!("Failed to parse patto settings: {:?}", e);
            }
        }
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        log::info!("did_open: {:?}", params.text_document.uri);
        self.on_change(TextDocumentItem {
            language_id: "".to_string(),
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
        })
        .await
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            language_id: "".to_string(),
            text: std::mem::take(&mut params.content_changes[0].text),
            version: params.text_document.version,
        })
        .await
    }

    async fn did_save(&self, param: DidSaveTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("file {} saved!", param.text_document.uri.as_str()),
            )
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        //self.repository.document_map.remove(&uri);
        //self.repository.ast_map.remove(&uri);
        self.client
            .log_message(MessageType::INFO, format!("file {} is closed!", uri))
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = Repository::normalize_url_percent_encoding(
            &params.text_document_position.text_document.uri,
        );
        let position = params.text_document_position.position;
        let completions = self.gather_completion_items(&uri, position).await;
        Ok(completions.map(CompletionResponse::Array))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(MessageType::LOG, format!("command executed!: {:?}", params))
            .await;

        match params.command.as_str() {
            "experimental/aggregate_tasks" => {
                let mut tasks: Vec<(Url, AstNode, Deadline)> = vec![];

                let repo_bind = self.repository.lock().unwrap();
                let Some(repo) = repo_bind.as_ref() else {
                    return Ok(None);
                };
                repo.ast_map.iter().for_each(|entry| {
                    let mut tasklines = vec![];
                    gather_tasks(entry.value(), &mut tasklines);
                    tasklines.into_iter().for_each(|(line, due)| {
                        tasks.push((entry.key().clone(), line.clone(), due.clone()));
                    });
                });
                tasks.sort_by_key(|(_uri, _line, due): &(_, _, Deadline)| due.clone());
                let ret = json!(tasks
                    .iter()
                    .map(|(uri, line, due)| {
                        TaskInformation::new(
                            Location::new(uri.clone(), get_node_range(line)),
                            line.extract_str().trim_start().to_string(),
                            "".to_string(),
                            due.clone(),
                        )
                    })
                    .collect::<Vec<_>>());
                //self.client
                //    .log_message(MessageType::INFO, format!("response: {:?}", ret))
                //    .await;
                return Ok(Some(ret));
            }
            "experimental/retrieve_two_hop_notes" => {
                let repo_bind = self.repository.lock().unwrap();
                let Some(repo) = repo_bind.as_ref() else {
                    return Ok(None);
                };
                let Ok(graph) = repo.document_graph.lock() else {
                    return Ok(None);
                };
                let Some(url) = params
                    .arguments
                    .first()
                    .and_then(|a| a.as_str())
                    .and_then(|url| Url::parse(url).ok())
                else {
                    return Ok(None);
                };
                let Some(node) = graph.get(&url) else {
                    return Ok(None);
                };
                let mut twohop_urls = node
                    .iter_out()
                    .map(|edge| {
                        let target = edge.target();
                        let connected_urls = target
                            .iter_in()
                            .map(|edge| edge.source().key().clone())
                            .filter(|n| n != target.key() && n != &url)
                            .collect::<Vec<Url>>();
                        (target.key().clone(), connected_urls)
                    })
                    .filter(|x| !x.1.is_empty())
                    .collect::<Vec<(Url, Vec<_>)>>();
                twohop_urls.sort_by_key(|x| -(x.1.len() as i16));
                twohop_urls.dedup();
                log::debug!("urls: {:?}", twohop_urls);
                return Ok(Some(json!(twohop_urls)));
            }
            "patto/snapshotPapers" => {
                self.client
                    .log_message(MessageType::INFO, "Taking snapshot of papers...")
                    .await;
                match self.paper_catalog.refresh().await {
                    Ok(_) => {
                        self.client
                            .show_message(
                                MessageType::INFO,
                                "Paper snapshot completed successfully.",
                            )
                            .await;
                        return Ok(None);
                    }
                    Err(e) => {
                        let msg = format!("Failed to take paper snapshot: {}", e);
                        self.client.show_message(MessageType::ERROR, &msg).await;
                        log::error!("{}", msg);
                        return Ok(None);
                    }
                }
            }
            "patto/renderAsMarkdown" => {
                // Arguments: [uri, startLine?, endLine?, flavor?]
                // If startLine/endLine not provided, render entire document
                // If flavor not provided, use default from settings
                let Some(uri_str) = params.arguments.first().and_then(|a| a.as_str()) else {
                    return Ok(None);
                };
                let Ok(uri) = Url::parse(uri_str) else {
                    return Ok(None);
                };
                let uri = Repository::normalize_url_percent_encoding(&uri);

                // Parse optional range (0-indexed, inclusive)
                let start_line = params
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_u64())
                    .map(|n| n as usize);
                let end_line = params
                    .arguments
                    .get(2)
                    .and_then(|a| a.as_u64())
                    .map(|n| n as usize);

                // Parse optional flavor, falling back to settings default, then "standard"
                let flavor_str = params
                    .arguments
                    .get(3)
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        self.settings
                            .lock()
                            .unwrap()
                            .markdown
                            .default_flavor
                            .clone()
                    })
                    .unwrap_or_else(|| "standard".to_string());
                let flavor = match flavor_str.to_lowercase().as_str() {
                    "obsidian" => MarkdownFlavor::Obsidian,
                    "github" => MarkdownFlavor::GitHub,
                    _ => MarkdownFlavor::Standard,
                };

                let repo_bind = self.repository.lock().unwrap();
                let Some(repo) = repo_bind.as_ref() else {
                    return Ok(None);
                };
                let Some(ast) = repo.ast_map.get(&uri) else {
                    return Ok(None);
                };

                let options = MarkdownRendererOptions::new(flavor).with_frontmatter(false);
                let renderer = MarkdownRenderer::new(options);
                let mut output = Vec::new();

                let result = if let (Some(start), Some(end)) = (start_line, end_line) {
                    renderer.format_range(ast.value(), &mut output, start, end)
                } else {
                    renderer.format(ast.value(), &mut output)
                };

                if result.is_err() {
                    log::error!("Failed to render markdown: {:?}", result);
                    return Ok(None);
                }

                let markdown = String::from_utf8_lossy(&output).to_string();
                return Ok(Some(json!(markdown)));
            }
            c => {
                log::info!("unknown command: {}", c);
            }
        }
        log::info!("unhandled command execution: {:?}", params);
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let definition = async {
            let uri = Repository::normalize_url_percent_encoding(
                &params.text_document_position_params.text_document.uri,
            );
            let repo_lock = self.repository.lock().unwrap();
            let repo = repo_lock.as_ref()?;
            let ast = repo.ast_map.get(&uri)?;
            let rope = repo.document_map.get(&uri)?;

            let position = params.text_document_position_params.position;
            // let char = rope.try_line_to_char(position.line as usize).ok()?;
            // self.client.log_message(MessageType::INFO, &format!("{:#?}, {}", ast.value(), offset)).await;
            let line = rope.get_line(position.line as usize)?;
            // NOTE: spans in our parser (and in pest) are in bytes, not chars
            let posbyte = utf16_to_byte_idx(line.as_str()?, position.character as usize);
            let Some(node_route) = locate_node_route(&ast, position.line as usize, posbyte) else {
                log::debug!("Node not found at {:?}, posbyte: {:?}", position, posbyte);
                return None;
            };
            //if node_route.len() == 0 {
            //    log::info!("-- route.len() is 0");
            //    return None;
            // }
            let Some((link, anchor)) = node_route.iter().find_map(|n| {
                if let AstNodeKind::WikiLink { link, anchor } = &n.kind() {
                    Some((link, anchor))
                } else {
                    None
                }
            }) else {
                log::debug!("it is not wikilink");
                return None;
            };
            let Some(root_uri) = self.root_uri.lock().unwrap().as_ref().cloned() else {
                log::debug!("root_uri is not set");
                return None;
            };
            let linkuri = repo.link_to_uri(link, &root_uri).unwrap_or(uri);
            let start = Range::new(Position::new(0, 0), Position::new(0, 1));
            if let Some(anchor) = anchor {
                let range = repo
                    .ast_map
                    .get(&linkuri)
                    .and_then(|r| {
                        let linkast = r.value();
                        find_anchor(linkast, anchor)
                    })
                    .map_or(start, |anchored_line| get_node_range(&anchored_line));
                Some(GotoDefinitionResponse::Scalar(Location::new(
                    linkuri, range,
                )))
            } else {
                Some(GotoDefinitionResponse::Scalar(Location::new(
                    linkuri, start,
                )))
            }
        }
        .await;
        Ok(definition)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let references = async {
            let uri = Repository::normalize_url_percent_encoding(
                &params.text_document_position.text_document.uri,
            );

            let repo_lock = self.repository.lock().unwrap();
            let repo = repo_lock.as_ref()?;
            let Ok(graph) = repo.document_graph.lock() else {
                log::debug!("failed to lock graph");
                return None;
            };
            let Some(node) = graph.get(&uri) else {
                log::debug!("node not found in the graph");
                return None;
            };

            let mut references = Vec::new();

            // Iterate through all incoming edges
            for edge in node.iter_in() {
                let source_uri = edge.source().key();
                let edge_data = edge.value();

                // Get the rope for UTF-16 conversion
                let source_rope = repo.document_map.get(source_uri);

                // Create a Location for each link location
                for link_loc in &edge_data.locations {
                    // Get line content for UTF-16 conversion
                    if let Some(rope) = source_rope.as_ref() {
                        if let Some(line) = rope.value().get_line(link_loc.source_line) {
                            if let Some(line_str) = line.as_str() {
                                // Convert byte offsets to UTF-16 positions for LSP
                                let start_char =
                                    utf16_from_byte_idx(line_str, link_loc.source_col_range.0)
                                        as u32;
                                let end_char =
                                    utf16_from_byte_idx(line_str, link_loc.source_col_range.1)
                                        as u32;

                                let range = Range::new(
                                    Position::new(link_loc.source_line as u32, start_char),
                                    Position::new(link_loc.source_line as u32, end_char),
                                );
                                references.push(Location::new(source_uri.clone(), range));
                            }
                        }
                    }
                }
            }

            log::debug!(
                "references retrieved from graph: {} locations",
                references.len()
            );
            Some(references)
        }
        .await;
        Ok(references)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = Repository::normalize_url_percent_encoding(&params.text_document.uri);

        let result = || -> Option<SemanticTokensResult> {
            let repo_lock = self.repository.lock().unwrap();
            let repo = repo_lock.as_ref()?;

            let ast = repo.ast_map.get(&uri)?;
            let data = get_semantic_tokens(ast.value());

            // Generate result_id based on document version (use 0 if not available)
            let version = repo
                .document_map
                .get(&uri)
                .map(|rope| rope.value().len_bytes() as i32)
                .unwrap_or(0);
            let result_id = generate_result_id(&uri, version);

            // Cache the tokens for future delta requests
            let cached = CachedSemanticTokens {
                result_id: result_id.clone(),
                tokens: data.clone(),
                document_version: version,
            };
            self.token_cache.lock().unwrap().insert(uri.clone(), cached);

            Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: Some(result_id),
                data,
            }))
        }();

        Ok(result)
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        let uri = Repository::normalize_url_percent_encoding(&params.text_document.uri);

        let result = || -> Option<SemanticTokensRangeResult> {
            let repo_lock = self.repository.lock().unwrap();
            let repo = repo_lock.as_ref()?;

            let ast = repo.ast_map.get(&uri)?;
            let data = get_semantic_tokens_range(
                ast.value(),
                params.range.start.line,
                params.range.end.line,
            );

            Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
                result_id: None,
                data,
            }))
        }();

        Ok(result)
    }

    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> Result<Option<SemanticTokensFullDeltaResult>> {
        let uri = Repository::normalize_url_percent_encoding(&params.text_document.uri);

        let result = || -> Option<SemanticTokensFullDeltaResult> {
            let repo_lock = self.repository.lock().unwrap();
            let repo = repo_lock.as_ref()?;

            let ast = repo.ast_map.get(&uri)?;
            let new_tokens = get_semantic_tokens(ast.value());

            // Generate new result_id
            let version = repo
                .document_map
                .get(&uri)
                .map(|rope| rope.value().len_bytes() as i32)
                .unwrap_or(0);
            let new_result_id = generate_result_id(&uri, version);

            // Check if we have cached tokens with the requested previous result_id
            let cache_lock = self.token_cache.lock().unwrap();
            let cached = cache_lock.get(&uri);

            let delta_result = if let Some(cached_tokens) = cached {
                if cached_tokens.result_id == params.previous_result_id {
                    // Compute delta
                    let edits = compute_semantic_tokens_delta(&cached_tokens.tokens, &new_tokens);

                    // Drop cache lock before updating it
                    drop(cache_lock);

                    // Update cache with new tokens
                    let new_cached = CachedSemanticTokens {
                        result_id: new_result_id.clone(),
                        tokens: new_tokens.clone(),
                        document_version: version,
                    };
                    self.token_cache
                        .lock()
                        .unwrap()
                        .insert(uri.clone(), new_cached);

                    // Return delta
                    Some(SemanticTokensFullDeltaResult::TokensDelta(
                        SemanticTokensDelta {
                            result_id: Some(new_result_id),
                            edits,
                        },
                    ))
                } else {
                    // Previous result_id doesn't match, return full tokens
                    drop(cache_lock);

                    let new_cached = CachedSemanticTokens {
                        result_id: new_result_id.clone(),
                        tokens: new_tokens.clone(),
                        document_version: version,
                    };
                    self.token_cache
                        .lock()
                        .unwrap()
                        .insert(uri.clone(), new_cached);

                    Some(SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                        result_id: Some(new_result_id),
                        data: new_tokens,
                    }))
                }
            } else {
                // No cached tokens, return full tokens
                drop(cache_lock);

                let new_cached = CachedSemanticTokens {
                    result_id: new_result_id.clone(),
                    tokens: new_tokens.clone(),
                    document_version: version,
                };
                self.token_cache
                    .lock()
                    .unwrap()
                    .insert(uri.clone(), new_cached);

                Some(SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                    result_id: Some(new_result_id),
                    data: new_tokens,
                }))
            };

            delta_result
        }();

        Ok(result)
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = Repository::normalize_url_percent_encoding(&params.text_document.uri);
        let position = params.position;

        let prepare_result = || -> Option<PrepareRenameResponse> {
            let repo_lock = self.repository.lock().unwrap();
            let repo = repo_lock.as_ref()?;
            let ast = repo.ast_map.get(&uri)?;
            let rope = repo.document_map.get(&uri)?;

            let line = rope.value().get_line(position.line as usize)?;
            let line_str = line.as_str()?;
            let posbyte = utf16_to_byte_idx(line_str, position.character as usize);

            // Try to find anchor definition at cursor
            if let Some((anchor_name, anchor_loc)) =
                find_anchor_at_position(&ast, position.line as usize, posbyte)
            {
                // Return range of the anchor name (excluding # prefix for short form, or {@anchor } for long form)
                // The location includes the full anchor expression
                let start_char = utf16_from_byte_idx(line_str, anchor_loc.span.0) as u32;
                let end_char = utf16_from_byte_idx(line_str, anchor_loc.span.1) as u32;

                let range = Range::new(
                    Position::new(position.line, start_char),
                    Position::new(position.line, end_char),
                );

                return Some(PrepareRenameResponse::RangeWithPlaceholder {
                    range,
                    placeholder: anchor_name,
                });
            }

            // Try to find WikiLink at cursor
            if let Some(node_route) = locate_node_route(&ast, position.line as usize, posbyte) {
                for node in &node_route {
                    if let AstNodeKind::WikiLink { link, .. } = &node.kind() {
                        // Return range of the link name (excluding anchor and brackets)
                        // The node range includes brackets, we need to extract just the link text
                        let loc = node.location();
                        let link_start = loc.span.0 + 1; // Skip '['
                        let link_end = link_start + link.len();

                        let start_char = utf16_from_byte_idx(line_str, link_start) as u32;
                        let end_char = utf16_from_byte_idx(line_str, link_end) as u32;

                        let range = Range::new(
                            Position::new(position.line, start_char),
                            Position::new(position.line, end_char),
                        );

                        return Some(PrepareRenameResponse::RangeWithPlaceholder {
                            range,
                            placeholder: link.to_string(),
                        });
                    }
                }
            }

            // If not on a WikiLink, allow renaming the current file
            // Get the file name from the URI
            if let Ok(path) = uri.to_file_path() {
                if let Some(file_stem) = path.file_stem() {
                    if let Some(name) = file_stem.to_str() {
                        // Return a synthetic range at the beginning of the file
                        let range = Range::new(Position::new(0, 0), Position::new(0, 0));

                        return Some(PrepareRenameResponse::RangeWithPlaceholder {
                            range,
                            placeholder: name.to_string(),
                        });
                    }
                }
            }

            None
        }();

        Ok(prepare_result)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = Repository::normalize_url_percent_encoding(
            &params.text_document_position.text_document.uri,
        );
        let position = params.text_document_position.position;
        let new_name = params.new_name.trim();

        // Validate new name
        if new_name.is_empty() {
            return Err(tower_lsp::jsonrpc::Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: "Name cannot be empty".into(),
                data: None,
            });
        }

        // Check if we're renaming an anchor
        let anchor_rename_result = || -> Option<WorkspaceEdit> {
            let repo_lock = self.repository.lock().unwrap();
            let repo = repo_lock.as_ref()?;
            let ast = repo.ast_map.get(&uri)?;
            let rope = repo.document_map.get(&uri)?;

            let line = rope.value().get_line(position.line as usize)?;
            let line_str = line.as_str()?;
            let posbyte = utf16_to_byte_idx(line_str, position.character as usize);

            // Check if cursor is on an anchor definition
            let (old_anchor_name, anchor_loc) =
                find_anchor_at_position(&ast, position.line as usize, posbyte)?;

            log::info!("Renaming anchor '{}' to '{}'", old_anchor_name, new_name);

            // Validate anchor name (similar rules to note names but allow # prefix)
            let clean_new_name = new_name.trim_start_matches('#');
            if clean_new_name.is_empty() {
                return None;
            }
            if clean_new_name.contains('/')
                || clean_new_name.contains('\\')
                || clean_new_name.contains('#')
            {
                return None;
            }

            let mut document_changes = Vec::new();

            // Get the current file's link name for finding references
            let current_file_link = if let Ok(path) = uri.to_file_path() {
                repo.path_to_link(&path)?
            } else {
                return None;
            };

            // 1. Update the anchor definition in the current file
            // The anchor definition can be in two forms:
            // - Short form: #anchor_name (span includes #)
            // - Long form: {@anchor anchor_name} (span includes the whole expression)
            let anchor_text = &line_str[anchor_loc.span.0..anchor_loc.span.1];
            let new_anchor_text = if anchor_text.starts_with("{@anchor") {
                format!("{{@anchor {}}}", clean_new_name)
            } else {
                // Short form #anchor
                format!("#{}", clean_new_name)
            };

            let start_char = utf16_from_byte_idx(line_str, anchor_loc.span.0) as u32;
            let end_char = utf16_from_byte_idx(line_str, anchor_loc.span.1) as u32;

            let anchor_edit = TextEdit {
                range: Range::new(
                    Position::new(anchor_loc.row as u32, start_char),
                    Position::new(anchor_loc.row as u32, end_char),
                ),
                new_text: new_anchor_text,
            };

            document_changes.push(DocumentChangeOperation::Edit(TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: None,
                },
                edits: vec![OneOf::Left(anchor_edit)],
            }));

            // 2. Find all links in the repository that reference this file with this anchor
            if let Ok(graph) = repo.document_graph.lock() {
                if let Some(target_node) = graph.get(&uri) {
                    // Iterate through all incoming edges (links pointing to this file)
                    for edge in target_node.iter_in() {
                        let source_uri = edge.source().key();
                        let edge_data = edge.value();

                        // Get source document rope for line access
                        let source_rope = repo.document_map.get(source_uri)?;

                        let mut edits = Vec::new();

                        // Create TextEdit for each link location that references this anchor
                        for link_loc in &edge_data.locations {
                            if link_loc.target_anchor.as_ref() == Some(&old_anchor_name) {
                                if let Some(line) =
                                    source_rope.value().get_line(link_loc.source_line)
                                {
                                    if let Some(src_line_str) = line.as_str() {
                                        // Build new link text with updated anchor
                                        let new_link_text =
                                            format!("[{}#{}]", current_file_link, clean_new_name);

                                        // Convert byte offsets to UTF-16
                                        let start_char = utf16_from_byte_idx(
                                            src_line_str,
                                            link_loc.source_col_range.0,
                                        )
                                            as u32;
                                        let end_char = utf16_from_byte_idx(
                                            src_line_str,
                                            link_loc.source_col_range.1,
                                        )
                                            as u32;

                                        let range = Range::new(
                                            Position::new(link_loc.source_line as u32, start_char),
                                            Position::new(link_loc.source_line as u32, end_char),
                                        );

                                        edits.push(OneOf::Left(TextEdit {
                                            range,
                                            new_text: new_link_text,
                                        }));
                                    }
                                }
                            }
                        }

                        if !edits.is_empty() {
                            document_changes.push(DocumentChangeOperation::Edit(
                                TextDocumentEdit {
                                    text_document: OptionalVersionedTextDocumentIdentifier {
                                        uri: source_uri.clone(),
                                        version: None,
                                    },
                                    edits,
                                },
                            ));
                        }
                    }
                }
            }

            Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(document_changes)),
                ..Default::default()
            })
        }();

        // If anchor rename succeeded, return it
        if anchor_rename_result.is_some() {
            return Ok(anchor_rename_result);
        }

        // Otherwise, try note renaming (existing logic)
        if new_name.contains('/') || new_name.contains('\\') {
            return Err(tower_lsp::jsonrpc::Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: "Note name cannot contain path separators".into(),
                data: None,
            });
        }

        if new_name.ends_with(".pn") {
            return Err(tower_lsp::jsonrpc::Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: "Note name should not include .pn extension".into(),
                data: None,
            });
        }

        let rename_result = || -> Option<WorkspaceEdit> {
            let repo_lock = self.repository.lock().unwrap();
            let repo = repo_lock.as_ref()?;
            let ast = repo.ast_map.get(&uri)?;
            let rope = repo.document_map.get(&uri)?;

            // Find what's being renamed
            let line = rope.value().get_line(position.line as usize)?;
            let line_str = line.as_str()?;
            let posbyte = utf16_to_byte_idx(line_str, position.character as usize);

            let old_name = if let Some(node_route) =
                locate_node_route(&ast, position.line as usize, posbyte)
            {
                // Find WikiLink in the route
                node_route.iter().find_map(|node| {
                    if let AstNodeKind::WikiLink { link, .. } = &node.kind() {
                        Some(link.clone())
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            // If no WikiLink found at cursor, rename the current file
            let old_name = if let Some(name) = old_name {
                name
            } else {
                // Get the current file name
                if let Ok(path) = uri.to_file_path() {
                    if let Some(file_stem) = path.file_stem() {
                        if let Some(name) = file_stem.to_str() {
                            name.to_string()
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            };

            log::info!("Renaming note '{}' to '{}'", old_name, new_name);

            // Check if new name conflicts
            let root_uri = self.root_uri.lock().unwrap().as_ref().cloned()?;
            if let Some(new_uri) = repo.link_to_uri(new_name, &root_uri) {
                if let Ok(new_path) = new_uri.to_file_path() {
                    if new_path.exists() {
                        log::warn!("Target file already exists: {:?}", new_path);
                        return None;
                    }
                }
            }

            // Find all references to the old note
            let old_uri = repo.link_to_uri(&old_name, &root_uri)?;

            // Check if the target file actually exists
            if let Ok(old_path) = old_uri.to_file_path() {
                if !old_path.exists() {
                    log::warn!("Target file does not exist: {:?}", old_path);
                    return None;
                }
            }

            let mut document_changes = Vec::new();

            // Collect all references and build text edits
            if let Ok(graph) = repo.document_graph.lock() {
                if let Some(target_node) = graph.get(&old_uri) {
                    // Iterate through all incoming edges
                    for edge in target_node.iter_in() {
                        let source_uri = edge.source().key();
                        let edge_data = edge.value();

                        // Get source document rope for line access
                        let source_rope = repo.document_map.get(source_uri)?;

                        let mut edits = Vec::new();

                        // Create TextEdit for each link location
                        for link_loc in &edge_data.locations {
                            if let Some(line) = source_rope.value().get_line(link_loc.source_line) {
                                if let Some(line_str) = line.as_str() {
                                    // Build new link text preserving anchor
                                    let new_link_text =
                                        if let Some(ref anchor_name) = link_loc.target_anchor {
                                            format!("[{}#{}]", new_name, anchor_name)
                                        } else {
                                            format!("[{}]", new_name)
                                        };

                                    // Convert byte offsets to UTF-16
                                    let start_char =
                                        utf16_from_byte_idx(line_str, link_loc.source_col_range.0)
                                            as u32;
                                    let end_char =
                                        utf16_from_byte_idx(line_str, link_loc.source_col_range.1)
                                            as u32;

                                    let range = Range::new(
                                        Position::new(link_loc.source_line as u32, start_char),
                                        Position::new(link_loc.source_line as u32, end_char),
                                    );

                                    edits.push(OneOf::Left(TextEdit {
                                        range,
                                        new_text: new_link_text,
                                    }));
                                }
                            }
                        }

                        if !edits.is_empty() {
                            document_changes.push(DocumentChangeOperation::Edit(
                                TextDocumentEdit {
                                    text_document: OptionalVersionedTextDocumentIdentifier {
                                        uri: source_uri.clone(),
                                        version: None,
                                    },
                                    edits,
                                },
                            ));
                        }
                    }
                }
            }

            // Add file rename operation
            let new_uri = repo.link_to_uri(new_name, &root_uri)?;
            document_changes.push(DocumentChangeOperation::Op(ResourceOp::Rename(
                RenameFile {
                    old_uri: old_uri.clone(),
                    new_uri: new_uri.clone(),
                    options: Some(RenameFileOptions {
                        overwrite: Some(false),
                        ignore_if_exists: Some(false),
                    }),
                    annotation_id: None,
                },
            )));

            Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(document_changes)),
                ..Default::default()
            })
        }();

        if rename_result.is_none() {
            return Err(tower_lsp::jsonrpc::Error {
                code: tower_lsp::jsonrpc::ErrorCode::InternalError,
                message: "Failed to prepare rename operation".into(),
                data: None,
            });
        }

        Ok(rename_result)
    }
}

use clap::Parser as ClapParser;
use clap_verbosity_flag::{InfoLevel, Verbosity};

#[derive(ClapParser)]
#[command(version, about, long_about=None)]
struct Cli {
    /// an input file to parse
    #[arg(short, long, value_name = "FILE")]
    debuglogfile: Option<PathBuf>,
    /// an output html file
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,
}

fn init_logger(filter_level: log::LevelFilter, logfile: Option<PathBuf>) {
    let mut loggers = Vec::new();
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
    let (service, socket) = LspService::new(move |client| Backend {
        client,
        repository: Arc::new(Mutex::new(None)),
        root_uri: Arc::new(Mutex::new(None)),
        paper_catalog: shared_catalog.clone(),
        settings: Arc::new(Mutex::new(PattoSettings::default())),
        token_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
    });
    log::info!("Patto Language Server Protocol started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Patto Language Server Protocol exits");
}
