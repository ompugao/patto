//! Read-only views of the clone: status and per-note commit times.

use std::collections::{HashMap, HashSet};

use git2::{Oid, Repository};

use crate::api::error::PattoResult;
use crate::api::git::commit::{current_branch, diff_paths, dirty_paths, normalize_attachments_dir};
use crate::api::git::pause::conflict_remote;
use crate::api::git::{head_oid, GitStatus};

/// Note paths whose working copy differs from what was committed.
///
/// Empty when the directory is not a repository, so callers can treat "no git"
/// and "nothing changed" alike.
pub fn locally_modified_notes(root: &str) -> PattoResult<HashSet<String>> {
    let Ok(repo) = Repository::open(root) else {
        return Ok(HashSet::new());
    };
    Ok(dirty_paths(&repo, "")?.into_iter().collect())
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
    let mut pending = notes_in_head(&repo)?;

    for oid in walk.take(MAX_COMMITS) {
        let commit = repo.find_commit(oid?)?;
        let tree = commit.tree()?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        // No pathspec: libgit2 matches it against every entry of every tree,
        // which made the walk several times slower than filtering here.
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
        let millis = commit.time().seconds() * 1000;

        for path in diff_paths(&diff) {
            // Newest first, so the first time seen is the answer.
            if path.ends_with(".pn") && !times.contains_key(&path) {
                pending.remove(&path);
                times.insert(path, millis);
            }
        }
        if pending.is_empty() {
            break;
        }
    }

    Ok(times)
}

fn notes_in_head(repo: &Repository) -> PattoResult<HashSet<String>> {
    let mut notes = HashSet::new();
    if let Ok(head) = repo.head().and_then(|h| h.peel_to_tree()) {
        head.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if let Some(name) = entry.name().ok().filter(|n| n.ends_with(".pn")) {
                notes.insert(format!("{dir}{name}"));
            }
            git2::TreeWalkResult::Ok
        })?;
    }
    Ok(notes)
}

pub fn git_status(root: String, attachments_dir: String) -> PattoResult<GitStatus> {
    let repo = Repository::open(&root)?;
    let dirty = dirty_paths(&repo, &normalize_attachments_dir(&attachments_dir))?;
    let branch = current_branch(&repo).unwrap_or_else(|_| "HEAD".to_string());
    let (ahead, behind) = ahead_behind(&repo, &branch);
    let has_remote = repo.find_remote("origin").is_ok();

    Ok(GitStatus {
        branch,
        dirty,
        ahead: ahead as u32,
        behind: behind as u32,
        has_remote,
        conflict_pending: conflict_remote(&repo).is_some(),
    })
}

fn ahead_behind(repo: &Repository, branch: &str) -> (usize, usize) {
    let (Some(local), Some(upstream)) = (head_oid(repo), upstream_oid(repo, branch)) else {
        return (0, 0);
    };
    repo.graph_ahead_behind(local, upstream).unwrap_or((0, 0))
}

fn upstream_oid(repo: &Repository, branch: &str) -> Option<Oid> {
    repo.find_branch(&format!("origin/{branch}"), git2::BranchType::Remote)
        .ok()?
        .get()
        .target()
}
