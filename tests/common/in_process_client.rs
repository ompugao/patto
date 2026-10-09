use std::collections::HashMap;

use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService};
use url::Url;

use patto::lsp::task_edits::{
    collect_task_snapshots, detect_task_transitions, generate_edits_for_transition,
};
use patto::lsp::{paper::PaperCatalog, Backend};
use patto::parser::AstNode;
use patto::task::TaskSnapshot;

use crate::common::TestWorkspace;

/// Drives a `Backend` directly, without a transport. Server-to-client traffic
/// is drained and dropped, so `workspace/applyEdit` and notifications are no-ops.
pub struct InProcessLspClient {
    service: LspService<Backend>,
}

impl InProcessLspClient {
    /// A client that has already sent `initialize` and `initialized` for the
    /// workspace and waited for its initial scan.
    pub async fn new(workspace: &TestWorkspace) -> Self {
        let (service, socket) =
            LspService::build(|client| Backend::new(client, PaperCatalog::default())).finish();

        tokio::spawn(async move {
            futures::pin_mut!(socket);
            while futures::StreamExt::next(&mut socket).await.is_some() {}
        });

        let mut test_client = Self { service };
        test_client.initialize(workspace.root_uri()).await;
        test_client.initialized().await;
        test_client
    }

    fn backend(&self) -> &Backend {
        self.service.inner()
    }

    async fn initialize(&mut self, workspace_root: Url) {
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(workspace_root),
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    rename: Some(RenameClientCapabilities {
                        prepare_support: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        self.backend().initialize(params).await.unwrap();
    }

    async fn initialized(&mut self) {
        self.backend().initialized(InitializedParams {}).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    pub async fn did_open(&mut self, uri: Url, content: String) {
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "patto".to_string(),
                version: 1,
                text: content,
            },
        };
        self.backend().did_open(params).await;
    }

    pub async fn did_change(&mut self, uri: Url, version: i32, content: String) {
        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: content,
            }],
        };
        self.backend().did_change(params).await;
    }

    pub async fn did_close(&mut self, uri: Url) {
        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        };
        self.backend().did_close(params).await;
    }

    pub async fn definition(
        &mut self,
        uri: Url,
        line: u32,
        character: u32,
    ) -> Option<GotoDefinitionResponse> {
        let params = GotoDefinitionParams {
            text_document_position_params: position_params(uri, line, character),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        self.backend().goto_definition(params).await.ok().flatten()
    }

    pub async fn references(
        &mut self,
        uri: Url,
        line: u32,
        character: u32,
    ) -> Option<Vec<Location>> {
        let params = ReferenceParams {
            text_document_position: position_params(uri, line, character),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };
        self.backend().references(params).await.ok().flatten()
    }

    pub async fn completion(
        &mut self,
        uri: Url,
        line: u32,
        character: u32,
    ) -> Option<CompletionResponse> {
        let params = CompletionParams {
            text_document_position: position_params(uri, line, character),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };
        self.backend().completion(params).await.ok().flatten()
    }

    pub async fn prepare_rename(
        &mut self,
        uri: Url,
        line: u32,
        character: u32,
    ) -> Option<PrepareRenameResponse> {
        self.backend()
            .prepare_rename(position_params(uri, line, character))
            .await
            .ok()
            .flatten()
    }

    pub async fn rename(
        &mut self,
        uri: Url,
        line: u32,
        character: u32,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        let params = RenameParams {
            text_document_position: position_params(uri, line, character),
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        };
        self.backend().rename(params).await.ok().flatten()
    }

    /// `None` when the request itself failed; `Some(None)` when the command
    /// returned no value.
    pub async fn execute_command(
        &mut self,
        command: &str,
        arguments: Vec<serde_json::Value>,
    ) -> Option<Option<serde_json::Value>> {
        let params = ExecuteCommandParams {
            command: command.to_string(),
            arguments,
            work_done_progress_params: Default::default(),
        };
        self.backend().execute_command(params).await.ok()
    }

    pub async fn semantic_tokens(&mut self, uri: Url) -> Option<SemanticTokensResult> {
        let params = SemanticTokensParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        self.backend()
            .semantic_tokens_full(params)
            .await
            .ok()
            .flatten()
    }

    pub async fn semantic_tokens_range(
        &mut self,
        uri: Url,
        range: Range,
    ) -> Option<SemanticTokensRangeResult> {
        let params = SemanticTokensRangeParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        self.backend()
            .semantic_tokens_range(params)
            .await
            .ok()
            .flatten()
    }

    pub async fn folding_range(&mut self, uri: Url) -> Option<Vec<FoldingRange>> {
        let params = FoldingRangeParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        self.backend().folding_range(params).await.ok().flatten()
    }

    pub async fn aggregate_tasks(&mut self) -> Option<Option<serde_json::Value>> {
        self.execute_command("experimental/aggregate_tasks", vec![])
            .await
    }

    pub async fn two_hop_links(&mut self, uri: Url) -> Option<Option<serde_json::Value>> {
        self.execute_command(
            "experimental/retrieve_two_hop_notes",
            vec![serde_json::json!(uri.to_string())],
        )
        .await
    }

    pub async fn tasks_review(
        &mut self,
        timeframe: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Option<Option<serde_json::Value>> {
        let args = [Some(timeframe), from, to]
            .into_iter()
            .flatten()
            .map(|arg| serde_json::json!(arg))
            .collect();
        self.execute_command("experimental/tasks_review", args)
            .await
    }

    /// Dispatches the few notifications tests send as raw JSON.
    pub async fn notify(&mut self, method: &str, params: serde_json::Value) {
        match method {
            "textDocument/didChange" => {
                if let Ok(p) = serde_json::from_value::<DidChangeTextDocumentParams>(params) {
                    self.backend().did_change(p).await;
                }
            }
            "textDocument/didSave" => {
                if let Ok(p) = serde_json::from_value::<DidSaveTextDocumentParams>(params) {
                    self.backend().did_save(p).await;
                }
            }
            _ => {}
        }
    }

    pub fn get_ast(&self, uri: &Url) -> Option<AstNode> {
        self.backend().document_ast(uri)
    }

    /// The sticky last-valid task snapshots of a file, for testing the
    /// keystroke-gap bridging.
    pub fn get_last_valid_task_snapshots(&self, uri: &Url) -> Option<HashMap<usize, TaskSnapshot>> {
        self.backend().task_snapshots(uri)
    }

    /// The edits the task pipeline would apply when a document goes from
    /// `old_ast` to `new_ast`. `workspace/applyEdit` is dropped by this client,
    /// so tests inspect the edits here instead.
    pub fn completion_edits(&self, old_ast: &AstNode, new_ast: &AstNode) -> Vec<TextEdit> {
        let now = chrono::Local::now().naive_local();
        let old_snapshots = collect_task_snapshots(old_ast);
        let new_snapshots = collect_task_snapshots(new_ast);
        detect_task_transitions(&new_snapshots, &old_snapshots)
            .iter()
            .flat_map(|t| generate_edits_for_transition(t, now))
            .collect()
    }
}

fn position_params(uri: Url, line: u32, character: u32) -> TextDocumentPositionParams {
    TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri },
        position: Position { line, character },
    }
}
