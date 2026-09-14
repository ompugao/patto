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

pub mod cli;
pub mod importer;
pub mod line_tracker;
pub mod lsp;
pub mod markdown;
pub mod parser;
pub mod preview;
pub mod renderer;
pub mod repository;
pub mod task;
pub mod utils;
