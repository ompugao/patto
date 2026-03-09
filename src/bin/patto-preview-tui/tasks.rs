use chrono::Local;
use patto::{parser::Deadline, repository::Repository};
use tower_lsp::lsp_types::Url;
use tui_widget_list::ListState;

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

/// A single display entry in the flat tasks list.
#[derive(Clone)]
pub(crate) enum TaskEntry {
    /// Deadline group section header.
    SectionHeader(String),
    /// A single task item.
    TaskItem {
        text: String,
        file_name: String,
        uri: Url,
        /// 0-based line number within the file.
        line: usize,
        due_str: String,
        category: DeadlineCategory,
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
    pub(crate) entries: Vec<TaskEntry>,
    pub(crate) list_state: ListState,
}

impl TasksPanel {
    pub(crate) fn new() -> Self {
        Self {
            visible: false,
            entries: Vec::new(),
            list_state: ListState::default(),
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
        self.list_state = ListState::default();
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

            let bucket_idx = category_order.iter().position(|c| *c == cat).unwrap_or(5);
            buckets[bucket_idx].push(TaskEntry::TaskItem {
                text,
                file_name,
                uri: uri.clone(),
                line,
                due_str,
                category: cat,
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

    /// Select the first selectable entry.
    fn select_first_item(&mut self) {
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.is_selectable() {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    pub(crate) fn navigate_down(&mut self) {
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

    pub(crate) fn navigate_up(&mut self) {
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

    /// Resolve the current selection to a navigation target: `(uri, line)`.
    pub(crate) fn resolve_cursor(&self) -> Option<(Url, usize)> {
        let idx = self.list_state.selected?;
        match self.entries.get(idx)? {
            TaskEntry::TaskItem { uri, line, .. } => Some((uri.clone(), *line)),
            _ => None,
        }
    }
}
