use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use patto::parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let content = if args.len() > 1 {
        // Read from file
        fs::read_to_string(&args[1])?
    } else {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };

    let result = parser::parse_text(&content);

    if result.parse_errors.is_empty() {
        eprintln!("✓ Syntax is valid.");
        process::exit(0);
    } else {
        eprintln!("✗ Found {} syntax error(s):", result.parse_errors.len());
        for (i, err) in result.parse_errors.iter().enumerate() {
            eprintln!("\nError {}: {}", i + 1, err);
        }
        process::exit(1);
    }
}
