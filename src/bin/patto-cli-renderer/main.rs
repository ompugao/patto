mod ansi_output;

use clap::Parser;
use patto::{parser, tui_renderer};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Render a patto (.pn) file as ANSI-colored text to stdout.\n\
             Designed to be used as a preview command for fzf, yazi, lf, etc."
)]
struct Args {
    /// Path to the .pn file to render
    file: String,

    /// Output width in columns (0 = no truncation).
    /// Defaults to $FZF_PREVIEW_COLUMNS, then $COLUMNS, then 80.
    #[arg(short, long, default_value_t = 0)]
    width: usize,

    /// Disable ANSI color codes (plain text output)
    #[arg(long)]
    no_color: bool,
}

fn main() {
    let mut args = Args::parse();

    // Auto-detect width from environment if not overridden
    if args.width == 0 {
        args.width = std::env::var("FZF_PREVIEW_COLUMNS")
            .ok()
            .and_then(|v| v.parse().ok())
            .or_else(|| {
                std::env::var("COLUMNS")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(80);
    }

    let path = PathBuf::from(&args.file);
    if !path.exists() || !path.is_file() {
        eprintln!("patto-cli-renderer: file not found: {}", args.file);
        std::process::exit(1);
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("patto-cli-renderer: cannot read {}: {}", args.file, e);
            std::process::exit(1);
        }
    };

    let result = parser::parse_text(&content);
    let doc = tui_renderer::render_ast(&result.ast);
    ansi_output::render_to_stdout(&doc, args.width, args.no_color);
}
