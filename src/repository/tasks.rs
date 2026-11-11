use chrono::NaiveDate;
use url::Url;

use crate::ast_query::{gather_task_lines, TaskLine};
use crate::parser::TaskStatus;

use super::Repository;

/// Inclusive date range; `None` leaves that end open.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DateRange {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

impl DateRange {
    pub fn contains(&self, date: NaiveDate) -> bool {
        self.from.is_none_or(|f| date >= f) && self.to.is_none_or(|t| date <= t)
    }
}

/// Which tasks [`Repository::tasks`] returns. Resolved tasks are matched by the
/// date they were resolved on, so one without that date never matches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskQuery {
    /// Todo, Doing and Paused tasks.
    pub open: bool,
    /// Done tasks whose `completed_at` falls in the range.
    pub done: Option<DateRange>,
    /// Archived tasks whose `archived_at` falls in the range.
    pub archived: Option<DateRange>,
}

impl TaskQuery {
    pub fn open() -> Self {
        Self {
            open: true,
            ..Self::default()
        }
    }

    pub fn matches(&self, task: &TaskLine) -> bool {
        let resolved_in = |range: Option<DateRange>| {
            range
                .zip(task.resolved_on)
                .is_some_and(|(r, d)| r.contains(d))
        };
        match task.status {
            TaskStatus::Done => resolved_in(self.done),
            TaskStatus::Archived => resolved_in(self.archived),
            _ => self.open,
        }
    }
}

impl Repository {
    /// Tasks across the workspace that match `query`: open tasks by deadline
    /// first, then resolved tasks by the date they were resolved on.
    pub fn tasks(&self, query: &TaskQuery) -> Vec<(Url, TaskLine)> {
        let mut tasks: Vec<(Url, TaskLine)> = Vec::new();
        for entry in self.ast_map.iter() {
            let mut lines = Vec::new();
            gather_task_lines(entry.value(), &mut lines);
            tasks.extend(
                lines
                    .into_iter()
                    .filter(|task| query.matches(task))
                    .map(|task| (entry.key().clone(), task)),
            );
        }
        tasks.sort_by(|(_, a), (_, b)| sort_key(a).cmp(&sort_key(b)));
        tasks
    }
}

fn sort_key(task: &TaskLine) -> (bool, Option<NaiveDate>, &crate::parser::Deadline) {
    let resolved = matches!(task.status, TaskStatus::Done | TaskStatus::Archived);
    (resolved, task.resolved_on, &task.due)
}
