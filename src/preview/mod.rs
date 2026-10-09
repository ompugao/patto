//! Shared pieces of the preview front-ends (`patto-preview`, `patto-preview-tui`).

pub mod lsp_bridge;

#[cfg(feature = "preview")]
mod embeds;
#[cfg(feature = "preview")]
pub mod server;
#[cfg(feature = "preview")]
pub mod session;

/// Whether `file_path` resolves inside `root` once `..` and symlinks are
/// followed; the path a client sends is not trusted.
#[cfg(feature = "preview")]
pub(crate) fn within_root(root: &std::path::Path, file_path: &std::path::Path) -> bool {
    match (
        std::fs::canonicalize(root),
        std::fs::canonicalize(file_path),
    ) {
        (Ok(root), Ok(file)) => file.starts_with(root),
        _ => false,
    }
}
#[cfg(feature = "preview")]
mod static_files;
#[cfg(feature = "preview")]
mod user_files;
