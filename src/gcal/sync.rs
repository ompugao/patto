//! Sync logic for Google Calendar integration.

use crate::gcal::auth::get_access_token;
use crate::gcal::config::GcalConfig;
use crate::gcal::event_mapper::{task_to_event, PattoTask};
use crate::gcal::state::{create_fingerprint, similarity, MatchConfidence, SyncState, SyncedTask};
use crate::parser::{parse_text, AstNode, AstNodeKind, Property, Span, TaskStatus};
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::Path;

/// Sync statistics
#[derive(Debug, Default)]
pub struct SyncStats {
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Sync action to be performed
#[derive(Debug)]
pub enum SyncAction {
    Create(PattoTask),
    Update { task: PattoTask, event_id: String },
    Delete { event_id: String, reason: String },
    Skip { task: PattoTask, reason: String },
}

/// Main sync orchestrator
pub struct GcalSync {
    config: GcalConfig,
    state: SyncState,
    client: reqwest::Client,
    access_token: String,
}

impl GcalSync {
    /// Create a new sync instance
    pub async fn new(config: GcalConfig) -> Result<Self> {
        let access_token = get_access_token(&config).await?;
        let state_path = GcalConfig::state_path()?;
        let state = SyncState::load(&state_path).unwrap_or_default();
        let client = reqwest::Client::new();

        Ok(Self {
            config,
            state,
            client,
            access_token,
        })
    }

    /// Perform a full sync
    pub async fn sync(&mut self, repo_path: &Path, dry_run: bool) -> Result<SyncStats> {
        let mut stats = SyncStats::default();

        // 1. Gather all tasks from patto files
        let tasks = self.gather_tasks(repo_path)?;
        log::info!("Found {} tasks with deadlines", tasks.len());

        // 2. Determine sync actions
        log::debug!("Determining sync actions...");
        let actions = self.determine_actions(&tasks)?;
        log::debug!("Found {} actions to take", actions.len());

        // 3. Execute actions (or just report if dry_run)
        for action in actions {
            match action {
                SyncAction::Create(task) => {
                    if dry_run {
                        println!(
                            "Would create: \"{}\" ({})",
                            task.fingerprint.content_snippet, task.deadline
                        );
                        stats.created += 1;
                    } else {
                        log::debug!("About to create event...");
                        match self.create_event(&task).await {
                            Ok(event_id) => {
                                log::info!("Created event: {}", task.fingerprint.content_snippet);
                                self.state.upsert(SyncedTask {
                                    calendar_event_id: event_id,
                                    fingerprint: task.fingerprint,
                                    last_synced: Utc::now(),
                                    needs_update: false,
                                });
                                stats.created += 1;
                            }
                            Err(e) => {
                                stats.errors.push(format!(
                                    "Failed to create '{}': {}",
                                    task.fingerprint.content_snippet, e
                                ));
                            }
                        }
                    }
                }
                SyncAction::Update { task, event_id } => {
                    if dry_run {
                        println!(
                            "Would update: \"{}\" ({})",
                            task.fingerprint.content_snippet, task.deadline
                        );
                        stats.updated += 1;
                    } else {
                        match self.update_event(&event_id, &task).await {
                            Ok(_) => {
                                log::info!("Updated event: {}", task.fingerprint.content_snippet);
                                self.state.upsert(SyncedTask {
                                    calendar_event_id: event_id,
                                    fingerprint: task.fingerprint,
                                    last_synced: Utc::now(),
                                    needs_update: false,
                                });
                                stats.updated += 1;
                            }
                            Err(e) => {
                                stats.errors.push(format!(
                                    "Failed to update '{}': {}",
                                    task.fingerprint.content_snippet, e
                                ));
                            }
                        }
                    }
                }
                SyncAction::Delete { event_id, reason } => {
                    if dry_run {
                        println!("Would delete event: {} ({})", event_id, reason);
                        stats.deleted += 1;
                    } else {
                        match self.delete_event(&event_id).await {
                            Ok(_) => {
                                log::info!("Deleted event: {}", event_id);
                                // Find and remove from state
                                let hash_to_remove: Option<String> = self
                                    .state
                                    .tasks
                                    .iter()
                                    .find(|(_, t)| t.calendar_event_id == event_id)
                                    .map(|(h, _)| h.clone());
                                if let Some(hash) = hash_to_remove {
                                    self.state.remove(&hash);
                                }
                                stats.deleted += 1;
                            }
                            Err(e) => {
                                stats
                                    .errors
                                    .push(format!("Failed to delete '{}': {}", event_id, e));
                            }
                        }
                    }
                }
                SyncAction::Skip { task, reason } => {
                    log::debug!(
                        "Skipping '{}': {}",
                        task.fingerprint.content_snippet,
                        reason
                    );
                    stats.skipped += 1;
                }
            }
        }

        // 4. Save state
        if !dry_run {
            self.state.last_full_sync = Some(Utc::now());
            let state_path = GcalConfig::state_path()?;
            self.state.save(&state_path)?;
        }

        Ok(stats)
    }

    /// Gather all tasks from patto files in the repository
    fn gather_tasks(&self, repo_path: &Path) -> Result<Vec<PattoTask>> {
        let mut tasks = Vec::new();

        // Find all .pn files
        let pattern = repo_path.join("**/*.pn");
        let pattern_str = pattern.to_string_lossy();

        for entry in glob::glob(&pattern_str)? {
            let path = entry?;
            let content = std::fs::read_to_string(&path)?;
            let relative_path = path
                .strip_prefix(repo_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            // Parse the file
            let parse_result = parse_text(&content);
            let ast = parse_result.ast;

            // Extract tasks from AST
            self.extract_tasks_from_ast(&ast, &relative_path, &content, &mut tasks);
        }

        Ok(tasks)
    }

    /// Extract tasks from AST recursively
    fn extract_tasks_from_ast(
        &self,
        node: &AstNode,
        file_path: &str,
        file_content: &str,
        tasks: &mut Vec<PattoTask>,
    ) {
        if let AstNodeKind::Line { ref properties } = node.kind() {
            for prop in properties {
                if let Property::Task {
                    status,
                    due,
                    location,
                } = prop
                {
                    // Skip done tasks unless configured otherwise
                    if matches!(status, TaskStatus::Done) && !self.config.sync_done_tasks {
                        continue;
                    }

                    let line_number = location.row + 1;
                    let raw_line = node.extract_str();
                    let trimmed_content = raw_line.trim().to_string();

                    // Calculate span offset due to trimming (leading whitespace removed)
                    let leading_ws = raw_line.len() - raw_line.trim_start().len();
                    let task_marker_span = {
                        let orig_span = &location.span;
                        // Adjust span to be relative to trimmed content
                        let start = orig_span.0.saturating_sub(leading_ws);
                        let end = orig_span.1.saturating_sub(leading_ws);
                        if start < end && end <= trimmed_content.len() {
                            Some(Span(start, end))
                        } else {
                            None
                        }
                    };

                    let fingerprint =
                        create_fingerprint(file_path, line_number, &trimmed_content, due, None);

                    tasks.push(PattoTask {
                        file_path: file_path.to_string(),
                        line_number,
                        content: trimmed_content,
                        deadline: due.clone(),
                        status: match status {
                            TaskStatus::Todo => TaskStatus::Todo,
                            TaskStatus::Doing => TaskStatus::Doing,
                            TaskStatus::Done => TaskStatus::Done,
                        },
                        fingerprint,
                        task_marker_span,
                    });
                    break;
                }
            }
        }

        // Recurse into children
        for child in node.value().children.lock().unwrap().iter() {
            self.extract_tasks_from_ast(child, file_path, file_content, tasks);
        }
    }

    /// Determine what sync actions need to be taken
    fn determine_actions(&self, tasks: &[PattoTask]) -> Result<Vec<SyncAction>> {
        let mut actions = Vec::new();
        let mut matched_hashes: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for task in tasks {
            let content_hash = &task.fingerprint.content_hash;

            // Check if this task has an uninterpretable (no) deadline
            let has_no_due_date =
                matches!(task.deadline, crate::parser::Deadline::Uninterpretable(_));

            // Try exact match by content hash
            if let Some(synced) = self.state.find_by_hash(content_hash) {
                matched_hashes.insert(content_hash.clone());

                // Check if update needed (deadline changed, etc.)
                // Also always update tasks with no due date so they stay on "today"
                if synced.fingerprint.deadline != task.fingerprint.deadline
                    || synced.fingerprint.content_snippet != task.fingerprint.content_snippet
                    || has_no_due_date
                {
                    actions.push(SyncAction::Update {
                        task: task.clone(),
                        event_id: synced.calendar_event_id.clone(),
                    });
                } else {
                    actions.push(SyncAction::Skip {
                        task: task.clone(),
                        reason: "Already synced, no changes".to_string(),
                    });
                }
                continue;
            }

            // Try fuzzy match for edited tasks
            let nearby = self
                .state
                .find_by_location(&task.file_path, task.line_number, 10);
            let mut best_match: Option<(&SyncedTask, f64)> = None;

            for synced in nearby {
                let sim = similarity(
                    &task.fingerprint.content_snippet,
                    &synced.fingerprint.content_snippet,
                );
                if sim > 0.5 {
                    if best_match.is_none() || sim > best_match.unwrap().1 {
                        best_match = Some((synced, sim));
                    }
                }
            }

            if let Some((synced, sim)) = best_match {
                let confidence = MatchConfidence::from_similarity(sim);
                matched_hashes.insert(synced.fingerprint.content_hash.clone());

                if confidence.should_auto_update() {
                    actions.push(SyncAction::Update {
                        task: task.clone(),
                        event_id: synced.calendar_event_id.clone(),
                    });
                } else {
                    // Low confidence - create new event to be safe
                    log::warn!(
                        "Low confidence match ({:.1}%) for '{}', creating new event",
                        sim * 100.0,
                        task.fingerprint.content_snippet
                    );
                    actions.push(SyncAction::Create(task.clone()));
                }
                continue;
            }

            // No match found - create new event
            actions.push(SyncAction::Create(task.clone()));
        }

        // Find orphaned events (synced but no longer exist in files)
        for (hash, synced) in &self.state.tasks {
            if !matched_hashes.contains(hash) {
                actions.push(SyncAction::Delete {
                    event_id: synced.calendar_event_id.clone(),
                    reason: format!(
                        "Task '{}' no longer exists in {}",
                        synced.fingerprint.content_snippet, synced.fingerprint.file_path
                    ),
                });
            }
        }

        Ok(actions)
    }

    /// Create a new calendar event using reqwest
    async fn create_event(&self, task: &PattoTask) -> Result<String> {
        log::info!(
            "Creating event for task: {}",
            task.fingerprint.content_snippet
        );
        let event = task_to_event(task, &self.config);

        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            urlencoding::encode(&self.config.calendar_id)
        );

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&event)
            .send()
            .await
            .context("Failed to send create event request")?;

        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create event: {}", error_text);
        }

        let created: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse create event response")?;

        let event_id = created
            .get("id")
            .and_then(|v| v.as_str())
            .context("Created event has no ID")?
            .to_string();

        Ok(event_id)
    }

    /// Update an existing calendar event using reqwest
    async fn update_event(&self, event_id: &str, task: &PattoTask) -> Result<()> {
        let event = task_to_event(task, &self.config);

        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            urlencoding::encode(&self.config.calendar_id),
            urlencoding::encode(event_id)
        );

        let resp = self
            .client
            .put(&url)
            .bearer_auth(&self.access_token)
            .json(&event)
            .send()
            .await
            .context("Failed to send update event request")?;

        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update event: {}", error_text);
        }

        Ok(())
    }

    /// Delete a calendar event using reqwest
    async fn delete_event(&self, event_id: &str) -> Result<()> {
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            urlencoding::encode(&self.config.calendar_id),
            urlencoding::encode(event_id)
        );

        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to send delete event request")?;

        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete event: {}", error_text);
        }

        Ok(())
    }
}
