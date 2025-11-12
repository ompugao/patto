use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use urlencoding::decode;

use str_indices::utf16::{from_byte_idx as utf16_from_byte_idx, to_byte_idx as utf16_to_byte_idx};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use patto::parser::{self, AstNode, AstNodeKind, Deadline, ParserResult, Property, TaskStatus};
use patto::repository::{Repository, RepositoryMessage};
use patto::semantic_token::LEGEND_TYPE;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

//#[derive(Debug)]
struct Backend {
    client: Client,
    repository: Arc<Mutex<Option<Repository>>>,
    root_uri: Arc<Mutex<Option<Url>>>,
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
    let diagnostics: Vec<Diagnostic> = parse_errors
        .into_iter()
        .map(|item| {
            let (message, loc) = match item {
                parser::ParserError::InvalidIndentation(loc) => {
                    (format!("Invalid indentation:\n{}", loc), loc.clone())
                }
                parser::ParserError::ParseError(loc, mes) => {
                    (format!("Failed to parse: {}", mes), loc.clone())
                }
            };

            let start_position = Position::new(loc.row as u32, loc.span.0 as u32);
            let end_position = Position::new(loc.row as u32, loc.span.1 as u32);
            Diagnostic::new(
                Range::new(start_position, end_position),
                Some(DiagnosticSeverity::ERROR),
                None,
                None,
                message,
                None,
                None,
            )
        })
        .collect();
    (ast, diagnostics)
}

fn gather_anchors(parent: &AstNode, anchors: &mut Vec<String>) {
    if let AstNodeKind::Line { ref properties } = &parent.kind() {
        for prop in properties {
            if let Property::Anchor { name } = prop {
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
            if let Property::Task { status, due } = prop {
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
            if let Property::Anchor { name } = prop {
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

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        let uri = Repository::normalize_url_percent_encoding(&params.uri);

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
                            let _ = client.send_notification::<notification::Progress>(
                                ProgressParams {
                                    token: token.clone(),
                                    value: ProgressParamsValue::WorkDone(
                                        WorkDoneProgress::Begin(WorkDoneProgressBegin {
                                            title: "Scanning notes".to_string(),
                                            message: Some(format!("0/{} files", total_files)),
                                            percentage: Some(0),
                                            cancellable: Some(false),
                                        })
                                    ),
                                }
                            ).await;
                            progress_active = true;
                            
                            client.log_message(
                                MessageType::INFO,
                                format!("Starting to scan {} patto files", total_files)
                            ).await;
                        }
                        
                        RepositoryMessage::ScanProgress { scanned, total } => {
                            if progress_active {
                                let percentage = if total > 0 {
                                    ((scanned * 100) / total) as u32
                                } else {
                                    0
                                };
                                
                                let _ = client.send_notification::<notification::Progress>(
                                    ProgressParams {
                                        token: token.clone(),
                                        value: ProgressParamsValue::WorkDone(
                                            WorkDoneProgress::Report(WorkDoneProgressReport {
                                                message: Some(format!("{}/{} files", scanned, total)),
                                                percentage: Some(percentage),
                                                cancellable: Some(false),
                                            })
                                        ),
                                    }
                                ).await;
                            }
                        }
                        
                        RepositoryMessage::ScanCompleted { total_files } => {
                            if progress_active {
                                let _ = client.send_notification::<notification::Progress>(
                                    ProgressParams {
                                        token: token.clone(),
                                        value: ProgressParamsValue::WorkDone(
                                            WorkDoneProgress::End(WorkDoneProgressEnd {
                                                message: Some("Complete".to_string()),
                                            })
                                        ),
                                    }
                                ).await;
                                progress_active = false;
                            }
                            
                            client.log_message(
                                MessageType::INFO,
                                format!("Scan completed: {} files indexed", total_files)
                            ).await;
                        }
                        
                        _ => {}
                    }
                }
            });
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
                                        language: Some("pn".to_string()),
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
                                full: Some(SemanticTokensFullOptions::Bool(true)),
                            },
                            static_registration_options: StaticRegistrationOptions::default(),
                        },
                    ),
                ),
                // definition: Some(GotoCapability::default()),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(false)),
                ..ServerCapabilities::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "patto-lsp server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
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
        let completions = || -> Option<Vec<CompletionItem>> {
            if let Some(repo) = self.repository.lock().unwrap().as_ref() {
                let rope = repo.document_map.get(&uri)?;
                // NOTE: trigger_character is not supported by vim-lsp.
                // we manually consider the context.
                // if let Some(context) = params.context {
                //     match context.trigger_character.as_deref() {
                //         Some("[") => {
                //             if let Some(root_uri) = self.root_uri.lock().unwrap().as_ref() {
                //                 let files = self.document_map.iter().map(|e| {
                //                     let mut path = decode(&e.key()[root_uri.to_string().len()+1..]).unwrap().to_string();
                //                     if path.ends_with(".pn") {
                //                         path = path.strip_suffix(".pn").unwrap().to_string();
                //                     }
                //                     CompletionItem {
                //                         label: path.clone(),
                //                         kind: Some(CompletionItemKind::FILE),
                //                         filter_text: Some(path.clone()),
                //                         insert_text: Some(path),
                //                         ..Default::default()
                //                     }
                //                 })
                //                 .collect();
                //                 return Some(files);
                //             }
                //         }
                //         Some("@") => {
                //             let commands = vec!["code", "math", "table", "quote", "img"];
                //             let completions = commands
                //                 .iter()
                //                 .map(|x| {
                //                     let command = x.to_string();
                //                     CompletionItem {
                //                         label: command.clone(),
                //                         kind: Some(CompletionItemKind::FUNCTION),
                //                         filter_text: Some(command.clone()),
                //                         insert_text: Some(command),
                //                         ..Default::default()
                //                     }
                //                 })
                //                 .collect();
                //             return Some(completions);
                //         }
                //         _ => {

                //         }
                //     }
                // }

                // match line.char((position.character as usize).saturating_sub(1)) {
                //     '[' => {
                //         if let Some(root_uri) = self.root_uri.lock().unwrap().as_ref() {
                //             let files = self.document_map.iter().map(|e| {
                //                 let mut path = decode(&e.key()[root_uri.to_string().len()+1..]).unwrap().to_string();
                //                 if path.ends_with(".pn") {
                //                     path = path.strip_suffix(".pn").unwrap().to_string();
                //                 }
                //                 CompletionItem {
                //                     label: path.clone(),
                //                     kind: Some(CompletionItemKind::FILE),
                //                     filter_text: Some(path.clone()),
                //                     insert_text: Some(path),
                //                     ..Default::default()
                //                 }
                //             })
                //             .collect();
                //             return Some(files);
                //         }
                //     },
                //     c => {
                //         //log::info!("{}", c);
                //     }
                // }

                let line = rope.get_line(position.line as usize)?;
                let cur_col = line.byte_to_char(utf16_to_byte_idx(
                    line.as_str()?,
                    position.character as usize,
                ));
                let prev_col = cur_col.saturating_sub(1);
                //let prev_col_byte = line.char_to_byte(prev_col);
                let c = line.char(prev_col);
                if c == '#' {
                    let slice = line.slice(..cur_col);
                    if let Some(foundbracket) =
                        slice.chars_at(cur_col).reversed().position(|c| c == '[')
                    {
                        let maybelink = slice.len_chars().saturating_sub(foundbracket); // -1 since the cursor at first points to the end of the line `\n`.
                        let s = line.slice(maybelink..prev_col).as_str()?;
                        log::debug!("link? {}, from {}, found at {}", s, maybelink, foundbracket);
                        let Some(root_uri) = self.root_uri.lock().unwrap().as_ref().cloned() else {
                            log::debug!("root_uri is not set");
                            return None;
                        };
                        let linkuri = repo.link_to_uri(s, &root_uri).unwrap_or(uri);
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
                if let Some(foundbracket) =
                    slice.chars_at(cur_col).reversed().position(|c| c == '[')
                {
                    let maybelink = slicelen.saturating_sub(foundbracket);
                    let s = line.slice(maybelink..cur_col).as_str()?;
                    log::debug!(
                        "matching {}, from {}, found at {}",
                        s,
                        maybelink,
                        foundbracket
                    );

                    //if let Some(root_uri) = self.root_uri.lock().unwrap().as_ref()
                    if let Some(root_uri_str) = self
                        .root_uri
                        .lock()
                        .unwrap()
                        .as_ref()
                        .and_then(|root_uri| root_uri.to_file_path().ok())
                    {
                        let baselen = root_uri_str.to_string_lossy().len();
                        let matcher = SkimMatcherV2::default();
                        let files = repo
                            .document_map
                            .iter()
                            .filter_map(|e| {
                                //// this is a bit slow
                                //let Some(mut path) = uri_to_link(&e.key(), &root_uri) else {
                                //    return None;
                                //};
                                let mut path = decode(
                                    &e.key().to_file_path().unwrap().to_string_lossy()
                                        [baselen + 1..],
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
                                            range: Range {
                                                start: Position {
                                                    line: position.line,
                                                    character: utf16_from_byte_idx(
                                                        line.as_str()?,
                                                        line.char_to_byte(maybelink),
                                                    )
                                                        as u32,
                                                },
                                                end: Position {
                                                    line: position.line,
                                                    character: position.character,
                                                },
                                            },
                                        })),
                                        ..Default::default()
                                    });
                                }
                                None
                            })
                            .collect();
                        //log::info!("files: {:?}", files);
                        return Some(files);
                    }
                }

                // `line.slice(..position.character as usize).chars().reversed().position(|c| c == '@') { ...` does not work,
                // because, in ropery, iterator is a cursor, and `reversed` just changes the moving direction and does not move its position at the end of the elements.
                // see https://docs.rs/ropey/latest/ropey/iter/index.html#a-possible-point-of-confusion
                //     https://github.com/cessen/ropey/issues/93
                let slice = line.slice(..cur_col);
                let slicelen = slice.len_chars();
                if let Some(foundat) = slice.chars_at(cur_col).reversed().position(|c| c == '@') {
                    // `chars_at` puts the cursor at the end of the line.
                    let maybecommand = slicelen.saturating_sub(foundat); // -1 since the cursor at first points to the end of the line `\n`.
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
                                //insert_text: Some("[@code ${1:lang}]$0".to_string()),
                                insert_text_format: Some(InsertTextFormat::SNIPPET),
                                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                    new_text: "[@code ${1:lang}]$0".to_string(),
                                    range: Range {
                                        start: Position {
                                            line: position.line,
                                            character: utf16_from_byte_idx(
                                                line.as_str()?,
                                                line.char_to_byte(maybecommand),
                                            )
                                                as u32,
                                        },
                                        end: Position {
                                            line: position.line,
                                            character: position.character,
                                        },
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
                                                line.as_str()?,
                                                line.char_to_byte(maybecommand),
                                            )
                                                as u32,
                                        },
                                        end: Position {
                                            line: position.line,
                                            character: position.character,
                                        },
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
                                                line.as_str()?,
                                                line.char_to_byte(maybecommand),
                                            )
                                                as u32,
                                        },
                                        end: Position {
                                            line: position.line,
                                            character: position.character,
                                        },
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
                                                line.as_str()?,
                                                line.char_to_byte(maybecommand),
                                            )
                                                as u32,
                                        },
                                        end: Position {
                                            line: position.line,
                                            character: position.character,
                                        },
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
                                    ), // T%H:%M:%S
                                    range: Range {
                                        start: Position {
                                            line: position.line,
                                            character: utf16_from_byte_idx(
                                                line.as_str()?,
                                                line.char_to_byte(maybecommand),
                                            )
                                                as u32,
                                        },
                                        end: Position {
                                            line: position.line,
                                            character: position.character,
                                        },
                                    },
                                })),
                                ..Default::default()
                            };
                            return Some(vec![item]);
                        }
                        &_ => {}
                    }
                }
                None
            } else {
                None
            }
        }();
        //log::debug!("completions: {:?}", completions);
        Ok(completions.map(CompletionResponse::Array))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(
                MessageType::INFO,
                format!("command executed!: {:?}", params),
            )
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
            //TODO record and use range
            let start = Range::new(Position::new(0, 0), Position::new(0, 1));
            log::debug!("references retrieved from graph");
            Some(
                node.iter_in()
                    .map(|e| Location::new(e.source().key().clone(), start))
                    .collect::<_>(),
            )
        }
        .await;
        Ok(references)
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
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        let repository = Arc::new(Mutex::new(None)); // Root will be set in initialize
        Backend {
            client,
            repository,
            root_uri: Arc::new(Mutex::new(None)),
        }
    });
    log::info!("Patto Language Server Protocol started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Patto Language Server Protocol exits");
}
