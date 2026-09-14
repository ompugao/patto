//! Git sync over HTTPS with a personal access token.
//!
//! Notes live in a normal clone on the device. Sync is commit → fetch →
//! fast-forward or merge → push, with conflicts resolved in favour of the local
//! copy so the phone never blocks on a merge it cannot show.

use std::path::Path;
use std::sync::OnceLock;

use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{
    AnnotatedCommit, Cred, FetchOptions, FileFavor, MergeOptions, ProxyOptions, PushOptions,
    RemoteCallbacks, Repository, Signature,
};

use crate::api::error::{GitErrorKind, PattoError, PattoResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCreds {
    pub username: String,
    pub token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitPhase {
    Connecting,
    Counting,
    Receiving,
    Resolving,
    CheckingOut,
    Committing,
    Merging,
    Pushing,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitProgress {
    pub phase: GitPhase,
    pub current: u32,
    pub total: u32,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    UpToDate,
    FastForward,
    /// A real merge commit was made; `auto_resolved` lists files where the local
    /// copy was kept.
    Merged {
        auto_resolved: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub committed: bool,
    pub commit_id: Option<String>,
    pub merge: MergeOutcome,
    pub pushed: bool,
    /// Note paths that changed on disk during the sync, so the app can refresh
    /// just those.
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatus {
    pub branch: String,
    pub dirty: Vec<String>,
    pub ahead: u32,
    pub behind: u32,
    pub has_remote: bool,
}

static CERT_FILE: OnceLock<String> = OnceLock::new();

/// Point libgit2 at a CA bundle. The vendored OpenSSL ships no trust store, so
/// without this every HTTPS fetch fails to verify the server certificate.
/// The app bundles `cacert.pem` as an asset and passes its path once at startup.
pub fn git_init_runtime(ca_bundle_path: String) -> PattoResult<()> {
    if !Path::new(&ca_bundle_path).is_file() {
        return Err(PattoError::NotFound(ca_bundle_path));
    }
    if CERT_FILE.set(ca_bundle_path.clone()).is_err() {
        return Ok(());
    }
    // Safety: called once, before any repository is opened.
    unsafe {
        git2::opts::set_ssl_cert_file(&ca_bundle_path)?;
    }
    Ok(())
}

fn callbacks<'a>(
    creds: &GitCreds,
    on_progress: &'a (dyn Fn(GitProgress) + Send + Sync),
) -> RemoteCallbacks<'a> {
    let username = creds.username.clone();
    let token = creds.token.clone();
    let mut cb = RemoteCallbacks::new();

    // libgit2 retries credentials until one is accepted; without this guard a
    // wrong token loops instead of reporting an auth failure.
    let mut attempted = false;
    cb.credentials(move |_url, username_from_url, _allowed| {
        if attempted {
            return Err(git2::Error::from_str("authentication failed"));
        }
        attempted = true;
        let user = if username.is_empty() {
            username_from_url.unwrap_or("git")
        } else {
            username.as_str()
        };
        Cred::userpass_plaintext(user, &token)
    });

    cb.transfer_progress(move |stats| {
        let phase = if stats.received_objects() < stats.total_objects() {
            GitPhase::Receiving
        } else {
            GitPhase::Resolving
        };
        on_progress(GitProgress {
            phase,
            current: stats.received_objects() as u32,
            total: stats.total_objects() as u32,
            bytes: stats.received_bytes() as u64,
        });
        true
    });

    cb.push_transfer_progress(move |current, total, bytes| {
        on_progress(GitProgress {
            phase: GitPhase::Pushing,
            current: current as u32,
            total: total as u32,
            bytes: bytes as u64,
        });
    });

    cb.push_update_reference(|reference, status| match status {
        None => Ok(()),
        Some(msg) => Err(git2::Error::from_str(&format!(
            "remote rejected {reference}: {msg}"
        ))),
    });

    cb
}

fn fetch_options<'a>(
    creds: &GitCreds,
    on_progress: &'a (dyn Fn(GitProgress) + Send + Sync),
) -> FetchOptions<'a> {
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callbacks(creds, on_progress));
    let mut proxy = ProxyOptions::new();
    proxy.auto();
    opts.proxy_options(proxy);
    opts
}

pub fn git_clone(
    url: String,
    root: String,
    branch: Option<String>,
    creds: GitCreds,
    on_progress: impl Fn(GitProgress) + Send + Sync,
) -> PattoResult<()> {
    on_progress(GitProgress {
        phase: GitPhase::Connecting,
        current: 0,
        total: 0,
        bytes: 0,
    });

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options(&creds, &on_progress));
    if let Some(branch) = branch.as_deref().filter(|b| !b.is_empty()) {
        builder.branch(branch);
    }

    builder.clone(&url, Path::new(&root))?;
    on_progress(GitProgress {
        phase: GitPhase::Done,
        current: 0,
        total: 0,
        bytes: 0,
    });
    Ok(())
}

fn current_branch(repo: &Repository) -> PattoResult<String> {
    let head = repo.head()?;
    Ok(head.shorthand()?.to_string())
}

pub fn git_status(root: String) -> PattoResult<GitStatus> {
    let repo = Repository::open(&root)?;

    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let dirty: Vec<String> = repo
        .statuses(Some(&mut opts))?
        .iter()
        .filter_map(|e| e.path().ok().map(str::to_string))
        .filter(|p| p.ends_with(".pn"))
        .collect();

    let branch = current_branch(&repo).unwrap_or_else(|_| "HEAD".to_string());
    let has_remote = repo.find_remote("origin").is_ok();

    let (ahead, behind) = match upstream_oid(&repo, &branch) {
        Some(upstream) => {
            let local = repo.head().and_then(|h| h.peel_to_commit()).map(|c| c.id());
            match local {
                Ok(local) => repo.graph_ahead_behind(local, upstream).unwrap_or((0, 0)),
                Err(_) => (0, 0),
            }
        }
        None => (0, 0),
    };

    Ok(GitStatus {
        branch,
        dirty,
        ahead: ahead as u32,
        behind: behind as u32,
        has_remote,
    })
}

fn upstream_oid(repo: &Repository, branch: &str) -> Option<git2::Oid> {
    repo.find_branch(&format!("origin/{branch}"), git2::BranchType::Remote)
        .ok()?
        .get()
        .target()
}

/// Stage every note change and commit, returning the new commit id if the tree
/// actually differs from HEAD.
fn commit_notes(repo: &Repository, sig: &Signature) -> PattoResult<Option<git2::Oid>> {
    let mut index = repo.index()?;
    index.add_all(["*.pn"], git2::IndexAddOption::DEFAULT, None)?;
    index.update_all(["*"], None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    if let Some(parent) = &parent {
        if parent.tree_id() == tree_id {
            return Ok(None);
        }
    }

    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let oid = repo.commit(
        Some("HEAD"),
        sig,
        sig,
        "patto-flutter: sync notes",
        &tree,
        &parents,
    )?;
    Ok(Some(oid))
}

fn changed_between(repo: &Repository, before: Option<git2::Oid>) -> Vec<String> {
    let Some(before) = before else {
        return Vec::new();
    };
    let after = match repo.head().and_then(|h| h.peel_to_commit()) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    if after.id() == before {
        return Vec::new();
    }

    let old_tree = repo.find_commit(before).and_then(|c| c.tree()).ok();
    let new_tree = after.tree().ok();
    let Ok(diff) = repo.diff_tree_to_tree(old_tree.as_ref(), new_tree.as_ref(), None) else {
        return Vec::new();
    };

    let mut paths = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            for file in [delta.new_file(), delta.old_file()] {
                if let Some(p) = file.path().and_then(|p| p.to_str()) {
                    if p.ends_with(".pn") && !paths.contains(&p.to_string()) {
                        paths.push(p.to_string());
                    }
                }
            }
            true
        },
        None,
        None,
        None,
    )
    .ok();
    paths
}

/// Resolve leftover index conflicts by keeping our side; drop the file when we
/// deleted it. `FileFavor::Ours` covers content conflicts but not add/add or
/// modify/delete.
fn resolve_conflicts_ours(repo: &Repository) -> PattoResult<Vec<String>> {
    let mut index = repo.index()?;
    if !index.has_conflicts() {
        return Ok(Vec::new());
    }

    // The iterator borrows the index, so decide everything first.
    let mut decisions = Vec::new();
    for conflict in index.conflicts()? {
        let conflict = conflict?;
        match (conflict.our, conflict.their) {
            (Some(our), _) => {
                let path = String::from_utf8_lossy(&our.path).to_string();
                decisions.push((path, Some(our)));
            }
            (None, Some(their)) => {
                let path = String::from_utf8_lossy(&their.path).to_string();
                decisions.push((path, None));
            }
            (None, None) => {}
        }
    }

    let mut resolved = Vec::new();
    for (path, ours) in decisions {
        // Drop all three conflict stages before staging a resolution, otherwise
        // the index stays "not fully merged" and no tree can be written.
        index.conflict_remove(Path::new(&path))?;
        match ours {
            Some(mut entry) => {
                // Stage 0 marks the entry resolved.
                entry.flags &= !0x3000;
                index.add(&entry)?;
            }
            None => {
                index.remove_path(Path::new(&path)).ok();
            }
        }
        resolved.push(path);
    }
    index.write()?;

    repo.checkout_index(Some(&mut index), Some(CheckoutBuilder::new().force()))?;
    Ok(resolved)
}

fn merge_fetched(
    repo: &Repository,
    branch: &str,
    fetched: &AnnotatedCommit,
    sig: &Signature,
    on_progress: &(dyn Fn(GitProgress) + Send + Sync),
) -> PattoResult<MergeOutcome> {
    let (analysis, _) = repo.merge_analysis(&[fetched])?;

    if analysis.is_up_to_date() {
        return Ok(MergeOutcome::UpToDate);
    }

    if analysis.is_fast_forward() || analysis.is_unborn() {
        let refname = format!("refs/heads/{branch}");
        match repo.find_reference(&refname) {
            Ok(mut reference) => {
                reference.set_target(fetched.id(), "fast-forward")?;
            }
            Err(_) => {
                repo.reference(&refname, fetched.id(), true, "fast-forward")?;
            }
        }
        repo.set_head(&refname)?;
        repo.checkout_head(Some(CheckoutBuilder::new().force()))?;
        return Ok(MergeOutcome::FastForward);
    }

    on_progress(GitProgress {
        phase: GitPhase::Merging,
        current: 0,
        total: 0,
        bytes: 0,
    });

    let mut merge_opts = MergeOptions::new();
    merge_opts.file_favor(FileFavor::Ours);
    let mut checkout = CheckoutBuilder::new();
    repo.merge(&[fetched], Some(&mut merge_opts), Some(&mut checkout))?;

    let auto_resolved = resolve_conflicts_ours(repo)?;

    let mut index = repo.index()?;
    let tree = repo.find_tree(index.write_tree()?)?;
    let head_commit = repo.head()?.peel_to_commit()?;
    let their_commit = repo.find_commit(fetched.id())?;

    repo.commit(
        Some("HEAD"),
        sig,
        sig,
        &format!("patto-flutter: merge origin/{branch}"),
        &tree,
        &[&head_commit, &their_commit],
    )?;
    repo.cleanup_state()?;

    Ok(MergeOutcome::Merged { auto_resolved })
}

/// Commit local edits, integrate the remote, and push.
///
/// The push is retried when the remote moved between our fetch and our push.
pub fn git_sync(
    root: String,
    author_name: String,
    author_email: String,
    creds: GitCreds,
    on_progress: impl Fn(GitProgress) + Send + Sync,
) -> PattoResult<SyncReport> {
    const PUSH_ATTEMPTS: usize = 3;

    let repo = Repository::open(&root)?;
    let sig = Signature::now(&author_name, &author_email)?;
    let branch = current_branch(&repo)?;
    let before = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .map(|c| c.id());

    on_progress(GitProgress {
        phase: GitPhase::Committing,
        current: 0,
        total: 0,
        bytes: 0,
    });
    let commit_id = commit_notes(&repo, &sig)?;

    if repo.find_remote("origin").is_err() {
        return Err(PattoError::Git {
            kind: GitErrorKind::NoRemote,
            message: "no 'origin' remote configured".to_string(),
        });
    }

    let mut merge = MergeOutcome::UpToDate;
    let mut pushed = false;
    let mut last_error = None;

    for _ in 0..PUSH_ATTEMPTS {
        let mut remote = repo.find_remote("origin")?;
        remote.fetch(
            &[&format!("refs/heads/{branch}")],
            Some(&mut fetch_options(&creds, &on_progress)),
            None,
        )?;

        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetched = repo.reference_to_annotated_commit(&fetch_head)?;
        let outcome = merge_fetched(&repo, &branch, &fetched, &sig, &on_progress)?;
        if !matches!(outcome, MergeOutcome::UpToDate) {
            merge = outcome;
        }

        on_progress(GitProgress {
            phase: GitPhase::Pushing,
            current: 0,
            total: 0,
            bytes: 0,
        });

        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(callbacks(&creds, &on_progress));
        let mut proxy = ProxyOptions::new();
        proxy.auto();
        push_opts.proxy_options(proxy);

        match remote.push(
            &[&format!("refs/heads/{branch}:refs/heads/{branch}")],
            Some(&mut push_opts),
        ) {
            Ok(()) => {
                pushed = true;
                break;
            }
            Err(e) => {
                let err = PattoError::from(e);
                let retryable = matches!(
                    err,
                    PattoError::Git {
                        kind: GitErrorKind::NonFastForward,
                        ..
                    }
                ) || matches!(&err, PattoError::Git { message, .. } if message.contains("rejected"));
                last_error = Some(err);
                if !retryable {
                    break;
                }
            }
        }
    }

    if !pushed {
        if let Some(err) = last_error {
            return Err(err);
        }
    }

    on_progress(GitProgress {
        phase: GitPhase::Done,
        current: 0,
        total: 0,
        bytes: 0,
    });

    Ok(SyncReport {
        committed: commit_id.is_some(),
        commit_id: commit_id.map(|id| id.to_string()),
        merge,
        pushed,
        changed_paths: changed_between(&repo, before),
    })
}
