//! Bringing a fetched commit into the branch: fast-forward or an in-memory
//! merge that is only committed when no note clashes.

use git2::build::CheckoutBuilder;
use git2::{Oid, Repository, Signature};

use crate::api::error::PattoResult;
use crate::api::git::{GitPhase, GitProgress, MergeOutcome};

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

/// Settles every clash on a file that is not a note in favour of this device's
/// copy, the way the app settles every conflict it cannot present: such a file
/// is an attachment or other binary that the line merge would only corrupt.
/// Keeps ours where both sides have one, and the deletion where this side
/// deleted it.
pub(crate) fn settle_file_conflicts(index: &mut git2::Index) -> PattoResult<()> {
    let mut clashes = Vec::new();
    for conflict in index.conflicts()? {
        let conflict = conflict?;
        let Some(entry) = conflict
            .our
            .as_ref()
            .or(conflict.their.as_ref())
            .or(conflict.ancestor.as_ref())
        else {
            continue;
        };
        let path = String::from_utf8_lossy(&entry.path).to_string();
        if !path.ends_with(".pn") {
            clashes.push((path, conflict.our));
        }
    }
    for (path, ours) in clashes {
        index.conflict_remove(std::path::Path::new(&path))?;
        if let Some(mut entry) = ours {
            // Stage 0 marks the entry resolved.
            entry.flags &= !0x3000;
            index.add(&entry)?;
        }
    }
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
pub(super) fn integrate(
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
    settle_file_conflicts(&mut index)?;

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
