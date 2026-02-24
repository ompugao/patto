use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use clap::Parser as ClapParser;

#[derive(ClapParser)]
#[command(version, about, long_about=None)]
struct Cli {
    /// an input file to parse
    #[arg(short, long, value_name = "FILE")]
    file: PathBuf,
    /// an output html file
    #[arg(short, long, value_name = "OUTPUT")]
    output: PathBuf,
    /// theme, light or dark
    #[arg(short, long, value_name = "THEME")]
    theme: String,

    /// debug log file
    #[arg(short, long, value_name = "FILE")]
    debuglogfile: Option<PathBuf>,
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,
}

use patto::parser;
use patto::renderer;
use patto::renderer::Renderer;

use clap_verbosity_flag::{InfoLevel, Verbosity};
use std::fs::File;

const PATTO_CSS: &str = include_str!("../../assets/patto-html.css");

fn init_logger(filter_level: log::LevelFilter, logfile: Option<PathBuf>) {
    let mut loggers = Vec::new();
    if let Some(filename) = logfile {
        loggers.push(simplelog::WriteLogger::new(
            filter_level,
            simplelog::Config::default(),
            File::create(filename).unwrap(),
        ) as Box<dyn simplelog::SharedLogger>)
    }
    simplelog::CombinedLogger::init(loggers).unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    init_logger(args.verbose.log_level_filter(), args.debuglogfile);

    let text = fs::read_to_string(&args.file).expect("cannot read input file");
    let parser::ParserResult {
        ast: rootnode,
        parse_errors: _,
    } = parser::parse_text(&text);

    let options = renderer::HtmlRendererOptions {};
    let renderer = renderer::HtmlRenderer::new(options);

    let theme_class = match args.theme.as_str() {
        "dark" => "theme-dark",
        _ => "theme-light",
    };
    let hljs_theme = if args.theme == "dark" { "-dark" } else { "" };

    let mut writer = BufWriter::new(fs::File::create(&args.output)?);
    write!(
        writer,
        r#"<!DOCTYPE html>
<html lang="en" class="{theme_class}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>
{PATTO_CSS}
</style>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github{hljs_theme}.min.css">
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<script>hljs.highlightAll();</script>
<script id="MathJax-script" async src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
</head>
<body>
<script type="module">
import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';
mermaid.initialize({{ startOnLoad: true, theme: 'forest' }});
</script>
<div class="patto-container">
"#
    )?;
    renderer.format(&rootnode, &mut writer)?;
    writeln!(writer, "</div>")?;
    writeln!(writer, "</body>")?;
    writeln!(writer, "</html>")?;
    Ok(())
}
