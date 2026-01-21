use std::sync::{Arc, Mutex};
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService};
use url::Url;

use patto::lsp::{Backend, paper::PaperCatalog, PattoSettings};

use crate::common::TestWorkspace;

/// In-process LSP test client that directly uses Backend
pub struct InProcessLspClient {
    backend: Arc<Backend>,
}

impl InProcessLspClient {
    /// Create a new in-process LSP client
    pub async fn new(workspace: &TestWorkspace) -> Self {
        let workspace_root = workspace.root_uri();
        
        // Create the LspService
        let (service, socket) = LspService::build(|client| {
            Backend {
                client,
                repository: Arc::new(Mutex::new(None)),
                root_uri: Arc::new(Mutex::new(None)),
                paper_catalog: PaperCatalog::default(),
                settings: Arc::new(Mutex::new(PattoSettings::default())),
            }
        }).finish();

        // Spawn a task to consume and discard all socket messages (client notifications/requests)
        tokio::spawn(async move {
            futures::pin_mut!(socket);
            while let Some(_msg) = futures::StreamExt::next(&mut socket).await {
                // Discard all messages from server to client
            }
        });

        // Get a reference to the Backend from the service
        // We need to extract it before we move service
        let backend_ptr = service.inner() as *const Backend;
        let backend = unsafe { Arc::from_raw(backend_ptr) };
        // Prevent drop by cloning and then forgetting
        let backend_clone = Arc::clone(&backend);
        std::mem::forget(backend);
        
        // Keep the service alive
        std::mem::forget(service);

        let mut test_client = Self { backend: backend_clone };
        
        // Initialize
        test_client.initialize(workspace_root).await;
        test_client.initialized().await;
        
        test_client
    }

    /// Initialize the LSP server
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

        self.backend.initialize(params).await.unwrap();
    }

    /// Send initialized notification
    async fn initialized(&mut self) {
        self.backend.initialized(InitializedParams {}).await;
        // Wait for workspace scanning to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    /// Open a document
    pub async fn did_open(&mut self, uri: Url, content: String) {
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "patto".to_string(),
                version: 1,
                text: content,
            },
        };
        self.backend.did_open(params).await;
    }

    /// Close a document
    pub async fn did_close(&mut self, uri: Url) {
        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        };
        self.backend.did_close(params).await;
    }

    /// Go to definition
    pub async fn definition(&mut self, uri: Url, line: u32, character: u32) -> Option<GotoDefinitionResponse> {
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        self.backend.goto_definition(params).await.ok().flatten()
    }

    /// Find references
    pub async fn references(&mut self, uri: Url, line: u32, character: u32) -> Option<Vec<Location>> {
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };
        self.backend.references(params).await.ok().flatten()
    }

    /// Request completion
    pub async fn completion(&mut self, uri: Url, line: u32, character: u32) -> Option<CompletionResponse> {
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };
        self.backend.completion(params).await.ok().flatten()
    }

    /// Prepare rename
    pub async fn prepare_rename(&mut self, uri: Url, line: u32, character: u32) -> Option<PrepareRenameResponse> {
        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line, character },
        };
        self.backend.prepare_rename(params).await.ok().flatten()
    }

    /// Rename
    pub async fn rename(&mut self, uri: Url, line: u32, character: u32, new_name: &str) -> Option<WorkspaceEdit> {
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        };
        self.backend.rename(params).await.ok().flatten()
    }

    /// Execute command
    pub async fn execute_command(&mut self, command: &str, arguments: Vec<serde_json::Value>) -> Option<Option<serde_json::Value>> {
        let params = ExecuteCommandParams {
            command: command.to_string(),
            arguments,
            work_done_progress_params: Default::default(),
        };
        self.backend.execute_command(params).await.ok()
    }

    /// Get semantic tokens for full document
    pub async fn semantic_tokens(&mut self, uri: Url) -> Option<SemanticTokensResult> {
        let params = SemanticTokensParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        self.backend.semantic_tokens_full(params).await.ok().flatten()
    }

    /// Get semantic tokens for a range
    pub async fn semantic_tokens_range(&mut self, uri: Url, range: Range) -> Option<SemanticTokensRangeResult> {
        let params = SemanticTokensRangeParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        self.backend.semantic_tokens_range(params).await.ok().flatten()
    }

    /// Aggregate tasks (Patto-specific)
    pub async fn aggregate_tasks(&mut self) -> Option<Option<serde_json::Value>> {
        self.execute_command("experimental/aggregate_tasks", vec![]).await
    }

    /// Get two-hop links (Patto-specific)
    pub async fn two_hop_links(&mut self, uri: Url) -> Option<Option<serde_json::Value>> {
        self.execute_command("experimental/retrieve_two_hop_notes", vec![serde_json::json!(uri.to_string())]).await
    }

    /// Send a generic notification
    pub async fn notify(&mut self, method: &str, params: serde_json::Value) {
        // For specific notifications like didChange, didSave
        if method == "textDocument/didChange" {
            if let Ok(p) = serde_json::from_value::<DidChangeTextDocumentParams>(params) {
                self.backend.did_change(p).await;
            }
        } else if method == "textDocument/didSave" {
            if let Ok(p) = serde_json::from_value::<DidSaveTextDocumentParams>(params) {
                self.backend.did_save(p).await;
            }
        }
    }
}
