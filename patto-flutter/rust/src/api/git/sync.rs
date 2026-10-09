//! The sync itself: commit, fetch, integrate, push, retry.

use git2::{Remote, Repository, Signature};

use crate::api::error::{GitErrorKind, PattoError, PattoResult};
use crate::api::git::commit::{
    changed_between, commit_notes, current_branch, normalize_attachments_dir,
};
use crate::api::git::integrate::integrate;
use crate::api::git::pause::{clear_conflict, pause_sync};
use crate::api::git::remote::{fetch_branch, push};
use crate::api::git::{
    head_oid, origin, GitCreds, GitPhase, GitProgress, MergeOutcome, OnProgress, SyncReport,
};

/// Commit local edits, integrate the remote, and push.
///
/// The push is retried when the remote moved between our fetch and our push.
/// When the remote cannot be merged cleanly, the local commits go to a side
/// branch instead and the report says which notes clash.
pub fn git_sync(
    root: String,
    attachments_dir: String,
    author_name: String,
    author_email: String,
    creds: GitCreds,
    on_progress: impl Fn(GitProgress) + Send + Sync,
) -> PattoResult<SyncReport> {
    let attachments_dir = normalize_attachments_dir(&attachments_dir);
    let repo = Repository::open(&root)?;
    let sig = Signature::now(&author_name, &author_email)?;
    let branch = current_branch(&repo)?;
    let before = head_oid(&repo);

    on_progress(GitProgress::at(GitPhase::Committing));
    let commit_id = commit_notes(&repo, &sig, &attachments_dir)?;
    let mut remote = origin(&repo)?;

    let (merge, pushed, conflict_cleared) =
        match integrate_and_push(&repo, &mut remote, &branch, &sig, &creds, &on_progress)? {
            Outcome::Pushed(merge) => {
                // The branch on the remote now holds both sides.
                let cleared = clear_conflict(&repo, &mut remote, &creds, &on_progress)?;
                (merge, true, cleared)
            }
            Outcome::Paused { side_branch, paths } => (
                MergeOutcome::Conflicted { side_branch, paths },
                false,
                false,
            ),
        };
    on_progress(GitProgress::at(GitPhase::Done));

    Ok(SyncReport {
        committed: commit_id.is_some(),
        commit_id: commit_id.map(|id| id.to_string()),
        merge,
        pushed,
        conflict_cleared,
        changed_paths: changed_between(&repo, before),
    })
}

enum Outcome {
    Pushed(MergeOutcome),
    Paused {
        side_branch: String,
        paths: Vec<String>,
    },
}

/// Fetch, integrate and push, starting over when the remote moved in between.
fn integrate_and_push(
    repo: &Repository,
    remote: &mut Remote,
    branch: &str,
    sig: &Signature,
    creds: &GitCreds,
    on_progress: &OnProgress<'_>,
) -> PattoResult<Outcome> {
    const PUSH_ATTEMPTS: usize = 3;

    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    let mut merge = MergeOutcome::UpToDate;
    let mut last_error = None;

    for _ in 0..PUSH_ATTEMPTS {
        let fetched = fetch_branch(repo, remote, branch, creds, on_progress)?;
        match integrate(repo, branch, fetched, sig, on_progress)? {
            MergeOutcome::Conflicted { paths, .. } => {
                let side_branch = pause_sync(repo, remote, branch, fetched, creds, on_progress)?;
                return Ok(Outcome::Paused { side_branch, paths });
            }
            // A retry that finds nothing new must not hide the merge made
            // by the attempt before it.
            MergeOutcome::UpToDate => {}
            outcome => merge = outcome,
        }

        match push(remote, &refspec, creds, on_progress) {
            Ok(()) => return Ok(Outcome::Pushed(merge)),
            Err(e) => {
                let err = PattoError::from(e);
                if !is_retryable(&err) {
                    return Err(err);
                }
                last_error = Some(err);
            }
        }
    }

    Err(last_error.expect("every attempt pushes or records why it could not"))
}

/// Whether a push failed only because the remote moved since the last fetch.
fn is_retryable(err: &PattoError) -> bool {
    matches!(
        err,
        PattoError::Git {
            kind: GitErrorKind::NonFastForward,
            ..
        }
    ) || matches!(err, PattoError::Git { message, .. } if message.contains("rejected"))
}
