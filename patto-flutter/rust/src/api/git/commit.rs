//! What the phone commits, and how to find out what changed.

use git2::{Diff, Oid, Repository, Signature};

use crate::api::error::PattoResult;
use crate::api::git::head_commit;

pub(crate) fn current_branch(repo: &Repository) -> PattoResult<String> {
    let head = repo.head()?;
    Ok(head.shorthand()?.to_string())
}

/// The folder under the root that the editor copies inserted files (images,
/// PDFs) into, as the workspace configures it; synced along with the notes.
/// Empty means no folder is synced.
pub(crate) fn normalize_attachments_dir(dir: &str) -> String {
    dir.trim().trim_matches('/').to_string()
}

/// Whether the phone commits changes to this path: notes and attachments,
/// nothing else that may turn up in a working copy.
fn is_synced(path: &str, attachments_dir: &str) -> bool {
    path.ends_with(".pn") || is_attachment(path, attachments_dir)
}

fn is_attachment(path: &str, attachments_dir: &str) -> bool {
    !attachments_dir.is_empty()
        && path
            .strip_prefix(attachments_dir)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Synced paths that differ from HEAD, including untracked ones.
pub(super) fn dirty_paths(repo: &Repository, attachments_dir: &str) -> PattoResult<Vec<String>> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    Ok(repo
        .statuses(Some(&mut opts))?
        .iter()
        .filter_map(|e| e.path().ok().map(str::to_string))
        .filter(|p| is_synced(p, attachments_dir))
        .collect())
}

/// Stage every note and attachment change and commit, returning the new
/// commit id if the tree actually differs from HEAD.
pub(crate) fn commit_notes(
    repo: &Repository,
    sig: &Signature,
    attachments_dir: &str,
) -> PattoResult<Option<Oid>> {
    let mut index = repo.index()?;
    let mut pathspecs = vec!["*.pn"];
    if !attachments_dir.is_empty() {
        pathspecs.push(attachments_dir);
    }
    index.add_all(pathspecs, git2::IndexAddOption::DEFAULT, None)?;
    index.update_all(["*"], None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parent = head_commit(repo).ok();
    if parent.as_ref().is_some_and(|p| p.tree_id() == tree_id) {
        return Ok(None);
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

/// Paths whose content differs between `before` and HEAD.
pub(crate) fn changed_between(repo: &Repository, before: Option<Oid>) -> Vec<String> {
    let (Some(before), Ok(after)) = (before, head_commit(repo)) else {
        return Vec::new();
    };
    if after.id() == before {
        return Vec::new();
    }

    let old_tree = repo.find_commit(before).and_then(|c| c.tree()).ok();
    let new_tree = after.tree().ok();
    repo.diff_tree_to_tree(old_tree.as_ref(), new_tree.as_ref(), None)
        .map(|diff| diff_paths(&diff))
        .unwrap_or_default()
}

/// Every path a diff touches, once each, counting both the old and the new
/// name of a rename.
pub(crate) fn diff_paths(diff: &Diff) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for delta in diff.deltas() {
        for file in [delta.new_file(), delta.old_file()] {
            if let Some(path) = file.path().and_then(|p| p.to_str()) {
                if !paths.iter().any(|q| q == path) {
                    paths.push(path.to_string());
                }
            }
        }
    }
    paths
}
