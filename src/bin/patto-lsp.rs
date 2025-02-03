use log;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use regex;
use urlencoding::{encode, decode};


use chrono;
use dashmap::DashMap;
use ropey::Rope;
use str_indices::utf16::{
    from_byte_idx as utf16_from_byte_idx,
    to_byte_idx as utf16_to_byte_idx
};

use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use tokio;
use tower_lsp::jsonrpc::{Result, Error};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use patto::parser::{self, AstNode, AstNodeKind, Property, ParserResult, TaskStatus, Deadline};
use patto::semantic_token::LEGEND_TYPE;

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use gdsl::sync_digraph::{Graph, Node as GraphNode};

//#[derive(Debug)]
struct Backend {
    client: Client,
    ast_map: Arc<DashMap<Url, AstNode>>,
    document_map: Arc<DashMap<Url, Rope>>,
    document_graph: Arc<Mutex<Graph<Url, AstNode, ()>>>,
    root_uri: Arc<Mutex<Option<Url>>>,
    //semantic_token_map: DashMap<String, Vec<ImCompleteSemanticToken>>,
}

fn get_node_range(from: &AstNode) -> Range {
    let row = from.location().row as u32;
    let s = utf16_from_byte_idx(from.extract_str(), from.location().span.0) as u32;
    let e = utf16_from_byte_idx(from.extract_str(), from.location().span.1) as u32;
    Range::new(Position::new(row, s), Position::new(row, e))
}

fn normalize_url_percent_encoding(url: &Url) -> Url {
    let re = regex::Regex::new(r"%[0-9a-fA-F]{2}").unwrap();
    let normalized = re.replace_all(url.as_str(), |caps: &regex::Captures| {
        caps[0].to_uppercase()
    });

    Url::parse(&normalized).unwrap_or(url.clone())
}

fn scan_directory(
    client: &Client,
    dir: PathBuf,
    document_map: Arc<DashMap<Url, Rope>>,
    document_graph: Arc<Mutex<Graph<Url, AstNode, ()>>>,
    ast_map: Arc<DashMap<Url, AstNode>>,
) -> Result<()> {
    scan_directory_impl(&client, &dir, document_map, Arc::clone(&document_graph), Arc::clone(&ast_map))?;
    let root_uri = Url::from_directory_path(&dir).unwrap();
    log::debug!("hogehoge");
    if let Ok(mut graph) = document_graph.lock() {
        log::debug!("fuga");
        ast_map.iter().for_each(|ref_multi| {
            let uri = ref_multi.key();
            log::debug!("uri:{}", uri);
            let ast = ref_multi.value();
            let mut wikilinks = vec![];
            gather_wikilinks(&ast, &mut wikilinks);
            let link_uris = wikilinks.iter()
                .filter_map(|(ref link, ref _anchor)| link_to_uri(link, &root_uri))
                .collect::<Vec<_>>();
            let ast_clone = ast.clone();
            let node = graph.get(&uri).unwrap_or_else(|| {
                let n = GraphNode::new(uri.clone(), ast_clone);
                graph.insert(n.clone());
                n
            });
            for link_uri in link_uris.iter() {
                //let from_str = uri_to_link(&uri, &root_uri).unwrap();
                //let to_str = uri_to_link(&link_uri, &root_uri).unwrap();
                log::debug!("checking from {:?} to {:?}", uri, link_uri);
                if !node.iter_out().any(|x| x.target().key() == link_uri) {
                    // if not connected, connect
                    // TODO connect nodes even when nodes are non-existent 
                    log::debug!("connecting {} to {}", uri, link_uri);
                    if let Some(linked_ast) = ast_map.get(&link_uri) {
                        node.connect(&graph.get(&link_uri).unwrap_or_else(|| {
                            let n = GraphNode::new(link_uri.clone(), linked_ast.clone());
                            graph.insert(n.clone());
                            n
                        }), ());
                    }
                }
            }
        });
    }
    Ok(())
}

fn scan_directory_impl(
    client: &Client,
    dir: &PathBuf,
    document_map: Arc<DashMap<Url, Rope>>,
    document_graph: Arc<Mutex<Graph<Url, AstNode, ()>>>,
    ast_map: Arc<DashMap<Url, AstNode>>,
) -> Result<()> {
    let Ok(paths) = std::fs::read_dir(dir) else {
        return Err(Error::internal_error());
    };

    for path in paths {
        let path = path.unwrap().path();
        if path.is_dir() {
            scan_directory_impl(client, &path,
                Arc::clone(&document_map), Arc::clone(&document_graph), Arc::clone(&ast_map))?;
        } else if path.extension().map_or(false, |ext| ext == "pn") {
            log::debug!("Found file: {:?}", path);
            if let Ok(uri) = Url::from_file_path(path.clone()) {
                let _ = std::fs::read_to_string(path.clone()).map(|x| {
                    let (ast, _diagnostics) = parse_text(&x.as_str());
                    let rope = ropey::Rope::from_str(&x.as_str());
                    log::debug!("{:?} parsed and registered", path);
                    document_map.insert(uri.clone(), rope);
                    ast_map.insert(uri, ast);
                });
            } else {
                log::error!("Failed to parse uri: {}", path.display());
            }
        }
    }
    Ok(())
}

fn link_to_uri(link: &str, root_uri: &Url) -> Option<Url> {
    if link.len() > 0 {
        let mut linkuri = root_uri.clone();
        linkuri.set_path(format!("{}{}.pn", root_uri.path(), encode(link)).as_str());
        return Some(normalize_url_percent_encoding(&linkuri))
    }
    return None;
}


fn uri_to_link(uri: &Url, base: &Url) -> Option<String> {
    if base.scheme() != uri.scheme() {
        log::debug!("Different scheme, cannot subtract: {}, {}", uri, base);
        return None;
    }

    let base_path = base.path_segments().map(|c| c.map(|segment| decode(segment).unwrap()).collect::<Vec<_>>()).unwrap_or_default();
    let uri_path = uri.path_segments().map(|c| c.map(|segment| decode(segment).unwrap()).collect::<Vec<_>>()).unwrap_or_default();

    if !uri_path.starts_with(&base_path) {
        log::debug!("uri is not inside base: {}, {}", uri, base);
        return None; // uri is not inside base
    }

    // Extract the remainder after the base path
    let relative_path = &uri_path[base_path.len()..];
    Some(relative_path.join("/"))
}

// fn encode_uri(s: &String) -> Option<Url> {
//     log::debug!("--encoding--");
//     log::debug!("s: {}", s);
//     let mut new_url = Url::from_file_path(s).ok()?;
//     log::debug!("new_url: {}", new_url);
//     new_url.set_path(new_url.path_segments()?.map(|seg| {
//         log::debug!("seg: {}", seg);
//         encode(seg).to_owned()
//     }).collect::<Vec<_>>().join("/").as_str());
//     Some(new_url)
// }

async fn scan_workspace(
    client: Client,
    workspace_path: PathBuf,
    document_map: Arc<DashMap<Url, Rope>>,
    document_graph: Arc<Mutex<Graph<Url, AstNode, ()>>>,
    ast_map: Arc<DashMap<Url, AstNode>>,
) -> Result<()> {
    client.log_message(MessageType::INFO, &format!("Scanning workspace {:?}...", workspace_path)).await;

    let client2 = client.clone();
    // Use blocking I/O in a spawned blocking task
    let _ = tokio::task::spawn_blocking(move || {
        log::debug!("Start reading dir: {:?}", workspace_path);
        let _ = scan_directory(&client, workspace_path, document_map, document_graph, ast_map);
    }).await;

    client2.log_message(MessageType::INFO, "Workspace scan complete.").await;

    Ok(())
}

fn parse_text(text: &str) -> (AstNode, Vec<Diagnostic>) {
    let ParserResult { ast, parse_errors } = parser::parse_text(text);
    let diagnostics: Vec<Diagnostic> = parse_errors
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
                Some(Diagnostic::new(
                    Range::new(start_position, end_position),
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    None,
                    message,
                    None,
                    None
                ))
            }()
        })
        .collect();
    return (ast, diagnostics);
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
                if ! matches!(status, TaskStatus::Done) {
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

fn gather_wikilinks(parent: &AstNode, wikilinks: &mut Vec<(String, Option<String>)>) {
    if let AstNodeKind::WikiLink { link, anchor } = &parent.kind() {
        wikilinks.push((link.clone(), anchor.clone()));
    }

    for content in parent.value().contents.lock().unwrap().iter() {
        gather_wikilinks(content, wikilinks);
    }

    for child in parent.value().children.lock().unwrap().iter() {
        gather_wikilinks(child, wikilinks);
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct TaskInformation {
    /// The location of this task
    pub location: Location,

    /// The text of this task
    pub text: String,

    pub message: String,
    //pub due: Deadline,
}

impl TaskInformation {
    pub fn new(location: Location, text: String, message: String) -> Self {
        Self {
            location,
            text,
            message,
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

    return parent.value().children.lock().unwrap().iter().find_map(|child| {
        return find_anchor(child, anchor);
    }).map(|x| x.clone());
}

fn locate_node_route(parent: &AstNode, row: usize, col: usize) -> Option<Vec<AstNode>> {
    if let Some(route) = locate_node_route_impl(parent, row, col){
        //route.reverse();
        return Some(route);
    }
    return None;
}

fn locate_node_route_impl(parent: &AstNode, row: usize, col: usize) -> Option<Vec<AstNode>> {
    let parentrow = parent.location().row;
    log::debug!("finding row, col ({}, {}), scanning row: {}", row, col, parentrow);
    if matches!(parent.kind(), AstNodeKind::Dummy) ||
        parentrow < row {
        for child in parent.value().children.lock().unwrap().iter() {
            if let Some(mut route) = locate_node_route_impl(child, row, col) {
                route.push(parent.clone());
                return Some(route);
            }
        }
    } else if parentrow == row {
        if parent.value().contents.lock().unwrap().len() == 0 {
            log::debug!("{:?} must be leaf", parent.extract_str());
            return Some(vec![parent.clone()]);
        }
        for content in parent.value().contents.lock().unwrap().iter() {
            if content.location().span.contains(col) {
                log::debug!("in content: {:?}, spanning ({}, {})", content.extract_str(), content.location().span.0, content.location().span.1);
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

        let uri = normalize_url_percent_encoding(&params.uri);
        self.document_map
            .insert(uri.clone(), rope);
        let (ast, diagnostics) = parse_text(&params.text);
        let mut wikilinks = vec![];
        gather_wikilinks(&ast, &mut wikilinks);
        let root_uri_opt = self.root_uri.lock().unwrap().as_ref().cloned();
        if let Some(root_uri) = root_uri_opt {
            let wikilink_uris = wikilinks.iter()
                .filter_map(|(ref link, ref _anchor)| link_to_uri(link, &root_uri)) // ignore self-link
                .collect::<Vec<_>>();
            if let Ok(mut graph) = self.document_graph.try_lock() {
                let ast_clone = ast.clone();
                let node = graph.get(&uri).unwrap_or_else(|| {
                    let n = GraphNode::new(uri.clone(), ast_clone);
                    graph.insert(n.clone());
                    n
                });
                for wikilink_uri in wikilink_uris.iter() {
                    if !node.iter_out().any(|x| x.target().key() == wikilink_uri) {
                        // if not connected, connect
                        if let Some(linked_ast) = self.ast_map.get(&wikilink_uri) {
                            node.connect(&graph.get(&wikilink_uri).unwrap_or_else(|| {
                                let n = GraphNode::new(wikilink_uri.clone(), linked_ast.clone());
                                graph.insert(n.clone());
                                n
                            }), ());
                        }
                    }
                }
                // if connected, disconnect
                for edge in node.iter_out() {
                    if !wikilink_uris.contains(edge.target().key()) {
                        let _ = edge.source().disconnect(edge.target().key());
                    }
                }
            }
        }


        self.client
            .publish_diagnostics(
                params.uri,
                diagnostics,
                Some(params.version),
            )
            .await;

        //self.client.log_message(MessageType::INFO, &format!("num of diags: {}", diagnostics.len())).await;
        //log::info!("{}", ast);
        self.ast_map.insert(uri, ast);
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
                let document_graph = Arc::clone(&self.document_graph);
                tokio::spawn(async move {
                    // Run the workspace scan in the background
                    if let Err(e) = scan_workspace(client, path, document_map, document_graph, ast_map).await {
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
                position_encoding: Some(PositionEncodingKind::UTF16),  // vscode only supports utf-16 ;(
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec!["[", "#", "@img", "@math", "@quote", "@table", "@task"].into_iter().map(ToString::to_string).collect()),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["experimental/aggregate_tasks".to_string()],
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
            .log_message(MessageType::INFO, format!("file {} saved!", param.text_document.uri.as_str()))
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.document_map.remove(&uri);
        self.ast_map.remove(&uri);
        self.client
            .log_message(MessageType::INFO, format!("file {} is closed!", uri))
            .await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = normalize_url_percent_encoding(&params.text_document_position.text_document.uri);
        let position = params.text_document_position.position;
        let completions = || -> Option<Vec<CompletionItem>> {
            let rope = self.document_map.get(&uri)?;
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
            let cur_col = line.byte_to_char(utf16_to_byte_idx(line.as_str()?, position.character as usize));
            let prev_col = cur_col.saturating_sub(1);
            //let prev_col_byte = line.char_to_byte(prev_col);
            let c = line.char(prev_col);
            if c == '#' {
                let slice = line.slice(..cur_col);
                if let Some(foundbracket) = slice.chars_at(cur_col).reversed().position(|c| c == '[') {
                    let maybelink = slice.len_chars().saturating_sub(foundbracket as usize);  // -1 since the cursor at first points to the end of the line `\n`.
                    let s = line.slice(maybelink..prev_col).as_str()?;
                    log::debug!("link? {}, from {}, found at {}", s, maybelink, foundbracket);
                    let Some(root_uri) = self.root_uri.lock().unwrap().as_ref().cloned() else {
                        log::debug!("root_uri is not set");
                        return None;
                    };
                    let linkuri = link_to_uri(s, &root_uri).unwrap_or(uri);
                    log::debug!("linkuri: {}", linkuri);
                    if let Some(ast) = self.ast_map.get(&linkuri) {
                        let mut anchors = vec![];
                        gather_anchors(ast.value(), &mut anchors);
                        return Some(anchors.iter().map(|anchor| {
                            CompletionItem {
                                label: format!("#{}", anchor).into(),
                                kind: Some(CompletionItemKind::REFERENCE),
                                filter_text: Some(anchor.to_string()),
                                insert_text: Some(anchor.to_string()),
                                ..Default::default()
                            }
                        }).collect());
                    }
                }
            }

            let slice = line.slice(..cur_col);
            let slicelen = slice.len_chars();
            if let Some(foundbracket) = slice.chars_at(cur_col).reversed().position(|c| c == '[') {
                let maybelink = slicelen.saturating_sub(foundbracket as usize);
                let s = line.slice(maybelink..cur_col).as_str()?;
                log::debug!("matching {}, from {}, found at {}", s, maybelink, foundbracket);

                //if let Some(root_uri) = self.root_uri.lock().unwrap().as_ref() 
                if let Some(root_uri_str) = self.root_uri.lock().unwrap().as_ref().and_then(|root_uri| root_uri.to_file_path().ok()) {
                    let baselen = root_uri_str.to_string_lossy().len();
                    let matcher = SkimMatcherV2::default();
                    let files = self.document_map.iter().filter_map(|e| {
                        //// this is a bit slow
                        //let Some(mut path) = uri_to_link(&e.key(), &root_uri) else {
                        //    return None;
                        //};
                        let mut path = decode(&e.key().to_file_path().unwrap().to_string_lossy()[baselen+1..]).unwrap().to_string();
                        if path.ends_with(".pn") {
                            path = path.strip_suffix(".pn").unwrap().to_string();
                        }
                        if matcher.fuzzy_match(&path, &s).is_some() {
                            return Some(CompletionItem {
                                label: format!("{}", path),
                                detail: Some(path.clone()),
                                kind: Some(CompletionItemKind::FILE),
                                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                                text_edit: Some(CompletionTextEdit::Edit(TextEdit{
                                    new_text: path.clone(),
                                    range: Range{start: Position{line: position.line, character: utf16_from_byte_idx(line.as_str()?, line.char_to_byte(maybelink)) as u32},
                                                end: Position{line: position.line, character: position.character}}
                                    })),
                                ..Default::default()
                            });
                        }
                        return None;
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
                let maybecommand = slicelen.saturating_sub(foundat as usize);  // -1 since the cursor at first points to the end of the line `\n`.
                let s = line.slice(maybecommand..cur_col).as_str()?;
                log::debug!("command? {}, from {}, found at {}", s, maybecommand, foundat);
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
                                range: Range{start: Position{line: position.line, character: utf16_from_byte_idx(line.as_str()?, line.char_to_byte(maybecommand)) as u32},
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
                                range: Range{start: Position{line: position.line, character: utf16_from_byte_idx(line.as_str()?, line.char_to_byte(maybecommand)) as u32},
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
                                range: Range{start: Position{line: position.line, character: utf16_from_byte_idx(line.as_str()?, line.char_to_byte(maybecommand)) as u32},
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
                                range: Range{start: Position{line: position.line, character: utf16_from_byte_idx(line.as_str()?, line.char_to_byte(maybecommand)) as u32},
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
                                range: Range{start: Position{line: position.line, character: utf16_from_byte_idx(line.as_str()?, line.char_to_byte(maybecommand)) as u32},
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
        //log::debug!("completions: {:?}", completions);
        Ok(completions.map(CompletionResponse::Array))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(MessageType::INFO, format!("command executed!: {:?}", params))
            .await;
 
        match params.command.as_str() {
            "experimental/aggregate_tasks" => {
                let mut tasks: Vec<(Url, AstNode, Deadline)> = vec![];
                self.ast_map.iter().for_each(|entry| {
                    let mut tasklines = vec![];
                    gather_tasks(entry.value(), &mut tasklines);
                    tasklines.into_iter().for_each(|(line, due)| {
                        tasks.push((entry.key().clone(), line.clone(), due.clone()));
                    });
                });
                tasks.sort_by_key(|(_uri, _line, due): &(_, _, Deadline)| due.clone());
                let ret = json!(tasks.iter().map(|(uri, line, _due)| {
                    //self.client.log_message(MessageType::INFO, format!("Task due on {}: {:?}", due, line)).await;
                    TaskInformation::new(Location::new(uri.clone(), get_node_range(&line)),
                                         line.extract_str().trim_start().to_string(),
                                         "".to_string())
                                         //due.clone())
                    //Location::new(Url::parse(uri).unwrap(), get_node_range(&line))
                }).collect::<Vec<_>>());
                //self.client
                //    .log_message(MessageType::INFO, format!("response: {:?}", ret))
                //    .await;
                return Ok(Some(ret));
            },
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
            let uri = normalize_url_percent_encoding(&params.text_document_position_params.text_document.uri);
            let ast = self.ast_map.get(&uri)?;
            let rope = self.document_map.get(&uri)?;

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
                if let AstNodeKind::WikiLink{link, anchor} = &n.kind() {
                    return Some((link, anchor));
                } else {
                    return None;
                }
            }) else {
                log::debug!("it is not wikilink");
                return None;
            };
            let Some(root_uri) = self.root_uri.lock().unwrap().as_ref().cloned() else {
                log::debug!("root_uri is not set");
                return None;
            };
            let linkuri = link_to_uri(link, &root_uri).unwrap_or(uri);
            let start = Range::new(Position::new(0, 0), Position::new(0, 1));
            if let Some(anchor) = anchor {
                let range = self.ast_map.get(&linkuri).and_then(|r| {
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
        }
        .await;
        Ok(definition)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let references = async {
            let uri = normalize_url_percent_encoding(&params.text_document_position.text_document.uri);
            if let Ok(graph) = self.document_graph.lock() {
                let Some(node) = graph.get(&uri) else {
                    log::debug!("node not found in the graph");
                    return None;
                };
                //TODO record and use range
                let start = Range::new(Position::new(0, 0), Position::new(0, 1));
                log::debug!("references retrieved from graph");
                return Some(node.iter_in().map(|e| Location::new(e.source().key().clone(), start)).collect::<_>());
            }
            log::debug!("failed to lock graph");
            return None;
        }
        .await;
        Ok(references)
    }

}

use clap::Parser as ClapParser;
use clap_verbosity_flag::{Verbosity, InfoLevel};

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
        loggers.push(
            simplelog::WriteLogger::new(
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

    let (service, socket) = LspService::new(|client| Backend {
        client,
        ast_map: Arc::new(DashMap::new()),
        document_map: Arc::new(DashMap::new()),
        document_graph: Arc::new(Mutex::new(Graph::new())),
        root_uri: Arc::new(Mutex::new(None)),
        //semantic_token_map: DashMap::new(),
    });
    log::info!("Patto Language Server Protocol started");
    Server::new(stdin, stdout, socket).serve(service).await;
    log::info!("Patto Language Server Protocol exits");
}
