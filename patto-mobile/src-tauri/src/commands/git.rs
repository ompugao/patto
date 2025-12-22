// Git operations for patto-mobile
// Using git2 crate with HTTPS + Personal Access Token authentication

use git2::{
    build::RepoBuilder, Cred, Direction, FetchOptions, PushOptions, RemoteCallbacks, Repository,
    Signature,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Result of git operations
#[derive(Debug, Serialize, Deserialize)]
pub struct GitResult {
    pub success: bool,
    pub message: String,
}

/// Git credentials stored in app state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitCredentials {
    pub username: String,
    pub token: String,
}

/// Progress event for frontend
#[derive(Debug, Clone, Serialize)]
pub struct CloneProgress {
    pub stage: String,
    pub received: usize,
    pub total: usize,
    pub percent: u32,
}

/// Create callbacks for authentication with optional progress reporting
fn create_callbacks_with_progress<'a>(
    credentials: &'a GitCredentials,
    app_handle: Option<&'a AppHandle>,
    last_percent: Option<Arc<AtomicU32>>,
) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    let username = credentials.username.clone();
    let token = credentials.token.clone();

    callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
        Cred::userpass_plaintext(&username, &token)
    });

    // Skip SSL certificate verification on mobile (CA certs not available)
    callbacks.certificate_check(|_cert, _host| Ok(git2::CertificateCheckStatus::CertificateOk));

    // Add transfer progress reporting
    if let (Some(app), Some(last_pct)) = (app_handle, last_percent) {
        let app = app.clone();
        callbacks.transfer_progress(move |stats| {
            let received = stats.received_objects();
            let total = stats.total_objects();
            let percent = if total > 0 {
                (received as u32 * 100) / total as u32
            } else {
                0
            };

            // Only emit every 5% to reduce event spam
            let prev = last_pct.load(Ordering::Relaxed);
            if percent >= prev + 5 || percent == 100 {
                last_pct.store(percent, Ordering::Relaxed);
                let _ = app.emit(
                    "clone-progress",
                    CloneProgress {
                        stage: "Receiving objects".to_string(),
                        received,
                        total,
                        percent,
                    },
                );
            }
            true
        });
    }

    callbacks
}

/// Create callbacks for authentication (without progress reporting)
fn create_callbacks(credentials: &GitCredentials) -> RemoteCallbacks<'_> {
    create_callbacks_with_progress(credentials, None, None)
}

/// Clone a repository from a remote URL (with progress events)
#[tauri::command]
pub async fn git_clone(
    app: AppHandle,
    url: String,
    dest: PathBuf,
    credentials: GitCredentials,
) -> Result<GitResult, String> {
    // Emit starting event
    let _ = app.emit(
        "clone-progress",
        CloneProgress {
            stage: "Starting clone".to_string(),
            received: 0,
            total: 0,
            percent: 0,
        },
    );

    // Run clone in a blocking thread to not block the main thread
    let result = tokio::task::spawn_blocking(move || {
        let last_percent = Arc::new(AtomicU32::new(0));
        let callbacks =
            create_callbacks_with_progress(&credentials, Some(&app), Some(last_percent));

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let mut builder = RepoBuilder::new();
        builder.fetch_options(fetch_options);

        match builder.clone(&url, &dest) {
            Ok(repo) => {
                // Emit indexing stage
                let _ = app.emit(
                    "clone-progress",
                    CloneProgress {
                        stage: "Fixing timestamps".to_string(),
                        received: 0,
                        total: 0,
                        percent: 100,
                    },
                );

                // Fix file timestamps to match commit times (with progress)
                let fixed = fix_file_timestamps(&repo, &dest, &app);

                // Emit complete
                let _ = app.emit(
                    "clone-progress",
                    CloneProgress {
                        stage: "Complete".to_string(),
                        received: 0,
                        total: 0,
                        percent: 100,
                    },
                );

                Ok(GitResult {
                    success: true,
                    message: format!(
                        "Successfully cloned to {:?} (fixed {} file timestamps)",
                        dest, fixed
                    ),
                })
            }
            Err(e) => Err(format!("Failed to clone repository: {}", e)),
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    result
}

/// Set file modification times to their last commit time (optimized)
/// Walks commits once to build file->timestamp map, then applies timestamps
fn fix_file_timestamps(repo: &Repository, repo_path: &PathBuf, app: &AppHandle) -> usize {
    use std::collections::HashMap;
    use std::fs;
    use std::time::{Duration, SystemTime};

    // Emit starting message
    let _ = app.emit(
        "clone-progress",
        CloneProgress {
            stage: "Building file history".to_string(),
            received: 0,
            total: 0,
            percent: 0,
        },
    );

    // Build map of file -> last commit timestamp by walking commits once
    let mut file_times: HashMap<String, i64> = HashMap::new();

    if let Ok(mut revwalk) = repo.revwalk() {
        let _ = revwalk.push_head();
        let _ = revwalk.set_sorting(git2::Sort::TIME | git2::Sort::REVERSE); // oldest first

        let commits: Vec<_> = revwalk.flatten().collect();
        let total_commits = commits.len();
        let mut processed_commits = 0;
        let mut last_percent = 0;

        for oid in commits {
            processed_commits += 1;
            let percent = if total_commits > 0 {
                (processed_commits * 50) / total_commits // First 50% for history
            } else {
                0
            };

            if percent >= last_percent + 10 {
                last_percent = percent;
                let _ = app.emit(
                    "clone-progress",
                    CloneProgress {
                        stage: "Analyzing commits".to_string(),
                        received: processed_commits,
                        total: total_commits,
                        percent: percent as u32,
                    },
                );
            }

            if let Ok(commit) = repo.find_commit(oid) {
                let commit_time = commit.time().seconds();

                if let Some(parent) = commit.parent(0).ok() {
                    if let (Ok(commit_tree), Ok(parent_tree)) = (commit.tree(), parent.tree()) {
                        if let Ok(diff) =
                            repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)
                        {
                            for delta in diff.deltas() {
                                if let Some(path) = delta.new_file().path() {
                                    file_times
                                        .insert(path.to_string_lossy().to_string(), commit_time);
                                }
                            }
                        }
                    }
                } else {
                    // Initial commit - all files are new
                    if let Ok(tree) = commit.tree() {
                        let _ = tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
                            if entry.kind() == Some(git2::ObjectType::Blob) {
                                let path = if dir.is_empty() {
                                    entry.name().unwrap_or("").to_string()
                                } else {
                                    format!("{}{}", dir, entry.name().unwrap_or(""))
                                };
                                file_times.insert(path, commit_time);
                            }
                            git2::TreeWalkResult::Ok
                        });
                    }
                }
            }
        }
    }

    // Now apply timestamps to files
    let total_files = file_times.len();
    let mut count = 0;
    let mut last_percent = 50; // Start at 50% (second half)

    for (i, (path, timestamp)) in file_times.iter().enumerate() {
        let percent = 50
            + if total_files > 0 {
                ((i + 1) * 50) / total_files
            } else {
                50
            };

        if percent >= last_percent + 10 || percent == 100 {
            last_percent = percent;
            let _ = app.emit(
                "clone-progress",
                CloneProgress {
                    stage: "Setting timestamps".to_string(),
                    received: i + 1,
                    total: total_files,
                    percent: percent as u32,
                },
            );
        }

        let file_path = repo_path.join(path);
        if file_path.exists() {
            let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(*timestamp as u64);
            if let Ok(file) = fs::File::open(&file_path) {
                let _ = file.set_modified(mtime);
                count += 1;
            }
        }
    }

    count
}

/// Pull changes from remote
#[tauri::command]
pub async fn git_pull(
    repo_path: PathBuf,
    credentials: GitCredentials,
) -> Result<GitResult, String> {
    tokio::task::spawn_blocking(move || {
        let repo =
            Repository::open(&repo_path).map_err(|e| format!("Failed to open repo: {}", e))?;

        // Get the current branch name
        let head = repo
            .head()
            .map_err(|e| format!("Failed to get HEAD: {}", e))?;
        let branch_name = head.shorthand().unwrap_or("main");

        // Fetch from origin
        let mut remote = repo
            .find_remote("origin")
            .map_err(|e| format!("Failed to find remote 'origin': {}", e))?;

        let callbacks = create_callbacks(&credentials);
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote
            .fetch(&[branch_name], Some(&mut fetch_options), None)
            .map_err(|e| format!("Failed to fetch: {}", e))?;

        // Get fetch head
        let fetch_head = repo
            .find_reference("FETCH_HEAD")
            .map_err(|e| format!("Failed to find FETCH_HEAD: {}", e))?;

        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(|e| format!("Failed to get fetch commit: {}", e))?;

        // Perform merge (fast-forward if possible)
        let analysis = repo
            .merge_analysis(&[&fetch_commit])
            .map_err(|e| format!("Failed merge analysis: {}", e))?;

        if analysis.0.is_up_to_date() {
            Ok(GitResult {
                success: true,
                message: "Already up to date".to_string(),
            })
        } else if analysis.0.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", branch_name);
            let mut reference = repo
                .find_reference(&refname)
                .map_err(|e| format!("Failed to find reference: {}", e))?;
            reference
                .set_target(fetch_commit.id(), "Fast-forward")
                .map_err(|e| format!("Failed to set target: {}", e))?;
            repo.set_head(&refname)
                .map_err(|e| format!("Failed to set HEAD: {}", e))?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| format!("Failed to checkout: {}", e))?;

            Ok(GitResult {
                success: true,
                message: "Fast-forward merge completed".to_string(),
            })
        } else {
            Err("Merge required - manual intervention needed".to_string())
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Commit all changes and push to remote
#[tauri::command]
pub async fn git_sync(
    repo_path: PathBuf,
    message: String,
    credentials: GitCredentials,
) -> Result<GitResult, String> {
    tokio::task::spawn_blocking(move || {
        let repo =
            Repository::open(&repo_path).map_err(|e| format!("Failed to open repo: {}", e))?;

        // Stage all changes
        let mut index = repo
            .index()
            .map_err(|e| format!("Failed to get index: {}", e))?;
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| format!("Failed to add files: {}", e))?;
        index
            .write()
            .map_err(|e| format!("Failed to write index: {}", e))?;

        // Check if there are changes to commit
        let tree_id = index
            .write_tree()
            .map_err(|e| format!("Failed to write tree: {}", e))?;
        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| format!("Failed to find tree: {}", e))?;

        // Get parent commit
        let head = repo.head().ok();
        let parent_commit = head.as_ref().and_then(|h| h.peel_to_commit().ok());

        // Create signature
        let signature = Signature::now("Patto Mobile", "patto@mobile.app")
            .map_err(|e| format!("Failed to create signature: {}", e))?;

        // Check if tree is different from parent
        let has_changes = match &parent_commit {
            Some(parent) => parent.tree_id() != tree_id,
            None => true,
        };

        if has_changes {
            // Create commit
            let parents: Vec<&git2::Commit> =
                parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &message,
                &tree,
                &parents,
            )
            .map_err(|e| format!("Failed to commit: {}", e))?;
        }

        // Push to origin
        let mut remote = repo
            .find_remote("origin")
            .map_err(|e| format!("Failed to find remote 'origin': {}", e))?;

        let callbacks = create_callbacks(&credentials);
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        let head = repo
            .head()
            .map_err(|e| format!("Failed to get HEAD: {}", e))?;
        let branch_name = head.shorthand().unwrap_or("main");
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

        remote
            .push(&[&refspec], Some(&mut push_options))
            .map_err(|e| format!("Failed to push: {}", e))?;

        Ok(GitResult {
            success: true,
            message: if has_changes {
                "Changes committed and pushed".to_string()
            } else {
                "No changes to commit, pushed to remote".to_string()
            },
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Configure remote URL for existing repository
#[tauri::command]
pub fn configure_remote(repo_path: PathBuf, remote_url: String) -> Result<GitResult, String> {
    let repo = Repository::open(&repo_path).map_err(|e| format!("Failed to open repo: {}", e))?;

    // Check if origin exists
    if repo.find_remote("origin").is_ok() {
        repo.remote_set_url("origin", &remote_url)
            .map_err(|e| format!("Failed to set remote URL: {}", e))?;
    } else {
        repo.remote("origin", &remote_url)
            .map_err(|e| format!("Failed to add remote: {}", e))?;
    }

    Ok(GitResult {
        success: true,
        message: format!("Remote 'origin' set to {}", remote_url),
    })
}

/// Initialize a new repository
#[tauri::command]
pub fn git_init(repo_path: PathBuf) -> Result<GitResult, String> {
    match Repository::init(&repo_path) {
        Ok(_repo) => Ok(GitResult {
            success: true,
            message: format!("Initialized empty repository in {:?}", repo_path),
        }),
        Err(e) => Err(format!("Failed to initialize repository: {}", e)),
    }
}

/// Get repository status summary
#[tauri::command]
pub fn git_status(repo_path: PathBuf) -> Result<GitStatus, String> {
    let repo = Repository::open(&repo_path).map_err(|e| format!("Failed to open repo: {}", e))?;

    let statuses = repo
        .statuses(None)
        .map_err(|e| format!("Failed to get status: {}", e))?;

    let mut modified = 0;
    let mut added = 0;
    let mut deleted = 0;
    let mut untracked = 0;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_wt_modified() || status.is_index_modified() {
            modified += 1;
        }
        if status.is_wt_new() {
            untracked += 1;
        }
        if status.is_index_new() {
            added += 1;
        }
        if status.is_wt_deleted() || status.is_index_deleted() {
            deleted += 1;
        }
    }

    // Get current branch
    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .and_then(|h| h.shorthand().map(|s| s.to_string()));

    Ok(GitStatus {
        branch,
        modified,
        added,
        deleted,
        untracked,
        is_clean: statuses.is_empty(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub untracked: usize,
    pub is_clean: bool,
}
