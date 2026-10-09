use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::PattoWorkspaceConfig;

/// Where a wiki link sits in its source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkLocation {
    /// Source line number (0-indexed)
    pub source_line: usize,
    /// Source column range (byte offsets within the line)
    pub source_col_range: (usize, usize),
    /// Target anchor name (if linking to specific anchor)
    pub target_anchor: Option<String>,
}

/// Edge of the document graph: every link from one document to another.
#[derive(Debug, Clone)]
pub struct LinkEdge {
    pub locations: Vec<LinkLocation>,
}

/// `LinkLocation` as the preview receives it.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LinkLocationData {
    /// Line number (0-indexed)
    pub line: usize,
    /// Column range within the line
    pub col_range: (usize, usize),
    /// Optional: text context around the link
    pub context: Option<String>,
    /// Target anchor (if linking to specific anchor)
    pub target_anchor: Option<String>,
}

/// One document linking to the file in question, with every place it does so.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackLinkData {
    /// Source file name (link name, not full path)
    pub source_file: String,
    pub locations: Vec<LinkLocationData>,
}

/// File metadata for sorting and display
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileMetadata {
    pub modified: u64, // Unix timestamp
    pub created: u64,  // Unix timestamp
    #[serde(rename = "linkCount")]
    pub link_count: u32,
}

/// Repository change notifications. Every path is absolute.
#[derive(Clone, Debug)]
pub enum RepositoryMessage {
    FileChanged(PathBuf, FileMetadata, String),
    FileAdded(PathBuf, FileMetadata),
    FileRemoved(PathBuf),
    BackLinksChanged(PathBuf, Vec<BackLinkData>),
    TwoHopLinksChanged(PathBuf, Vec<(String, Vec<String>)>),
    WorkspaceConfigChanged(PattoWorkspaceConfig),
    ScanStarted { total_files: usize },
    ScanProgress { scanned: usize, total: usize },
    ScanCompleted { total_files: usize },
}
