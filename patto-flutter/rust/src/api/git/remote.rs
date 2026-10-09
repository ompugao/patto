//! Talking to origin: credentials, progress callbacks, fetch, push and clone.

use std::path::Path;

use git2::build::RepoBuilder;
use git2::{
    Cred, CredentialType, FetchOptions, Oid, ProxyOptions, PushOptions, Remote, RemoteCallbacks,
    Repository,
};

use crate::api::error::PattoResult;
use crate::api::git::{GitCreds, GitPhase, GitProgress, OnProgress};

fn callbacks<'a>(creds: &GitCreds, on_progress: &'a OnProgress<'a>) -> RemoteCallbacks<'a> {
    let mut cb = RemoteCallbacks::new();
    cb.credentials(credential_provider(creds));
    cb.transfer_progress(move |stats| {
        on_progress(progress_of(&stats));
        true
    });
    cb.push_transfer_progress(move |current, total, bytes| {
        on_progress(GitProgress {
            phase: GitPhase::Pushing,
            current: current as u32,
            total: total as u32,
            bytes: bytes as u64,
        });
    });
    cb.certificate_check(log_certificate);
    cb.push_update_reference(fail_on_rejected_update);
    cb
}

/// libgit2 retries credentials until one is accepted; without the guard a
/// wrong token loops instead of reporting an auth failure.
fn credential_provider(
    creds: &GitCreds,
) -> impl FnMut(&str, Option<&str>, CredentialType) -> Result<Cred, git2::Error> {
    let username = creds.username.clone();
    let token = creds.token.clone();
    let mut attempted = false;
    move |_url, username_from_url, _allowed| {
        if attempted {
            return Err(git2::Error::from_str("authentication failed"));
        }
        attempted = true;
        let user = if username.is_empty() {
            username_from_url.unwrap_or("git")
        } else {
            username.as_str()
        };
        Cred::userpass_plaintext(user, &token)
    }
}

fn progress_of(stats: &git2::Progress<'_>) -> GitProgress {
    let phase = if stats.received_objects() < stats.total_objects() {
        GitPhase::Receiving
    } else {
        GitPhase::Resolving
    };
    GitProgress {
        phase,
        current: stats.received_objects() as u32,
        total: stats.total_objects() as u32,
        bytes: stats.received_bytes() as u64,
    }
}

/// libgit2 only reports "the certificate is invalid"; log what it actually
/// saw so a verification failure on device can be diagnosed.
fn log_certificate(
    cert: &git2::cert::Cert<'_>,
    host: &str,
) -> Result<git2::CertificateCheckStatus, git2::Error> {
    log::info!(
        "TLS certificate for {host}: x509={} valid_host={}",
        cert.as_x509().is_some(),
        !host.is_empty()
    );
    Ok(git2::CertificateCheckStatus::CertificatePassthrough)
}

fn fail_on_rejected_update(reference: &str, status: Option<&str>) -> Result<(), git2::Error> {
    match status {
        None => Ok(()),
        Some(msg) => Err(git2::Error::from_str(&format!(
            "remote rejected {reference}: {msg}"
        ))),
    }
}

fn auto_proxy<'a>() -> ProxyOptions<'a> {
    let mut proxy = ProxyOptions::new();
    proxy.auto();
    proxy
}

fn fetch_options<'a>(creds: &GitCreds, on_progress: &'a OnProgress<'a>) -> FetchOptions<'a> {
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callbacks(creds, on_progress));
    opts.proxy_options(auto_proxy());
    opts
}

fn push_options<'a>(creds: &GitCreds, on_progress: &'a OnProgress<'a>) -> PushOptions<'a> {
    let mut opts = PushOptions::new();
    opts.remote_callbacks(callbacks(creds, on_progress));
    opts.proxy_options(auto_proxy());
    opts
}

pub fn git_clone(
    url: String,
    root: String,
    branch: Option<String>,
    creds: GitCreds,
    on_progress: impl Fn(GitProgress) + Send + Sync,
) -> PattoResult<()> {
    on_progress(GitProgress::at(GitPhase::Connecting));

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options(&creds, &on_progress));
    if let Some(branch) = branch.as_deref().filter(|b| !b.is_empty()) {
        builder.branch(branch);
    }

    builder.clone(&url, Path::new(&root))?;
    on_progress(GitProgress::at(GitPhase::Done));
    Ok(())
}

/// Fetch `branch` from origin and return the commit it points at.
pub(crate) fn fetch_branch(
    repo: &Repository,
    remote: &mut Remote,
    branch: &str,
    creds: &GitCreds,
    on_progress: &OnProgress<'_>,
) -> PattoResult<Oid> {
    remote.fetch(
        &[&format!("refs/heads/{branch}")],
        Some(&mut fetch_options(creds, on_progress)),
        None,
    )?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    Ok(repo.reference_to_annotated_commit(&fetch_head)?.id())
}

pub(crate) fn push(
    remote: &mut Remote,
    refspec: &str,
    creds: &GitCreds,
    on_progress: &OnProgress<'_>,
) -> Result<(), git2::Error> {
    on_progress(GitProgress::at(GitPhase::Pushing));
    remote.push(&[refspec], Some(&mut push_options(creds, on_progress)))
}
