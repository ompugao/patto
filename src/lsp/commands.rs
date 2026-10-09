//! Handlers for `workspace/executeCommand`.
//!
//! Every handler returns the JSON value sent back to the client, or `None` when
//! the request cannot be served (no workspace, unknown document, bad argument).

use chrono::NaiveDate;
use serde_json::{json, Value};
use tower_lsp::lsp_types::{ExecuteCommandParams, MessageType, Url};

use crate::lsp::backend::{task_information, Backend};
use crate::markdown::{MarkdownFlavor, MarkdownRendererOptions};
use crate::parser::Deadline;
use crate::renderer::{MarkdownRenderer, Renderer};
use crate::repository::Repository;
use crate::tasks_view::{timeframe_bounds, ReviewTimeframe};

/// Commands this server advertises in its `initialize` response.
///
/// `experimental/scan_workspace` is still sent by the bundled editor plugins and
/// is accepted as a no-op: the workspace is scanned on `initialized` instead.
pub const SUPPORTED_COMMANDS: &[&str] = &[
    "experimental/aggregate_tasks",
    "experimental/retrieve_two_hop_notes",
    "experimental/scan_workspace",
    "experimental/tasks_review",
    "patto/snapshotPapers",
    "patto/renderAsMarkdown",
];

impl Backend {
    pub(super) async fn dispatch_command(&self, params: ExecuteCommandParams) -> Option<Value> {
        let args = params.arguments.as_slice();
        match params.command.as_str() {
            "experimental/aggregate_tasks" => self.aggregate_tasks(),
            "experimental/tasks_review" => self.tasks_review(args),
            "experimental/retrieve_two_hop_notes" => self.retrieve_two_hop_notes(args),
            "patto/snapshotPapers" => self.snapshot_papers().await,
            "patto/renderAsMarkdown" => self.render_as_markdown(args),
            "experimental/scan_workspace" => None,
            unknown => {
                log::info!("unknown command: {}", unknown);
                None
            }
        }
    }

    fn aggregate_tasks(&self) -> Option<Value> {
        let repository = self.repository.lock().unwrap();
        let tasks = repository.as_ref()?.aggregate_tasks();
        Some(json!(tasks
            .iter()
            .map(|(uri, line, due)| task_information(uri, line, due))
            .collect::<Vec<_>>()))
    }

    /// Arguments: `[timeframe, from_date?, to_date?]`, dates as `YYYY-MM-DD`.
    fn tasks_review(&self, args: &[Value]) -> Option<Value> {
        let timeframe = ReviewTimeframe::from_name(
            args.first().and_then(|a| a.as_str()).unwrap_or("today"),
            parse_date(args.get(1)),
            parse_date(args.get(2)),
        );
        let (from, to) = timeframe_bounds(&timeframe, chrono::Local::now().date_naive());

        let repository = self.repository.lock().unwrap();
        let tasks = repository.as_ref()?.aggregate_completed_tasks(from, to);

        Some(json!(tasks
            .iter()
            .map(|(uri, line, date)| {
                let info = task_information(uri, line, &Deadline::Date(*date));
                // `started_at` is deliberately dropped: on a done task it is stale,
                // and the review side must not add elapsed time on top of
                // `time_spent`, which is already the accumulated total.
                json!({
                    "location":     info.location,
                    "text":         info.text,
                    "status":       info.status,
                    "due":          info.due,
                    "scheduled":    info.scheduled,
                    "completed_at": date.format("%Y-%m-%d").to_string(),
                    "time_spent":   info.time_spent,
                })
            })
            .collect::<Vec<_>>()))
    }

    /// Arguments: `[note_uri]`. Returns the notes reachable in two hops, most
    /// strongly connected first.
    fn retrieve_two_hop_notes(&self, args: &[Value]) -> Option<Value> {
        let url = args
            .first()
            .and_then(|a| a.as_str())
            .and_then(|url| Url::parse(url).ok())?;

        let repository = self.repository.lock().unwrap();
        let graph = repository.as_ref()?.document_graph.lock().ok()?;
        let node = graph.get(&url)?;

        let mut two_hop = node
            .iter_out()
            .map(|edge| {
                let target = edge.target();
                let connected = target
                    .iter_in()
                    .map(|edge| edge.source().key().clone())
                    .filter(|n| n != target.key() && n != &url)
                    .collect::<Vec<Url>>();
                (target.key().clone(), connected)
            })
            .filter(|(_, connected)| !connected.is_empty())
            .collect::<Vec<_>>();

        two_hop.sort_by_key(|(_, connected)| -(connected.len() as i16));
        two_hop.dedup();
        Some(json!(two_hop))
    }

    async fn snapshot_papers(&self) -> Option<Value> {
        self.client
            .log_message(MessageType::INFO, "Taking snapshot of papers...")
            .await;

        match self.paper_catalog.refresh().await {
            Ok(()) => {
                self.client
                    .show_message(MessageType::INFO, "Paper snapshot completed successfully.")
                    .await;
            }
            Err(err) => {
                let message = format!("Failed to take paper snapshot: {}", err);
                log::error!("{}", message);
                self.client.show_message(MessageType::ERROR, &message).await;
            }
        }
        None
    }

    /// Arguments: `[uri, start_line?, end_line?, flavor?]`. Lines are 0-indexed
    /// and inclusive; without them the whole document is rendered.
    fn render_as_markdown(&self, args: &[Value]) -> Option<Value> {
        let uri = args
            .first()
            .and_then(|a| a.as_str())
            .and_then(|uri| Url::parse(uri).ok())?;
        let uri = Repository::normalize_url_percent_encoding(&uri);

        let start_line = args.get(1).and_then(|a| a.as_u64()).map(|n| n as usize);
        let end_line = args.get(2).and_then(|a| a.as_u64()).map(|n| n as usize);

        let flavor = args
            .get(3)
            .and_then(|a| a.as_str())
            .map(str::to_string)
            .or_else(|| {
                self.settings
                    .lock()
                    .unwrap()
                    .markdown
                    .default_flavor
                    .clone()
            })
            .map(|name| markdown_flavor(&name))
            .unwrap_or(MarkdownFlavor::Standard);

        let repository = self.repository.lock().unwrap();
        let ast = repository.as_ref()?.ast_map.get(&uri)?;

        let options = MarkdownRendererOptions::new(flavor).with_frontmatter(false);
        let renderer = MarkdownRenderer::new(options);
        let mut output = Vec::new();

        let result = match (start_line, end_line) {
            (Some(start), Some(end)) => renderer.format_range(ast.value(), &mut output, start, end),
            _ => renderer.format(ast.value(), &mut output),
        };
        if let Err(err) = result {
            log::error!("Failed to render markdown: {}", err);
            return None;
        }

        Some(json!(String::from_utf8_lossy(&output)))
    }
}

fn markdown_flavor(name: &str) -> MarkdownFlavor {
    match name.to_lowercase().as_str() {
        "obsidian" => MarkdownFlavor::Obsidian,
        "github" => MarkdownFlavor::GitHub,
        _ => MarkdownFlavor::Standard,
    }
}

fn parse_date(value: Option<&Value>) -> Option<NaiveDate> {
    let text = value?.as_str()?;
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}
