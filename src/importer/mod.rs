//! Markdown to Patto importer module
//!
//! This module provides functionality to convert markdown files to patto format.
//! It supports three import modes:
//! - Strict: Stop on first unsupported feature
//! - Lossy: Continue on errors, drop unsupported features
//! - Preserve: Wrap unsupported features in code blocks

mod converter;
mod options;
mod report;

pub use converter::MarkdownImporter;
pub use options::{ImportMode, ImportOptions, MarkdownInputFlavor};
pub use report::{ConversionReport, ConversionStatistics, ImportWarning, WarningKind};
