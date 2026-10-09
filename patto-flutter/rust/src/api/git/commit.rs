//! What the phone commits, and how to find out what changed.

use git2::{Repository, Signature};

use crate::api::error::PattoResult;

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
) -> PattoResult<Option<git2::Oid>> {
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
                    if !paths.contains(&p.to_string()) {
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
