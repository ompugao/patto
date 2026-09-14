//! The surface flutter_rust_bridge turns into Dart.
//!
//! Cheap, pure calls are `#[frb(sync)]` so Dart gets a plain return value.
//! Anything that touches the filesystem, parses a whole note or talks to a
//! remote is left asynchronous, so it runs on the Rust worker pool instead of
//! the UI isolate.

use flutter_rust_bridge::frb;

use crate::frb_generated::StreamSink;

pub use crate::api::error::{GitErrorKind, PattoError};
pub use crate::api::git::{GitCreds, GitPhase, GitProgress, GitStatus, MergeOutcome, SyncReport};
pub use crate::api::index::{BackLink, IndexProgress, IndexStats, LinkCount, TwoHop};
pub use crate::api::tasks::{PendingGroup, TaskEditResult, TaskItem};
pub use crate::api::types::{
    AnchorRef, Block, BlockKind, DateKind, EmbedKind, ImageRef, NoteSpan, NoteMeta, ParseIssue,
    RenderedNote, NoteTableCell, NoteTableRow, TaskDate, TaskInfo, TaskStatus,
};

use crate::api::error::PattoResult;
use crate::api::{git, index, render, store, tasks};

/// Called once at startup, before anything else.
#[frb(init)]
pub fn init_app() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    flutter_rust_bridge::setup_default_user_utils();
}

// ─── notes ───────────────────────────────────────────────────────────────────

pub fn list_notes(root: String) -> PattoResult<Vec<NoteMeta>> {
    store::list_notes(root)
}

pub fn read_note(root: String, rel_path: String) -> PattoResult<String> {
    store::read_note(root, rel_path)
}

pub fn write_note(root: String, rel_path: String, content: String) -> PattoResult<()> {
    store::write_note(root.clone(), rel_path.clone(), content)?;
    index::index_update_file(root, rel_path)
}

pub fn create_note(root: String, name: String, initial_content: String) -> PattoResult<NoteMeta> {
    let meta = store::create_note(root.clone(), name, initial_content)?;
    index::index_update_file(root, meta.rel_path.clone())?;
    Ok(meta)
}

pub fn delete_note(root: String, rel_path: String) -> PattoResult<()> {
    store::delete_note(root.clone(), rel_path.clone())?;
    index::index_update_file(root, rel_path)
}

pub fn search_notes(root: String, query: String, limit: u32) -> PattoResult<Vec<NoteMeta>> {
    store::search_notes(root, query, limit)
}

#[frb(sync)]
pub fn resolve_wiki_link(root: String, name: String) -> PattoResult<Option<String>> {
    store::resolve_wiki_link(root, name)
}

#[frb(sync)]
pub fn note_name_to_rel_path(name: String) -> PattoResult<String> {
    store::name_to_rel_path(&name)
}

#[frb(sync)]
pub fn rel_path_to_note_name(rel_path: String) -> String {
    store::rel_path_to_name(&rel_path)
}

// ─── rendering ───────────────────────────────────────────────────────────────

pub fn render_note(content: String) -> RenderedNote {
    render::render_note(content)
}

// ─── index ───────────────────────────────────────────────────────────────────

/// Scan the notes directory and build the link index, streaming progress.
pub fn index_build(root: String, sink: StreamSink<IndexProgress>) -> PattoResult<IndexStats> {
    index::index_build(root, |p| {
        let _ = sink.add(p);
    })
}

pub fn index_update_file(root: String, rel_path: String) -> PattoResult<()> {
    index::index_update_file(root, rel_path)
}

/// Re-parse only the notes that changed on disk; used after a git sync.
pub fn index_refresh(root: String) -> PattoResult<IndexStats> {
    index::index_refresh(root)
}

pub fn backlinks(root: String, rel_path: String) -> PattoResult<Vec<BackLink>> {
    index::backlinks(root, rel_path)
}

pub fn two_hop_links(root: String, rel_path: String) -> PattoResult<Vec<TwoHop>> {
    index::two_hop_links(root, rel_path)
}

pub fn link_counts(root: String) -> PattoResult<Vec<LinkCount>> {
    index::link_counts(root)
}

#[frb(sync)]
pub fn note_exists(root: String, name: String) -> bool {
    index::note_exists(root, name)
}

// ─── tasks ───────────────────────────────────────────────────────────────────

pub fn pending_tasks(root: String) -> PattoResult<Vec<TaskItem>> {
    tasks::pending_tasks(root, None)
}

/// `timeframe` is one of `today`, `yesterday`, `this_week`, `last_week`,
/// `this_month`, `custom`; `from` / `to` are `YYYY-MM-DD` and only read for
/// `custom`.
pub fn completed_tasks(
    root: String,
    timeframe: String,
    from: Option<String>,
    to: Option<String>,
) -> PattoResult<Vec<TaskItem>> {
    tasks::completed_tasks(root, timeframe, from, to, None)
}

pub fn set_task_status(
    root: String,
    rel_path: String,
    row: u32,
    status: TaskStatus,
) -> PattoResult<TaskEditResult> {
    tasks::set_task_status(root, rel_path, row, status)
}

// ─── git ─────────────────────────────────────────────────────────────────────

/// Must be called before any other git function; see [`git::git_init_runtime`].
pub fn git_init_runtime(ca_bundle_path: String) -> PattoResult<()> {
    git::git_init_runtime(ca_bundle_path)
}

pub fn git_clone(
    url: String,
    root: String,
    branch: Option<String>,
    creds: GitCreds,
    sink: StreamSink<GitProgress>,
) -> PattoResult<()> {
    git::git_clone(url, root, branch, creds, move |p| {
        let _ = sink.add(p);
    })
}

pub fn git_status(root: String) -> PattoResult<GitStatus> {
    git::git_status(root)
}

pub fn git_sync(
    root: String,
    author_name: String,
    author_email: String,
    creds: GitCreds,
    sink: StreamSink<GitProgress>,
) -> PattoResult<SyncReport> {
    let report = git::git_sync(root.clone(), author_name, author_email, creds, move |p| {
        let _ = sink.add(p);
    })?;
    index::index_refresh(root)?;
    Ok(report)
}
