// `ratatui-image` rejects both chafa backends at once, so guard the combination
// here where the error names our own features. This also means `--all-features`
// cannot be used on this crate; pick one backend explicitly.
#[cfg(all(
    feature = "preview-tui-chafa-dyn",
    feature = "preview-tui-chafa-static"
))]
compile_error!(
    "features `preview-tui-chafa-dyn` and `preview-tui-chafa-static` are mutually exclusive"
);

pub mod ast_query;
pub mod cli;
pub mod importer;
pub mod line_tracker;
#[cfg(feature = "lsp")]
pub mod lsp;
pub mod markdown;
pub mod parser;
// The bridge only needs tower-lsp and the repository, not the web server.
#[cfg(feature = "lsp")]
pub mod preview;
pub mod renderer;
#[cfg(feature = "repository")]
pub mod repository;
pub mod task;
pub mod task_edits;
pub mod tasks_view;
pub mod utils;
