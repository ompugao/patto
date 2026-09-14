/// Why a git operation failed. The app maps these to advice the user can act on
/// (re-enter the token, check the network, resolve on desktop).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitErrorKind {
    Auth,
    Network,
    Certificate,
    NotARepo,
    NoRemote,
    NonFastForward,
    Conflict,
    Other,
}

#[derive(Debug, thiserror::Error)]
pub enum PattoError {
    #[error("io error: {0}")]
    Io(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("invalid note name: {0}")]
    InvalidName(String),
    #[error("no task on line {row} of {path}")]
    NoTaskAtRow { path: String, row: u32 },
    #[error("index for {0} has not been built yet")]
    IndexNotBuilt(String),
    #[error("git error ({kind:?}): {message}")]
    Git { kind: GitErrorKind, message: String },
}

pub type PattoResult<T> = Result<T, PattoError>;

impl From<std::io::Error> for PattoError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => PattoError::NotFound(e.to_string()),
            _ => PattoError::Io(e.to_string()),
        }
    }
}

impl From<git2::Error> for PattoError {
    fn from(e: git2::Error) -> Self {
        use git2::{ErrorClass, ErrorCode};

        let kind = match (e.code(), e.class()) {
            (ErrorCode::Auth, _) => GitErrorKind::Auth,
            (_, ErrorClass::Http) if e.message().contains("401") => GitErrorKind::Auth,
            (_, ErrorClass::Ssl) => GitErrorKind::Certificate,
            (_, ErrorClass::Net) | (_, ErrorClass::Http) => GitErrorKind::Network,
            (ErrorCode::NotFound, ErrorClass::Repository) => GitErrorKind::NotARepo,
            (ErrorCode::NotFastForward, _) => GitErrorKind::NonFastForward,
            (ErrorCode::Conflict, _) | (ErrorCode::Unmerged, _) => GitErrorKind::Conflict,
            _ => GitErrorKind::Other,
        };

        PattoError::Git {
            kind,
            message: e.message().to_string(),
        }
    }
}
