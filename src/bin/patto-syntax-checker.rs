use std::env;
use std::path::PathBuf;
use std::process;

use patto::cli::read_input;
use patto::parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = env::args().nth(1).map(PathBuf::from);
    let content = read_input(file.as_deref())?;

    let result = parser::parse_text(&content);

    if result.parse_errors.is_empty() {
        eprintln!("\u{2713} Syntax is valid.");
        process::exit(0);
    }

    eprintln!(
        "\u{2717} Found {} syntax error(s):",
        result.parse_errors.len()
    );
    for (i, err) in result.parse_errors.iter().enumerate() {
        eprintln!("\nError {}: {}", i + 1, err);
    }
    process::exit(1);
}
