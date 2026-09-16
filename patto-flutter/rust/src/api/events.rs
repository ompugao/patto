//! Outcomes for the long-running, streaming operations.
//!
//! flutter_rust_bridge turns a function taking a `StreamSink` into a Dart
//! `Stream` and discards the function's own `Result`: the error surfaces on a
//! future nobody awaits, so Dart can never catch it. Everything these functions
//! report therefore travels through the sink, ending in exactly one terminal
//! event.

use crate::api::error::PattoError;
use crate::api::git::{GitProgress, SyncReport};
use crate::api::index::{IndexProgress, IndexStats};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub message: String,
    /// Set when the failure came from git, so the app can advise the user.
    pub git_kind: Option<crate::api::error::GitErrorKind>,
}

impl From<PattoError> for Failure {
    fn from(e: PattoError) -> Self {
        let git_kind = match &e {
            PattoError::Git { kind, .. } => Some(*kind),
            _ => None,
        };
        Failure {
            message: e.to_string(),
            git_kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneEvent {
    Progress { progress: GitProgress },
    Done,
    Failed { failure: Failure },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEvent {
    Progress { progress: GitProgress },
    Done { report: SyncReport },
    Failed { failure: Failure },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexEvent {
    Progress { progress: IndexProgress },
    Done { stats: IndexStats },
    Failed { failure: Failure },
}
