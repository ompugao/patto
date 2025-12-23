//! Conversion report types for markdown import

use super::options::{ImportMode, MarkdownInputFlavor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of warning during import
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningKind {
    /// Feature not supported by patto
    UnsupportedFeature,
    /// Conversion resulted in information loss
    LossyConversion,
    /// Content preserved in code block
    PreservedContent,
    /// Ambiguous format detected
    AmbiguousFormat,
}

impl std::fmt::Display for WarningKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WarningKind::UnsupportedFeature => write!(f, "unsupported_feature"),
            WarningKind::LossyConversion => write!(f, "lossy_conversion"),
            WarningKind::PreservedContent => write!(f, "preserved_content"),
            WarningKind::AmbiguousFormat => write!(f, "ambiguous_format"),
        }
    }
}

/// A warning generated during import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportWarning {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed, optional)
    pub column: Option<usize>,
    /// Type of warning
    pub kind: WarningKind,
    /// Feature that caused the warning
    pub feature: String,
    /// Human-readable message
    pub message: String,
    /// Suggestion for fixing (optional)
    pub suggestion: Option<String>,
}

impl std::fmt::Display for ImportWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(col) = self.column {
            write!(f, "Line {}, col {}: {}", self.line, col, self.message)
        } else {
            write!(f, "Line {}: {}", self.line, self.message)
        }
    }
}

/// Statistics about the conversion
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversionStatistics {
    /// Total lines in input
    pub total_lines: usize,
    /// Successfully converted lines
    pub converted_lines: usize,
    /// Lines that failed to convert (strict mode)
    pub failed_lines: usize,
    /// Number of warnings
    pub warning_count: usize,
    /// Number of errors
    pub error_count: usize,
    /// Count of each feature type converted
    pub feature_counts: HashMap<String, usize>,
    /// Count of unsupported features encountered
    pub unsupported_features: HashMap<String, usize>,
}

impl ConversionStatistics {
    /// Increment the count for a feature type
    pub fn increment_feature(&mut self, feature: &str) {
        *self.feature_counts.entry(feature.to_string()).or_insert(0) += 1;
    }

    /// Increment the count for an unsupported feature
    pub fn increment_unsupported(&mut self, feature: &str) {
        *self
            .unsupported_features
            .entry(feature.to_string())
            .or_insert(0) += 1;
    }
}

/// Complete conversion report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionReport {
    /// Input file path
    pub input_file: String,
    /// Output file path
    pub output_file: String,
    /// Import mode used
    pub mode: ImportMode,
    /// Detected or specified flavor
    pub flavor: MarkdownInputFlavor,
    /// Timestamp of conversion
    pub timestamp: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Conversion statistics
    pub statistics: ConversionStatistics,
    /// All warnings generated
    pub warnings: Vec<ImportWarning>,
}

impl ConversionReport {
    /// Create a new empty report
    pub fn new(input: &str, output: &str, mode: ImportMode, flavor: MarkdownInputFlavor) -> Self {
        Self {
            input_file: input.to_string(),
            output_file: output.to_string(),
            mode,
            flavor,
            timestamp: chrono::Utc::now().to_rfc3339(),
            duration_ms: 0,
            statistics: ConversionStatistics::default(),
            warnings: Vec::new(),
        }
    }

    /// Add a warning to the report
    pub fn add_warning(&mut self, warning: ImportWarning) {
        self.statistics.warning_count += 1;
        self.warnings.push(warning);
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Convert to human-readable text format
    pub fn to_text(&self) -> String {
        let mut output = String::new();

        output.push_str("Markdown Import Report\n");
        output.push_str("======================\n");
        output.push_str(&format!("Input:  {}\n", self.input_file));
        output.push_str(&format!("Output: {}\n", self.output_file));
        output.push_str(&format!("Mode:   {}\n", self.mode));
        output.push_str(&format!("Flavor: {}\n", self.flavor));
        output.push_str(&format!("Date:   {}\n", self.timestamp));
        output.push_str(&format!("Time:   {}ms\n\n", self.duration_ms));

        output.push_str("Statistics\n");
        output.push_str("----------\n");
        output.push_str(&format!(
            "Total lines:     {}\n",
            self.statistics.total_lines
        ));
        output.push_str(&format!(
            "Converted:       {} ({}%)\n",
            self.statistics.converted_lines,
            if self.statistics.total_lines > 0 {
                self.statistics.converted_lines * 100 / self.statistics.total_lines
            } else {
                100
            }
        ));
        output.push_str(&format!(
            "Failed:          {}\n",
            self.statistics.failed_lines
        ));
        output.push_str(&format!(
            "Warnings:        {}\n",
            self.statistics.warning_count
        ));
        output.push_str(&format!(
            "Errors:          {}\n\n",
            self.statistics.error_count
        ));

        if !self.statistics.feature_counts.is_empty() {
            output.push_str("Conversions\n");
            output.push_str("-----------\n");
            let mut features: Vec<_> = self.statistics.feature_counts.iter().collect();
            features.sort_by(|a, b| b.1.cmp(a.1));
            for (feature, count) in features {
                output.push_str(&format!("✓ {}: {}\n", feature, count));
            }
            output.push('\n');
        }

        if !self.warnings.is_empty() {
            output.push_str("Warnings\n");
            output.push_str("--------\n");
            for warning in &self.warnings {
                output.push_str(&format!("⚠ {}\n", warning));
                if let Some(suggestion) = &warning.suggestion {
                    output.push_str(&format!("  Suggestion: {}\n", suggestion));
                }
            }
            output.push('\n');
        }

        if !self.statistics.unsupported_features.is_empty() {
            output.push_str("Unsupported Features\n");
            output.push_str("--------------------\n");
            for (feature, count) in &self.statistics.unsupported_features {
                output.push_str(&format!("{}: {} dropped\n", feature, count));
            }
            output.push('\n');
        }

        output.push_str("Result\n");
        output.push_str("------\n");
        if self.statistics.error_count > 0 {
            output.push_str("✗ Conversion failed\n");
        } else if self.statistics.warning_count > 0 {
            output.push_str("✓ Conversion completed with warnings\n");
            output.push_str(&format!("✓ Output written to {}\n", self.output_file));
            output.push_str("ℹ Review warnings and manually edit if needed\n");
        } else {
            output.push_str("✓ Conversion completed successfully\n");
            output.push_str(&format!("✓ Output written to {}\n", self.output_file));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warning_display() {
        let warning = ImportWarning {
            line: 10,
            column: Some(5),
            kind: WarningKind::UnsupportedFeature,
            feature: "footnote".to_string(),
            message: "Dropped footnote [^1]".to_string(),
            suggestion: None,
        };
        assert_eq!(warning.to_string(), "Line 10, col 5: Dropped footnote [^1]");
    }

    #[test]
    fn test_warning_display_no_column() {
        let warning = ImportWarning {
            line: 15,
            column: None,
            kind: WarningKind::LossyConversion,
            feature: "heading".to_string(),
            message: "Converted heading to plain text".to_string(),
            suggestion: Some("Use [* text] for emphasis".to_string()),
        };
        assert_eq!(
            warning.to_string(),
            "Line 15: Converted heading to plain text"
        );
    }

    #[test]
    fn test_statistics_increment() {
        let mut stats = ConversionStatistics::default();
        stats.increment_feature("lists");
        stats.increment_feature("lists");
        stats.increment_feature("code_blocks");
        stats.increment_unsupported("footnotes");

        assert_eq!(stats.feature_counts.get("lists"), Some(&2));
        assert_eq!(stats.feature_counts.get("code_blocks"), Some(&1));
        assert_eq!(stats.unsupported_features.get("footnotes"), Some(&1));
    }

    #[test]
    fn test_report_to_json() {
        let report = ConversionReport::new(
            "input.md",
            "output.pn",
            ImportMode::Lossy,
            MarkdownInputFlavor::Standard,
        );
        let json = report.to_json().unwrap();
        assert!(json.contains("\"input_file\": \"input.md\""));
        assert!(json.contains("\"mode\": \"Lossy\""));
    }

    #[test]
    fn test_report_to_text() {
        let mut report = ConversionReport::new(
            "input.md",
            "output.pn",
            ImportMode::Lossy,
            MarkdownInputFlavor::Standard,
        );
        report.statistics.total_lines = 100;
        report.statistics.converted_lines = 100;
        report.statistics.increment_feature("lists");

        let text = report.to_text();
        assert!(text.contains("Markdown Import Report"));
        assert!(text.contains("Input:  input.md"));
        assert!(text.contains("Converted:       100 (100%)"));
        assert!(text.contains("✓ lists: 1"));
    }
}
