//! patto-markdown-importer - Convert markdown files to patto format
//!
//! Usage:
//!   patto-markdown-importer -f input.md -o output.pn
//!   patto-markdown-importer -f input.md -o output.pn --mode lossy
//!   patto-markdown-importer -d ./notes -o ./patto-notes --mode lossy
//!   cat input.md | patto-markdown-importer > output.pn

use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::{Parser as ClapParser, ValueEnum};

use patto::cli::{input_name, open_output, output_name, read_input};
use patto::importer::{
    ConversionReport, ImportMode, ImportOptions, ImportWarning, MarkdownImporter,
    MarkdownInputFlavor,
};

#[derive(ValueEnum, Clone, Copy, Debug)]
enum ModeArg {
    /// Stop at first unsupported feature
    Strict,
    /// Continue on errors, drop unsupported features
    Lossy,
    /// Wrap unsupported features in code blocks
    Preserve,
}

impl From<ModeArg> for ImportMode {
    fn from(mode: ModeArg) -> Self {
        match mode {
            ModeArg::Strict => ImportMode::Strict,
            ModeArg::Lossy => ImportMode::Lossy,
            ModeArg::Preserve => ImportMode::Preserve,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum FlavorArg {
    /// Standard CommonMark
    Standard,
    /// Obsidian-style markdown
    Obsidian,
    /// GitHub-flavored markdown
    Github,
}

impl From<FlavorArg> for MarkdownInputFlavor {
    fn from(flavor: FlavorArg) -> Self {
        match flavor {
            FlavorArg::Standard => MarkdownInputFlavor::Standard,
            FlavorArg::Obsidian => MarkdownInputFlavor::Obsidian,
            FlavorArg::Github => MarkdownInputFlavor::GitHub,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum ReportFormat {
    /// JSON format
    Json,
    /// Human-readable text
    Text,
}

#[derive(ClapParser)]
#[command(
    version,
    about = "Convert markdown files to patto format",
    long_about = "Imports markdown files to patto format with three modes:\n\n\
                  - strict: Stop on first unsupported feature\n\
                  - lossy: Continue on errors, drop unsupported features\n\
                  - preserve: Wrap unsupported features in code blocks\n\n\
                  If no input file is specified, reads from stdin.\n\
                  If no output file is specified, writes to stdout."
)]
struct Cli {
    /// Input markdown file (reads from stdin if not specified)
    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,

    /// Output patto file (writes to stdout if not specified)
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Import mode
    #[arg(short, long, value_enum, default_value = "strict")]
    mode: ModeArg,

    /// Input markdown flavor (auto-detect if not specified)
    #[arg(long, value_enum)]
    flavor: Option<FlavorArg>,

    /// Batch convert directory
    #[arg(short, long, value_name = "DIR")]
    directory: Option<PathBuf>,

    /// File pattern for batch conversion
    #[arg(long, default_value = "*.md")]
    pattern: String,

    /// Generate conversion report
    #[arg(long, value_name = "REPORT_FILE")]
    report: Option<PathBuf>,

    /// Report format
    #[arg(long, value_enum, default_value = "json")]
    report_format: ReportFormat,

    /// Dry run (show what would be converted without writing)
    #[arg(long)]
    dry_run: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

type Failure = Box<dyn Error>;

fn main() -> Result<(), Failure> {
    let args = Cli::parse();

    let mut options = ImportOptions::new(args.mode.into());
    if let Some(flavor) = args.flavor {
        options = options.with_flavor(flavor.into());
    }
    let importer = MarkdownImporter::new(options);

    match args.directory.as_deref() {
        Some(dir) => convert_directory(&importer, dir, &args),
        None => convert_single(&importer, &args),
    }
}

fn convert_single(importer: &MarkdownImporter, args: &Cli) -> Result<(), Failure> {
    let input = read_input(args.file.as_deref())?;
    let input_name = input_name(args.file.as_deref());
    let output_name = output_name(args.output.as_deref());

    let result = importer.import(&input, &input_name, &output_name)?;
    print_warnings(&result.report.warnings, "");

    if args.dry_run {
        eprintln!("\n{}", result.report.to_text());
        return Ok(());
    }

    let mut writer = open_output(args.output.as_deref())?;
    writer.write_all(result.patto_content.as_bytes())?;
    writer.flush()?;
    if let Some(path) = &args.output {
        print_single_summary(&input_name, path, &result.report);
    }

    if let Some(report_path) = &args.report {
        let content = match args.report_format {
            ReportFormat::Json => result.report.to_json()?,
            ReportFormat::Text => result.report.to_text(),
        };
        write_report(report_path, &content)?;
    }
    Ok(())
}

fn print_warnings(warnings: &[ImportWarning], indent: &str) {
    for warning in warnings {
        eprintln!("{indent}⚠ {warning}");
    }
}

fn print_single_summary(input_name: &str, output: &Path, report: &ConversionReport) {
    eprintln!(
        "✓ Converted {} to {} (mode: {}, flavor: {})",
        input_name,
        output.display(),
        report.mode,
        report.flavor
    );
    let converted = report.statistics.converted_lines;
    if report.warnings.is_empty() {
        eprintln!("✓ {} lines converted successfully", converted);
    } else {
        eprintln!(
            "✓ {} lines converted with {} warning(s)",
            converted,
            report.warnings.len()
        );
    }
}

fn write_report(path: &Path, content: &str) -> Result<(), Failure> {
    fs::write(path, content)?;
    eprintln!("✓ Report written to {}", path.display());
    Ok(())
}

#[derive(Default)]
struct BatchOutcome {
    reports: Vec<ConversionReport>,
    failed: usize,
}

impl BatchOutcome {
    fn processed(&self) -> usize {
        self.reports.len() + self.failed
    }

    fn total_warnings(&self) -> usize {
        self.reports.iter().map(|r| r.warnings.len()).sum()
    }
}

fn convert_directory(importer: &MarkdownImporter, dir: &Path, args: &Cli) -> Result<(), Failure> {
    let output_dir = args
        .output
        .as_deref()
        .ok_or("Output directory required for batch conversion")?;
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    let start_time = Instant::now();
    let mut outcome = BatchOutcome::default();
    for input_path in matching_files(dir, &args.pattern)? {
        let relative = input_path.strip_prefix(dir).unwrap_or(&input_path);
        let output_path = output_dir.join(relative.with_extension("pn"));
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        match convert_file(importer, &input_path, &output_path, args) {
            Ok(report) => outcome.reports.push(report),
            Err(message) => {
                eprintln!("{message}");
                outcome.failed += 1;
            }
        }
    }
    let duration = start_time.elapsed();

    print_batch_summary(&outcome, duration, args.dry_run);

    if let Some(report_path) = &args.report {
        let report = BatchReport::new(dir, output_dir, &outcome, duration);
        let content = match args.report_format {
            ReportFormat::Json => serde_json::to_string_pretty(&report)?,
            ReportFormat::Text => report.to_text(),
        };
        write_report(report_path, &content)?;
    }

    if outcome.failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn matching_files(dir: &Path, pattern: &str) -> Result<Vec<PathBuf>, Failure> {
    let pattern = format!("{}/{}", dir.display(), pattern);
    let paths = glob::glob(&pattern).map_err(|e| format!("Invalid pattern: {}", e))?;
    Ok(paths.filter_map(Result::ok).collect())
}

/// Convert one file of a batch. The error is the line to show the user.
fn convert_file(
    importer: &MarkdownImporter,
    input_path: &Path,
    output_path: &Path,
    args: &Cli,
) -> Result<ConversionReport, String> {
    if args.verbose {
        eprintln!(
            "Converting {} -> {}",
            input_path.display(),
            output_path.display()
        );
    }

    let input = fs::read_to_string(input_path)
        .map_err(|e| format!("✗ Failed to read {}: {}", input_path.display(), e))?;
    let result = importer
        .import(
            &input,
            &input_path.display().to_string(),
            &output_path.display().to_string(),
        )
        .map_err(|e| format!("✗ Failed to convert {}: {}", input_path.display(), e))?;

    if !args.dry_run {
        fs::write(output_path, &result.patto_content)
            .map_err(|e| format!("✗ Failed to write {}: {}", output_path.display(), e))?;
    }

    if args.verbose {
        print_warnings(&result.report.warnings, "  ");
    }
    Ok(result.report)
}

fn print_batch_summary(outcome: &BatchOutcome, duration: Duration, dry_run: bool) {
    eprintln!("\nBatch Conversion Summary");
    eprintln!("========================");
    eprintln!("Files processed: {}", outcome.processed());
    eprintln!("Succeeded:       {}", outcome.reports.len());
    eprintln!("Failed:          {}", outcome.failed);
    eprintln!("Total warnings:  {}", outcome.total_warnings());
    eprintln!("Duration:        {:?}", duration);

    if dry_run {
        eprintln!("\n(Dry run - no files were written)");
    }
}

#[derive(serde::Serialize)]
struct BatchReport {
    input_directory: String,
    output_directory: String,
    files_processed: usize,
    files_succeeded: usize,
    files_failed: usize,
    total_warnings: usize,
    duration_ms: u64,
    files: Vec<FileReport>,
}

#[derive(serde::Serialize)]
struct FileReport {
    input: String,
    output: String,
    status: String,
    warnings: usize,
    duration_ms: u64,
}

impl BatchReport {
    fn new(
        input_dir: &Path,
        output_dir: &Path,
        outcome: &BatchOutcome,
        duration: Duration,
    ) -> Self {
        let files = outcome
            .reports
            .iter()
            .map(|report| FileReport {
                input: report.input_file.clone(),
                output: report.output_file.clone(),
                status: if report.warnings.is_empty() {
                    "success".to_string()
                } else {
                    "success_with_warnings".to_string()
                },
                warnings: report.warnings.len(),
                duration_ms: report.duration_ms,
            })
            .collect();

        Self {
            input_directory: input_dir.display().to_string(),
            output_directory: output_dir.display().to_string(),
            files_processed: outcome.processed(),
            files_succeeded: outcome.reports.len(),
            files_failed: outcome.failed,
            total_warnings: outcome.total_warnings(),
            duration_ms: duration.as_millis() as u64,
            files,
        }
    }

    fn to_text(&self) -> String {
        let mut output = String::new();

        output.push_str("Batch Conversion Report\n");
        output.push_str("=======================\n");
        output.push_str(&format!("Input directory:  {}\n", self.input_directory));
        output.push_str(&format!("Output directory: {}\n", self.output_directory));
        output.push_str(&format!("Duration:         {}ms\n\n", self.duration_ms));

        output.push_str("Summary\n");
        output.push_str("-------\n");
        output.push_str(&format!("Files processed:  {}\n", self.files_processed));
        output.push_str(&format!("Succeeded:        {}\n", self.files_succeeded));
        output.push_str(&format!("Failed:           {}\n", self.files_failed));
        output.push_str(&format!("Total warnings:   {}\n\n", self.total_warnings));

        output.push_str("Files\n");
        output.push_str("-----\n");
        for file in &self.files {
            let status_icon = if file.status == "success" {
                "✓"
            } else {
                "⚠"
            };
            output.push_str(&format!(
                "{} {} -> {} ({} warnings, {}ms)\n",
                status_icon, file.input, file.output, file.warnings, file.duration_ms
            ));
        }

        output
    }
}
