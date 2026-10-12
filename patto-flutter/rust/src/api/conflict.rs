//! A sync paused at a conflict, seen from the app.
//!
//! When a sync cannot merge cleanly it keeps the remote commit under a
//! reference of its own and changes nothing else (see [`crate::api::git`]). This
//! module reads that state back: which notes clash, what each side did to
//! them, and applies the choices the user made on the phone.
//!
//! The merge is always recomputed from HEAD and the kept remote commit, so it
//! reflects any commits made since the sync paused.

use std::path::Path;

use git2::{Index, IndexEntry, Oid, Remote, Repository, Signature};

use crate::api::error::{GitErrorKind, PattoError, PattoResult};
use crate::api::git::{
    changed_between, clear_conflict, commit_merge, commit_notes, conflict_remote, current_branch,
    diff_paths, fetch_branch, head_commit, index_conflicts, normalize_attachments_dir, origin,
    push, resolved, settle_file_conflicts, side_branch, GitCreds, GitPhase, GitProgress,
    IndexConflict, MergeOutcome, OnProgress, SyncReport,
};
use crate::api::merge::{merge_lines, MergedNote};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Both sides edited the note.
    BothModified,
    /// Both sides created a note at the same path.
    BothAdded,
    /// The phone deleted the note; the remote edited it.
    DeletedByUs,
    /// The remote deleted the note; the phone edited it.
    DeletedByThem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictFile {
    pub path: String,
    pub kind: ConflictKind,
    /// Roughly how many lines each side changed.
    pub ours_changed: u32,
    pub theirs_changed: u32,
    /// Places where both sides changed the same lines.
    pub conflicts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCommit {
    pub id: String,
    pub summary: String,
    pub author: String,
    pub time_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingConflict {
    /// Where this device's commits were pushed, for merging on the desktop.
    pub side_branch: String,
    /// The newest remote commit the sync could not merge.
    pub remote: RemoteCommit,
    pub files: Vec<ConflictFile>,
    /// Notes the remote changed that merge cleanly but are held back with the
    /// rest until the merge is done.
    pub held_back: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictDetail {
    pub path: String,
    pub kind: ConflictKind,
    /// Blob ids of each side, handed back with the resolution so a note that
    /// changed in between is noticed.
    pub ours_id: Option<String>,
    pub theirs_id: Option<String>,
    /// Whole versions, for reading one side at a time. `None` when that side
    /// deleted the note.
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub merged: MergedNote,
}

/// How the user settled one note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub path: String,
    /// The ids from the [`ConflictDetail`] the user looked at.
    pub ours_id: Option<String>,
    pub theirs_id: Option<String>,
    /// The note's text, or `None` to delete it.
    pub content: Option<String>,
}

fn stale(message: impl Into<String>) -> PattoError {
    PattoError::Git {
        kind: GitErrorKind::Stale,
        message: message.into(),
    }
}

/// HEAD merged in memory with the kept remote commit.
struct TrialMerge {
    theirs: Oid,
    index: Index,
}

impl TrialMerge {
    fn conflicts(&self) -> PattoResult<Vec<IndexConflict>> {
        index_conflicts(&self.index)
    }
}

fn trial_merge(repo: &Repository) -> PattoResult<Option<TrialMerge>> {
    let Some(theirs) = conflict_remote(repo) else {
        return Ok(None);
    };
    let head = head_commit(repo)?;
    let mut index = repo.merge_commits(&head, &repo.find_commit(theirs)?, None)?;
    settle_file_conflicts(&mut index)?;
    Ok(Some(TrialMerge { theirs, index }))
}

fn kind_of(conflict: &IndexConflict) -> ConflictKind {
    match (&conflict.ancestor, &conflict.ours, &conflict.theirs) {
        (None, _, _) => ConflictKind::BothAdded,
        (Some(_), None, _) => ConflictKind::DeletedByUs,
        (Some(_), _, None) => ConflictKind::DeletedByThem,
        _ => ConflictKind::BothModified,
    }
}

fn blob_id(entry: &Option<IndexEntry>) -> Option<String> {
    entry.as_ref().map(|e| e.id.to_string())
}

fn blob_text(repo: &Repository, entry: Option<&IndexEntry>) -> PattoResult<Option<String>> {
    let Some(entry) = entry else {
        return Ok(None);
    };
    let blob = repo.find_blob(entry.id)?;
    Ok(Some(String::from_utf8_lossy(blob.content()).to_string()))
}

fn merge_sides(
    repo: &Repository,
    conflict: &IndexConflict,
) -> PattoResult<(Option<String>, Option<String>, MergedNote)> {
    let base = blob_text(repo, conflict.ancestor.as_ref())?.unwrap_or_default();
    let ours = blob_text(repo, conflict.ours.as_ref())?;
    let theirs = blob_text(repo, conflict.theirs.as_ref())?;
    // A deleted side is compared as empty, which shows everything the other
    // side has as its change.
    let merged = merge_lines(
        &base,
        ours.as_deref().unwrap_or(""),
        theirs.as_deref().unwrap_or(""),
    )?;
    Ok((ours, theirs, merged))
}

fn remote_commit(repo: &Repository, id: Oid) -> PattoResult<RemoteCommit> {
    let commit = repo.find_commit(id)?;
    let author = commit.author();
    Ok(RemoteCommit {
        id: id.to_string(),
        summary: String::from_utf8_lossy(commit.summary_bytes().unwrap_or_default()).to_string(),
        author: String::from_utf8_lossy(author.name_bytes()).to_string(),
        time_ms: commit.time().seconds() * 1000,
    })
}

/// Notes the remote changed since the common ancestor.
fn remote_changes(repo: &Repository, theirs: Oid) -> PattoResult<Vec<String>> {
    let base = repo.merge_base(head_commit(repo)?.id(), theirs)?;
    let old = repo.find_commit(base)?.tree()?;
    let new = repo.find_commit(theirs)?.tree()?;
    let diff = repo.diff_tree_to_tree(Some(&old), Some(&new), None)?;
    Ok(diff_paths(&diff)
        .into_iter()
        .filter(|p| p.ends_with(".pn"))
        .collect())
}

/// The paused sync, if there is one.
pub fn pending_conflict(root: String) -> PattoResult<Option<PendingConflict>> {
    let repo = Repository::open(&root)?;
    let Some(trial) = trial_merge(&repo)? else {
        return Ok(None);
    };

    let conflicts = trial.conflicts()?;
    let mut files = Vec::new();
    for conflict in &conflicts {
        let (_, _, merged) = merge_sides(&repo, conflict)?;
        let (ours_changed, theirs_changed) = merged.changed_lines();
        files.push(ConflictFile {
            path: conflict.path.clone(),
            kind: kind_of(conflict),
            ours_changed,
            theirs_changed,
            conflicts: merged.conflict_count() as u32,
        });
    }

    let held_back = remote_changes(&repo, trial.theirs)?
        .into_iter()
        .filter(|p| !conflicts.iter().any(|c| &c.path == p))
        .collect();

    Ok(Some(PendingConflict {
        side_branch: side_branch(&repo)?,
        remote: remote_commit(&repo, trial.theirs)?,
        files,
        held_back,
    }))
}

/// Everything needed to show and settle one clashing note.
pub fn conflict_detail(root: String, rel_path: String) -> PattoResult<ConflictDetail> {
    let repo = Repository::open(&root)?;
    let trial = trial_merge(&repo)?.ok_or_else(|| stale("no sync is waiting to be merged"))?;
    let conflicts = trial.conflicts()?;
    let conflict = conflicts
        .iter()
        .find(|c| c.path == rel_path)
        .ok_or_else(|| stale(format!("{rel_path} no longer conflicts")))?;

    let (ours, theirs, merged) = merge_sides(&repo, conflict)?;
    Ok(ConflictDetail {
        path: rel_path,
        kind: kind_of(conflict),
        ours_id: blob_id(&conflict.ours),
        theirs_id: blob_id(&conflict.theirs),
        ours,
        theirs,
        merged,
    })
}

/// Stage `resolution` in place of the conflict at its path.
fn apply(
    repo: &Repository,
    index: &mut Index,
    conflict: &IndexConflict,
    resolution: &Resolution,
) -> PattoResult<()> {
    if blob_id(&conflict.ours) != resolution.ours_id
        || blob_id(&conflict.theirs) != resolution.theirs_id
    {
        return Err(stale(format!(
            "{} changed after it was reviewed",
            resolution.path
        )));
    }

    index.conflict_remove(Path::new(&resolution.path))?;

    let Some(content) = &resolution.content else {
        return Ok(());
    };
    let template = conflict.any_side();
    let entry = IndexEntry {
        id: repo.blob(content.as_bytes())?,
        file_size: content.len() as u32,
        path: template.path.clone(),
        ..*template
    };
    index.add(&resolved(entry))?;
    Ok(())
}

fn apply_resolutions(
    repo: &Repository,
    index: &mut Index,
    resolutions: &[Resolution],
) -> PattoResult<()> {
    for conflict in index_conflicts(index)? {
        let resolution = resolutions
            .iter()
            .find(|r| r.path == conflict.path)
            .ok_or_else(|| stale(format!("{} has not been resolved", conflict.path)))?;
        apply(repo, index, &conflict, resolution)?;
    }
    if index.has_conflicts() {
        return Err(stale("a conflict is left unresolved"));
    }
    Ok(())
}

/// The kept remote commit must still be the tip of the branch, or the user
/// reviewed a merge that is no longer the one to make.
fn expect_remote_unmoved(
    repo: &Repository,
    remote: &mut Remote,
    branch: &str,
    creds: &GitCreds,
    on_progress: &OnProgress<'_>,
) -> PattoResult<()> {
    let expected = conflict_remote(repo).ok_or_else(|| stale("no sync is waiting to be merged"))?;
    let fetched = fetch_branch(repo, remote, branch, creds, on_progress)?;
    if fetched != expected {
        return Err(stale(
            "the remote changed while the conflict was being resolved",
        ));
    }
    Ok(())
}

/// Merge using the user's choices for every clashing note, then push.
///
/// Fails with [`GitErrorKind::Stale`] when the remote moved or a note changed
/// since it was reviewed; the app then shows the conflict again.
pub fn git_resolve(
    root: String,
    attachments_dir: String,
    author_name: String,
    author_email: String,
    creds: GitCreds,
    resolutions: Vec<Resolution>,
    on_progress: impl Fn(GitProgress) + Send + Sync,
) -> PattoResult<SyncReport> {
    let attachments_dir = normalize_attachments_dir(&attachments_dir);
    let repo = Repository::open(&root)?;
    let sig = Signature::now(&author_name, &author_email)?;
    let branch = current_branch(&repo)?;
    let before = head_commit(&repo)?.id();

    on_progress(GitProgress::at(GitPhase::Committing));
    // Edits made since the sync paused are part of our side; if they touched a
    // clashing note, its id no longer matches and the user reviews it again.
    let commit_id = commit_notes(&repo, &sig, &attachments_dir)?;

    let mut remote = origin(&repo)?;
    expect_remote_unmoved(&repo, &mut remote, &branch, &creds, &on_progress)?;

    on_progress(GitProgress::at(GitPhase::Merging));
    let TrialMerge { theirs, mut index } =
        trial_merge(&repo)?.ok_or_else(|| stale("no sync is waiting to be merged"))?;
    apply_resolutions(&repo, &mut index, &resolutions)?;
    commit_merge(
        &repo,
        &mut index,
        theirs,
        &format!("patto-flutter: merge origin/{branch} (resolved on the phone)"),
        &sig,
    )?;

    // Anything that goes wrong from here leaves an ordinary merge commit that the
    // next sync pushes.
    push(
        &mut remote,
        &format!("refs/heads/{branch}:refs/heads/{branch}"),
        &creds,
        &on_progress,
    )?;
    let conflict_cleared = clear_conflict(&repo, &mut remote, &creds, &on_progress)?;
    on_progress(GitProgress::at(GitPhase::Done));

    Ok(SyncReport {
        committed: commit_id.is_some(),
        commit_id: commit_id.map(|id| id.to_string()),
        merge: MergeOutcome::Merged,
        pushed: true,
        conflict_cleared,
        changed_paths: changed_between(&repo, Some(before)),
    })
}
