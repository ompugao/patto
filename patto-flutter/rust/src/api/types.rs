//! Data crossing the bridge to Dart.
//!
//! A note is delivered as a flat list of [`Block`]s so Flutter can build it with
//! a lazy list: a 10k-line note must not cost 10k widgets up front.

use patto::parser::{Deadline, TaskStatus as CoreTaskStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Todo,
    Doing,
    Paused,
    Done,
}

impl From<&CoreTaskStatus> for TaskStatus {
    fn from(s: &CoreTaskStatus) -> Self {
        match s {
            CoreTaskStatus::Todo => TaskStatus::Todo,
            CoreTaskStatus::Doing => TaskStatus::Doing,
            CoreTaskStatus::Paused => TaskStatus::Paused,
            CoreTaskStatus::Done => TaskStatus::Done,
        }
    }
}

impl From<TaskStatus> for CoreTaskStatus {
    fn from(s: TaskStatus) -> Self {
        match s {
            TaskStatus::Todo => CoreTaskStatus::Todo,
            TaskStatus::Doing => CoreTaskStatus::Doing,
            TaskStatus::Paused => CoreTaskStatus::Paused,
            TaskStatus::Done => CoreTaskStatus::Done,
        }
    }
}

/// How a date-ish task field was understood, so the UI knows whether to show a
/// time and whether the value is a real date at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateKind {
    Date,
    DateTime,
    Unparsed,
}

/// A task field rendered for display, e.g. `2026-12-31` / `2026-12-31T09:00`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDate {
    pub text: String,
    pub kind: DateKind,
}

impl TaskDate {
    pub fn from_deadline(d: &Deadline) -> Option<Self> {
        match d {
            Deadline::Date(date) => Some(TaskDate {
                text: date.format("%Y-%m-%d").to_string(),
                kind: DateKind::Date,
            }),
            Deadline::DateTime(dt) => Some(TaskDate {
                text: dt.format("%Y-%m-%dT%H:%M").to_string(),
                kind: DateKind::DateTime,
            }),
            // An empty `due=` is how the parser spells "no deadline".
            Deadline::Uninterpretable(s) if s.trim().is_empty() => None,
            Deadline::Uninterpretable(s) => Some(TaskDate {
                text: s.clone(),
                kind: DateKind::Unparsed,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInfo {
    pub status: TaskStatus,
    pub due: Option<TaskDate>,
    pub scheduled: Option<TaskDate>,
    pub completed_at: Option<TaskDate>,
    pub started_at: Option<TaskDate>,
    pub time_spent_minutes: Option<u32>,
    /// `true` for `!2026-12-31`, `false` for `{@task ...}`.
    pub is_shorthand: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    /// Remote URL, or a path relative to the notes root for local images.
    pub src: String,
    pub alt: Option<String>,
    pub is_local: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbedKind {
    Youtube { video_id: String },
    Twitter,
    SpeakerDeck,
    SlideShare,
    Pdf,
    Other,
}

/// A run of inline content inside a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteSpan {
    Text {
        text: String,
    },
    /// `[* bold]`, `[/ italic]`, `[_ underline]`, `[- deleted]`, nestable.
    Decoration {
        fontsize: i32,
        italic: bool,
        underline: bool,
        deleted: bool,
        children: Vec<NoteSpan>,
    },
    /// `[note]`, `[note#anchor]`, or `[#anchor]` (self link, `name` empty).
    WikiLink {
        name: String,
        anchor: Option<String>,
    },
    Url {
        url: String,
        title: Option<String>,
    },
    InlineCode {
        code: String,
    },
    InlineMath {
        tex: String,
    },
    Image {
        image: ImageRef,
    },
    Embed {
        url: String,
        title: Option<String>,
        kind: EmbedKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTableRow {
    pub cells: Vec<NoteTableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTableCell {
    pub spans: Vec<NoteSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Line {
        spans: Vec<NoteSpan>,
    },
    /// A line with no visible content; kept so vertical rhythm survives.
    Blank,
    Code {
        lang: String,
        lines: Vec<String>,
    },
    Math {
        tex: String,
    },
    Table {
        caption: Option<String>,
        rows: Vec<NoteTableRow>,
    },
    /// A line whose entire content is images.
    Images {
        images: Vec<ImageRef>,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// 0-based source line the block starts on.
    pub row: u32,
    /// Nesting level from tab indentation.
    pub depth: u32,
    /// 0 outside a quote, 1 for `[@quote]`, 2 for a quote inside a quote.
    pub quote_depth: u32,
    pub task: Option<TaskInfo>,
    pub anchors: Vec<String>,
    pub kind: BlockKind,
}

/// Where an anchor name lands in the flattened block list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorRef {
    pub name: String,
    pub block_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIssue {
    pub row: u32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedNote {
    pub blocks: Vec<Block>,
    pub anchors: Vec<AnchorRef>,
    pub errors: Vec<ParseIssue>,
}

/// A note as listed on the notes screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteMeta {
    /// Wiki-link name: path relative to the root, without the `.pn` extension.
    pub name: String,
    /// Path relative to the notes root, e.g. `dir/note.pn`.
    pub rel_path: String,
    pub modified_ms: i64,
    pub size_bytes: u64,
}
