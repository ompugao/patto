use serde::{Deserialize, Serialize};

/// Markdown export flavor - determines all rendering options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum MarkdownFlavor {
    /// CommonMark-compatible output
    Standard,
    /// Obsidian-native format with [[wikilinks]], ^anchors, emoji tasks
    Obsidian,
    /// GitHub-flavored markdown (GFM)
    GitHub,
}

impl std::fmt::Display for MarkdownFlavor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarkdownFlavor::Standard => write!(f, "standard"),
            MarkdownFlavor::Obsidian => write!(f, "obsidian"),
            MarkdownFlavor::GitHub => write!(f, "github"),
        }
    }
}

/// WikiLink format in output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikiLinkFormat {
    /// [[note]] style (Obsidian native)
    WikiStyle,
    /// [note](note.md) style (standard markdown)
    Markdown,
}

/// Task format in output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskFormat {
    /// Standard checkbox: - [ ] task (due: 2024-12-31)
    Checkbox,
    /// Obsidian Tasks plugin: - [ ] task 📅 2024-12-31
    ObsidianEmoji,
    /// Obsidian Dataview: - [ ] task [due:: 2024-12-31]
    ObsidianDataview,
}

/// Anchor format in output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorFormat {
    /// HTML comment: <!-- anchor: name -->
    HtmlComment,
    /// HTML anchor: <a id="name"></a>
    HtmlAnchor,
    /// Obsidian block reference: ^name
    ObsidianBlock,
    /// Keep as inline text: #name
    Inline,
}
