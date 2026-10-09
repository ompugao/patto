//! The sync itself: commit, fetch, integrate, push, retry.

use git2::{Repository, Signature};

use crate::api::error::{GitErrorKind, PattoError, PattoResult};
use crate::api::git::commit::{
    changed_between, commit_notes, current_branch, normalize_attachments_dir,
};
use crate::api::git::integrate::integrate;
use crate::api::git::pause::{clear_conflict, side_branch, CONFLICT_REF};
use crate::api::git::remote::{fetch_branch, push};
use crate::api::git::{GitCreds, GitPhase, GitProgress, MergeOutcome, SyncReport};

fn done(on_progress: &(dyn Fn(GitProgress) + Send + Sync)) {
    on_progress(GitProgress {
        phase: GitPhase::Done,
        current: 0,
        total: 0,
        bytes: 0,
    });
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
    const PUSH_ATTEMPTS: usize = 3;

    let attachments_dir = normalize_attachments_dir(&attachments_dir);
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
    let commit_id = commit_notes(&repo, &sig, &attachments_dir)?;

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
