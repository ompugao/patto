//! Shared pieces of the preview front-ends (`patto-preview`, `patto-preview-tui`).

pub mod lsp_bridge;

#[cfg(feature = "preview")]
mod embeds;
#[cfg(feature = "preview")]
pub mod server;
#[cfg(feature = "preview")]
pub mod session;
#[cfg(feature = "preview")]
mod static_files;
#[cfg(feature = "preview")]
mod user_files;
