//! Git sync over HTTPS with a personal access token.
//!
//! Notes live in a normal clone on the device. Sync is commit → fetch →
//! fast-forward or merge → push. When both sides changed the same lines nothing
//! is merged: the phone's commits are pushed to a branch of their own, so they
//! are safe, and the merge waits until it is done on the desktop or resolved in
//! the app (see [`crate::api::conflict`]).

mod commit;
mod integrate;
mod pause;
mod remote;
mod runtime;
mod status;
mod sync;

use git2::{Commit, Oid, Remote, Repository};

use crate::api::error::{GitErrorKind, PattoError, PattoResult};

pub use remote::git_clone;
pub use runtime::git_init_runtime;
pub use status::{git_status, locally_modified_notes, note_commit_times};
pub use sync::git_sync;

pub(crate) use commit::{
    changed_between, commit_notes, current_branch, diff_paths, normalize_attachments_dir,
};
pub(crate) use integrate::{
    commit_merge, index_conflicts, resolved, settle_file_conflicts, IndexConflict,
};
pub(crate) use pause::{clear_conflict, conflict_remote, side_branch};
pub(crate) use remote::{fetch_branch, push};

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

impl GitProgress {
    pub(crate) fn at(phase: GitPhase) -> Self {
        GitProgress {
            phase,
            current: 0,
            total: 0,
            bytes: 0,
        }
    }
}

pub(crate) type OnProgress<'a> = dyn Fn(GitProgress) + Send + Sync + 'a;

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
    /// Paths that changed on disk during the sync, so the app can refresh just
    /// those notes and forget cached pictures.
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

pub(crate) fn head_commit(repo: &Repository) -> PattoResult<Commit<'_>> {
    Ok(repo.head()?.peel_to_commit()?)
}

/// `None` on an unborn branch.
pub(crate) fn head_oid(repo: &Repository) -> Option<Oid> {
    head_commit(repo).ok().map(|c| c.id())
}

pub(crate) fn origin(repo: &Repository) -> PattoResult<Remote<'_>> {
    repo.find_remote("origin").map_err(|_| PattoError::Git {
        kind: GitErrorKind::NoRemote,
        message: "no 'origin' remote configured".to_string(),
    })
}
