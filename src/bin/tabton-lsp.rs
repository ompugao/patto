use log;
use std::fs::File;

use chrono;
use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use tabton::parser::{self, AstNode, ParserResult};
use tabton::semantic_token::LEGEND_TYPE;

#[derive(Debug)]
struct Backend {
    client: Client,
    ast_map: DashMap<String, AstNode>,
    document_map: DashMap<String, Rope>,
    //semantic_token_map: DashMap<String, Vec<ImCompleteSemanticToken>>,
}

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        log::info!("{}", &params.text);
        let rope = ropey::Rope::from_str(&params.text);
        self.document_map
            .insert(params.uri.to_string(), rope.clone());
        let doc = params.text.clone();
        let ParserResult { ast, parse_errors } = parser::parse_text(&doc);
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

        self.client
            .publish_diagnostics(
                params.uri.clone(),
                diagnostics.clone(),
                Some(params.version),
            )
            .await;

        //self.client.log_message(MessageType::INFO, &format!("num of diags: {}", diagnostics.len())).await;
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
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
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
                    commands: vec![],
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
                definition_provider: Some(OneOf::Left(false)),
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

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file saved!")
            .await;
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
            // if let Some(context) = params.context {
            //     if context.trigger_character.as_deref() == Some("@") {
            //         let commands = vec!["code", "math", "table", "quote", "img"];
            //         let completions = commands
            //             .iter()
            //             .map(|x| {
            //                 let command = x.to_string();
            //                 CompletionItem {
            //                     label: command.clone(),
            //                     kind: Some(CompletionItemKind::FUNCTION),
            //                     filter_text: Some(command.clone()),
            //                     insert_text: Some(command),
            //                     ..Default::default()
            //                 }
            //             })
            //             .collect();
            //         return Some(completions);
            //     }
            // }
            let line = rope.get_line(position.line as usize)?;
            let sline = line.as_str()?;
            if let Some(maybecommand) = sline[..position.character as usize].rfind('@') {
                let s = sline[maybecommand..position.character as usize].as_ref();
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
        ast_map: DashMap::new(),
        document_map: DashMap::new(),
        //semantic_token_map: DashMap::new(),
    });
    log::info!("Tabton Language Server Protocol started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Tabton Language Server Protocol exits");
}
