use futures::StreamExt;
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::atomic::{AtomicI32, Ordering};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio_util::codec::FramedRead;
use url::Url;

use crate::common::lsp_codec::LspCodec;
use crate::common::workspace::TestWorkspace;

/// LSP test client that spawns and communicates with patto-lsp binary
pub struct LspTestClient {
    process: Child,
    stdin: ChildStdin,
    stdout_reader: FramedRead<BufReader<ChildStdout>, LspCodec>,
    next_id: AtomicI32,
    workspace_root: Url,
}

impl LspTestClient {
    /// Create a new LSP test client for the given workspace
    /// This spawns a patto-lsp process and connects to it
    pub async fn new(workspace: &TestWorkspace) -> Self {
        let workspace_root = workspace.root_uri();

        // Use the freshly built binary from target directory
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/patto-lsp"
        } else {
            "target/release/patto-lsp"
        };

        // Spawn patto-lsp process
        let mut process = Command::new(binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Show errors for debugging
            .spawn()
            .expect("Failed to spawn patto-lsp. Run 'cargo build' first.");

        let stdin = process.stdin.take().expect("Failed to open stdin");
        let stdout = process.stdout.take().expect("Failed to open stdout");
        
        let stdout_reader = FramedRead::new(BufReader::new(stdout), LspCodec::default());

        Self {
            process,
            stdin,
            stdout_reader,
            next_id: AtomicI32::new(1),
            workspace_root,
        }
    }

    /// Allocate a new request ID
    fn next_id(&self) -> i32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Send a request and wait for response
    async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id();
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let message = serde_json::to_string(&request).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
        
        self.stdin
            .write_all(header.as_bytes())
            .await
            .expect("Failed to write request");
        self.stdin.flush().await.expect("Failed to flush");

        self.wait_for_response(id)
            .await
            .expect("No response received")
    }

    /// Send a notification (no response expected)
    async fn notify(&mut self, method: &str, params: Value) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let message = serde_json::to_string(&notification).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
        
        self.stdin
            .write_all(header.as_bytes())
            .await
            .expect("Failed to write notification");
        self.stdin.flush().await.expect("Failed to flush");
    }

    /// Wait for a specific response by ID
    async fn wait_for_response(&mut self, id: i32) -> Option<Value> {
        let timeout = tokio::time::Duration::from_secs(10);
        let start = tokio::time::Instant::now();

        while tokio::time::Instant::now() - start < timeout {
            tokio::select! {
                msg = self.stdout_reader.next() => {
                    if let Some(Ok(msg)) = msg {
                        if msg.get("id").and_then(|v| v.as_i64()) == Some(id as i64) {
                            return Some(msg);
                        }
                        // Ignore notifications
                    } else {
                        return None; // Stream ended
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    continue;
                }
            }
        }
        None
    }

    // === LSP Lifecycle Methods ===

    /// Initialize the LSP server
    pub async fn initialize(&mut self) -> Value {
        self.request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": self.workspace_root.to_string(),
                "capabilities": {
                    "textDocument": {
                        "rename": {
                            "prepareSupport": true
                        }
                    }
                }
            }),
        )
        .await
    }

    /// Send initialized notification
    pub async fn initialized(&mut self) {
        self.notify("initialized", json!({})).await;
        // Wait a bit for server to process
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // === Document Methods ===

    /// Open a document
    pub async fn did_open(&mut self, uri: Url, content: String) {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri.to_string(),
                    "languageId": "patto",
                    "version": 1,
                    "text": content
                }
            }),
        )
        .await;
        // Wait a bit for server to process
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    /// Close a document
    pub async fn did_close(&mut self, uri: Url) {
        self.notify(
            "textDocument/didClose",
            json!({
                "textDocument": {
                    "uri": uri.to_string()
                }
            }),
        )
        .await;
    }

    // === Rename Methods ===

    /// Prepare rename at position
    pub async fn prepare_rename(&mut self, uri: Url, line: u32, character: u32) -> Value {
        self.request(
            "textDocument/prepareRename",
            json!({
                "textDocument": { "uri": uri.to_string() },
                "position": { "line": line, "character": character }
            }),
        )
        .await
    }

    /// Rename at position
    pub async fn rename(&mut self, uri: Url, line: u32, character: u32, new_name: &str) -> Value {
        self.request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": uri.to_string() },
                "position": { "line": line, "character": character },
                "newName": new_name
            }),
        )
        .await
    }

    // === Definition Methods ===

    /// Go to definition
    pub async fn definition(&mut self, uri: Url, line: u32, character: u32) -> Value {
        self.request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri.to_string() },
                "position": { "line": line, "character": character }
            }),
        )
        .await
    }

    // === References Methods ===

    /// Find references
    pub async fn references(&mut self, uri: Url, line: u32, character: u32) -> Value {
        self.request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": uri.to_string() },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": true }
            }),
        )
        .await
    }

    // === Completion Methods ===

    /// Request completion
    pub async fn completion(&mut self, uri: Url, line: u32, character: u32) -> Value {
        self.request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.to_string() },
                "position": { "line": line, "character": character }
            }),
        )
        .await
    }

    // === Custom Commands ===

    /// Execute workspace command
    pub async fn execute_command(&mut self, command: &str, arguments: Value) -> Value {
        self.request(
            "workspace/executeCommand",
            json!({
                "command": command,
                "arguments": arguments
            }),
        )
        .await
    }

    /// Aggregate tasks (Patto-specific)
    pub async fn aggregate_tasks(&mut self) -> Value {
        self.execute_command("experimental/aggregate_tasks", json!([])).await
    }

    /// Get two-hop links (Patto-specific)
    pub async fn two_hop_links(&mut self, uri: Url) -> Value {
        self.execute_command(
            "experimental/retrieve_two_hop_notes",
            json!([{
                "uri": uri.to_string()
            }]),
        )
        .await
    }
}

impl Drop for LspTestClient {
    fn drop(&mut self) {
        // Try to kill the process
        let _ = self.process.start_kill();
    }
}
