use std::fs;
use std::io::BufWriter;
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
}

use tabton::parser;
use tabton::renderer;
use tabton::renderer::Renderer;

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let args = Cli::parse();
    // TODO memory inefficient
    let text = fs::read_to_string(args.file).expect("cannot read {filename}");
    let parser::ParserResult { ast: rootnode, parse_errors: _ } = parser::parse_text(&text);

    let mut writer = BufWriter::new(fs::File::create(args.output).unwrap());
    let options = renderer::Options {
        theme: args.theme,
        ..renderer::Options::default()
    };
    let renderer = renderer::HtmlRenderer::new(options);
    renderer.format(&rootnode, &mut writer)?;
    Ok(())
}

