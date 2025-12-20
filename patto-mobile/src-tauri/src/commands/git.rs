// Git operations for patto-mobile
// Using git2 crate with HTTPS + Personal Access Token authentication

use git2::{
    build::RepoBuilder, Cred, Direction, FetchOptions, PushOptions, RemoteCallbacks, Repository,
    Signature,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

/// Create callbacks for authentication
fn create_callbacks(credentials: &GitCredentials) -> RemoteCallbacks<'_> {
    let mut callbacks = RemoteCallbacks::new();
    let username = credentials.username.clone();
    let token = credentials.token.clone();

    callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
        Cred::userpass_plaintext(&username, &token)
    });

    // Skip SSL certificate verification on mobile (CA certs not available)
    callbacks.certificate_check(|_cert, _host| Ok(git2::CertificateCheckStatus::CertificateOk));

    callbacks
}

/// Clone a repository from a remote URL
#[tauri::command]
pub fn git_clone(
    url: String,
    dest: PathBuf,
    credentials: GitCredentials,
) -> Result<GitResult, String> {
    let callbacks = create_callbacks(&credentials);

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options);

    match builder.clone(&url, &dest) {
        Ok(repo) => {
            // Fix file timestamps to match commit times
            let fixed = fix_file_timestamps(&repo, &dest);
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
}

/// Set file modification times to their last commit time
fn fix_file_timestamps(repo: &Repository, repo_path: &PathBuf) -> usize {
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let mut count = 0;

    // Walk through all files in HEAD
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return 0,
    };

    let tree = match head.peel_to_tree() {
        Ok(t) => t,
        Err(_) => return 0,
    };

    // Walk the tree and get last commit time for each file
    let _ = tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() != Some(git2::ObjectType::Blob) {
            return git2::TreeWalkResult::Ok;
        }

        let path = if dir.is_empty() {
            entry.name().unwrap_or("").to_string()
        } else {
            format!("{}{}", dir, entry.name().unwrap_or(""))
        };

        // Get last commit time for this file
        if let Some(commit_time) = get_file_last_commit_time(repo, &path) {
            let file_path = repo_path.join(&path);
            if file_path.exists() {
                // Set the file's mtime
                let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(commit_time as u64);
                if let Ok(file) = fs::File::open(&file_path) {
                    let _ = file.set_modified(mtime);
                    count += 1;
                }
            }
        }

        git2::TreeWalkResult::Ok
    });

    count
}

/// Get the timestamp of the last commit that modified a file
fn get_file_last_commit_time(repo: &Repository, file_path: &str) -> Option<i64> {
    let mut revwalk = repo.revwalk().ok()?;
    revwalk.push_head().ok()?;
    revwalk.set_sorting(git2::Sort::TIME).ok()?;

    for oid in revwalk.flatten() {
        let commit = repo.find_commit(oid).ok()?;

        // Check if this commit modified the file
        if let Some(parent) = commit.parent(0).ok() {
            let commit_tree = commit.tree().ok()?;
            let parent_tree = parent.tree().ok()?;

            let diff = repo
                .diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)
                .ok()?;

            for delta in diff.deltas() {
                if let Some(new_path) = delta.new_file().path() {
                    if new_path.to_string_lossy() == file_path {
                        return Some(commit.time().seconds());
                    }
                }
            }
        } else {
            // Initial commit - check if file exists in this commit
            let tree = commit.tree().ok()?;
            if tree.get_path(std::path::Path::new(file_path)).is_ok() {
                return Some(commit.time().seconds());
            }
        }
    }

    None
}

/// Pull changes from remote
#[tauri::command]
pub fn git_pull(repo_path: PathBuf, credentials: GitCredentials) -> Result<GitResult, String> {
    let repo = Repository::open(&repo_path).map_err(|e| format!("Failed to open repo: {}", e))?;

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
}

/// Commit all changes and push to remote
#[tauri::command]
pub fn git_sync(
    repo_path: PathBuf,
    message: String,
    credentials: GitCredentials,
) -> Result<GitResult, String> {
    let repo = Repository::open(&repo_path).map_err(|e| format!("Failed to open repo: {}", e))?;

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
