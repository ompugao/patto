//! Handlers for `workspace/executeCommand`.
//!
//! Every handler returns the JSON value sent back to the client, or `None` when
//! the request cannot be served (no workspace, unknown document, bad argument).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_lsp::lsp_types::{ExecuteCommandParams, Location, MessageType, Url};

use crate::ast_query::task_label;
use crate::lsp::backend::Backend;
use crate::lsp::locate::node_range;
use crate::markdown::{MarkdownFlavor, MarkdownRendererOptions};
use crate::parser::{AstNode, AstNodeKind, Deadline, Property, TaskStatus};
use crate::renderer::{MarkdownRenderer, Renderer};
use crate::repository::Repository;
use crate::task::Duration;
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

/// One entry of the `experimental/aggregate_tasks` response.
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct TaskInformation {
    pub location: Location,
    /// Raw line text with the task property token removed.
    pub text: String,
    pub message: String,
    pub due: Deadline,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<Deadline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<Deadline>,
    /// Clock-in time of the running session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Deadline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_spent: Option<Duration>,
    pub status: TaskStatus,
}

fn task_information(uri: &Url, line: &AstNode, due: &Deadline) -> TaskInformation {
    let mut info = TaskInformation {
        location: Location::new(uri.clone(), node_range(line)),
        text: task_label(line),
        message: String::new(),
        due: due.clone(),
        scheduled: None,
        completed_at: None,
        started_at: None,
        time_spent: None,
        status: TaskStatus::Todo,
    };
    if let AstNodeKind::Line { properties } = line.kind() {
        for prop in properties {
            if let Property::Task {
                status,
                scheduled,
                completed_at,
                started_at,
                time_spent,
                ..
            } = prop
            {
                info.status = status.clone();
                info.scheduled = scheduled.clone();
                info.completed_at = completed_at.clone();
                info.started_at = started_at.clone();
                info.time_spent = time_spent.clone();
                break;
            }
        }
    }
    info
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

#[cfg(test)]
mod tests {
    use super::*;

    fn information(line: &str) -> TaskInformation {
        let ast = crate::parser::parse_text(line).ast;
        let children = ast.children();
        let uri = Url::parse("file:///notes/a.pn").unwrap();
        task_information(
            &uri,
            &children[0],
            &Deadline::Uninterpretable(String::new()),
        )
    }

    #[test]
    fn task_text_conceals_urls_but_keeps_link_titles() {
        let task = " {@task status=todo due=2026-06-01}";
        assert_eq!(
            information(&format!(
                "buy milk [https://example.com/foo milk title]{task}"
            ))
            .text,
            "buy milk [🔗milk title]"
        );
        assert_eq!(
            information(&format!(
                "[milk title https://example.com/foo] buy milk{task}"
            ))
            .text,
            "[milk title🔗] buy milk"
        );
        assert_eq!(
            information(&format!("buy milk [https://example.com/foo]{task}")).text,
            "buy milk [https://example.com/foo]"
        );
    }

    #[test]
    fn task_text_conceals_urls_in_multibyte_lines() {
        let task = " {@task status=todo due=2026-06-01}";
        assert_eq!(
            information(&format!("牛乳を買う [https://example.com/foo 牛乳]{task}")).text,
            "牛乳を買う [🔗牛乳]"
        );
        assert_eq!(
            information(&format!("[牛乳 https://example.com/foo] 牛乳を買う{task}")).text,
            "[牛乳🔗] 牛乳を買う"
        );
    }

    #[test]
    fn task_information_copies_every_task_field() {
        let info = information(
            "work {@task status=doing due=2026-06-01 scheduled=2026-05-30 started_at=2026-05-30T09:00 time_spent=1h30m}",
        );
        assert_eq!(info.status, TaskStatus::Doing);
        assert!(info.scheduled.is_some());
        assert!(info.started_at.is_some());
        assert!(info.time_spent.is_some());
        assert_eq!(info.completed_at, None);
        assert_eq!(info.location.range.start.line, 0);
    }
}
