use std::collections::BTreeMap;

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, TimeDelta};
use patto::{
    parser::{AstNode, AstNodeKind, Deadline, Property, TaskStatus},
    repository::Repository,
    task::Duration,
    tasks_view::{completed_group, pending_group, week_start, CompletedGroup, PendingGroup},
};
use tower_lsp::lsp_types::Url;
use tui_widget_list::ListState;

use crate::selection::{step_list, Step};

pub(crate) fn task_status_icon(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "○",
        TaskStatus::Doing => "◑",
        TaskStatus::Paused => "⏸",
        TaskStatus::Done => "✓",
    }
}

/// `"1h30m"`, `"45m"` or `"2h"`; `None` for zero or negative durations.
pub(crate) fn fmt_timedelta(td: TimeDelta) -> Option<String> {
    let minutes = td.num_minutes();
    (minutes > 0).then(|| Duration::from_minutes(minutes as u32).to_string())
}

/// Time spent on a task as of now: the stored total plus, for a task being
/// done, the running session. Computed at draw time so the counter stays
/// live without rescanning the repository.
pub(crate) fn total_elapsed(
    status: &TaskStatus,
    base: TimeDelta,
    started_at_dt: Option<NaiveDateTime>,
) -> TimeDelta {
    if matches!(status, TaskStatus::Doing) {
        if let Some(dt) = started_at_dt {
            let live = Local::now().naive_local() - dt;
            if live > TimeDelta::zero() {
                return base + live;
            }
        }
    }
    base
}

fn task_property(node: &AstNode) -> Option<&Property> {
    let AstNodeKind::Line { properties } = node.kind() else {
        return None;
    };
    properties
        .iter()
        .find(|prop| matches!(prop, Property::Task { .. }))
}

fn task_timing(node: &AstNode) -> (TaskStatus, TimeDelta, Option<NaiveDateTime>) {
    let Some(Property::Task {
        status,
        time_spent,
        started_at,
        ..
    }) = task_property(node)
    else {
        return (TaskStatus::Todo, TimeDelta::zero(), None);
    };
    let base = time_spent
        .as_ref()
        .map(|d| TimeDelta::minutes(d.total_minutes() as i64))
        .unwrap_or(TimeDelta::zero());
    let started_at_dt = match started_at {
        Some(Deadline::DateTime(dt)) => Some(*dt),
        _ => None,
    };
    (status.clone(), base, started_at_dt)
}

/// The line's text with every `{@task …}` annotation cut out and the
/// whitespace around the gaps collapsed.
fn node_display_text(node: &AstNode) -> String {
    let raw = node.extract_str();

    let mut annotation_spans: Vec<(usize, usize)> = Vec::new();
    if let AstNodeKind::Line { properties } = node.kind() {
        for prop in properties {
            if let Property::Task { location, .. } = prop {
                let (start, end) = (location.span.0, location.span.1);
                if end > start && end <= raw.len() {
                    annotation_spans.push((start, end));
                }
            }
        }
    }

    if annotation_spans.is_empty() {
        return raw.trim().to_string();
    }

    annotation_spans.sort_unstable();
    let mut kept = String::with_capacity(raw.len());
    let mut cursor = 0usize;
    for (start, end) in annotation_spans {
        if start > cursor {
            kept.push_str(&raw[cursor..start]);
        }
        cursor = end;
    }
    if cursor < raw.len() {
        kept.push_str(&raw[cursor..]);
    }
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn pending_label(group: PendingGroup) -> &'static str {
    match group {
        PendingGroup::Overdue => "⚠  Overdue",
        PendingGroup::Today => "  Today",
        PendingGroup::Tomorrow => "  Tomorrow",
        PendingGroup::ThisWeek => "  This Week",
        PendingGroup::ThisMonth => "  This Month",
        PendingGroup::Later => "  Later",
        PendingGroup::Uninterpretable => "  Uninterpretable Deadline",
    }
}

fn completed_label(group: CompletedGroup) -> &'static str {
    match group {
        CompletedGroup::Today => "✓ Today",
        CompletedGroup::Yesterday => "✓ Yesterday",
        CompletedGroup::ThisWeek => "✓ This Week",
        CompletedGroup::LastWeek => "✓ Last Week",
        CompletedGroup::ThisMonth => "✓ This Month",
        CompletedGroup::Older => "✓ Older",
    }
}

fn file_name_of(uri: &Url) -> String {
    uri.to_file_path()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| uri.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TasksView {
    Upcoming,
    Review,
}

pub(crate) struct TaskItem {
    pub(crate) text: String,
    pub(crate) file_name: String,
    pub(crate) uri: Url,
    /// 0-based line within the file.
    pub(crate) line: usize,
    pub(crate) due_str: String,
    pub(crate) group: PendingGroup,
    pub(crate) status: TaskStatus,
    /// Stored time spent; the running session is added at draw time by
    /// [`total_elapsed`].
    pub(crate) base_time_spent: TimeDelta,
    pub(crate) started_at_dt: Option<NaiveDateTime>,
}

impl TaskItem {
    fn new(uri: &Url, node: &AstNode, due: &Deadline, group: PendingGroup) -> Self {
        let due_str = match due {
            Deadline::Date(d) => d.format("%Y-%m-%d").to_string(),
            Deadline::DateTime(dt) => dt.format("%Y-%m-%d").to_string(),
            Deadline::Uninterpretable(s) => s.clone(),
        };
        let (status, base_time_spent, started_at_dt) = task_timing(node);
        Self {
            text: node_display_text(node),
            file_name: file_name_of(uri),
            uri: uri.clone(),
            line: node.location().row,
            due_str,
            group,
            status,
            base_time_spent,
            started_at_dt,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        matches!(self.status, TaskStatus::Doing | TaskStatus::Paused)
    }
}

pub(crate) enum TaskEntry {
    SectionHeader(String),
    Item(TaskItem),
    Placeholder(String),
}

impl TaskEntry {
    pub(crate) fn is_selectable(&self) -> bool {
        matches!(self, TaskEntry::Item(_))
    }
}

pub(crate) struct ReviewItem {
    pub(crate) text: String,
    pub(crate) file_name: String,
    pub(crate) uri: Url,
    /// 0-based line within the file.
    pub(crate) line: usize,
    /// `YYYY-MM-DD`
    pub(crate) completed_at: String,
    pub(crate) time_spent: TimeDelta,
}

impl ReviewItem {
    fn new(uri: &Url, node: &AstNode, date: NaiveDate) -> Self {
        let (_, time_spent, _) = task_timing(node);
        Self {
            text: node_display_text(node),
            file_name: file_name_of(uri),
            uri: uri.clone(),
            line: node.location().row,
            completed_at: date.format("%Y-%m-%d").to_string(),
            time_spent,
        }
    }
}

pub(crate) enum ReviewEntry {
    SectionHeader(String),
    Item(ReviewItem),
    Placeholder(String),
}

impl ReviewEntry {
    pub(crate) fn is_selectable(&self) -> bool {
        matches!(self, ReviewEntry::Item(_))
    }
}

pub(crate) struct TasksPanel {
    pub(crate) visible: bool,
    pub(crate) view: TasksView,
    pub(crate) entries: Vec<TaskEntry>,
    pub(crate) list_state: ListState,
    pub(crate) review_entries: Vec<ReviewEntry>,
    pub(crate) review_list_state: ListState,
}

impl TasksPanel {
    pub(crate) fn new() -> Self {
        Self {
            visible: false,
            view: TasksView::Upcoming,
            entries: Vec::new(),
            list_state: ListState::default(),
            review_entries: Vec::new(),
            review_list_state: ListState::default(),
        }
    }

    pub(crate) fn open(&mut self) {
        self.visible = true;
        if self.entries.is_empty() {
            self.list_state = ListState::default();
        }
    }

    pub(crate) fn close(&mut self) {
        self.visible = false;
        self.view = TasksView::Upcoming;
        self.list_state = ListState::default();
        self.review_list_state = ListState::default();
    }

    pub(crate) fn toggle_view(&mut self) {
        self.view = match self.view {
            TasksView::Upcoming => TasksView::Review,
            TasksView::Review => TasksView::Upcoming,
        };
    }

    pub(crate) fn refresh(&mut self, repository: &Repository) {
        self.entries = upcoming_entries(repository.aggregate_tasks());
        if self
            .list_state
            .selected
            .is_none_or(|i| i >= self.entries.len())
        {
            self.list_state = ListState::default();
            let first = self.entries.iter().position(TaskEntry::is_selectable);
            self.list_state.select(first);
        }
    }

    pub(crate) fn refresh_review(&mut self, repository: &Repository) {
        let today = Local::now().date_naive();
        let last_week_start = week_start(today) - chrono::Duration::days(7);
        let this_month_start = today.with_day(1).unwrap_or(today);
        let from = last_week_start.min(this_month_start);

        let completed = repository.aggregate_completed_tasks(Some(from), Some(today));
        self.review_entries = review_entries(completed);
        if self
            .review_list_state
            .selected
            .is_none_or(|i| i >= self.review_entries.len())
        {
            self.review_list_state = ListState::default();
            let first = self
                .review_entries
                .iter()
                .position(ReviewEntry::is_selectable);
            self.review_list_state.select(first);
        }
    }

    pub(crate) fn navigate_down(&mut self) {
        self.step(Step::Next);
    }

    pub(crate) fn navigate_up(&mut self) {
        self.step(Step::Prev);
    }

    fn step(&mut self, step: Step) {
        match self.view {
            TasksView::Upcoming => step_list(
                &mut self.list_state,
                &self.entries,
                TaskEntry::is_selectable,
                step,
            ),
            TasksView::Review => step_list(
                &mut self.review_list_state,
                &self.review_entries,
                ReviewEntry::is_selectable,
                step,
            ),
        }
    }

    /// The selected task as a navigation target: `(uri, line)`.
    pub(crate) fn resolve_cursor(&self) -> Option<(Url, usize)> {
        match self.view {
            TasksView::Upcoming => match self.entries.get(self.list_state.selected?)? {
                TaskEntry::Item(item) => Some((item.uri.clone(), item.line)),
                _ => None,
            },
            TasksView::Review => match self.review_entries.get(self.review_list_state.selected?)? {
                ReviewEntry::Item(item) => Some((item.uri.clone(), item.line)),
                _ => None,
            },
        }
    }

    /// Tasks being done or paused, in list order.
    pub(crate) fn active_tasks(&self) -> impl Iterator<Item = &TaskItem> {
        self.entries.iter().filter_map(|entry| match entry {
            TaskEntry::Item(item) if item.is_active() => Some(item),
            _ => None,
        })
    }
}

fn upcoming_entries(tasks: Vec<(Url, AstNode, Deadline)>) -> Vec<TaskEntry> {
    let today = Local::now().date_naive();
    let mut groups: BTreeMap<PendingGroup, Vec<TaskItem>> = BTreeMap::new();
    for (uri, node, due) in &tasks {
        let group = pending_group(due, today);
        groups
            .entry(group)
            .or_default()
            .push(TaskItem::new(uri, node, due, group));
    }

    let mut entries = Vec::new();
    for (group, items) in groups {
        entries.push(TaskEntry::SectionHeader(pending_label(group).to_string()));
        entries.extend(items.into_iter().map(TaskEntry::Item));
    }
    if entries.is_empty() {
        entries.push(TaskEntry::Placeholder("  (no pending tasks)".to_string()));
    }
    entries
}

fn review_entries(completed: Vec<(Url, AstNode, NaiveDate)>) -> Vec<ReviewEntry> {
    let today = Local::now().date_naive();
    let mut groups: BTreeMap<CompletedGroup, Vec<ReviewItem>> = BTreeMap::new();
    for (uri, node, date) in &completed {
        groups
            .entry(completed_group(*date, today))
            .or_default()
            .push(ReviewItem::new(uri, node, *date));
    }

    let mut entries = Vec::new();
    for (group, items) in groups {
        entries.push(ReviewEntry::SectionHeader(
            completed_label(group).to_string(),
        ));
        // Most recently completed first.
        entries.extend(items.into_iter().rev().map(ReviewEntry::Item));
    }
    if entries.is_empty() {
        entries.push(ReviewEntry::Placeholder(
            "  (no completed tasks in range)".to_string(),
        ));
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_line(text: &str) -> AstNode {
        let result = patto::parser::parse_text(text);
        let node = result.ast.children()[0].clone();
        node
    }

    #[test]
    fn durations_format_as_hours_and_minutes() {
        assert_eq!(
            fmt_timedelta(TimeDelta::minutes(90)).as_deref(),
            Some("1h30m")
        );
        assert_eq!(
            fmt_timedelta(TimeDelta::minutes(45)).as_deref(),
            Some("45m")
        );
        assert_eq!(
            fmt_timedelta(TimeDelta::minutes(120)).as_deref(),
            Some("2h")
        );
    }

    #[test]
    fn zero_and_negative_durations_have_no_text() {
        assert_eq!(fmt_timedelta(TimeDelta::zero()), None);
        assert_eq!(fmt_timedelta(TimeDelta::minutes(-5)), None);
    }

    #[test]
    fn a_paused_task_only_counts_its_stored_time() {
        let base = TimeDelta::minutes(10);
        let started = Local::now().naive_local() - TimeDelta::hours(1);
        assert_eq!(
            total_elapsed(&TaskStatus::Paused, base, Some(started)),
            base
        );
    }

    #[test]
    fn a_task_being_done_adds_its_running_session() {
        let base = TimeDelta::minutes(10);
        let started = Local::now().naive_local() - TimeDelta::hours(1);
        let elapsed = total_elapsed(&TaskStatus::Doing, base, Some(started));
        assert!(elapsed >= TimeDelta::minutes(70), "{elapsed:?}");
    }

    #[test]
    fn the_display_text_drops_the_task_annotation() {
        let node = parse_line("{@task status=todo due=2024-12-31} buy   milk\n");
        assert_eq!(node_display_text(&node), "buy milk");
    }

    #[test]
    fn task_timing_reads_status_and_stored_time() {
        let node = parse_line(
            "{@task status=doing due=2024-12-31 time_spent=1h30m started_at=2024-12-01T10:00} x\n",
        );
        let (status, base, started) = task_timing(&node);
        assert_eq!(status, TaskStatus::Doing);
        assert_eq!(base, TimeDelta::minutes(90));
        assert!(started.is_some());
    }

    #[test]
    fn upcoming_entries_are_grouped_under_deadline_headers() {
        let uri = Url::parse("file:///notes/a.pn").unwrap();
        let today = Local::now().date_naive();
        let node = parse_line("{@task status=todo due=2024-01-01} old\n");
        let entries = upcoming_entries(vec![
            (uri.clone(), node.clone(), Deadline::Date(today)),
            (uri, node, Deadline::Date(today - chrono::Duration::days(1))),
        ]);
        let headers: Vec<&str> = entries
            .iter()
            .filter_map(|e| match e {
                TaskEntry::SectionHeader(h) => Some(h.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(headers, vec!["⚠  Overdue", "  Today"]);
        assert_eq!(entries.len(), 4);
    }

    #[test]
    fn no_pending_tasks_leaves_a_placeholder() {
        let entries = upcoming_entries(Vec::new());
        assert!(matches!(&entries[..], [TaskEntry::Placeholder(_)]));
    }

    #[test]
    fn review_entries_list_the_most_recent_completion_first() {
        let uri = Url::parse("file:///notes/a.pn").unwrap();
        let today = Local::now().date_naive();
        let entries = review_entries(vec![
            (uri.clone(), parse_line("first\n"), today),
            (uri, parse_line("second\n"), today),
        ]);
        let texts: Vec<&str> = entries
            .iter()
            .filter_map(|e| match e {
                ReviewEntry::Item(item) => Some(item.text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["second", "first"]);
    }

    #[test]
    fn navigation_skips_headers_and_wraps_around() {
        let uri = Url::parse("file:///notes/a.pn").unwrap();
        let today = Local::now().date_naive();
        let node = parse_line("{@task status=todo due=2024-01-01} t\n");
        let mut panel = TasksPanel::new();
        panel.entries = upcoming_entries(vec![
            (uri.clone(), node.clone(), Deadline::Date(today)),
            (uri, node, Deadline::Date(today - chrono::Duration::days(1))),
        ]);
        panel.list_state.select(Some(1));
        panel.navigate_down();
        assert_eq!(panel.list_state.selected, Some(3));
        panel.navigate_down();
        assert_eq!(panel.list_state.selected, Some(1));
        panel.navigate_up();
        assert_eq!(panel.list_state.selected, Some(3));
    }

    #[test]
    fn active_tasks_are_those_being_done_or_paused() {
        let uri = Url::parse("file:///notes/a.pn").unwrap();
        let today = Local::now().date_naive();
        let entries = upcoming_entries(vec![
            (
                uri.clone(),
                parse_line("{@task status=doing due=2024-01-01} a\n"),
                Deadline::Date(today),
            ),
            (
                uri.clone(),
                parse_line("{@task status=todo due=2024-01-01} b\n"),
                Deadline::Date(today),
            ),
            (
                uri,
                parse_line("{@task status=paused due=2024-01-01} c\n"),
                Deadline::Date(today),
            ),
        ]);
        let mut panel = TasksPanel::new();
        panel.entries = entries;
        let active: Vec<&str> = panel.active_tasks().map(|t| t.text.as_str()).collect();
        assert_eq!(active, vec!["a", "c"]);
    }
}
