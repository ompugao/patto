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

    #[arg(short, long, value_name = "BOOL")]
    use_hard_line_break: bool,

    /// an output html file
    #[arg(short, long, value_name = "OUTPUT")]
    output: PathBuf,
}

use patto::parser;
use patto::renderer;
use patto::renderer::Renderer;

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let args = Cli::parse();
    // TODO memory inefficient
    let text = fs::read_to_string(args.file).expect("cannot read {filename}");
    let parser::ParserResult { ast: rootnode, parse_errors: _ } = parser::parse_text(&text);

    let mut writer = BufWriter::new(fs::File::create(args.output).unwrap());
    let options = renderer::MarkdownRendererOptions {
        use_hard_line_break: args.use_hard_line_break,
    };
    let renderer = renderer::MarkdownRenderer::new(options);
    renderer.format(&rootnode, &mut writer)?;
    Ok(())
}


