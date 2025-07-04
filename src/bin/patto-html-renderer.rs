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

    /// an input file to parse
    #[arg(short, long, value_name = "FILE")]
    debuglogfile: Option<PathBuf>,
    /// an output html file
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,
}

use patto::parser;
use patto::renderer;
use patto::renderer::Renderer;

use log;
use std::fs::File;
use clap_verbosity_flag::{Verbosity, InfoLevel};

fn init_logger(filter_level: log::LevelFilter, logfile: Option<PathBuf>) {
    let mut loggers = Vec::new();
    if let Some(filename) = logfile {
        loggers.push(
            simplelog::WriteLogger::new(
                filter_level,
                simplelog::Config::default(),
                File::create(filename).unwrap(),
            ) as Box<dyn simplelog::SharedLogger>)
    }
    simplelog::CombinedLogger::init(loggers).unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let args = Cli::parse();
    // TODO memory inefficient
    let text = fs::read_to_string(&args.file).expect("cannot read {filename}");
    let parser::ParserResult { ast: rootnode, parse_errors: _ } = parser::parse_text(&text);

    let mut writer = BufWriter::new(fs::File::create(args.output).unwrap());
    let options = renderer::HtmlRendererOptions {
        ..renderer::HtmlRendererOptions::default()
    };

    init_logger(args.verbose.log_level_filter(), args.debuglogfile);
    let renderer = renderer::HtmlRenderer::new(options);

    let str_s = format!(r#"
<html>
<head>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/sakura.css/css/sakura.css" type="text/css" media="screen">
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/sakura.css/css/sakura-vader.css" type="text/css" media="screen and (prefers-color-scheme: {})">
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github{}.min.css" type="text/css" type="text/css">
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<script>hljs.highlightAll();</script>
<script id="MathJax-script" async src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
</head>
<body style="max-width: max-content">
<script type="module">
import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';
mermaid.initialize({{ startOnLoad: true, theme: 'forest' }});
</script>
<section style="width: 1920px; max-width: 100%;">
<article>"#, args.theme, if args.theme == "dark" {"-dark"} else {""});
    write!(writer, "{}", str_s)?;
    renderer.format(&rootnode, &mut writer)?;
    write!(writer, "</article>\n")?;
    write!(writer, "</section>\n")?;
    write!(writer, "</body>\n")?;
    write!(writer, "</html>\n")?;
    Ok(())
}
