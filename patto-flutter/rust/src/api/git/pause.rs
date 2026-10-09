//! The state a sync leaves behind when it stops at a conflict: the remote
//! commit kept under [`CONFLICT_REF`] and the side branch this device pushes to
//! until the merge is done.

use git2::{Oid, Remote, Repository};

use crate::api::error::PattoResult;
use crate::api::git::remote::push;
use crate::api::git::{GitCreds, OnProgress};

/// Holds the remote commit a paused sync could not merge. Its presence is what
/// "a merge is pending" means.
const CONFLICT_REF: &str = "refs/patto/conflict-remote";

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

/// Keep the fetched commit for a later merge and park this device's commits on
/// the side branch, so they are safe on the remote while the merge waits.
/// Returns the side branch.
pub(super) fn pause_sync(
    repo: &Repository,
    remote: &mut Remote,
    branch: &str,
    fetched: Oid,
    creds: &GitCreds,
    on_progress: &OnProgress<'_>,
) -> PattoResult<String> {
    repo.reference(CONFLICT_REF, fetched, true, "patto: sync paused")?;
    let side = side_branch(repo)?;
    // The side branch belongs to this device, so overwriting it is safe.
    push(
        remote,
        &format!("+refs/heads/{branch}:refs/heads/{side}"),
        creds,
        on_progress,
    )?;
    Ok(side)
}

/// The merge is done: forget the paused sync and delete the side branch on the
/// remote. Returns whether there was anything to clear.
pub(crate) fn clear_conflict(
    repo: &Repository,
    remote: &mut Remote,
    creds: &GitCreds,
    on_progress: &OnProgress<'_>,
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
