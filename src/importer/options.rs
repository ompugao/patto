//! Import options and types for markdown to patto conversion

use serde::{Deserialize, Serialize};

/// Import mode determines how unsupported features are handled
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ImportMode {
    /// Stop at first unsupported feature with error
    #[default]
    Strict,
    /// Continue on errors, drop unsupported features with warnings
    Lossy,
    /// Wrap unsupported markdown in code blocks for manual editing
    Preserve,
}

impl std::fmt::Display for ImportMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportMode::Strict => write!(f, "strict"),
            ImportMode::Lossy => write!(f, "lossy"),
            ImportMode::Preserve => write!(f, "preserve"),
        }
    }
}

/// Detected or specified input markdown flavor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MarkdownInputFlavor {
    /// Standard CommonMark
    #[default]
    Standard,
    /// Obsidian-style markdown (WikiLinks, Dataview, Tasks plugin)
    Obsidian,
    /// GitHub-flavored markdown
    GitHub,
}

impl std::fmt::Display for MarkdownInputFlavor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarkdownInputFlavor::Standard => write!(f, "standard"),
            MarkdownInputFlavor::Obsidian => write!(f, "obsidian"),
            MarkdownInputFlavor::GitHub => write!(f, "github"),
        }
    }
}

/// Options for markdown import
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Import mode (strict, lossy, preserve)
    pub mode: ImportMode,
    /// Input markdown flavor (None = auto-detect)
    pub flavor: Option<MarkdownInputFlavor>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            mode: ImportMode::Strict,
            flavor: None,
        }
    }
}

impl ImportOptions {
    /// Create new import options with specified mode
    pub fn new(mode: ImportMode) -> Self {
        Self {
            mode,
            flavor: None,
        }
    }

    /// Set the input flavor
    pub fn with_flavor(mut self, flavor: MarkdownInputFlavor) -> Self {
        self.flavor = Some(flavor);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_mode_display() {
        assert_eq!(ImportMode::Strict.to_string(), "strict");
        assert_eq!(ImportMode::Lossy.to_string(), "lossy");
        assert_eq!(ImportMode::Preserve.to_string(), "preserve");
    }

    #[test]
    fn test_flavor_display() {
        assert_eq!(MarkdownInputFlavor::Standard.to_string(), "standard");
        assert_eq!(MarkdownInputFlavor::Obsidian.to_string(), "obsidian");
        assert_eq!(MarkdownInputFlavor::GitHub.to_string(), "github");
    }

    #[test]
    fn test_default_options() {
        let opts = ImportOptions::default();
        assert_eq!(opts.mode, ImportMode::Strict);
        assert_eq!(opts.flavor, None);
    }

    #[test]
    fn test_options_builder() {
        let opts = ImportOptions::new(ImportMode::Lossy).with_flavor(MarkdownInputFlavor::Obsidian);
        assert_eq!(opts.mode, ImportMode::Lossy);
        assert_eq!(opts.flavor, Some(MarkdownInputFlavor::Obsidian));
    }
}
