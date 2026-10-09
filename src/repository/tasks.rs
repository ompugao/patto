use url::Url;

use crate::ast_query::{gather_completed_tasks, gather_tasks};
use crate::parser::{AstNode, Deadline};

use super::Repository;

impl Repository {
    /// Collect all non-Done tasks across the entire workspace, sorted by deadline.
    ///
    /// Returns `(uri, ast_node, deadline)` tuples where `ast_node` is the task line.
    pub fn aggregate_tasks(&self) -> Vec<(Url, AstNode, Deadline)> {
        let mut tasks: Vec<(Url, AstNode, Deadline)> = Vec::new();
        self.ast_map.iter().for_each(|entry| {
            let mut tasklines = Vec::new();
            gather_tasks(entry.value(), &mut tasklines);
            for (node, due) in tasklines {
                tasks.push((entry.key().clone(), node, due));
            }
        });
        tasks.sort_by_key(|(_, _, due)| due.clone());
        tasks
    }

    /// Collect Done tasks whose `completed_at` falls within [from, to] (inclusive).
    /// Pass `None` for either bound to leave it open.
    pub fn aggregate_completed_tasks(
        &self,
        from: Option<chrono::NaiveDate>,
        to: Option<chrono::NaiveDate>,
    ) -> Vec<(Url, AstNode, chrono::NaiveDate)> {
        let mut tasks: Vec<(Url, AstNode, chrono::NaiveDate)> = Vec::new();
        self.ast_map.iter().for_each(|entry| {
            let mut completed = Vec::new();
            gather_completed_tasks(entry.value(), &mut completed);
            for (node, date) in completed {
                let in_range = from.is_none_or(|f| date >= f) && to.is_none_or(|t| date <= t);
                if in_range {
                    tasks.push((entry.key().clone(), node, date));
                }
            }
        });
        tasks.sort_by_key(|(_, _, date)| *date);
        tasks
    }
}
