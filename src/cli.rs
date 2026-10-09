//! Helpers shared by the `patto-*` binaries.
//!
//! Reading input is pure std, so it stays in the lean build: `patto-syntax-checker`
//! needs no features. Logging pulls in `simplelog` and rides on the `cli` feature.

use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// Install the logger. With no `logfile` nothing is logged: the LSP and the
/// renderers speak a protocol on stdout/stderr, so logging must be opt-in.
#[cfg(feature = "cli")]
pub fn init_logger(
    filter_level: log::LevelFilter,
    logfile: Option<std::path::PathBuf>,
) -> io::Result<()> {
    let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = Vec::new();
    if let Some(filename) = logfile {
        loggers.push(simplelog::WriteLogger::new(
            filter_level,
            simplelog::Config::default(),
            fs::File::create(filename)?,
        ));
    }
    simplelog::CombinedLogger::init(loggers).map_err(io::Error::other)
}

/// Read `path`, or stdin when no path is given.
pub fn read_input(path: Option<&Path>) -> io::Result<String> {
    match path {
        Some(path) => fs::read_to_string(path),
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            Ok(buffer)
        }
    }
}

/// Display name for an input source, for messages addressed to the user.
pub fn input_name(path: Option<&Path>) -> String {
    path.map(|p| p.display().to_string())
        .unwrap_or_else(|| "stdin".to_string())
}
