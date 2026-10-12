//! Markdown to Patto converter
//!
//! Converts markdown content to patto format using pulldown-cmark for parsing.
//! Builds patto's AST directly for consistency with the native parser.

use std::time::Instant;

use pulldown_cmark::{Options, Parser};
use regex::Regex;

use super::conversion::Conversion;
use super::options::{ImportOptions, MarkdownInputFlavor};
use super::report::ConversionReport;
use crate::parser::AstNode;
use crate::renderer::{PattoRenderer, Renderer};

/// Error type for import operations
#[derive(Debug, Clone)]
pub struct ImportError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ImportError {}

/// Result of markdown import
#[derive(Debug)]
pub struct ImportResult {
    /// Converted patto AST (root node)
    pub ast: AstNode,
    /// Converted patto content as string (for convenience)
    pub patto_content: String,
    /// Conversion report
    pub report: ConversionReport,
}

/// Markdown to Patto importer
pub struct MarkdownImporter {
    options: ImportOptions,
}

impl MarkdownImporter {
    /// Create a new importer with the given options
    pub fn new(options: ImportOptions) -> Self {
        Self { options }
    }

    /// Detect the markdown flavor from content
    pub fn detect_flavor(content: &str) -> MarkdownInputFlavor {
        let obsidian_wikilink = Regex::new(r"\[\[[^\]]+\]\]").unwrap();
        let obsidian_block_ref = Regex::new(r"\s\^[a-zA-Z0-9-]+$").unwrap();
        let obsidian_dataview = Regex::new(r"\[due::\s*\d{4}-\d{2}-\d{2}\]").unwrap();
        let obsidian_task_emoji = Regex::new(r"📅\s*\d{4}-\d{2}-\d{2}").unwrap();

        if obsidian_wikilink.is_match(content)
            || obsidian_block_ref.is_match(content)
            || obsidian_dataview.is_match(content)
            || obsidian_task_emoji.is_match(content)
        {
            return MarkdownInputFlavor::Obsidian;
        }

        let github_mention = Regex::new(r"@[a-zA-Z0-9_-]+").unwrap();
        let github_issue_ref = Regex::new(r"#\d+").unwrap();
        if github_mention.is_match(content) || github_issue_ref.is_match(content) {
            return MarkdownInputFlavor::GitHub;
        }

        MarkdownInputFlavor::Standard
    }

    /// Import markdown content to patto format
    pub fn import(
        &self,
        markdown: &str,
        input_path: &str,
        output_path: &str,
    ) -> Result<ImportResult, ImportError> {
        let start_time = Instant::now();

        let flavor = self
            .options
            .flavor
            .unwrap_or_else(|| Self::detect_flavor(markdown));

        let mut report = ConversionReport::new(input_path, output_path, self.options.mode, flavor);
        report.statistics.total_lines = markdown.lines().count();

        let ast = self.convert_to_ast(markdown, &mut report)?;
        let patto_content = render_patto(&ast)?;

        report.statistics.converted_lines =
            report.statistics.total_lines - report.statistics.failed_lines;
        report.duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(ImportResult {
            ast,
            patto_content,
            report,
        })
    }

    fn convert_to_ast(
        &self,
        markdown: &str,
        report: &mut ConversionReport,
    ) -> Result<AstNode, ImportError> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_FOOTNOTES);

        let mut conversion = Conversion::new(&self.options, report);
        for event in Parser::new_ext(markdown, options) {
            conversion.handle(event)?;
        }
        Ok(conversion.finish())
    }
}

fn render_patto(ast: &AstNode) -> Result<String, ImportError> {
    let mut patto_content = Vec::new();
    PattoRenderer::new()
        .format(ast, &mut patto_content)
        .map_err(|e| ImportError {
            line: 0,
            message: format!("Failed to render AST: {}", e),
        })?;
    String::from_utf8(patto_content).map_err(|e| ImportError {
        line: 0,
        message: format!("Invalid UTF-8 in output: {}", e),
    })
}
