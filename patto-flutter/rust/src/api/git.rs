//! Git sync over HTTPS with a personal access token.
//!
//! Notes live in a normal clone on the device. Sync is commit → fetch →
//! fast-forward or merge → push. When both sides changed the same lines nothing
//! is merged: the phone's commits are pushed to a branch of their own, so they
//! are safe, and the merge waits until it is done on the desktop or resolved in
//! the app (see [`crate::api::conflict`]).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use foreign_types_shared::ForeignType;
use git2::build::{CheckoutBuilder, RepoBuilder};

use git2::{
    Cred, FetchOptions, Oid, ProxyOptions, PushOptions, Remote, RemoteCallbacks, Repository,
    Signature,
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
    /// A merge commit was made; no lines clashed.
    Merged,
    /// Both sides changed the same lines, so nothing was merged. The phone's
    /// commits were pushed to `side_branch` instead, and the remote's changes
    /// are held back until the merge is done.
    Conflicted {
        side_branch: String,
        paths: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub committed: bool,
    pub commit_id: Option<String>,
    pub merge: MergeOutcome,
    pub pushed: bool,
    /// A merge that was pending has now been completed, on the desktop or in
    /// the app, and the side branch removed.
    pub conflict_cleared: bool,
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
    /// The last sync stopped at a conflict that has not been merged yet.
    pub conflict_pending: bool,
}

static CERT_FILE: OnceLock<String> = OnceLock::new();

/// Give libgit2 the trust roots it needs to verify an HTTPS server.
///
/// The vendored OpenSSL ships no trust store, and on Android `openssl-src`
/// configures it with `no-stdio`, so OpenSSL there cannot open a PEM file at
/// all: pointing libgit2 at a path fails with a "BIO lib" error and
/// `SSL_CERT_FILE` is equally useless. The certificates are therefore parsed
/// here and handed to libgit2 one at a time, which only touches memory.
///
/// The app bundles `cacert.pem` as an asset and passes its path once at startup.
pub fn git_init_runtime(ca_bundle_path: String) -> PattoResult<()> {
    let path = Path::new(&ca_bundle_path);
    if !path.is_file() {
        return Err(PattoError::NotFound(ca_bundle_path));
    }
    let pem = std::fs::read(path)?;

    if CERT_FILE.set(ca_bundle_path.clone()).is_err() {
        return Ok(());
    }

    let certs = openssl::x509::X509::stack_from_pem(&pem).map_err(|e| PattoError::Git {
        kind: GitErrorKind::Certificate,
        message: format!("could not parse the CA bundle: {e}"),
    })?;

    // libgit2 has to be initialised before its options are set; opening a
    // throwaway repository path is the cheapest way to force that.
    let _ = Repository::open(".");

    let mut added = 0usize;
    for cert in &certs {
        // Safety: libgit2 takes its own reference to the certificate.
        let code = unsafe {
            libgit2_sys::git_libgit2_opts(
                libgit2_sys::GIT_OPT_ADD_SSL_X509_CERT as libc::c_int,
                cert.as_ptr(),
            )
        };
        if code >= 0 {
            added += 1;
        }
    }

    if added == 0 {
        return Err(PattoError::Git {
            kind: GitErrorKind::Certificate,
            message: "libgit2 accepted none of the bundled trust roots".to_string(),
        });
    }

    log::info!("loaded {added} of {} trust roots", certs.len());
    Ok(())
}

pub(crate) fn callbacks<'a>(
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

    // libgit2 only reports "the certificate is invalid"; log what it actually
    // saw so a verification failure on device can be diagnosed.
    cb.certificate_check(|cert, host| {
        log::info!(
            "TLS certificate for {host}: x509={} valid_host={}",
            cert.as_x509().is_some(),
            !host.is_empty()
        );
        Ok(git2::CertificateCheckStatus::CertificatePassthrough)
    });

    cb.push_update_reference(|reference, status| match status {
        None => Ok(()),
        Some(msg) => Err(git2::Error::from_str(&format!(
            "remote rejected {reference}: {msg}"
        ))),
    });

    cb
}

pub(crate) fn fetch_options<'a>(
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

pub(crate) fn current_branch(repo: &Repository) -> PattoResult<String> {
    let head = repo.head()?;
    Ok(head.shorthand()?.to_string())
}

/// Note paths that differ from HEAD, including untracked ones.
fn dirty_notes(repo: &Repository) -> PattoResult<Vec<String>> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    Ok(repo
        .statuses(Some(&mut opts))?
        .iter()
        .filter_map(|e| e.path().ok().map(str::to_string))
        .filter(|p| p.ends_with(".pn"))
        .collect())
}

/// Note paths whose working copy differs from what was committed.
///
/// Empty when the directory is not a repository, so callers can treat "no git"
/// and "nothing changed" alike.
pub fn locally_modified_notes(root: &str) -> PattoResult<HashSet<String>> {
    let Ok(repo) = Repository::open(root) else {
        return Ok(HashSet::new());
    };
    Ok(dirty_notes(&repo)?.into_iter().collect())
}

/// When each note was last committed, in milliseconds since the epoch.
///
/// A clone gives every file the same modification time, so the filesystem says
/// nothing about when a note was actually written. This walks the history once,
/// newest first, and records the first commit that touched each note.
///
/// Merges are followed along the first parent only: what matters is when a
/// change arrived on this branch. The walk ends once every note in HEAD has a
/// time, and after [`MAX_COMMITS`] so a long history cannot stall the app; notes
/// older than that keep their file time. Deleted notes are only recorded if
/// they turn up before then.
pub fn note_commit_times(root: &str) -> PattoResult<HashMap<String, i64>> {
    /// Deep enough for any personal notes repository.
    const MAX_COMMITS: usize = 20_000;

    let mut times: HashMap<String, i64> = HashMap::new();

    let Ok(repo) = Repository::open(root) else {
        return Ok(times);
    };
    let mut walk = repo.revwalk()?;
    if walk.push_head().is_err() {
        // An unborn branch has no history yet.
        return Ok(times);
    }
    walk.simplify_first_parent()?;
    walk.set_sorting(git2::Sort::TIME)?;

    // Notes in HEAD still waiting for a time; the walk ends when none are left.
    let mut pending: HashSet<String> = HashSet::new();
    if let Ok(head) = repo.head().and_then(|h| h.peel_to_tree()) {
        head.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if let Some(name) = entry.name().ok().filter(|n| n.ends_with(".pn")) {
                pending.insert(format!("{dir}{name}"));
            }
            git2::TreeWalkResult::Ok
        })?;
    }

    for oid in walk.take(MAX_COMMITS) {
        let commit = repo.find_commit(oid?)?;
        let tree = commit.tree()?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        // No pathspec: libgit2 matches it against every entry of every tree,
        // which made the walk several times slower than filtering here.
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        let millis = commit.time().seconds() * 1000;

        for delta in diff.deltas() {
            for file in [delta.new_file(), delta.old_file()] {
                if let Some(path) = file.path().and_then(|p| p.to_str()) {
                    // Newest first, so the first time seen is the answer.
                    if path.ends_with(".pn") && !times.contains_key(path) {
                        pending.remove(path);
                        times.insert(path.to_string(), millis);
                    }
                }
            }
        }
        if pending.is_empty() {
            break;
        }
    }

    Ok(times)
}

pub fn git_status(root: String) -> PattoResult<GitStatus> {
    let repo = Repository::open(&root)?;
    let dirty = dirty_notes(&repo)?;

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
        conflict_pending: conflict_remote(&repo).is_some(),
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
pub(crate) fn commit_notes(repo: &Repository, sig: &Signature) -> PattoResult<Option<git2::Oid>> {
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

pub(crate) fn changed_between(repo: &Repository, before: Option<git2::Oid>) -> Vec<String> {
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

/// Holds the remote commit a paused sync could not merge. Its presence is what
/// "a merge is pending" means.
pub(crate) const CONFLICT_REF: &str = "refs/patto/conflict-remote";

/// Repository config key naming the branch this device pushes to while a merge
/// is pending.
const SIDE_BRANCH_KEY: &str = "patto.sidebranch";

/// The remote commit a paused sync is waiting to merge, if any.
pub(crate) fn conflict_remote(repo: &Repository) -> Option<Oid> {
    repo.find_reference(CONFLICT_REF).ok()?.target()
}

/// The branch this device pushes its commits to while a merge is pending.
///
/// Chosen once per clone and remembered, so repeated conflicts reuse it.
pub(crate) fn side_branch(repo: &Repository) -> PattoResult<String> {
    let config = repo.config()?;
    if let Ok(name) = config.get_string(SIDE_BRANCH_KEY) {
        return Ok(name);
    }

    // Unique enough to keep two phones apart without asking for a name.
    let seed = format!(
        "{:?}{}{}",
        std::time::SystemTime::now(),
        std::process::id(),
        repo.path().display()
    );
    let hash = Oid::hash_object(git2::ObjectType::Blob, seed.as_bytes())?;
    let name = format!("mobile/{}", &hash.to_string()[..6]);

    config
        .open_level(git2::ConfigLevel::Local)?
        .set_str(SIDE_BRANCH_KEY, &name)?;
    Ok(name)
}

/// The merge is done: forget the paused sync and delete the side branch on the
/// remote. Returns whether there was anything to clear.
pub(crate) fn clear_conflict(
    repo: &Repository,
    remote: &mut Remote,
    creds: &GitCreds,
    on_progress: &(dyn Fn(GitProgress) + Send + Sync),
) -> PattoResult<bool> {
    let Ok(mut reference) = repo.find_reference(CONFLICT_REF) else {
        return Ok(false);
    };
    reference.delete()?;

    // The desktop may already have deleted it; that is fine.
    let side = side_branch(repo)?;
    if let Err(e) = push(remote, &format!(":refs/heads/{side}"), creds, on_progress) {
        log::info!("could not delete {side} on the remote: {e}");
    }
    Ok(true)
}

pub(crate) fn push(
    remote: &mut Remote,
    refspec: &str,
    creds: &GitCreds,
    on_progress: &(dyn Fn(GitProgress) + Send + Sync),
) -> Result<(), git2::Error> {
    on_progress(GitProgress {
        phase: GitPhase::Pushing,
        current: 0,
        total: 0,
        bytes: 0,
    });

    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(callbacks(creds, on_progress));
    let mut proxy = ProxyOptions::new();
    proxy.auto();
    push_opts.proxy_options(proxy);
    remote.push(&[refspec], Some(&mut push_opts))
}

/// Fetch `branch` from origin and return the commit it points at.
pub(crate) fn fetch_branch(
    repo: &Repository,
    remote: &mut Remote,
    branch: &str,
    creds: &GitCreds,
    on_progress: &(dyn Fn(GitProgress) + Send + Sync),
) -> PattoResult<Oid> {
    remote.fetch(
        &[&format!("refs/heads/{branch}")],
        Some(&mut fetch_options(creds, on_progress)),
        None,
    )?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    Ok(repo.reference_to_annotated_commit(&fetch_head)?.id())
}

/// Move `branch` to `target` and check it out.
fn fast_forward(repo: &Repository, branch: &str, target: Oid) -> PattoResult<()> {
    let refname = format!("refs/heads/{branch}");
    match repo.find_reference(&refname) {
        Ok(mut reference) => {
            reference.set_target(target, "fast-forward")?;
        }
        Err(_) => {
            repo.reference(&refname, target, true, "fast-forward")?;
        }
    }
    repo.set_head(&refname)?;
    repo.checkout_head(Some(CheckoutBuilder::new().force()))?;
    Ok(())
}

/// Paths the index still has conflicts for.
pub(crate) fn conflict_paths(index: &git2::Index) -> PattoResult<Vec<String>> {
    let mut paths = Vec::new();
    for conflict in index.conflicts()? {
        let conflict = conflict?;
        if let Some(entry) = conflict.our.or(conflict.their).or(conflict.ancestor) {
            paths.push(String::from_utf8_lossy(&entry.path).to_string());
        }
    }
    Ok(paths)
}

/// Commit a merge of HEAD and `theirs` whose tree is `index`, and check it out.
pub(crate) fn commit_merge(
    repo: &Repository,
    index: &mut git2::Index,
    theirs: Oid,
    message: &str,
    sig: &Signature,
) -> PattoResult<()> {
    let tree = repo.find_tree(index.write_tree_to(repo)?)?;
    let head_commit = repo.head()?.peel_to_commit()?;
    let their_commit = repo.find_commit(theirs)?;
    repo.commit(
        Some("HEAD"),
        sig,
        sig,
        message,
        &tree,
        &[&head_commit, &their_commit],
    )?;
    repo.checkout_head(Some(CheckoutBuilder::new().force()))?;
    Ok(())
}

/// Bring the fetched commit into `branch`, unless lines clash.
///
/// The merge is tried in memory first, so a conflict leaves the working copy
/// and the branch exactly as they were.
fn integrate(
    repo: &Repository,
    branch: &str,
    fetched: Oid,
    sig: &Signature,
    on_progress: &(dyn Fn(GitProgress) + Send + Sync),
) -> PattoResult<MergeOutcome> {
    let annotated = repo.find_annotated_commit(fetched)?;
    let (analysis, _) = repo.merge_analysis(&[&annotated])?;

    if analysis.is_up_to_date() {
        return Ok(MergeOutcome::UpToDate);
    }
    if analysis.is_fast_forward() || analysis.is_unborn() {
        fast_forward(repo, branch, fetched)?;
        return Ok(MergeOutcome::FastForward);
    }

    on_progress(GitProgress {
        phase: GitPhase::Merging,
        current: 0,
        total: 0,
        bytes: 0,
    });

    let head = repo.head()?.peel_to_commit()?;
    let theirs = repo.find_commit(fetched)?;
    let mut index = repo.merge_commits(&head, &theirs, None)?;

    if index.has_conflicts() {
        return Ok(MergeOutcome::Conflicted {
            side_branch: String::new(),
            paths: conflict_paths(&index)?,
        });
    }

    commit_merge(
        repo,
        &mut index,
        fetched,
        &format!("patto-flutter: merge origin/{branch}"),
        sig,
    )?;
    Ok(MergeOutcome::Merged)
}

fn done(on_progress: &(dyn Fn(GitProgress) + Send + Sync)) {
    on_progress(GitProgress {
        phase: GitPhase::Done,
        current: 0,
        total: 0,
        bytes: 0,
    });
}

/// Whether a push failed only because the remote moved since the last fetch.
pub(crate) fn is_retryable(err: &PattoError) -> bool {
    matches!(
        err,
        PattoError::Git {
            kind: GitErrorKind::NonFastForward,
            ..
        }
    ) || matches!(err, PattoError::Git { message, .. } if message.contains("rejected"))
}

/// Commit local edits, integrate the remote, and push.
///
/// The push is retried when the remote moved between our fetch and our push.
/// When the remote cannot be merged cleanly, the local commits go to a side
/// branch instead and the report says which notes clash.
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

    let Ok(mut remote) = repo.find_remote("origin") else {
        return Err(PattoError::Git {
            kind: GitErrorKind::NoRemote,
            message: "no 'origin' remote configured".to_string(),
        });
    };

    let mut merge = MergeOutcome::UpToDate;
    let mut pushed = false;
    let mut last_error = None;

    for _ in 0..PUSH_ATTEMPTS {
        let fetched = fetch_branch(&repo, &mut remote, &branch, &creds, &on_progress)?;

        match integrate(&repo, &branch, fetched, &sig, &on_progress)? {
            MergeOutcome::Conflicted { paths, .. } => {
                repo.reference(CONFLICT_REF, fetched, true, "patto: sync paused")?;
                let side = side_branch(&repo)?;
                // The side branch belongs to this device, so overwriting it is safe.
                push(
                    &mut remote,
                    &format!("+refs/heads/{branch}:refs/heads/{side}"),
                    &creds,
                    &on_progress,
                )?;
                done(&on_progress);
                return Ok(SyncReport {
                    committed: commit_id.is_some(),
                    commit_id: commit_id.map(|id| id.to_string()),
                    merge: MergeOutcome::Conflicted {
                        side_branch: side,
                        paths,
                    },
                    pushed: false,
                    conflict_cleared: false,
                    changed_paths: changed_between(&repo, before),
                });
            }
            MergeOutcome::UpToDate => {}
            outcome => merge = outcome,
        }

        match push(
            &mut remote,
            &format!("refs/heads/{branch}:refs/heads/{branch}"),
            &creds,
            &on_progress,
        ) {
            Ok(()) => {
                pushed = true;
                break;
            }
            Err(e) => {
                let err = PattoError::from(e);
                let retryable = is_retryable(&err);
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

    // The branch on the remote now holds both sides.
    let conflict_cleared = clear_conflict(&repo, &mut remote, &creds, &on_progress)?;
    done(&on_progress);

    Ok(SyncReport {
        committed: commit_id.is_some(),
        commit_id: commit_id.map(|id| id.to_string()),
        merge,
        pushed,
        conflict_cleared,
        changed_paths: changed_between(&repo, before),
    })
}
