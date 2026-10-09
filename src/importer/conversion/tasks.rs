use regex::Regex;

use super::Conversion;
use crate::parser::{Deadline, Location, Property, TaskStatus};

impl Conversion<'_> {
    /// The task property for a checklist item, with the dates written in its text.
    pub(super) fn task_property(&mut self, checked: bool) -> Vec<Property> {
        let text: String = self.pending.iter().map(|node| node.extract_str()).collect();
        let date = |value: Option<String>| {
            value
                .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
                .map(Deadline::Date)
        };

        self.report.statistics.increment_feature("tasks");
        vec![Property::Task {
            status: if checked {
                TaskStatus::Done
            } else {
                TaskStatus::Todo
            },
            status_is_canonical: true,
            due: date(extract_due_date(&text)).unwrap_or(Deadline::Uninterpretable(String::new())),
            scheduled: date(extract_scheduled_date(&text)),
            completed_at: date(extract_completed_at_date(&text)),
            started_at: None,
            time_spent: None,
            location: Location::default(),
        }]
    }
}

/// Due date written as `📅 D`, `(due: D)`, `[due:: D]` or `@D`.
fn extract_due_date(text: &str) -> Option<String> {
    first_capture(
        text,
        &[
            r"📅\s*(\d{4}-\d{2}-\d{2})",
            r"\(due:\s*(\d{4}-\d{2}-\d{2})\)",
            r"\[due::\s*(\d{4}-\d{2}-\d{2})\]",
            r"@(\d{4}-\d{2}-\d{2})",
        ],
    )
}

/// Scheduled date written as `⏳ D`, `(scheduled: D)` or `[scheduled:: D]`.
fn extract_scheduled_date(text: &str) -> Option<String> {
    first_capture(
        text,
        &[
            r"⏳\s*(\d{4}-\d{2}-\d{2})",
            r"\(scheduled:\s*(\d{4}-\d{2}-\d{2})\)",
            r"\[scheduled::\s*(\d{4}-\d{2}-\d{2})\]",
        ],
    )
}

/// Completion date written as `✅ D`, `(completed: D)` or `[completed_at:: D]`.
fn extract_completed_at_date(text: &str) -> Option<String> {
    first_capture(
        text,
        &[
            r"✅\s*(\d{4}-\d{2}-\d{2})",
            r"\(completed:\s*(\d{4}-\d{2}-\d{2})\)",
            r"\[completed_at::\s*(\d{4}-\d{2}-\d{2})\]",
        ],
    )
}

/// First capture group matched by any of `patterns`, tried in order.
fn first_capture(text: &str, patterns: &[&str]) -> Option<String> {
    patterns.iter().find_map(|pattern| {
        Regex::new(pattern)
            .unwrap()
            .captures(text)?
            .get(1)
            .map(|m| m.as_str().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_due_date() {
        assert_eq!(
            extract_due_date("task 📅 2024-12-31"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            extract_due_date("task (due: 2024-12-31)"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            extract_due_date("task [due:: 2024-12-31]"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            extract_due_date("task @2024-12-31"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(extract_due_date("task without date"), None);
    }
}
