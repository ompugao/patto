use std::fmt;
use std::ops;
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use super::Deadline;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Span(pub usize, pub usize);

impl ops::Add<usize> for Span {
    type Output = Self;

    fn add(self, offset: usize) -> Self {
        Span(self.0 + offset, self.1 + offset)
    }
}

impl Span {
    pub fn contains(&self, col: usize) -> bool {
        self.0 <= col && col < self.1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Location {
    pub row: usize,
    #[serde(serialize_with = "serialize_arc_str")]
    pub input: Arc<str>,
    pub span: Span,
}

fn serialize_arc_str<S>(string: &Arc<str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(string)
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.input)?;
        write!(
            f,
            "{}",
            self.input[..self.span.0]
                .chars()
                .map(|c| {
                    if c != '\t' {
                        ' '
                    } else {
                        c
                    }
                })
                .collect::<String>()
        )?;
        write!(f, "{:^<1$}", "", self.span.1 - self.span.0)
    }
}

impl From<pest::Span<'_>> for Span {
    fn from(from: pest::Span<'_>) -> Span {
        Self(from.start(), from.end())
    }
}

impl From<pest::error::InputLocation> for Span {
    fn from(from: pest::error::InputLocation) -> Span {
        match from {
            pest::error::InputLocation::Pos(pos) => Span(pos, pos + 1),
            pest::error::InputLocation::Span(span) => Span(span.0, span.1),
        }
    }
}

impl Location {
    fn as_str(&self) -> &str {
        &self.input[self.span.0..self.span.1]
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Annotation<T> {
    pub value: T,
    pub location: Location,
}

#[derive(Debug, Default, Serialize)]
pub struct AstNodeInternal {
    #[serde(serialize_with = "serialize_mutex_vec")]
    contents: Mutex<Vec<AstNode>>,
    #[serde(serialize_with = "serialize_mutex_vec")]
    children: Mutex<Vec<AstNode>>,
    kind: AstNodeKind,
    #[serde(serialize_with = "serialize_mutex_opt_i64")]
    stable_id: Mutex<Option<i64>>,
}

fn serialize_mutex_vec<S, T: Serialize>(
    mutex: &Mutex<Vec<T>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let vec = mutex.lock().unwrap();
    vec.serialize(serializer)
}

fn serialize_mutex_opt_i64<S>(mutex: &Mutex<Option<i64>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let opt = mutex.lock().unwrap();
    opt.serialize(serializer)
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
pub enum TaskStatus {
    #[default]
    Todo,
    Doing,
    Paused,
    Done,
}

impl TaskStatus {
    /// `inprogress` and `wip` are read as aliases of `doing` but never written back.
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword {
            "todo" => Some(TaskStatus::Todo),
            "doing" | "inprogress" | "wip" => Some(TaskStatus::Doing),
            "paused" => Some(TaskStatus::Paused),
            "done" => Some(TaskStatus::Done),
            _ => None,
        }
    }

    pub fn keyword(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::Doing => "doing",
            TaskStatus::Paused => "paused",
            TaskStatus::Done => "done",
        }
    }
}

#[derive(Debug, Serialize)]
pub enum Property {
    Task {
        status: TaskStatus,
        /// `false` if the status value was unrecognised (partial edit); transitions
        /// involving a non-canonical status are skipped by the diff logic.
        status_is_canonical: bool,
        due: Deadline,
        scheduled: Option<Deadline>,
        completed_at: Option<Deadline>,
        /// Timestamp of the most recent clock-in (set when transitioning to Doing).
        started_at: Option<Deadline>,
        /// Accumulated time spent across all completed sessions.
        time_spent: Option<crate::task::Duration>,
        location: Location,
    },
    Anchor {
        name: String,
        location: Location,
    },
}

#[derive(Debug, Default, Serialize)]
#[serde(tag = "type")]
pub enum AstNodeKind {
    Line {
        properties: Vec<Property>,
    },
    Quote,
    QuoteContent {
        properties: Vec<Property>,
    },
    Math {
        inline: bool,
    },
    MathContent,
    Code {
        lang: String,
        inline: bool,
    },
    CodeContent,
    Table {
        caption: Option<String>,
    },
    TableRow,
    TableColumn,
    Image {
        src: String,
        alt: Option<String>,
    },
    WikiLink {
        link: String,
        anchor: Option<String>,
    },
    Link {
        link: String,
        title: Option<String>,
    },
    Embed {
        link: String,
        title: Option<String>,
    },

    Decoration {
        fontsize: isize,
        italic: bool,
        underline: bool,
        deleted: bool,
    },

    Text,
    HorizontalLine,
    #[default]
    Dummy,
}

type AstNodeImpl = Annotation<AstNodeInternal>;
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct AstNode(Arc<Annotation<AstNodeInternal>>);

impl AstNode {
    pub fn new(input: &str, row: usize, span: Option<Span>, kind: Option<AstNodeKind>) -> Self {
        AstNode(Arc::new(AstNodeImpl {
            value: AstNodeInternal {
                contents: Mutex::new(vec![]),
                children: Mutex::new(vec![]),
                kind: kind.unwrap_or(AstNodeKind::Dummy),
                stable_id: Mutex::new(None),
            },
            location: Location {
                row,
                input: Arc::from(input),
                span: span.unwrap_or(Span(0, input.len())),
            },
        }))
    }

    pub fn line(input: &str, row: usize, span: Option<Span>, props: Option<Vec<Property>>) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Line {
                properties: props.unwrap_or_default(),
            }),
        )
    }
    pub fn code(input: &str, row: usize, span: Option<Span>, lang: &str, inline: bool) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Code {
                lang: lang.to_string(),
                inline,
            }),
        )
    }
    pub fn codecontent(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::CodeContent {}))
    }
    pub fn math(input: &str, row: usize, span: Option<Span>, inline: bool) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Math { inline }))
    }
    pub fn mathcontent(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::MathContent {}))
    }
    pub fn quote(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Quote))
    }
    pub fn quotecontent(
        input: &str,
        row: usize,
        span: Option<Span>,
        props: Option<Vec<Property>>,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::QuoteContent {
                properties: props.unwrap_or_default(),
            }),
        )
    }
    pub fn wikilink(
        input: &str,
        row: usize,
        span: Option<Span>,
        link: &str,
        anchor: Option<&str>,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::WikiLink {
                link: link.to_string(),
                anchor: anchor.map(str::to_string),
            }),
        )
    }
    pub fn link(
        input: &str,
        row: usize,
        span: Option<Span>,
        link: &str,
        title: Option<&str>,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Link {
                link: link.to_string(),
                title: title.map(str::to_string),
            }),
        )
    }
    pub fn text(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Text))
    }
    pub fn horizontal_line(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::HorizontalLine))
    }
    pub fn image(
        input: &str,
        row: usize,
        span: Option<Span>,
        src: &str,
        alt: Option<&str>,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Image {
                src: src.to_string(),
                alt: alt.map(str::to_string),
            }),
        )
    }
    pub fn decoration(
        input: &str,
        row: usize,
        span: Option<Span>,
        fontsize: isize,
        italic: bool,
        underline: bool,
        deleted: bool,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Decoration {
                fontsize,
                italic,
                underline,
                deleted,
            }),
        )
    }
    pub fn table(input: &str, row: usize, span: Option<Span>, caption: Option<&str>) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Table {
                caption: caption.map(ToOwned::to_owned),
            }),
        )
    }
    pub fn tablerow(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::TableRow))
    }
    pub fn tablecolumn(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::TableColumn))
    }

    pub fn embed(
        input: &str,
        row: usize,
        span: Option<Span>,
        link: &str,
        title: Option<&str>,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Embed {
                link: link.to_string(),
                title: title.map(str::to_string),
            }),
        )
    }

    pub fn kind(&self) -> &AstNodeKind {
        &self.0.value.kind
    }

    /// Inline contents of this node (text, links, decorations, ...).
    pub fn contents(&self) -> MutexGuard<'_, Vec<AstNode>> {
        self.0.value.contents.lock().unwrap()
    }

    /// Nested block children of this node (indented lines, block body, ...).
    pub fn children(&self) -> MutexGuard<'_, Vec<AstNode>> {
        self.0.value.children.lock().unwrap()
    }

    pub fn stable_id(&self) -> Option<i64> {
        *self.0.value.stable_id.lock().unwrap()
    }

    pub fn set_stable_id(&self, stable_id: i64) {
        *self.0.value.stable_id.lock().unwrap() = Some(stable_id);
    }

    pub fn add_content(&self, content: AstNode) {
        self.contents().push(content);
    }
    pub fn add_contents(&self, contents: Vec<AstNode>) {
        self.contents().extend(contents);
    }
    pub fn add_child(&self, child: AstNode) {
        self.children().push(child);
    }
    pub fn location(&self) -> &Location {
        &self.0.location
    }
    pub fn extract_str(&self) -> &str {
        self.location().as_str()
    }
}

impl Clone for AstNode {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl fmt::Display for AstNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "extracted: {}", self.extract_str())?;
        for content in self.contents().iter() {
            write!(f, "-- {}", content)?;
        }
        if let AstNodeKind::Line { properties } = &self.kind() {
            for prop in properties {
                writeln!(f, "property -- {:?}", prop)?;
            }
        }
        for (i, child) in self.children().iter().enumerate() {
            writeln!(f, "\t{i}child -- {:?}", child)?;
        }
        Ok(())
    }
}
