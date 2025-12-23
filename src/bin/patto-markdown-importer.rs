//! patto-markdown-importer - Convert markdown files to patto format
//!
//! Usage:
//!   patto-markdown-importer -f input.md -o output.pn
//!   patto-markdown-importer -f input.md -o output.pn --mode lossy
//!   patto-markdown-importer -d ./notes -o ./patto-notes --mode lossy
//!   cat input.md | patto-markdown-importer > output.pn

use std::fs;
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{Parser as ClapParser, ValueEnum};

use patto::importer::{
    ConversionReport, ImportMode, ImportOptions, MarkdownImporter, MarkdownInputFlavor,
};

#[derive(ValueEnum, Clone, Debug)]
enum ModeArg {
    /// Stop at first unsupported feature
    Strict,
    /// Continue on errors, drop unsupported features
    Lossy,
    /// Wrap unsupported features in code blocks
    Preserve,
}

#[derive(ValueEnum, Clone, Debug)]
enum FlavorArg {
    /// Standard CommonMark
    Standard,
    /// Obsidian-style markdown
    Obsidian,
    /// GitHub-flavored markdown
    Github,
}

#[derive(ValueEnum, Clone, Debug)]
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    // Convert mode enum
    let mode = match args.mode {
        ModeArg::Strict => ImportMode::Strict,
        ModeArg::Lossy => ImportMode::Lossy,
        ModeArg::Preserve => ImportMode::Preserve,
    };

    // Convert flavor enum (clone to avoid partial move)
    let flavor = args.flavor.clone().map(|f| match f {
        FlavorArg::Standard => MarkdownInputFlavor::Standard,
        FlavorArg::Obsidian => MarkdownInputFlavor::Obsidian,
        FlavorArg::Github => MarkdownInputFlavor::GitHub,
    });

    // Build options
    let mut options = ImportOptions::new(mode);
    if let Some(f) = flavor {
        options = options.with_flavor(f);
    }

    let importer = MarkdownImporter::new(options);

    // Handle batch conversion
    if let Some(ref dir) = args.directory {
        return batch_convert(&importer, dir, &args);
    }

    // Single file conversion
    let (input_content, input_name) = match &args.file {
        Some(path) => (fs::read_to_string(path)?, path.display().to_string()),
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            (buffer, "stdin".to_string())
        }
    };

    let output_name = args
        .output
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "stdout".to_string());

    // Import
    let result = importer.import(&input_content, &input_name, &output_name)?;

    // Show warnings if verbose
    if args.verbose || !result.report.warnings.is_empty() {
        for warning in &result.report.warnings {
            eprintln!("⚠ {}", warning);
        }
    }

    // Dry run - just show report
    if args.dry_run {
        eprintln!("\n{}", result.report.to_text());
        return Ok(());
    }

    // Write output
    match &args.output {
        Some(path) => {
            let mut writer = BufWriter::new(fs::File::create(path)?);
            writer.write_all(result.patto_content.as_bytes())?;
            writer.flush()?;

            eprintln!(
                "✓ Converted {} to {} (mode: {}, flavor: {})",
                input_name,
                path.display(),
                result.report.mode,
                result.report.flavor
            );

            if result.report.warnings.is_empty() {
                eprintln!(
                    "✓ {} lines converted successfully",
                    result.report.statistics.converted_lines
                );
            } else {
                eprintln!(
                    "✓ {} lines converted with {} warning(s)",
                    result.report.statistics.converted_lines,
                    result.report.warnings.len()
                );
            }
        }
        None => {
            let stdout = io::stdout();
            let mut writer = BufWriter::new(stdout.lock());
            writer.write_all(result.patto_content.as_bytes())?;
            writer.flush()?;
        }
    }

    // Write report if requested
    if let Some(report_path) = args.report {
        write_report(&result.report, &report_path, &args.report_format)?;
        eprintln!("✓ Report written to {}", report_path.display());
    }

    Ok(())
}

fn batch_convert(
    importer: &MarkdownImporter,
    dir: &Path,
    args: &Cli,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = args
        .output
        .as_ref()
        .ok_or("Output directory required for batch conversion")?;

    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    let start_time = Instant::now();
    let mut total_files = 0;
    let mut succeeded = 0;
    let mut failed = 0;
    let mut total_warnings = 0;
    let mut all_reports = Vec::new();

    // Find all matching files
    let pattern = format!("{}/{}", dir.display(), args.pattern);
    let entries: Vec<_> = glob::glob(&pattern)
        .map_err(|e| format!("Invalid pattern: {}", e))?
        .filter_map(|e| e.ok())
        .collect();

    for entry in entries {
        total_files += 1;

        let input_path = entry.clone();
        let relative = entry
            .strip_prefix(dir)
            .unwrap_or(&entry)
            .with_extension("pn");
        let output_path = output_dir.join(relative);

        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        if args.verbose {
            eprintln!("Converting {} -> {}", input_path.display(), output_path.display());
        }

        let input_content = match fs::read_to_string(&input_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("✗ Failed to read {}: {}", input_path.display(), e);
                failed += 1;
                continue;
            }
        };

        let result = match importer.import(
            &input_content,
            &input_path.display().to_string(),
            &output_path.display().to_string(),
        ) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("✗ Failed to convert {}: {}", input_path.display(), e);
                failed += 1;
                continue;
            }
        };

        total_warnings += result.report.warnings.len();
        all_reports.push(result.report.clone());

        if !args.dry_run {
            if let Err(e) = fs::write(&output_path, &result.patto_content) {
                eprintln!("✗ Failed to write {}: {}", output_path.display(), e);
                failed += 1;
                continue;
            }
        }

        succeeded += 1;

        if args.verbose && !result.report.warnings.is_empty() {
            for warning in &result.report.warnings {
                eprintln!("  ⚠ {}", warning);
            }
        }
    }

    let duration = start_time.elapsed();

    eprintln!("\nBatch Conversion Summary");
    eprintln!("========================");
    eprintln!("Files processed: {}", total_files);
    eprintln!("Succeeded:       {}", succeeded);
    eprintln!("Failed:          {}", failed);
    eprintln!("Total warnings:  {}", total_warnings);
    eprintln!("Duration:        {:?}", duration);

    if args.dry_run {
        eprintln!("\n(Dry run - no files were written)");
    }

    // Write batch report if requested
    if let Some(report_path) = &args.report {
        let batch_report = create_batch_report(
            dir,
            output_dir,
            &all_reports,
            duration.as_millis() as u64,
        );
        
        let report_content = match args.report_format {
            ReportFormat::Json => serde_json::to_string_pretty(&batch_report)?,
            ReportFormat::Text => format_batch_report_text(&batch_report),
        };
        
        fs::write(report_path, report_content)?;
        eprintln!("✓ Report written to {}", report_path.display());
    }

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn write_report(
    report: &ConversionReport,
    path: &Path,
    format: &ReportFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = match format {
        ReportFormat::Json => report.to_json()?,
        ReportFormat::Text => report.to_text(),
    };
    fs::write(path, content)?;
    Ok(())
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

fn create_batch_report(
    input_dir: &Path,
    output_dir: &Path,
    reports: &[ConversionReport],
    duration_ms: u64,
) -> BatchReport {
    let files: Vec<FileReport> = reports
        .iter()
        .map(|r| FileReport {
            input: r.input_file.clone(),
            output: r.output_file.clone(),
            status: if r.warnings.is_empty() {
                "success".to_string()
            } else {
                "success_with_warnings".to_string()
            },
            warnings: r.warnings.len(),
            duration_ms: r.duration_ms,
        })
        .collect();

    let total_warnings: usize = reports.iter().map(|r| r.warnings.len()).sum();

    BatchReport {
        input_directory: input_dir.display().to_string(),
        output_directory: output_dir.display().to_string(),
        files_processed: reports.len(),
        files_succeeded: reports.len(),
        files_failed: 0,
        total_warnings,
        duration_ms,
        files,
    }
}

fn format_batch_report_text(report: &BatchReport) -> String {
    let mut output = String::new();
    
    output.push_str("Batch Conversion Report\n");
    output.push_str("=======================\n");
    output.push_str(&format!("Input directory:  {}\n", report.input_directory));
    output.push_str(&format!("Output directory: {}\n", report.output_directory));
    output.push_str(&format!("Duration:         {}ms\n\n", report.duration_ms));
    
    output.push_str("Summary\n");
    output.push_str("-------\n");
    output.push_str(&format!("Files processed:  {}\n", report.files_processed));
    output.push_str(&format!("Succeeded:        {}\n", report.files_succeeded));
    output.push_str(&format!("Failed:           {}\n", report.files_failed));
    output.push_str(&format!("Total warnings:   {}\n\n", report.total_warnings));
    
    output.push_str("Files\n");
    output.push_str("-----\n");
    for file in &report.files {
        let status_icon = if file.status == "success" { "✓" } else { "⚠" };
        output.push_str(&format!(
            "{} {} -> {} ({} warnings, {}ms)\n",
            status_icon, file.input, file.output, file.warnings, file.duration_ms
        ));
    }
    
    output
}
