//! Maps Patto tasks to Google Calendar events.

use crate::gcal::config::GcalConfig;
use crate::gcal::state::TaskFingerprint;
use crate::parser::{Deadline, Span, TaskStatus};
use chrono::{DateTime, TimeZone, Utc};

/// Represents a Patto task ready for sync
#[derive(Debug, Clone)]
pub struct PattoTask {
    /// File path relative to repository root
    pub file_path: String,

    /// Line number in the file (1-indexed)
    pub line_number: usize,

    /// Full text content of the task (trimmed line)
    pub content: String,

    /// Task deadline
    pub deadline: Deadline,

    /// Task status
    pub status: TaskStatus,

    /// Task fingerprint for tracking
    pub fingerprint: TaskFingerprint,

    /// Span of the task marker within the content (relative to trimmed content)
    pub task_marker_span: Option<Span>,
}

/// Maps a Patto task to a Google Calendar event
/// Returns a simple JSON-serializable struct instead of google_calendar3::api::Event
/// because the latter serializes all None fields as null which the API rejects.
pub fn task_to_event(task: &PattoTask, config: &GcalConfig) -> SimpleEvent {
    let title = format_event_title(task, config);
    let description = format_event_description(task, config);
    let (start, end) = format_event_time(task, config);

    SimpleEvent {
        summary: title,
        description,
        start,
        end,
        color_id: Some(status_to_color(&task.status)),
        extended_properties: Some(SimpleExtendedProperties {
            private: {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "patto_hash".to_string(),
                    task.fingerprint.content_hash.clone(),
                );
                map.insert("patto_file".to_string(), task.file_path.clone());
                map.insert("patto_line".to_string(), task.line_number.to_string());
                map
            },
        }),
    }
}

/// Simplified event structure for API requests
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleEvent {
    pub summary: String,
    pub description: String,
    pub start: SimpleEventDateTime,
    pub end: SimpleEventDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_properties: Option<SimpleExtendedProperties>,
}

/// Simplified event date/time
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleEventDateTime {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
}

/// Simplified extended properties
#[derive(Debug, Clone, serde::Serialize)]
pub struct SimpleExtendedProperties {
    pub private: std::collections::HashMap<String, String>,
}

/// Format the event title
fn format_event_title(task: &PattoTask, config: &GcalConfig) -> String {
    // Get first line of content, strip the deadline marker
    let first_line = task.content.lines().next().unwrap_or("").trim();

    // Remove task marker using span if available, otherwise fall back to pattern matching
    let clean_title = extract_clean_title(first_line, task.task_marker_span.as_ref());

    if config.event_prefix.is_empty() {
        clean_title
    } else {
        format!("{} {}", config.event_prefix, clean_title)
    }
}

/// Extract clean title by removing task marker using span information
fn extract_clean_title(line: &str, task_marker_span: Option<&Span>) -> String {
    if let Some(span) = task_marker_span {
        // Use parser-provided span to extract text before and after the marker
        let start = span.0.min(line.len());
        let end = span.1.min(line.len());

        if start <= end && end <= line.len() {
            let before = line[..start].trim_end();
            let after = line[end..].trim_start();
            let result = if !before.is_empty() && !after.is_empty() {
                format!("{} {}", before, after)
            } else {
                format!("{}{}", before, after)
            };
            let result = result.trim().to_string();
            if !result.is_empty() {
                return result;
            }
        }
    }
    log::warn!("task span is not provided!");
    return line.trim().to_string();
}

/// Format the event description
fn format_event_description(task: &PattoTask, config: &GcalConfig) -> String {
    let mut description = String::new();

    if config.include_file_path {
        description.push_str(&format!(
            "📁 File: {}:{}\n\n",
            task.file_path, task.line_number
        ));
    }

    description.push_str(&task.content);

    description.push_str("\n\n---\n");
    description.push_str("Synced from Patto Notes");

    description
}

/// Format event start and end times
fn format_event_time(
    task: &PattoTask,
    config: &GcalConfig,
) -> (SimpleEventDateTime, SimpleEventDateTime) {
    match &task.deadline {
        Deadline::DateTime(dt) => {
            // Convert NaiveDateTime to DateTime<Utc>
            let start_utc: DateTime<Utc> = Utc.from_utc_datetime(dt);
            let end_dt = *dt + chrono::Duration::hours(config.default_duration_hours as i64);
            let end_utc: DateTime<Utc> = Utc.from_utc_datetime(&end_dt);

            let start = SimpleEventDateTime {
                date: None,
                date_time: Some(start_utc.to_rfc3339()),
                time_zone: config.timezone.clone(),
            };

            let end = SimpleEventDateTime {
                date: None,
                date_time: Some(end_utc.to_rfc3339()),
                time_zone: config.timezone.clone(),
            };

            (start, end)
        }
        Deadline::Date(d) => {
            // All-day event
            let next_day = *d + chrono::Duration::days(1);

            let start = SimpleEventDateTime {
                date: Some(d.format("%Y-%m-%d").to_string()),
                date_time: None,
                time_zone: None,
            };

            let end = SimpleEventDateTime {
                date: Some(next_day.format("%Y-%m-%d").to_string()),
                date_time: None,
                time_zone: None,
            };

            (start, end)
        }
        Deadline::Uninterpretable(s) => {
            // Try to parse or default to today
            let today = chrono::Local::now().date_naive();
            let next_day = today + chrono::Duration::days(1);

            let start = SimpleEventDateTime {
                date: Some(today.format("%Y-%m-%d").to_string()),
                date_time: None,
                time_zone: None,
            };

            let end = SimpleEventDateTime {
                date: Some(next_day.format("%Y-%m-%d").to_string()),
                date_time: None,
                time_zone: None,
            };

            log::warn!("Uninterpretable deadline '{}', using today as fallback", s);

            (start, end)
        }
    }
}

/// Map task status to Google Calendar color ID
fn status_to_color(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Todo => "9".to_string(),  // Blue
        TaskStatus::Doing => "5".to_string(), // Yellow/Orange
        TaskStatus::Done => "8".to_string(),  // Gray
    }
}

/// Extract patto hash from a calendar event response (JSON value)
pub fn extract_patto_hash(event: &serde_json::Value) -> Option<String> {
    event
        .get("extendedProperties")
        .and_then(|ep| ep.get("private"))
        .and_then(|private| private.get("patto_hash"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test the span-based extraction (primary method)
    #[test]
    fn test_extract_clean_title_with_span() {
        // "!2024-12-31 Complete docs" - marker at start, span (0, 11)
        assert_eq!(
            extract_clean_title("!2024-12-31 Complete docs", Some(&Span(0, 11))),
            "Complete docs"
        );

        // "Meeting {@task due=2024-12-31}" - marker at end, span (8, 30)
        assert_eq!(
            extract_clean_title("Meeting {@task due=2024-12-31}", Some(&Span(8, 30))),
            "Meeting"
        );

        // "hoge {@task status=todo due=2026-01-19} fuga" - marker in middle
        assert_eq!(
            extract_clean_title(
                "hoge {@task status=todo due=2026-01-19} fuga",
                Some(&Span(5, 39))
            ),
            "hoge fuga"
        );
    }
}
