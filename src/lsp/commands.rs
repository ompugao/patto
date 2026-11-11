//! Handlers for `workspace/executeCommand`.
//!
//! Every handler returns the JSON value sent back to the client, or `None` when
//! the request cannot be served (no workspace, unknown document, bad argument).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_lsp::lsp_types::{ExecuteCommandParams, Location, MessageType, Url};

use crate::ast_query::{task_label, TaskLine};
use crate::lsp::backend::Backend;
use crate::lsp::locate::node_range;
use crate::markdown::{MarkdownFlavor, MarkdownRendererOptions};
use crate::parser::{AstNodeKind, Deadline, Property, TaskStatus};
use crate::renderer::{MarkdownRenderer, Renderer};
use crate::repository::{DateRange, Repository, TaskQuery};
use crate::task::Duration;
use crate::tasks_view::{timeframe_bounds, ReviewTimeframe};

/// Commands this server advertises in its `initialize` response.
///
/// `experimental/scan_workspace` is still sent by the bundled editor plugins and
/// is accepted as a no-op: the workspace is scanned on `initialized` instead.
pub const SUPPORTED_COMMANDS: &[&str] = &[
    "experimental/retrieve_two_hop_notes",
    "experimental/scan_workspace",
    "patto/tasks",
    "patto/snapshotPapers",
    "patto/renderAsMarkdown",
];

impl Backend {
    pub(super) async fn dispatch_command(&self, params: ExecuteCommandParams) -> Option<Value> {
        let args = params.arguments.as_slice();
        match params.command.as_str() {
            "patto/tasks" => self.tasks(args),
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

    /// Arguments: `[TasksParams]`. Anything but an object (nothing, `null`, or
    /// the `[]` some clients send for an empty table) means open tasks only.
    fn tasks(&self, args: &[Value]) -> Option<Value> {
        let params: TasksParams = match args.first() {
            Some(value @ Value::Object(_)) => serde_json::from_value(value.clone()).ok()?,
            _ => TasksParams::default(),
        };
        let query = params.query(chrono::Local::now().date_naive());

        let repository = self.repository.lock().unwrap();
        let tasks = repository.as_ref()?.tasks(&query);
        Some(json!(tasks
            .iter()
            .map(|(uri, task)| task_entry(uri, task))
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

        two_hop.sort_by_key(|(_, connected)| std::cmp::Reverse(connected.len()));
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

/// The argument of `patto/tasks`.
#[derive(Debug, Deserialize)]
#[serde(default)]
struct TasksParams {
    open: bool,
    /// Resolved tasks to add; none when absent.
    resolved: Option<ResolvedParams>,
}

impl Default for TasksParams {
    fn default() -> Self {
        Self {
            open: true,
            resolved: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ResolvedParams {
    /// A `ReviewTimeframe` name; when absent, `from` / `to` bound the range.
    timeframe: Option<String>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    include_archived: bool,
}

impl TasksParams {
    fn query(&self, today: NaiveDate) -> TaskQuery {
        let range = self.resolved.as_ref().map(|resolved| {
            let timeframe = match &resolved.timeframe {
                Some(name) => ReviewTimeframe::from_name(name, None, None),
                None => ReviewTimeframe::Custom {
                    from: resolved.from,
                    to: resolved.to,
                },
            };
            let (from, to) = timeframe_bounds(&timeframe, today);
            (DateRange { from, to }, resolved.include_archived)
        });
        TaskQuery {
            open: self.open,
            done: range.map(|(range, _)| range),
            archived: range.and_then(|(range, archived)| archived.then_some(range)),
        }
    }
}

/// One entry of the `patto/tasks` response.
#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub struct TaskEntry {
    pub location: Location,
    /// Raw line text with the task property token removed.
    pub text: String,
    pub status: TaskStatus,
    pub due: Deadline,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<Deadline>,
    /// Clock-in time of the running session. Left out on resolved tasks, where
    /// it is stale and `time_spent` already holds the total.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Deadline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_spent: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<NaiveDate>,
}

fn task_entry(uri: &Url, task: &TaskLine) -> TaskEntry {
    let mut entry = TaskEntry {
        location: Location::new(uri.clone(), node_range(&task.line)),
        text: task_label(&task.line),
        status: task.status.clone(),
        due: task.due.clone(),
        scheduled: None,
        started_at: None,
        time_spent: None,
        completed_at: None,
        archived_at: None,
    };
    if let AstNodeKind::Line { properties } = task.line.kind() {
        if let Some(Property::Task {
            scheduled,
            started_at,
            time_spent,
            ..
        }) = properties
            .iter()
            .find(|prop| matches!(prop, Property::Task { .. }))
        {
            entry.scheduled = scheduled.clone();
            entry.started_at = started_at.clone();
            entry.time_spent = time_spent.clone();
        }
    }
    match task.status {
        TaskStatus::Done => {
            entry.completed_at = task.resolved_on;
            entry.started_at = None;
        }
        TaskStatus::Archived => {
            entry.archived_at = task.resolved_on;
            entry.started_at = None;
        }
        _ => {}
    }
    entry
}

fn markdown_flavor(name: &str) -> MarkdownFlavor {
    match name.to_lowercase().as_str() {
        "obsidian" => MarkdownFlavor::Obsidian,
        "github" => MarkdownFlavor::GitHub,
        _ => MarkdownFlavor::Standard,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_query::gather_task_lines;

    fn entry(line: &str) -> TaskEntry {
        let ast = crate::parser::parse_text(line).ast;
        let mut tasks = Vec::new();
        gather_task_lines(&ast, &mut tasks);
        let uri = Url::parse("file:///notes/a.pn").unwrap();
        task_entry(&uri, &tasks[0])
    }

    fn query(params: Value) -> TaskQuery {
        let today = NaiveDate::from_ymd_opt(2026, 5, 13).unwrap();
        serde_json::from_value::<TasksParams>(params)
            .unwrap()
            .query(today)
    }

    fn range(from: Option<&str>, to: Option<&str>) -> Option<DateRange> {
        let date = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        Some(DateRange {
            from: from.map(date),
            to: to.map(date),
        })
    }

    #[test]
    fn an_empty_argument_asks_for_open_tasks_only() {
        assert_eq!(query(json!({})), TaskQuery::open());
    }

    #[test]
    fn a_timeframe_is_resolved_against_today() {
        assert_eq!(
            query(json!({ "open": false, "resolved": { "timeframe": "this_week" } })),
            TaskQuery {
                open: false,
                done: range(Some("2026-05-11"), Some("2026-05-13")),
                archived: None,
            }
        );
    }

    #[test]
    fn from_and_to_bound_the_range_and_archived_follows_it() {
        assert_eq!(
            query(json!({ "resolved": { "from": "2026-04-14", "include_archived": true } })),
            TaskQuery {
                open: true,
                done: range(Some("2026-04-14"), None),
                archived: range(Some("2026-04-14"), None),
            }
        );
    }

    #[test]
    fn task_text_conceals_urls_but_keeps_link_titles() {
        let task = " {@task status=todo due=2026-06-01}";
        assert_eq!(
            entry(&format!(
                "buy milk [https://example.com/foo milk title]{task}"
            ))
            .text,
            "buy milk [🔗milk title]"
        );
        assert_eq!(
            entry(&format!(
                "[milk title https://example.com/foo] buy milk{task}"
            ))
            .text,
            "[milk title🔗] buy milk"
        );
        assert_eq!(
            entry(&format!("buy milk [https://example.com/foo]{task}")).text,
            "buy milk [https://example.com/foo]"
        );
    }

    #[test]
    fn task_text_conceals_urls_in_multibyte_lines() {
        let task = " {@task status=todo due=2026-06-01}";
        assert_eq!(
            entry(&format!("牛乳を買う [https://example.com/foo 牛乳]{task}")).text,
            "牛乳を買う [🔗牛乳]"
        );
        assert_eq!(
            entry(&format!("[牛乳 https://example.com/foo] 牛乳を買う{task}")).text,
            "[牛乳🔗] 牛乳を買う"
        );
    }

    #[test]
    fn task_entry_copies_every_task_field() {
        let info = entry(
            "work {@task status=doing due=2026-06-01 scheduled=2026-05-30 started_at=2026-05-30T09:00 time_spent=1h30m}",
        );
        assert_eq!(info.status, TaskStatus::Doing);
        assert!(info.scheduled.is_some());
        assert!(info.started_at.is_some());
        assert!(info.time_spent.is_some());
        assert_eq!(info.completed_at, None);
        assert_eq!(info.location.range.start.line, 0);
    }

    #[test]
    fn a_resolved_entry_carries_its_date_and_drops_the_stale_clock_in() {
        let done = entry("ship {@task status=done completed_at=2026-05-12T18:00 started_at=2026-05-12T09:00 time_spent=2h}");
        assert_eq!(done.completed_at, NaiveDate::from_ymd_opt(2026, 5, 12));
        assert_eq!(done.started_at, None);
        assert!(done.time_spent.is_some());

        let dropped = entry("idea {@task status=archived archived_at=2026-05-10}");
        assert_eq!(dropped.archived_at, NaiveDate::from_ymd_opt(2026, 5, 10));
        assert_eq!(dropped.completed_at, None);
    }
}
