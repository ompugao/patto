//! One-time libgit2 setup.

use std::path::Path;
use std::sync::OnceLock;

use foreign_types_shared::ForeignType;
use git2::Repository;

use crate::api::error::{GitErrorKind, PattoError, PattoResult};

static CERT_FILE: OnceLock<String> = OnceLock::new();

/// Give libgit2 the trust roots it needs to verify an HTTPS server.
///
/// The vendored OpenSSL ships no trust store, and on Android `openssl-src`
/// configures it with `no-stdio`, so OpenSSL there cannot open a PEM file at
/// all: pointing libgit2 at a path fails with a "BIO lib" error and
/// `SSL_CERT_FILE` is equally useless. The certificates are therefore parsed
/// here and handed to libgit2 one at a time, which only touches memory.
///
/// The app bundles `cacert.pem` as an asset and passes its path once at startup.
pub fn git_init_runtime(ca_bundle_path: String) -> PattoResult<()> {
    let path = Path::new(&ca_bundle_path);
    if !path.is_file() {
        return Err(PattoError::NotFound(ca_bundle_path));
    }
    let pem = std::fs::read(path)?;

    if CERT_FILE.set(ca_bundle_path.clone()).is_err() {
        return Ok(());
    }

    let certs = openssl::x509::X509::stack_from_pem(&pem).map_err(|e| PattoError::Git {
        kind: GitErrorKind::Certificate,
        message: format!("could not parse the CA bundle: {e}"),
    })?;

    // libgit2 has to be initialised before its options are set; opening a
    // throwaway repository path is the cheapest way to force that.
    let _ = Repository::open(".");

    let mut added = 0usize;
    for cert in &certs {
        // Safety: libgit2 takes its own reference to the certificate.
        let code = unsafe {
            libgit2_sys::git_libgit2_opts(
                libgit2_sys::GIT_OPT_ADD_SSL_X509_CERT as libc::c_int,
                cert.as_ptr(),
            )
        };
        if code >= 0 {
            added += 1;
        }
    }

    if added == 0 {
        return Err(PattoError::Git {
            kind: GitErrorKind::Certificate,
            message: "libgit2 accepted none of the bundled trust roots".to_string(),
        });
    }

    log::info!("loaded {added} of {} trust roots", certs.len());
    Ok(())
}
