use chrono::{Local, Timelike};
use patto::{
    parser::{AstNodeKind, Deadline, Property, TaskStatus},
    repository::Repository,
};
use tower_lsp::lsp_types::Url;
use tui_widget_list::ListState;

/// Return the status-icon character for a `TaskStatus`.
pub(crate) fn task_status_icon(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "○",
        TaskStatus::Doing => "◑",
        TaskStatus::Paused => "⏸",
        TaskStatus::Done => "✓",
    }
}

/// Format a total number of minutes as a human-readable string.
fn fmt_minutes(total: u32) -> String {
    let h = total / 60;
    let m = total % 60;
    if h > 0 && m > 0 {
        format!("{}h{}m", h, m)
    } else if h > 0 {
        format!("{}h", h)
    } else {
        format!("{}m", m)
    }
}

/// Extract task metadata (status, total_time_spent string, started_at string) from an `AstNode`.
///
/// For Doing tasks with a `started_at` datetime, `time_spent` is the **total**
/// elapsed time: accumulated `time_spent` + live elapsed since `started_at`,
/// matching the behaviour of PR #103 in the Lua trouble/fidget sources.
fn extract_task_meta(
    node: &patto::parser::AstNode,
) -> (TaskStatus, Option<String>, Option<String>) {
    if let AstNodeKind::Line { ref properties } = node.kind() {
        for prop in properties {
            if let Property::Task {
                status,
                time_spent,
                started_at,
                ..
            } = prop
            {
                // Base accumulated minutes from stored time_spent field.
                let base_minutes: u32 = time_spent
                    .as_ref()
                    .map(|d| d.hours * 60 + d.minutes)
                    .unwrap_or(0);

                // Live session minutes: only added for Doing tasks that have a started_at datetime.
                let live_minutes: u32 = if matches!(status, TaskStatus::Doing) {
                    if let Some(Deadline::DateTime(dt)) = started_at {
                        let now = Local::now().naive_local();
                        let elapsed = now.signed_duration_since(*dt);
                        elapsed.num_minutes().max(0) as u32
                    } else {
                        0
                    }
                } else {
                    0
                };

                let total_minutes = base_minutes + live_minutes;
                let ts_str = if total_minutes > 0 {
                    Some(fmt_minutes(total_minutes))
                } else {
                    None
                };

                let sa_str = started_at.as_ref().and_then(|dl| match dl {
                    Deadline::DateTime(dt) => Some(format!("{:02}:{:02}", dt.hour(), dt.minute())),
                    _ => None,
                });

                return (status.clone(), ts_str, sa_str);
            }
        }
    }
    (TaskStatus::Todo, None, None)
}

/// Deadline grouping categories, matching the Lua trouble.nvim source behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeadlineCategory {
    Overdue,
    Today,
    Tomorrow,
    ThisWeek,
    ThisMonth,
    Later,
    NoDeadline,
    Uninterpretable,
}

impl DeadlineCategory {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            DeadlineCategory::Overdue => "⚠  Overdue",
            DeadlineCategory::Today => "  Today",
            DeadlineCategory::Tomorrow => "  Tomorrow",
            DeadlineCategory::ThisWeek => "  This Week",
            DeadlineCategory::ThisMonth => "  This Month",
            DeadlineCategory::Later => "  Later",
            DeadlineCategory::NoDeadline => "  No Deadline",
            DeadlineCategory::Uninterpretable => "  Uninterpretable Deadline",
        }
    }
}

/// Classify a `Deadline` into a display category relative to today.
pub(crate) fn deadline_category(due: &Deadline) -> DeadlineCategory {
    let today = Local::now().date_naive();
    match due {
        Deadline::Date(d) => {
            let diff = (*d - today).num_days();
            if diff < 0 {
                DeadlineCategory::Overdue
            } else if diff == 0 {
                DeadlineCategory::Today
            } else if diff == 1 {
                DeadlineCategory::Tomorrow
            } else {
                // days until the next Saturday (weekday 5 in chrono = Saturday)
                use chrono::Datelike;
                let days_until_sat =
                    (5 - today.weekday().num_days_from_monday() as i64).rem_euclid(7);
                if diff <= days_until_sat {
                    DeadlineCategory::ThisWeek
                } else {
                    // end of current month
                    let month_end = if today.month() == 12 {
                        chrono::NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
                    } else {
                        chrono::NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
                    }
                    .map(|d| d.pred_opt().unwrap_or(d));
                    if let Some(end) = month_end {
                        if *d <= end {
                            DeadlineCategory::ThisMonth
                        } else {
                            DeadlineCategory::Later
                        }
                    } else {
                        DeadlineCategory::Later
                    }
                }
            }
        }
        Deadline::DateTime(dt) => deadline_category(&Deadline::Date(dt.date())),
        Deadline::Uninterpretable(_) => DeadlineCategory::Uninterpretable,
    }
}

/// Which view is currently shown in the tasks panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TasksView {
    /// Upcoming / active tasks, grouped by deadline.
    Upcoming,
    /// Recently completed tasks, grouped by recency.
    Review,
}

/// Recency grouping for completed tasks — mirrors the Lua `patto_tasks_review` source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletedCategory {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    Older,
}

impl CompletedCategory {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            CompletedCategory::Today => "✓ Today",
            CompletedCategory::Yesterday => "✓ Yesterday",
            CompletedCategory::ThisWeek => "✓ This Week",
            CompletedCategory::LastWeek => "✓ Last Week",
            CompletedCategory::ThisMonth => "✓ This Month",
            CompletedCategory::Older => "✓ Older",
        }
    }

    /// Ordering index (smaller = more recent).
    pub(crate) fn order(&self) -> usize {
        match self {
            CompletedCategory::Today => 0,
            CompletedCategory::Yesterday => 1,
            CompletedCategory::ThisWeek => 2,
            CompletedCategory::LastWeek => 3,
            CompletedCategory::ThisMonth => 4,
            CompletedCategory::Older => 5,
        }
    }
}

/// Classify a completed task's `completed_at` date into a recency bucket.
pub(crate) fn completed_category(date: chrono::NaiveDate) -> CompletedCategory {
    use chrono::Datelike;
    let today = Local::now().date_naive();
    let diff = (today - date).num_days();
    if diff == 0 {
        CompletedCategory::Today
    } else if diff == 1 {
        CompletedCategory::Yesterday
    } else {
        // Day of week: Monday=0 … Sunday=6
        let dow = today.weekday().num_days_from_monday() as i64;
        // Start of this week (Monday)
        let this_week_start = today - chrono::Duration::days(dow);
        // Start of last week
        let last_week_start = this_week_start - chrono::Duration::days(7);
        // Start of this month
        let this_month_start =
            chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);

        if date >= this_week_start {
            CompletedCategory::ThisWeek
        } else if date >= last_week_start {
            CompletedCategory::LastWeek
        } else if date >= this_month_start {
            CompletedCategory::ThisMonth
        } else {
            CompletedCategory::Older
        }
    }
}

/// A single display entry in the flat review (completed-tasks) list.
#[derive(Clone)]
pub(crate) enum ReviewEntry {
    /// Recency group section header.
    SectionHeader(String),
    /// A single completed task item.
    ReviewItem {
        text: String,
        file_name: String,
        uri: Url,
        /// 0-based line number within the file.
        line: usize,
        /// Formatted `completed_at` date string `YYYY-MM-DD`.
        completed_at: String,
        /// Formatted time-spent string, e.g. `"1h30m"`.
        time_spent: Option<String>,
        category: CompletedCategory,
    },
    /// Placeholder text when list is empty.
    Placeholder(String),
}

impl ReviewEntry {
    pub(crate) fn is_selectable(&self) -> bool {
        matches!(self, ReviewEntry::ReviewItem { .. })
    }
}
#[derive(Clone)]
pub(crate) enum TaskEntry {
    /// Deadline group section header.
    SectionHeader(String),
    /// A single task item (upcoming / active).
    TaskItem {
        text: String,
        file_name: String,
        uri: Url,
        /// 0-based line number within the file.
        line: usize,
        due_str: String,
        category: DeadlineCategory,
        /// Task status (○ todo / ◑ doing / ⏸ paused).
        status: TaskStatus,
        /// Formatted time-spent string, e.g. `"1h30m"`. Present for doing/paused tasks.
        time_spent: Option<String>,
        /// Formatted started-at string (HH:MM). Present for doing/paused tasks.
        started_at: Option<String>,
    },
    /// Placeholder "(none)" when a section is empty.
    Placeholder(String),
}

impl TaskEntry {
    pub(crate) fn is_selectable(&self) -> bool {
        matches!(self, TaskEntry::TaskItem { .. })
    }
}

/// Self-contained tasks panel state.
pub(crate) struct TasksPanel {
    pub(crate) visible: bool,
    /// Which view is currently showing.
    pub(crate) view: TasksView,
    /// Upcoming/active task entries.
    pub(crate) entries: Vec<TaskEntry>,
    pub(crate) list_state: ListState,
    /// Completed-task review entries.
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
        // Preserve selection if already populated, otherwise reset.
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

    /// Toggle between Upcoming and Review views.
    pub(crate) fn toggle_view(&mut self) {
        self.view = match self.view {
            TasksView::Upcoming => TasksView::Review,
            TasksView::Review => TasksView::Upcoming,
        };
    }

    /// Re-fetch tasks from the repository and rebuild the flat entry list.
    pub(crate) fn refresh(&mut self, repository: &Repository) {
        let tasks = repository.aggregate_tasks();
        self.rebuild_entries(tasks);
        // Keep or reset selection
        if self
            .list_state
            .selected
            .map_or(true, |i| i >= self.entries.len())
        {
            self.list_state = ListState::default();
            self.select_first_item();
        }
    }

    /// Re-fetch completed tasks and rebuild the review entry list.
    pub(crate) fn refresh_review(&mut self, repository: &Repository) {
        use chrono::Datelike;
        let today = Local::now().date_naive();
        // Date range: from start-of-last-week or start-of-this-month (whichever is earlier)
        let dow = today.weekday().num_days_from_monday() as i64;
        let this_week_start = today - chrono::Duration::days(dow);
        let last_week_start = this_week_start - chrono::Duration::days(7);
        let this_month_start =
            chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
        let from = last_week_start.min(this_month_start);

        let completed = repository.aggregate_completed_tasks(Some(from), Some(today));
        self.rebuild_review_entries(completed);
        if self
            .review_list_state
            .selected
            .map_or(true, |i| i >= self.review_entries.len())
        {
            self.review_list_state = ListState::default();
            self.select_first_review_item();
        }
    }

    /// Rebuild flat entry list from raw task data, grouping by deadline category.
    fn rebuild_entries(
        &mut self,
        tasks: Vec<(tower_lsp::lsp_types::Url, patto::parser::AstNode, Deadline)>,
    ) {
        // Group into ordered categories
        let category_order = [
            DeadlineCategory::Overdue,
            DeadlineCategory::Today,
            DeadlineCategory::Tomorrow,
            DeadlineCategory::ThisWeek,
            DeadlineCategory::ThisMonth,
            DeadlineCategory::Later,
            DeadlineCategory::NoDeadline,
            DeadlineCategory::Uninterpretable,
        ];

        let mut buckets: Vec<Vec<TaskEntry>> = vec![Vec::new(); category_order.len()];

        for (uri, node, due) in &tasks {
            let cat = deadline_category(due);
            let due_str = match due {
                Deadline::Date(d) => d.format("%Y-%m-%d").to_string(),
                Deadline::DateTime(dt) => dt.format("%Y-%m-%d").to_string(),
                Deadline::Uninterpretable(s) => s.clone(),
            };
            let file_name = uri
                .to_file_path()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| uri.to_string());
            let line = node.location().row;
            let text = node.extract_str().trim_start().to_string();
            let (status, time_spent, started_at) = extract_task_meta(node);

            let bucket_idx = category_order.iter().position(|c| *c == cat).unwrap_or(5);
            buckets[bucket_idx].push(TaskEntry::TaskItem {
                text,
                file_name,
                uri: uri.clone(),
                line,
                due_str,
                category: cat,
                status,
                time_spent,
                started_at,
            });
        }

        let mut entries: Vec<TaskEntry> = Vec::new();
        for (cat, bucket) in category_order.iter().zip(buckets.iter()) {
            if !bucket.is_empty() {
                entries.push(TaskEntry::SectionHeader(cat.label().to_string()));
                entries.extend_from_slice(bucket);
            }
        }

        if entries.is_empty() {
            entries.push(TaskEntry::Placeholder("  (no pending tasks)".to_string()));
        }

        self.entries = entries;
    }

    /// Rebuild the review entry list from completed task data, grouped by recency.
    fn rebuild_review_entries(
        &mut self,
        completed: Vec<(Url, patto::parser::AstNode, chrono::NaiveDate)>,
    ) {
        let category_order = [
            CompletedCategory::Today,
            CompletedCategory::Yesterday,
            CompletedCategory::ThisWeek,
            CompletedCategory::LastWeek,
            CompletedCategory::ThisMonth,
            CompletedCategory::Older,
        ];

        let mut buckets: Vec<Vec<ReviewEntry>> = vec![Vec::new(); category_order.len()];

        for (uri, node, date) in &completed {
            let cat = completed_category(*date);
            let completed_at = date.format("%Y-%m-%d").to_string();
            let file_name = uri
                .to_file_path()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| uri.to_string());
            let line = node.location().row;
            let text = node.extract_str().trim_start().to_string();
            let (_, time_spent, _) = extract_task_meta(node);

            let bucket_idx = category_order.iter().position(|c| c == &cat).unwrap_or(5);
            buckets[bucket_idx].push(ReviewEntry::ReviewItem {
                text,
                file_name,
                uri: uri.clone(),
                line,
                completed_at,
                time_spent,
                category: cat,
            });
        }

        // Reverse each bucket so most-recently-completed appears first.
        for bucket in &mut buckets {
            bucket.reverse();
        }

        let mut entries: Vec<ReviewEntry> = Vec::new();
        for (cat, bucket) in category_order.iter().zip(buckets.iter()) {
            if !bucket.is_empty() {
                entries.push(ReviewEntry::SectionHeader(cat.label().to_string()));
                entries.extend_from_slice(bucket);
            }
        }

        if entries.is_empty() {
            entries.push(ReviewEntry::Placeholder(
                "  (no completed tasks in range)".to_string(),
            ));
        }

        self.review_entries = entries;
    }

    /// Select the first selectable entry.
    fn select_first_item(&mut self) {
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.is_selectable() {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    /// Select the first selectable review entry.
    fn select_first_review_item(&mut self) {
        for (i, entry) in self.review_entries.iter().enumerate() {
            if entry.is_selectable() {
                self.review_list_state.select(Some(i));
                return;
            }
        }
    }

    pub(crate) fn navigate_down(&mut self) {
        match self.view {
            TasksView::Upcoming => self.navigate_down_upcoming(),
            TasksView::Review => self.navigate_down_review(),
        }
    }

    fn navigate_down_upcoming(&mut self) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let start = self.list_state.selected.unwrap_or(0);
        let mut next = (start + 1) % len;
        for _ in 0..len {
            if self.entries[next].is_selectable() {
                break;
            }
            next = (next + 1) % len;
        }
        if self.entries[next].is_selectable() {
            self.list_state.select(Some(next));
        }
    }

    fn navigate_down_review(&mut self) {
        let len = self.review_entries.len();
        if len == 0 {
            return;
        }
        let start = self.review_list_state.selected.unwrap_or(0);
        let mut next = (start + 1) % len;
        for _ in 0..len {
            if self.review_entries[next].is_selectable() {
                break;
            }
            next = (next + 1) % len;
        }
        if self.review_entries[next].is_selectable() {
            self.review_list_state.select(Some(next));
        }
    }

    pub(crate) fn navigate_up(&mut self) {
        match self.view {
            TasksView::Upcoming => self.navigate_up_upcoming(),
            TasksView::Review => self.navigate_up_review(),
        }
    }

    fn navigate_up_upcoming(&mut self) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let start = self.list_state.selected.unwrap_or(0);
        let mut prev = if start == 0 { len - 1 } else { start - 1 };
        for _ in 0..len {
            if self.entries[prev].is_selectable() {
                break;
            }
            prev = if prev == 0 { len - 1 } else { prev - 1 };
        }
        if self.entries[prev].is_selectable() {
            self.list_state.select(Some(prev));
        }
    }

    fn navigate_up_review(&mut self) {
        let len = self.review_entries.len();
        if len == 0 {
            return;
        }
        let start = self.review_list_state.selected.unwrap_or(0);
        let mut prev = if start == 0 { len - 1 } else { start - 1 };
        for _ in 0..len {
            if self.review_entries[prev].is_selectable() {
                break;
            }
            prev = if prev == 0 { len - 1 } else { prev - 1 };
        }
        if self.review_entries[prev].is_selectable() {
            self.review_list_state.select(Some(prev));
        }
    }

    /// Resolve the current selection to a navigation target: `(uri, line)`.
    /// Dispatches to the current view.
    pub(crate) fn resolve_cursor(&self) -> Option<(Url, usize)> {
        match self.view {
            TasksView::Upcoming => {
                let idx = self.list_state.selected?;
                match self.entries.get(idx)? {
                    TaskEntry::TaskItem { uri, line, .. } => Some((uri.clone(), *line)),
                    _ => None,
                }
            }
            TasksView::Review => {
                let idx = self.review_list_state.selected?;
                match self.review_entries.get(idx)? {
                    ReviewEntry::ReviewItem { uri, line, .. } => Some((uri.clone(), *line)),
                    _ => None,
                }
            }
        }
    }

    /// Return a list of (status, text) for all active (Doing/Paused) tasks.
    /// Used by the fidget-style overlay.
    pub(crate) fn active_tasks(&self) -> Vec<(TaskStatus, String)> {
        self.entries
            .iter()
            .filter_map(|e| {
                if let TaskEntry::TaskItem { status, text, .. } = e {
                    if matches!(status, TaskStatus::Doing | TaskStatus::Paused) {
                        return Some((status.clone(), text.clone()));
                    }
                }
                None
            })
            .collect()
    }
}
