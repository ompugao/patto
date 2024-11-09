use log;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use urlencoding::{encode, decode};


use chrono;
use dashmap::DashMap;
use ropey::Rope;

use serde_json::Value;
use tokio;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use tabton::parser::{self, AstNode, AstNodeKind, Property, ParserResult};
use tabton::semantic_token::LEGEND_TYPE;

#[derive(Debug)]
struct Backend {
    client: Client,
    ast_map: Arc<DashMap<String, AstNode>>,
    document_map: Arc<DashMap<String, Rope>>,
    root_uri: Arc<Mutex<Option<Url>>>,
    //semantic_token_map: DashMap<String, Vec<ImCompleteSemanticToken>>,
}

fn get_node_range(from: &AstNode) -> Range {
    let row = from.location().row as u32;
    let s = from.location().span.0 as u32;
    let e = from.location().span.1 as u32;
    Range::new(Position::new(row, s), Position::new(row, e))
}

async fn scan_workspace(
    client: Client,
    workspace_path: PathBuf,
    document_map: Arc<DashMap<String, Rope>>,
    ast_map: Arc<DashMap<String, AstNode>>,
) -> Result<()> {
    client.log_message(MessageType::INFO, &format!("Scanning workspace {:?}...", workspace_path)).await;

    // Use blocking I/O in a spawned blocking task
    let _ = tokio::task::spawn_blocking(move || {
        log::debug!("Start reading dir: {:?}", workspace_path);
        let paths = std::fs::read_dir(workspace_path)
            .expect("Unable to read workspace directory");

        for path in paths {
            let file_path = path.unwrap().path();
            if file_path.extension().map_or(false, |ext| ext == "tb") {
                log::info!("Found file: {:?}", file_path);
                let uri = Url::from_file_path(file_path.clone()).unwrap();
                let _ = std::fs::read_to_string(file_path).map(|x| {
                    let (ast, _diagnostics) = parse_text(&x.as_str());
                    let rope = ropey::Rope::from_str(&x.as_str());
                    let uri_s = uri.to_string();
                    document_map.insert(uri_s.clone(), rope);
                    ast_map.insert(uri_s, ast);
                });
            }
        }
        log::debug!("{:?}", ast_map);
    })
    .await;

    client.log_message(MessageType::INFO, "Workspace scan complete.").await;

    Ok(())
}

fn parse_text(text: &str) -> (AstNode, Vec<Diagnostic>) {
    let ParserResult { ast, parse_errors } = parser::parse_text(text);
    let diagnostics = parse_errors
        .into_iter()
        .filter_map(|item| {
            let (message, loc) = match item {
                parser::ParserError::InvalidIndentation(loc) => {
                    (format!("Invalid indentation:\n{}", loc), loc.clone())
                }
                parser::ParserError::ParseError(loc, mes) => {
                    (format!("Failed to parse: {}", mes), loc.clone())
                }
            };

            || -> Option<Diagnostic> {
                let start_position = Position::new(loc.row as u32, loc.span.0 as u32);
                let end_position = Position::new(loc.row as u32, loc.span.1 as u32);
                Some(Diagnostic::new_simple(
                    Range::new(start_position, end_position),
                    message,
                ))
            }()
        })
        .collect::<Vec<_>>();
    return (ast, diagnostics);
}

fn find_anchor(parent: &AstNode, anchor: &str) -> Option<AstNode> {
    if let AstNodeKind::Line { ref properties } = &parent.value().kind {
        for prop in properties {
            if let Property::Anchor { name } = prop {
                if name == anchor {
                    return Some(parent.clone());
                }
            }
        }
    }

    return parent.value().children.lock().unwrap().iter().find_map(|child| {
        return find_anchor(child, anchor);
    }).map(|x| x.clone());
}

fn locate_node_route(parent: &AstNode, row: usize, col: usize) -> Option<Vec<AstNode>> {
    if let Some(mut route) = locate_node_route_impl(parent, row, col){
        //route.reverse();
        return Some(route);
    }
    return None;
}

fn locate_node_route_impl(parent: &AstNode, row: usize, col: usize) -> Option<Vec<AstNode>> {
    let parentrow = parent.location().row;
    log::debug!("finding row {}, scanning row: {}", row, parentrow);
    if matches!(parent.value().kind, AstNodeKind::Dummy) ||
        parentrow < row {
        for (i, child) in parent.value().children.lock().unwrap().iter().enumerate() {
            if let Some(mut route) = locate_node_route_impl(child, row, col) {
                route.push(parent.clone());
                return Some(route);
            }
        }
    } else if parentrow == row {
        if parent.value().contents.lock().unwrap().len() == 0 {
            log::debug!("must be leaf");
            return Some(vec![parent.clone()]);
        }
        for content in parent.value().contents.lock().unwrap().iter() {
            if content.location().span.contains(col) {
                log::debug!("in content: ({}, {})", content.location().span.0, content.location().span.1);
                if let Some(mut route) = locate_node_route_impl(content, row, col) {
                    route.push(parent.clone());
                    return Some(route);
                }
            }
        }
    }
    return None;
}

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        let rope = ropey::Rope::from_str(&params.text);
        self.document_map
            .insert(params.uri.to_string(), rope);
        let (ast, diagnostics) = parse_text(&params.text);

        self.client
            .publish_diagnostics(
                params.uri.clone(),
                diagnostics,
                Some(params.version),
            )
            .await;

        //self.client.log_message(MessageType::INFO, &format!("num of diags: {}", diagnostics.len())).await;
        //log::info!("{}", ast);
        self.ast_map.insert(params.uri.to_string(), ast);
        // self.client
        //     .log_message(MessageType::INFO, &format!("{:?}", semantic_tokens))
        //     .await;
        // self.semantic_token_map
        //     .insert(params.uri.to_string(), semantic_tokens);
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri {
            {
                let mut backend_root_uri = self.root_uri.lock().unwrap();
                *backend_root_uri = Some(root_uri.clone());
            }
            if let Ok(path) = root_uri.to_file_path() {
                let client = self.client.clone();
                self.client.log_message(MessageType::INFO, &format!("scanning root_uri {:?}", path)).await;
                let ast_map = Arc::clone(&self.ast_map);
                let document_map = Arc::clone(&self.document_map);
                tokio::spawn(async move {
                    // Run the workspace scan in the background
                    if let Err(e) = scan_workspace(client, path, document_map, ast_map).await {
                        log::warn!("Failed to scan workspace: {:?}", e);
                    }
                });
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
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec!["[", "@img", "@math", "@quote", "@table", "@task"].into_iter().map(ToString::to_string).collect()),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["tabton-lsp.aggregate_tasks".to_string()],
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
                                        language: Some("tb".to_string()),
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
                references_provider: Some(OneOf::Left(false)),
                rename_provider: Some(OneOf::Left(false)),
                ..ServerCapabilities::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "tabton-lsp server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
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
            .log_message(MessageType::INFO, format!("file {} saved!", param.text_document.uri.as_str()))
            .await;
        // if let Some(ast) = self.ast_map.get(param.text_document.uri.as_str()) {
        //     self.client
        //         .log_message(MessageType::INFO, format!("{:?}", *ast))
        //         .await;
        // }
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.document_map.remove(&uri);
        self.ast_map.remove(&uri);
        self.client
            .log_message(MessageType::INFO, format!("file {} is closed!", uri))
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let completions = || -> Option<Vec<CompletionItem>> {
            let rope = self.document_map.get(&uri.to_string())?;
            if let Some(context) = params.context {
                match context.trigger_character.as_deref() {
                    Some("[") => {
                        if let Some(root_uri) = self.root_uri.lock().unwrap().as_ref() {
                            let files = self.document_map.iter().map(|e| {
                                let mut path = decode(&e.key()[root_uri.to_string().len()+1..]).unwrap().to_string();
                                if path.ends_with(".tb") {
                                    path = path.strip_suffix(".tb").unwrap().to_string();
                                }
                                CompletionItem {
                                    label: path.clone(),
                                    kind: Some(CompletionItemKind::FILE),
                                    filter_text: Some(path.clone()),
                                    insert_text: Some(path),
                                    ..Default::default()
                                }
                            })
                            .collect();
                            return Some(files);
                        }
                    }
                    //Some("@") => {
                    //    let commands = vec!["code", "math", "table", "quote", "img"];
                    //    let completions = commands
                    //        .iter()
                    //        .map(|x| {
                    //            let command = x.to_string();
                    //            CompletionItem {
                    //                label: command.clone(),
                    //                kind: Some(CompletionItemKind::FUNCTION),
                    //                filter_text: Some(command.clone()),
                    //                insert_text: Some(command),
                    //                ..Default::default()
                    //            }
                    //        })
                    //        .collect();
                    //    return Some(completions);
                    //}
                    _ => {

                    }
                }
            }
            let line = rope.get_line(position.line as usize)?;
            let linechars = line.slice(..position.character as usize).chars();
            log::info!("line: {:?}", linechars);
            if let Some(findat) = linechars.clone().reversed().position(|c| { c == '@'}) {
                let maybecommand = linechars.len() - findat;
                let s = line.slice(maybecommand..position.character as usize).as_str()?;
                log::info!("command {}", s);
                match s {
                    "@code" => {
                        let item = CompletionItem {
                            label: "@code".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("code command".to_string()),
                            //insert_text: Some("[@code ${1:lang}]$0".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit{
                                new_text: "[@code ${1:lang}]$0".to_string(),
                                range: Range{start: Position{line: position.line, character: (maybecommand) as u32},
                                             end: Position{line: position.line, character: position.character}}
                                })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    },
                    "@math" => {
                        let item = CompletionItem {
                            label: "@math".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("math command".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit{
                                new_text: "[@math]$0".to_string(),
                                range: Range{start: Position{line: position.line, character: (maybecommand) as u32},
                                             end: Position{line: position.line, character: position.character}}
                                })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    },
                    "@quote" => {
                        let item = CompletionItem {
                            label: "@quote".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("quote command".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit{
                                new_text: "[@quote]$0".to_string(),
                                range: Range{start: Position{line: position.line, character: (maybecommand) as u32},
                                             end: Position{line: position.line, character: position.character}}
                                })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    },
                    "@img" => {
                        let item = CompletionItem {
                            label: "@img".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("img command".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit{
                                new_text: "[@img ${1:path} \"${2:alt_text}\"]$0".to_string(),
                                range: Range{start: Position{line: position.line, character: (maybecommand) as u32},
                                             end: Position{line: position.line, character: position.character}}
                                })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    },
                    "@task" => {
                        let item = CompletionItem {
                            label: "@task".to_string(),
                            kind: Some(CompletionItemKind::SNIPPET),
                            detail: Some("task property".to_string()),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit{
                                new_text: format!("{{@task status=${{1:todo}} due=${{2:{}}}}}$0", chrono::Local::now().format("%Y-%m-%d")),  // T%H:%M:%S
                                range: Range{start: Position{line: position.line, character: (maybecommand) as u32},
                                             end: Position{line: position.line, character: position.character}}
                                })),
                            ..Default::default()
                        };
                        return Some(vec![item]);
                    },
                    &_ => {},
                }
            }
            return None;
        }();
        log::debug!("completions: {:?}", completions);
        Ok(completions.map(CompletionResponse::Array))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        match params.command.as_str() {
            "tabton-lsp.aggregate_tasks" => {
                let _: Vec<_> = self.ast_map.iter().map(|e| {
                    //TODO show results in panel. this requires client-side development.
                    log::info!("{:?}, {:?}", e.key(), e.value());
                    Some(1)
                }).collect();
            },
            c => {
                log::info!("unknown command: {}", c);
            }
        }
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let definition = async {
            let uri = params.text_document_position_params.text_document.uri;
            let ast = self.ast_map.get(uri.as_str())?;
            // let rope = self.document_map.get(uri.as_str())?;

            let position = params.text_document_position_params.position;
            // let char = rope.try_line_to_char(position.line as usize).ok()?;
            // self.client.log_message(MessageType::INFO, &format!("{:#?}, {}", ast.value(), offset)).await;
            let Some(node_route) = locate_node_route(&ast, position.line as usize, position.character as usize) else {
                return None;
            };
            //if node_route.len() == 0 {
            //    log::info!("-- route.len() is 0");
            //    return None;
            //
            let Some((link, anchor)) = node_route.iter().find_map(|n| {
                if let AstNodeKind::WikiLink{link, anchor} = &n.value().kind {
                    return Some((link, anchor));
                } else {
                    return None;
                }
            }) else {
                log::debug!("it is not wikilink");
                return None;
            };
            if let Some(root_uri) = self.root_uri.lock().unwrap().as_ref() {
                //let linkuri = root_uri.join(format!("{}.{}", link, "tb").as_str()).expect("url join should work");
                let mut linkuri = root_uri.clone();
                linkuri.set_path(format!("{}/{}.tb", root_uri.path(), encode(link)).as_str());
                let start = Range::new(Position::new(0, 0), Position::new(0, 1));
                if let Some(anchor) = anchor {
                    let range = self.ast_map.get(linkuri.as_str()).and_then(|r| {
                        let linkast = r.value();
                        find_anchor(linkast, anchor)
                    }).map_or(start,
                        |anchored_line| {
                            get_node_range(&anchored_line)
                        });
                    Some(GotoDefinitionResponse::Scalar(Location::new(linkuri, range)))
                } else {
                    Some(GotoDefinitionResponse::Scalar(Location::new(linkuri, start)))
                }
            } else {
                log::debug!("root_uri is not set");
                None
            }
        }
        .await;
        Ok(definition)
    }

}

fn init_logger() {
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

    let (service, socket) = LspService::new(|client| Backend {
        client,
        ast_map: Arc::new(DashMap::new()),
        document_map: Arc::new(DashMap::new()),
        root_uri: Arc::new(Mutex::new(None)),
        //semantic_token_map: DashMap::new(),
    });
    log::info!("Tabton Language Server Protocol started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Tabton Language Server Protocol exits");
}
