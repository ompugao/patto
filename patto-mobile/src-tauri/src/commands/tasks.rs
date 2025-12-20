// Task aggregation for patto-mobile
// Gathers tasks from all notes and categorizes by deadline

use chrono::{Local, NaiveDate, NaiveDateTime};
use patto::parser::{
    self, AstNode, AstNodeKind, Deadline, Property, TaskStatus as PattoTaskStatus,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Single task item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskItem {
    pub file_path: String,
    pub file_name: String,
    pub line_number: usize,
    pub content: String,
    pub status: String,
    pub due_date: Option<String>,
    pub due_timestamp: Option<i64>,
}

/// Categorized tasks by deadline
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAggregation {
    pub overdue: Vec<TaskItem>,
    pub today: Vec<TaskItem>,
    pub this_week: Vec<TaskItem>,
    pub later: Vec<TaskItem>,
    pub no_deadline: Vec<TaskItem>,
    pub done: Vec<TaskItem>,
}

/// Get all tasks from workspace categorized by deadline
#[tauri::command]
pub fn get_all_tasks(root: PathBuf) -> Result<TaskAggregation, String> {
    let mut aggregation = TaskAggregation::default();
    let today = Local::now().date_naive();
    let week_end = today + chrono::Duration::days(7);

    // Collect all patto files
    let files = collect_patto_files(&root).map_err(|e| e.to_string())?;

    for file_path in files {
        let full_path = root.join(&file_path);
        if let Ok(content) = fs::read_to_string(&full_path) {
            let tasks = extract_tasks_from_content(&content, &file_path);

            for task in tasks {
                match task.status.as_str() {
                    "done" => {
                        aggregation.done.push(task);
                    }
                    _ => {
                        // Categorize by deadline
                        match &task.due_timestamp {
                            Some(ts) => {
                                let due_date = NaiveDateTime::from_timestamp_opt(*ts, 0)
                                    .map(|dt| dt.date())
                                    .unwrap_or(today);

                                if due_date < today {
                                    aggregation.overdue.push(task);
                                } else if due_date == today {
                                    aggregation.today.push(task);
                                } else if due_date <= week_end {
                                    aggregation.this_week.push(task);
                                } else {
                                    aggregation.later.push(task);
                                }
                            }
                            None => {
                                aggregation.no_deadline.push(task);
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort each category by due date
    aggregation
        .overdue
        .sort_by(|a, b| a.due_timestamp.cmp(&b.due_timestamp));
    aggregation
        .today
        .sort_by(|a, b| a.due_timestamp.cmp(&b.due_timestamp));
    aggregation
        .this_week
        .sort_by(|a, b| a.due_timestamp.cmp(&b.due_timestamp));
    aggregation
        .later
        .sort_by(|a, b| a.due_timestamp.cmp(&b.due_timestamp));

    Ok(aggregation)
}

fn collect_patto_files(root: &Path) -> std::io::Result<Vec<String>> {
    let mut files = Vec::new();
    collect_patto_files_recursive(root, root, &mut files)?;
    Ok(files)
}

fn collect_patto_files_recursive(
    root: &Path,
    dir: &Path,
    files: &mut Vec<String>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files
        if path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        if path.is_dir() {
            collect_patto_files_recursive(root, &path, files)?;
        } else if path.extension().map(|e| e == "pn").unwrap_or(false) {
            let relative = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string());
            files.push(relative);
        }
    }
    Ok(())
}

fn extract_tasks_from_content(content: &str, file_path: &str) -> Vec<TaskItem> {
    let parse_result = parser::parse_text(content);
    let mut tasks = Vec::new();

    let file_name = PathBuf::from(file_path)
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    extract_tasks_from_ast(&parse_result.ast, file_path, &file_name, &mut tasks);

    tasks
}

fn extract_tasks_from_ast(
    node: &AstNode,
    file_path: &str,
    file_name: &str,
    tasks: &mut Vec<TaskItem>,
) {
    // Properties are inside Line and QuoteContent kinds
    let properties = match node.kind() {
        AstNodeKind::Line { properties } => Some(properties),
        AstNodeKind::QuoteContent { properties } => Some(properties),
        _ => None,
    };

    if let Some(props) = properties {
        for prop in props {
            if let Property::Task {
                status,
                due,
                location,
            } = prop
            {
                let status_str = match status {
                    PattoTaskStatus::Todo => "todo",
                    PattoTaskStatus::Doing => "doing",
                    PattoTaskStatus::Done => "done",
                };

                let (due_date, due_timestamp) = match due {
                    Deadline::DateTime(dt) => (
                        Some(dt.format("%Y-%m-%d %H:%M").to_string()),
                        Some(dt.and_utc().timestamp()),
                    ),
                    Deadline::Date(d) => (
                        Some(d.format("%Y-%m-%d").to_string()),
                        d.and_hms_opt(23, 59, 59).map(|dt| dt.and_utc().timestamp()),
                    ),
                    Deadline::Uninterpretable(s) => (Some(s.clone()), None),
                };

                // Extract line content
                let line_content = node.extract_str().lines().next().unwrap_or("").to_string();

                tasks.push(TaskItem {
                    file_path: file_path.to_string(),
                    file_name: file_name.to_string(),
                    line_number: location.row,
                    content: line_content,
                    status: status_str.to_string(),
                    due_date,
                    due_timestamp,
                });
            }
        }
    }

    // Recurse into contents
    for child in node.value().contents.lock().unwrap().iter() {
        extract_tasks_from_ast(child, file_path, file_name, tasks);
    }
    // Recurse into children
    for child in node.value().children.lock().unwrap().iter() {
        extract_tasks_from_ast(child, file_path, file_name, tasks);
    }
}

/// Get tasks from a single file
#[tauri::command]
pub fn get_file_tasks(root: PathBuf, file_path: String) -> Result<Vec<TaskItem>, String> {
    let full_path = root.join(&file_path);

    if !full_path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let content = fs::read_to_string(&full_path).map_err(|e| format!("Failed to read: {}", e))?;
    Ok(extract_tasks_from_content(&content, &file_path))
}

/// Task summary counts
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub total: usize,
    pub overdue: usize,
    pub today: usize,
    pub this_week: usize,
    pub later: usize,
    pub no_deadline: usize,
    pub done: usize,
}

/// Get task summary counts
#[tauri::command]
pub fn get_task_summary(root: PathBuf) -> Result<TaskSummary, String> {
    let tasks = get_all_tasks(root)?;

    Ok(TaskSummary {
        total: tasks.overdue.len()
            + tasks.today.len()
            + tasks.this_week.len()
            + tasks.later.len()
            + tasks.no_deadline.len(),
        overdue: tasks.overdue.len(),
        today: tasks.today.len(),
        this_week: tasks.this_week.len(),
        later: tasks.later.len(),
        no_deadline: tasks.no_deadline.len(),
        done: tasks.done.len(),
    })
}
