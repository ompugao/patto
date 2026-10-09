//! Talking to origin: credentials, progress callbacks, fetch, push and clone.

use std::path::Path;

use git2::build::RepoBuilder;
use git2::{
    Cred, FetchOptions, Oid, ProxyOptions, PushOptions, Remote, RemoteCallbacks, Repository,
};

use crate::api::error::PattoResult;
use crate::api::git::{GitCreds, GitPhase, GitProgress};

fn callbacks<'a>(
    creds: &GitCreds,
    on_progress: &'a (dyn Fn(GitProgress) + Send + Sync),
) -> RemoteCallbacks<'a> {
    let username = creds.username.clone();
    let token = creds.token.clone();
    let mut cb = RemoteCallbacks::new();

    // libgit2 retries credentials until one is accepted; without this guard a
    // wrong token loops instead of reporting an auth failure.
    let mut attempted = false;
    cb.credentials(move |_url, username_from_url, _allowed| {
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
    });

    cb.transfer_progress(move |stats| {
        let phase = if stats.received_objects() < stats.total_objects() {
            GitPhase::Receiving
        } else {
            GitPhase::Resolving
        };
        on_progress(GitProgress {
            phase,
            current: stats.received_objects() as u32,
            total: stats.total_objects() as u32,
            bytes: stats.received_bytes() as u64,
        });
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

    // libgit2 only reports "the certificate is invalid"; log what it actually
    // saw so a verification failure on device can be diagnosed.
    cb.certificate_check(|cert, host| {
        log::info!(
            "TLS certificate for {host}: x509={} valid_host={}",
            cert.as_x509().is_some(),
            !host.is_empty()
        );
        Ok(git2::CertificateCheckStatus::CertificatePassthrough)
    });

    cb.push_update_reference(|reference, status| match status {
        None => Ok(()),
        Some(msg) => Err(git2::Error::from_str(&format!(
            "remote rejected {reference}: {msg}"
        ))),
    });

    cb
}

fn fetch_options<'a>(
    creds: &GitCreds,
    on_progress: &'a (dyn Fn(GitProgress) + Send + Sync),
) -> FetchOptions<'a> {
    let mut opts = FetchOptions::new();
    opts.remote_callbacks(callbacks(creds, on_progress));
    let mut proxy = ProxyOptions::new();
    proxy.auto();
    opts.proxy_options(proxy);
    opts
}

pub fn git_clone(
    url: String,
    root: String,
    branch: Option<String>,
    creds: GitCreds,
    on_progress: impl Fn(GitProgress) + Send + Sync,
) -> PattoResult<()> {
    on_progress(GitProgress {
        phase: GitPhase::Connecting,
        current: 0,
        total: 0,
        bytes: 0,
    });

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options(&creds, &on_progress));
    if let Some(branch) = branch.as_deref().filter(|b| !b.is_empty()) {
        builder.branch(branch);
    }

    builder.clone(&url, Path::new(&root))?;
    on_progress(GitProgress {
        phase: GitPhase::Done,
        current: 0,
        total: 0,
        bytes: 0,
    });
    Ok(())
}

pub(crate) fn push(
    remote: &mut Remote,
    refspec: &str,
    creds: &GitCreds,
    on_progress: &(dyn Fn(GitProgress) + Send + Sync),
) -> Result<(), git2::Error> {
    on_progress(GitProgress {
        phase: GitPhase::Pushing,
        current: 0,
        total: 0,
        bytes: 0,
    });

    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(callbacks(creds, on_progress));
    let mut proxy = ProxyOptions::new();
    proxy.auto();
    push_opts.proxy_options(proxy);
    remote.push(&[refspec], Some(&mut push_opts))
}

/// Fetch `branch` from origin and return the commit it points at.
pub(crate) fn fetch_branch(
    repo: &Repository,
    remote: &mut Remote,
    branch: &str,
    creds: &GitCreds,
    on_progress: &(dyn Fn(GitProgress) + Send + Sync),
) -> PattoResult<Oid> {
    remote.fetch(
        &[&format!("refs/heads/{branch}")],
        Some(&mut fetch_options(creds, on_progress)),
        None,
    )?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    Ok(repo.reference_to_annotated_commit(&fetch_head)?.id())
}
