//! Git API for Flutter
//!
//! This module provides git operations for the Patto Flutter app via flutter_rust_bridge.
//! It uses git2-rs (libgit2) for native git operations.

use flutter_rust_bridge::frb;
use git2::{
    build::RepoBuilder, Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository,
    ResetType, Signature, CertificateCheckStatus, Error, ProxyOptions,
};
use std::path::Path;

/// Git configuration for repository access
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub struct GitConfig {
    pub repo_url: String,
    pub branch: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Progress information for git operations
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub struct GitProgress {
    pub phase: String,
    pub current: u32,
    pub total: u32,
}

/// File information from the repository
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub modified: i64,
}

/// Git operation result
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub struct GitResult {
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// Clone a git repository
///
/// # Arguments
/// * `url` - Repository URL (HTTPS)
/// * `path` - Local path to clone to
/// * `branch` - Branch to checkout
/// * `username` - Optional username for authentication
/// * `password` - Optional password/token for authentication
#[frb]
pub fn clone_repository(
    url: String,
    path: String,
    branch: String,
    username: Option<String>,
    password: Option<String>,
) -> GitResult {
    let result = clone_repo_internal(&url, &path, &branch, username.as_deref(), password.as_deref());
    
    match result {
        Ok(_) => GitResult {
            success: true,
            message: Some("Repository cloned successfully".to_string()),
            error: None,
        },
        Err(e) => GitResult {
            success: false,
            message: None,
            error: Some(e.to_string()),
        },
    }
}

/// Pull latest changes from remote
///
/// # Arguments
/// * `path` - Local repository path
/// * `branch` - Branch to pull
/// * `username` - Optional username for authentication
/// * `password` - Optional password/token for authentication
#[frb]
pub fn pull_repository(
    path: String,
    branch: String,
    username: Option<String>,
    password: Option<String>,
) -> GitResult {
    let result = pull_repo_internal(&path, &branch, username.as_deref(), password.as_deref());
    
    match result {
        Ok(_) => GitResult {
            success: true,
            message: Some("Pull completed successfully".to_string()),
            error: None,
        },
        Err(e) => GitResult {
            success: false,
            message: None,
            error: Some(e.to_string()),
        },
    }
}

/// Fetch latest changes from remote without merging
#[frb]
pub fn fetch_repository(
    path: String,
    branch: String,
    username: Option<String>,
    password: Option<String>,
) -> GitResult {
    let result = fetch_repo_internal(&path, &branch, username.as_deref(), password.as_deref());
    
    match result {
        Ok(_) => GitResult {
            success: true,
            message: Some("Fetch completed successfully".to_string()),
            error: None,
        },
        Err(e) => GitResult {
            success: false,
            message: None,
            error: Some(e.to_string()),
        },
    }
}

/// List .pn files in the repository
///
/// Returns a list of all .pn files with their metadata.
#[frb(sync)]
pub fn list_pn_files(repo_path: String) -> Result<Vec<FileInfo>, String> {
    list_pn_files_internal(&repo_path).map_err(|e| e.to_string())
}

/// Read file content from the repository
#[frb(sync)]
pub fn read_file_content(file_path: String) -> Result<String, String> {
    std::fs::read_to_string(&file_path).map_err(|e| e.to_string())
}

/// Check if a directory is a valid git repository
#[frb(sync)]
pub fn is_git_repository(path: String) -> bool {
    Repository::open(&path).is_ok()
}

/// Get the current branch name
#[frb(sync)]
pub fn get_current_branch(path: String) -> Result<String, String> {
    let repo = Repository::open(&path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    
    head.shorthand()
        .map(|s| s.to_string())
        .ok_or_else(|| "Could not determine branch name".to_string())
}

/// Delete repository directory
#[frb]
pub fn delete_repository(path: String) -> GitResult {
    match std::fs::remove_dir_all(&path) {
        Ok(_) => GitResult {
            success: true,
            message: Some("Repository deleted".to_string()),
            error: None,
        },
        Err(e) => GitResult {
            success: false,
            message: None,
            error: Some(e.to_string()),
        },
    }
}

// Internal implementations

fn clone_repo_internal(
    url: &str,
    path: &str,
    branch: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Repository, git2::Error> {
    let callbacks = build_callbacks(username, password);
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    let mut proxy_options = ProxyOptions::new();
    proxy_options.auto(false);
    fetch_options.proxy_options(proxy_options);
    
    let mut builder = RepoBuilder::new();
    builder.branch(branch);
    builder.fetch_options(fetch_options);
    
    builder.clone(url, Path::new(path))
}

fn build_callbacks(
    username: Option<&str>,
    password: Option<&str>,
) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    let username = username.map(str::to_owned);
    let password = password.map(str::to_owned);
    let mut tried_credentials = false;

    callbacks.certificate_check(|_cert, _host| Ok(CertificateCheckStatus::CertificateOk));

    callbacks.credentials(move |_url, username_from_url, _allowed_types| {
        if tried_credentials {
            return Err(Error::from_str("authentication already attempted"));
        }
        tried_credentials = true;

        if let Some(pass) = password.as_ref() {
            let user = username
                .as_deref()
                .or(username_from_url)
                .unwrap_or("git");
            Cred::userpass_plaintext(user, pass)
        } else {
            Cred::default()
        }
    });

    callbacks
}

fn fetch_repo_internal(
    path: &str,
    branch: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<(), git2::Error> {
    let repo = Repository::open(path)?;
    let mut remote = repo.find_remote("origin")?;
    
    let callbacks = build_callbacks(username, password);
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    let mut proxy_options = ProxyOptions::new();
    proxy_options.auto(false);
    fetch_options.proxy_options(proxy_options);
    
    let refspec = format!("refs/heads/{}", branch);
    remote.fetch(&[&refspec], Some(&mut fetch_options), None)?;
    
    Ok(())
}

fn pull_repo_internal(
    path: &str,
    branch: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<(), git2::Error> {
    // First fetch
    fetch_repo_internal(path, branch, username, password)?;
    
    let repo = Repository::open(path)?;
    
    // Get the fetch head
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    
    // Do a fast-forward merge
    let refname = format!("refs/heads/{}", branch);
    let mut reference = repo.find_reference(&refname)?;
    
    reference.set_target(fetch_commit.id(), "Fast-forward pull")?;
    repo.set_head(&refname)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    
    Ok(())
}

fn list_pn_files_internal(repo_path: &str) -> Result<Vec<FileInfo>, std::io::Error> {
    let mut files = Vec::new();
    let repo_path = Path::new(repo_path);
    
    fn visit_dirs(
        dir: &Path,
        base: &Path,
        files: &mut Vec<FileInfo>,
    ) -> Result<(), std::io::Error> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                // Skip .git directory
                if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                    continue;
                }
                
                if path.is_dir() {
                    visit_dirs(&path, base, files)?;
                } else if path.extension().map(|e| e == "pn").unwrap_or(false) {
                    let relative_path = path.strip_prefix(base).unwrap_or(&path);
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let metadata = std::fs::metadata(&path)?;
                    let modified = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    
                    files.push(FileInfo {
                        path: relative_path.to_string_lossy().to_string(),
                        name,
                        modified,
                    });
                }
            }
        }
        Ok(())
    }
    
    visit_dirs(repo_path, repo_path, &mut files)?;
    
    // Sort by modified time (most recent first)
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    
    fn temp_dir() -> PathBuf {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "patto_test_{}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            counter
        ));
        // Clean up if exists from previous run
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
    
    #[test]
    fn test_is_git_repository() {
        let dir = temp_dir();
        assert!(!is_git_repository(dir.to_string_lossy().to_string()));
        fs::remove_dir_all(&dir).ok();
    }
    
    #[test]
    fn test_list_pn_files_empty() {
        let dir = temp_dir();
        let result = list_pn_files(dir.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
        fs::remove_dir_all(&dir).ok();
    }
    
    #[test]
    fn test_list_pn_files_with_files() {
        let dir = temp_dir();
        fs::write(dir.join("test.pn"), "Hello").unwrap();
        fs::write(dir.join("other.txt"), "World").unwrap();
        
        let result = list_pn_files(dir.to_string_lossy().to_string());
        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "test");
        
        fs::remove_dir_all(&dir).ok();
    }
}
