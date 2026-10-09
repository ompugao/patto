//! Bringing a fetched commit into the branch: fast-forward or an in-memory
//! merge that is only committed when no note clashes.

use std::path::Path;

use git2::build::CheckoutBuilder;
use git2::{Index, IndexEntry, Oid, Repository, Signature};

use crate::api::error::PattoResult;
use crate::api::git::{head_commit, GitPhase, GitProgress, MergeOutcome, OnProgress};

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

/// One path the index could not merge, with whichever sides exist.
pub(crate) struct IndexConflict {
    pub path: String,
    pub ancestor: Option<IndexEntry>,
    pub ours: Option<IndexEntry>,
    pub theirs: Option<IndexEntry>,
}

impl IndexConflict {
    /// Whichever side exists, to copy mode and timestamps from.
    pub(crate) fn any_side(&self) -> &IndexEntry {
        self.ours
            .as_ref()
            .or(self.theirs.as_ref())
            .or(self.ancestor.as_ref())
            .expect("a conflict has at least one side")
    }
}

/// The index's conflicts in index order.
pub(crate) fn index_conflicts(index: &Index) -> PattoResult<Vec<IndexConflict>> {
    let mut out = Vec::new();
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
        out.push(IndexConflict {
            path,
            ancestor: conflict.ancestor,
            ours: conflict.our,
            theirs: conflict.their,
        });
    }
    Ok(out)
}

/// The stage bits of an entry's flags; stage 0 is a resolved path.
const STAGE_MASK: u16 = 0x3000;

/// `entry` as the resolved content of its path.
pub(crate) fn resolved(mut entry: IndexEntry) -> IndexEntry {
    entry.flags &= !STAGE_MASK;
    entry
}

/// Settles every clash on a file that is not a note in favour of this device's
/// copy, the way the app settles every conflict it cannot present: such a file
/// is an attachment or other binary that the line merge would only corrupt.
/// Keeps ours where both sides have one, and the deletion where this side
/// deleted it.
pub(crate) fn settle_file_conflicts(index: &mut Index) -> PattoResult<()> {
    let files = index_conflicts(index)?
        .into_iter()
        .filter(|c| !c.path.ends_with(".pn"));
    for file in files {
        index.conflict_remove(Path::new(&file.path))?;
        if let Some(ours) = file.ours {
            index.add(&resolved(ours))?;
        }
    }
    Ok(())
}

fn conflict_paths(index: &Index) -> PattoResult<Vec<String>> {
    Ok(index_conflicts(index)?
        .into_iter()
        .map(|c| c.path)
        .collect())
}

/// Commit a merge of HEAD and `theirs` whose tree is `index`, and check it out.
pub(crate) fn commit_merge(
    repo: &Repository,
    index: &mut Index,
    theirs: Oid,
    message: &str,
    sig: &Signature,
) -> PattoResult<()> {
    let tree = repo.find_tree(index.write_tree_to(repo)?)?;
    let head_commit = head_commit(repo)?;
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
    on_progress: &OnProgress<'_>,
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

    on_progress(GitProgress::at(GitPhase::Merging));

    let head = head_commit(repo)?;
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
