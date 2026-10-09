use std::io::Write;
use std::path::PathBuf;

use clap::{Parser as ClapParser, ValueEnum};

use patto::cli::{input_name, open_output, read_input};
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

    let flavor = match args.flavor {
        FlavorArg::Standard => MarkdownFlavor::Standard,
        FlavorArg::Obsidian => MarkdownFlavor::Obsidian,
        FlavorArg::Github => MarkdownFlavor::GitHub,
    };

    let mut options = MarkdownRendererOptions::new(flavor);

    if args.no_frontmatter {
        options = options.with_frontmatter(false);
    }

    let text = read_input(args.file.as_deref())?;

    let parser::ParserResult {
        ast: rootnode,
        parse_errors,
    } = parser::parse_text(&text);

    if !parse_errors.is_empty() {
        eprintln!("Warning: {} parse error(s) found", parse_errors.len());
        for error in parse_errors.iter().take(5) {
            eprintln!("  {}", error);
        }
    }

    let renderer = MarkdownRenderer::new(options);
    let mut writer = open_output(args.output.as_deref())?;
    renderer.format(&rootnode, &mut writer)?;
    writer.flush()?;

    if let Some(path) = &args.output {
        eprintln!(
            "✓ Exported {} to {} (flavor: {})",
            input_name(args.file.as_deref()),
            path.display(),
            flavor
        );
    }

    Ok(())
}
