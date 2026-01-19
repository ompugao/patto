//! State management for tracking synced tasks.
//!
//! Uses content-based hashing to track tasks even when line numbers change.

use crate::parser::Deadline;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

/// Represents a fingerprint of a task for tracking across file changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFingerprint {
    /// Path to the file containing the task (relative to repo root)
    pub file_path: String,

    /// Last known line number (hint for fuzzy matching)
    pub last_known_line: usize,

    /// SHA256 hash of normalized task content (primary identifier)
    pub content_hash: String,

    /// Short snippet of the task content for display
    pub content_snippet: String,

    /// Task deadline as string
    pub deadline: String,

    /// Hash of surrounding context lines for disambiguation
    pub context_hash: Option<String>,
}

/// Represents a synced task entry in the state file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncedTask {
    /// Google Calendar event ID
    pub calendar_event_id: String,

    /// Task fingerprint for matching
    #[serde(flatten)]
    pub fingerprint: TaskFingerprint,

    /// When this task was last synced
    pub last_synced: DateTime<Utc>,

    /// Whether the event needs to be updated
    #[serde(default)]
    pub needs_update: bool,
}

/// The sync state containing all tracked tasks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncState {
    /// All synced tasks, keyed by content_hash for fast lookup
    pub tasks: HashMap<String, SyncedTask>,

    /// Last full sync timestamp
    pub last_full_sync: Option<DateTime<Utc>>,
}

impl SyncState {
    /// Load state from the state file
    pub fn load(state_path: &Path) -> anyhow::Result<Self> {
        if state_path.exists() {
            let content = std::fs::read_to_string(state_path)?;
            let state: SyncState = serde_json::from_str(&content)?;
            Ok(state)
        } else {
            Ok(Self::default())
        }
    }

    /// Save state to the state file
    pub fn save(&self, state_path: &Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(state_path, content)?;
        Ok(())
    }

    /// Find a synced task by content hash
    pub fn find_by_hash(&self, content_hash: &str) -> Option<&SyncedTask> {
        self.tasks.get(content_hash)
    }

    /// Find a synced task by file and approximate line (fuzzy matching)
    pub fn find_by_location(
        &self,
        file_path: &str,
        line: usize,
        tolerance: usize,
    ) -> Vec<&SyncedTask> {
        self.tasks
            .values()
            .filter(|task| {
                task.fingerprint.file_path == file_path
                    && (task.fingerprint.last_known_line as isize - line as isize).unsigned_abs()
                        <= tolerance
            })
            .collect()
    }

    /// Add or update a synced task
    pub fn upsert(&mut self, task: SyncedTask) {
        self.tasks
            .insert(task.fingerprint.content_hash.clone(), task);
    }

    /// Remove a synced task by content hash
    pub fn remove(&mut self, content_hash: &str) -> Option<SyncedTask> {
        self.tasks.remove(content_hash)
    }

    /// Get all synced tasks for a specific file
    pub fn tasks_for_file(&self, file_path: &str) -> Vec<&SyncedTask> {
        self.tasks
            .values()
            .filter(|task| task.fingerprint.file_path == file_path)
            .collect()
    }
}

/// Compute a content hash for a task
pub fn compute_content_hash(content: &str) -> String {
    let normalized = normalize_content(content);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..16]) // Use first 16 bytes (32 hex chars)
}

/// Normalize content for hashing (trim, normalize whitespace)
fn normalize_content(content: &str) -> String {
    content
        .lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Compute a context hash from surrounding lines
pub fn compute_context_hash(lines: &[&str]) -> String {
    let combined = lines.join("\n");
    compute_content_hash(&combined)
}

/// Calculate similarity between two strings using Levenshtein distance
pub fn similarity(a: &str, b: &str) -> f64 {
    let distance = strsim::levenshtein(a, b);
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 1.0;
    }
    1.0 - (distance as f64 / max_len as f64)
}

/// Match confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchConfidence {
    /// Exact hash match
    Exact,
    /// High similarity (>0.9)
    High,
    /// Medium similarity (>0.7)
    Medium,
    /// Low similarity (>0.5)
    Low,
    /// No match found
    NotFound,
}

impl MatchConfidence {
    pub fn from_similarity(sim: f64) -> Self {
        if sim >= 0.99 {
            Self::Exact
        } else if sim >= 0.9 {
            Self::High
        } else if sim >= 0.7 {
            Self::Medium
        } else if sim >= 0.5 {
            Self::Low
        } else {
            Self::NotFound
        }
    }

    pub fn should_auto_update(&self) -> bool {
        matches!(self, Self::Exact | Self::High)
    }
}

/// Create a task fingerprint from task data
pub fn create_fingerprint(
    file_path: &str,
    line: usize,
    content: &str,
    deadline: &Deadline,
    context_lines: Option<&[&str]>,
) -> TaskFingerprint {
    let snippet = content.chars().take(50).collect::<String>();
    let content_hash = compute_content_hash(&content);

    TaskFingerprint {
        file_path: file_path.to_string(),
        last_known_line: line,
        content_hash,
        content_snippet: snippet,
        deadline: deadline.to_string(),
        context_hash: context_lines.map(compute_context_hash),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_stability() {
        let content = "Complete documentation";
        let hash1 = compute_content_hash(content);
        let hash2 = compute_content_hash(content);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_normalization() {
        let content1 = "  Complete   documentation  ";
        let content2 = "Complete documentation";
        let hash1 = compute_content_hash(content1);
        let hash2 = compute_content_hash(content2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_similarity() {
        assert!(similarity("hello", "hello") > 0.99);
        assert!(similarity("hello", "hallo") > 0.7);
        assert!(similarity("hello", "world") < 0.5);
    }
}
