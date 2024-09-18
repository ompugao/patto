use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;

use clap::{Parser as ClapParser, Subcommand};

#[derive(ClapParser)]
#[command(version, about, long_about=None)]
struct Cli {
    /// an input file to parse
    #[arg(short, long, value_name = "FILE")]
    file: PathBuf,
    /// an output html file
    #[arg(short, long, value_name = "OUTPUT")]
    output: PathBuf,
}

use tabton::parser;
use tabton::renderer;
use tabton::renderer::Renderer;

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let args = Cli::parse();
    // TODO memory inefficient
    let text = fs::read_to_string(args.file).expect("cannot read {filename}");
    let rootnode = parser::parse_text(&text);

    let mut writer = BufWriter::new(fs::File::create(args.output).unwrap());
    let renderer = renderer::HtmlRenderer::new(renderer::Options::default());
    renderer.format(&rootnode, &mut writer)?;
    Ok(())
}
