//! The `LanguageServer` implementation. Each request is unwrapped here and
//! delegated to the sibling module that owns the feature.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use serde::Deserialize;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::lsp::capabilities::server_capabilities;
use crate::lsp::folding::collect_folding_ranges;
use crate::lsp::paper::PaperCatalog;
use crate::lsp::semantic_token::{get_semantic_tokens, get_semantic_tokens_range};
use crate::parser::AstNode;
use crate::repository::Repository;
use crate::task::TaskSnapshot;

/// Settings sent by the client through `workspace/didChangeConfiguration`.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PattoSettings {
    #[serde(default)]
    pub(super) markdown: MarkdownSettings,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownSettings {
    /// `standard`, `obsidian` or `github`.
    #[serde(default)]
    pub(super) default_flavor: Option<String>,
}

pub struct Backend {
    pub(super) client: Client,
    /// Set once the client sends `initialize` with a workspace root.
    pub(super) repository: Arc<Mutex<Option<Repository>>>,
    pub(super) root_uri: Arc<Mutex<Option<Url>>>,
    pub(super) paper_catalog: PaperCatalog,
    pub(super) settings: Arc<Mutex<PattoSettings>>,
    /// Last successfully parsed task snapshot per file, keyed by row.
    /// Retained across keystrokes so that mid-edit parse failures (e.g. `status=`)
    /// don't lose the `Doing` state needed to compute elapsed time on clock-out.
    pub(super) last_valid_task_snapshots: Arc<DashMap<Url, HashMap<usize, TaskSnapshot>>>,
}

impl Backend {
    /// The workspace root is not known yet; it arrives with `initialize`.
    pub fn new(client: Client, paper_catalog: PaperCatalog) -> Self {
        Self {
            client,
            repository: Arc::new(Mutex::new(None)),
            root_uri: Arc::new(Mutex::new(None)),
            paper_catalog,
            settings: Arc::new(Mutex::new(PattoSettings::default())),
            last_valid_task_snapshots: Arc::new(DashMap::new()),
        }
    }

    /// Parsed AST of a document the workspace knows about.
    pub fn document_ast(&self, uri: &Url) -> Option<AstNode> {
        self.with_document_ast(uri, AstNode::clone)
    }

    /// Task snapshots from the last successful parse of a document.
    pub fn task_snapshots(&self, uri: &Url) -> Option<HashMap<usize, TaskSnapshot>> {
        let uri = Repository::normalize_url_percent_encoding(uri);
        self.last_valid_task_snapshots
            .get(&uri)
            .map(|entry| entry.value().clone())
    }

    fn with_document_ast<T>(&self, uri: &Url, read: impl FnOnce(&AstNode) -> T) -> Option<T> {
        let uri = Repository::normalize_url_percent_encoding(uri);
        let repository = self.repository.lock().unwrap();
        let ast = repository.as_ref()?.ast_map.get(&uri)?;
        Some(read(ast.value()))
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri {
            self.open_workspace(root_uri).await;
        }
        Ok(InitializeResult {
            server_info: None,
            capabilities: server_capabilities(),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "patto-lsp server initialized!")
            .await;
        self.announce_paper_provider();
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.update_settings(params.settings);
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        log::info!("did_open: {:?}", params.text_document.uri);
        let document = params.text_document;
        self.on_change(document.uri, document.text, document.version)
            .await
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        let text = std::mem::take(&mut params.content_changes[0].text);
        self.on_change(params.text_document.uri, text, params.text_document.version)
            .await
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("file {} saved!", params.text_document.uri.as_str()),
            )
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("file {} is closed!", params.text_document.uri),
            )
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
        Ok(self.dispatch_command(params).await)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = Repository::normalize_url_percent_encoding(
            &params.text_document_position_params.text_document.uri,
        );
        let position = params.text_document_position_params.position;
        Ok(self
            .definition_at(&uri, position)
            .map(GotoDefinitionResponse::Scalar))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = Repository::normalize_url_percent_encoding(
            &params.text_document_position.text_document.uri,
        );
        Ok(self.references_to(&uri))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        Ok(self.with_document_ast(&params.text_document.uri, |ast| {
            SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: get_semantic_tokens(ast),
            })
        }))
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        let (start_line, end_line) = (params.range.start.line, params.range.end.line);
        Ok(self.with_document_ast(&params.text_document.uri, |ast| {
            SemanticTokensRangeResult::Tokens(SemanticTokens {
                result_id: None,
                data: get_semantic_tokens_range(ast, start_line, end_line),
            })
        }))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = Repository::normalize_url_percent_encoding(&params.text_document.uri);
        Ok(self.prepare_rename_at(&uri, params.position))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = Repository::normalize_url_percent_encoding(
            &params.text_document_position.text_document.uri,
        );
        self.rename_workspace_edit(
            &uri,
            params.text_document_position.position,
            params.new_name.trim(),
        )
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        Ok(self.with_document_ast(&params.text_document.uri, collect_folding_ranges))
    }
}
