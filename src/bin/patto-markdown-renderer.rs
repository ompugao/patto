use std::fs;
use std::io::{self, BufWriter, Read, Write};
use std::path::PathBuf;

use clap::{Parser as ClapParser, ValueEnum};

use patto::markdown::{MarkdownFlavor, MarkdownRendererOptions};
use patto::parser;
use patto::renderer::{MarkdownRenderer, Renderer};

#[derive(ValueEnum, Clone, Debug)]
enum FlavorArg {
    /// CommonMark-compatible output
    Standard,
    /// Obsidian-native format with [[wikilinks]], ^anchors, emoji tasks
    Obsidian,
    /// GitHub-flavored markdown (GFM)
    Github,
}

#[derive(ClapParser)]
#[command(
    version,
    about = "Convert patto notes to markdown",
    long_about = "Exports patto notes to various markdown flavors (Standard, Obsidian, GitHub).\n\n\
                  If no input file is specified, reads from stdin.\n\
                  If no output file is specified, writes to stdout."
)]
struct Cli {
    /// Input patto file (reads from stdin if not specified)
    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,

    /// Output markdown file (writes to stdout if not specified)
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Markdown flavor (determines all format options)
    #[arg(short = 'F', long, value_enum, default_value = "standard")]
    flavor: FlavorArg,

    /// Disable frontmatter (only affects Obsidian flavor)
    #[arg(long)]
    no_frontmatter: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    // Convert flavor enum
    let flavor = match args.flavor {
        FlavorArg::Standard => MarkdownFlavor::Standard,
        FlavorArg::Obsidian => MarkdownFlavor::Obsidian,
        FlavorArg::Github => MarkdownFlavor::GitHub,
    };

    // Build options from flavor
    let mut options = MarkdownRendererOptions::new(flavor);

    // Allow frontmatter override
    if args.no_frontmatter {
        options = options.with_frontmatter(false);
    }

    // Read input (from file or stdin)
    let text = match &args.file {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    let parser::ParserResult {
        ast: rootnode,
        parse_errors,
    } = parser::parse_text(&text);

    // Warn about parse errors but continue (to stderr)
    if !parse_errors.is_empty() {
        eprintln!("Warning: {} parse error(s) found", parse_errors.len());
        for error in parse_errors.iter().take(5) {
            eprintln!("  {}", error);
        }
    }

    // Render to output (file or stdout)
    let renderer = MarkdownRenderer::new(options);

    match &args.output {
        Some(path) => {
            let mut writer = BufWriter::new(fs::File::create(path)?);
            renderer.format(&rootnode, &mut writer)?;
            writer.flush()?;

            let input_name = args
                .file
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "stdin".to_string());
            eprintln!(
                "✓ Exported {} to {} (flavor: {})",
                input_name,
                path.display(),
                flavor
            );
        }
        None => {
            let stdout = io::stdout();
            let mut writer = BufWriter::new(stdout.lock());
            renderer.format(&rootnode, &mut writer)?;
            writer.flush()?;
        }
    }

    Ok(())
}
