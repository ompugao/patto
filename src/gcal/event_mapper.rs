//! Maps Patto tasks to Google Calendar events.

use crate::gcal::config::GcalConfig;
use crate::gcal::state::TaskFingerprint;
use crate::parser::{Deadline, TaskStatus};
use chrono::{DateTime, TimeZone, Utc};

/// Represents a Patto task ready for sync
#[derive(Debug, Clone)]
pub struct PattoTask {
    /// File path relative to repository root
    pub file_path: String,

    /// Line number in the file (1-indexed)
    pub line_number: usize,

    /// Full text content of the task
    pub content: String,

    /// Task deadline
    pub deadline: Deadline,

    /// Task status
    pub status: TaskStatus,

    /// Task fingerprint for tracking
    pub fingerprint: TaskFingerprint,
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
                map.insert("patto_hash".to_string(), task.fingerprint.content_hash.clone());
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

    // Remove deadline markers like !2024-12-31 or *2024-12-31
    let clean_title = remove_deadline_marker(first_line);

    if config.event_prefix.is_empty() {
        clean_title
    } else {
        format!("{} {}", config.event_prefix, clean_title)
    }
}

/// Remove deadline marker from a line (can be at beginning or end)
fn remove_deadline_marker(line: &str) -> String {
    // Check for {@task ...} format - extract content before and after the block
    if let Some(start_idx) = line.find("{@task") {
        if let Some(end_idx) = line.find('}') {
            let before = &line[..start_idx];
            let after = &line[end_idx+1..];
            let result = format!("{}{}", before, after).trim().to_string();
            if !result.is_empty() {
                return result;
            }
        }
    }

    // Pattern: [!*-]YYYY-MM-DD or [!*-]YYYY-MM-DDTHH:MM:SS at beginning
    if line.len() >= 11 {
        let first_char = line.chars().next().unwrap_or(' ');
        if matches!(first_char, '!' | '*' | '-') {
            // Check if followed by date pattern
            let rest = &line[1..];
            if rest.len() >= 10
                && rest.chars().take(4).all(|c| c.is_ascii_digit())
                && rest.chars().nth(4) == Some('-')
            {
                // Find where the date ends
                let date_end = if rest.len() >= 19 && rest.chars().nth(10) == Some('T') {
                    19 // DateTime format
                } else {
                    10 // Date format
                };

                return rest[date_end..].trim().to_string();
            }
        }
    }

    // Pattern: [!*-]YYYY-MM-DD or [!*-]YYYY-MM-DDTHH:MM:SS at end
    // Look for pattern like "Meeting !2024-12-31T14:00:00"
    let date_markers = ['!', '*', '-'];
    for marker in date_markers {
        if let Some(marker_pos) = line.rfind(marker) {
            let after_marker = &line[marker_pos + 1..];
            if after_marker.len() >= 10
                && after_marker.chars().take(4).all(|c| c.is_ascii_digit())
                && after_marker.chars().nth(4) == Some('-')
            {
                // This looks like a date marker at the end
                return line[..marker_pos].trim().to_string();
            }
        }
    }

    line.trim().to_string()
}

/// Format the event description
fn format_event_description(task: &PattoTask, config: &GcalConfig) -> String {
    let mut description = String::new();

    if config.include_file_path {
        description.push_str(&format!("📁 File: {}:{}\n\n", task.file_path, task.line_number));
    }

    description.push_str(&task.content);

    description.push_str("\n\n---\n");
    description.push_str("Synced from Patto Notes");

    description
}

/// Format event start and end times
fn format_event_time(task: &PattoTask, config: &GcalConfig) -> (SimpleEventDateTime, SimpleEventDateTime) {
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

    #[test]
    fn test_remove_deadline_marker_short() {
        assert_eq!(
            remove_deadline_marker("!2024-12-31 Complete docs"),
            "Complete docs"
        );
        assert_eq!(
            remove_deadline_marker("*2024-12-31 In progress"),
            "In progress"
        );
        assert_eq!(remove_deadline_marker("-2024-12-31 Done"), "Done");
    }

    #[test]
    fn test_remove_deadline_marker_datetime() {
        assert_eq!(
            remove_deadline_marker("Meeting !2024-12-31T14:00:00"),
            "Meeting"
        );
    }

    #[test]
    fn test_remove_deadline_marker_property_style() {
        assert_eq!(
            remove_deadline_marker("Meeting {@task due=2024-12-31T14:00:00}"),
            "Meeting"
        );
    }

    #[test]
    fn test_remove_deadline_marker_property_style2() {
        assert_eq!(
            remove_deadline_marker("Meeting {@task due=2024-12-31T14:00:00} 2"),
            "Meeting  2"
        );
    }

    #[test]
    fn test_remove_deadline_marker_property_style3() {
        assert_eq!(
            remove_deadline_marker("hoge {@task status=todo due=2026-01-19} fuga"),
            "hoge  fuga"
        );
    }
    





    #[test]
    fn test_remove_deadline_marker_no_marker() {
        assert_eq!(remove_deadline_marker("Regular text"), "Regular text");
    }
}
