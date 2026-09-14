//! An in-memory link and task index over the notes directory.
//!
//! The desktop `Repository` is not reused here: it starts a tokio task and a
//! filesystem watcher, and recomputes backlinks by walking the whole graph on
//! every query. On a phone the app is the only writer, so a plain map keyed by
//! note name is both smaller and faster. Only derived records are kept; ASTs are
//! dropped after parsing.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use parking_lot::{Mutex, RwLock};
use patto::ast_query;
use patto::parser::{self, AstNode, AstNodeKind, Deadline, Property, TaskStatus};

use crate::api::error::{PattoError, PattoResult};
use crate::api::store::{self, rel_path_to_name};

/// One wiki link found in a note.
#[derive(Debug, Clone)]
pub(crate) struct LinkRef {
    pub target: String,
    pub anchor: Option<String>,
    pub row: u32,
    /// The source line, trimmed, shown as context under a backlink.
    pub context: String,
}

/// A task line, kept so the tasks screen needs no re-parse.
#[derive(Debug, Clone)]
pub(crate) struct TaskRecord {
    pub row: u32,
    pub label: String,
    pub status: TaskStatus,
    pub due: Deadline,
    pub scheduled: Option<Deadline>,
    pub completed_at: Option<chrono::NaiveDate>,
    pub started_at: Option<Deadline>,
    pub time_spent_minutes: Option<u32>,
    pub is_shorthand: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct NoteRecord {
    pub name: String,
    pub rel_path: String,
    pub modified_ms: i64,
    pub size_bytes: u64,
    pub links: Vec<LinkRef>,
    pub tasks: Vec<TaskRecord>,
}

#[derive(Debug, Default)]
pub(crate) struct NoteIndex {
    /// Keyed by note name (relative path without `.pn`).
    pub notes: HashMap<String, NoteRecord>,
    /// target name -> source names that link to it.
    pub backlinks: HashMap<String, BTreeSet<String>>,
}

impl NoteIndex {
    fn unlink(&mut self, name: &str) {
        let Some(old) = self.notes.get(name) else {
            return;
        };
        for link in &old.links {
            if let Some(sources) = self.backlinks.get_mut(&link.target) {
                sources.remove(name);
                if sources.is_empty() {
                    self.backlinks.remove(&link.target);
                }
            }
        }
    }

    fn insert(&mut self, record: NoteRecord) {
        self.unlink(&record.name);
        for link in &record.links {
            self.backlinks
                .entry(link.target.clone())
                .or_default()
                .insert(record.name.clone());
        }
        self.notes.insert(record.name.clone(), record);
    }

    fn remove(&mut self, name: &str) {
        self.unlink(name);
        self.notes.remove(name);
    }
}

type SharedIndex = Arc<RwLock<NoteIndex>>;

/// Indexes live for the life of the process, keyed by notes root. A Dart hot
/// restart therefore does not force a rebuild.
fn registry() -> &'static Mutex<HashMap<PathBuf, SharedIndex>> {
    static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, SharedIndex>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key_for(root: &str) -> PathBuf {
    std::fs::canonicalize(root).unwrap_or_else(|_| PathBuf::from(root))
}

pub(crate) fn index_for(root: &str) -> SharedIndex {
    registry()
        .lock()
        .entry(key_for(root))
        .or_insert_with(|| Arc::new(RwLock::new(NoteIndex::default())))
        .clone()
}

/// Read access to a built index, erroring if `index_build` has not run.
pub(crate) fn with_index<T>(
    root: &str,
    f: impl FnOnce(&NoteIndex) -> PattoResult<T>,
) -> PattoResult<T> {
    let index = index_for(root);
    let guard = index.read();
    if guard.notes.is_empty() {
        return Err(PattoError::IndexNotBuilt(root.to_string()));
    }
    f(&guard)
}

fn record_for(root: &str, meta: &crate::api::types::NoteMeta) -> PattoResult<NoteRecord> {
    let content = store::read_note(root.to_string(), meta.rel_path.clone())?;
    Ok(record_from_content(meta, &content))
}

pub(crate) fn record_from_content(meta: &crate::api::types::NoteMeta, content: &str) -> NoteRecord {
    let ast = parser::parse_text(content).ast;

    let mut raw_links = Vec::new();
    ast_query::gather_wikilinks(&ast, &mut raw_links);
    let links = raw_links
        .into_iter()
        .filter(|(link, _, _)| !link.is_empty())
        .map(|(target, anchor, loc)| LinkRef {
            target,
            anchor,
            row: loc.row as u32,
            context: loc.input.trim().to_string(),
        })
        .collect();

    NoteRecord {
        name: meta.name.clone(),
        rel_path: meta.rel_path.clone(),
        modified_ms: meta.modified_ms,
        size_bytes: meta.size_bytes,
        links,
        tasks: collect_tasks(&ast),
    }
}

fn collect_tasks(ast: &AstNode) -> Vec<TaskRecord> {
    let mut out = Vec::new();
    ast_query::walk_lines(ast, &mut |line, _depth| {
        let AstNodeKind::Line { properties } = line.kind() else {
            return;
        };
        for prop in properties {
            let Property::Task {
                status,
                due,
                scheduled,
                completed_at,
                started_at,
                time_spent,
                location,
                ..
            } = prop
            else {
                continue;
            };

            out.push(TaskRecord {
                row: line.location().row as u32,
                label: ast_query::task_label(line),
                status: status.clone(),
                due: due.clone(),
                scheduled: scheduled.clone(),
                completed_at: completed_at.as_ref().and_then(|d| match d {
                    Deadline::Date(date) => Some(*date),
                    Deadline::DateTime(dt) => Some(dt.date()),
                    Deadline::Uninterpretable(_) => None,
                }),
                started_at: started_at.clone(),
                time_spent_minutes: time_spent.as_ref().map(|d| d.hours * 60 + d.minutes),
                is_shorthand: !crate::api::render::span_text(location)
                    .trim_start()
                    .starts_with("{@"),
            });
            break;
        }
    });
    out
}

/// Progress while scanning the notes directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexProgress {
    pub scanned: u32,
    pub total: u32,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStats {
    pub notes: u32,
    pub links: u32,
    pub tasks: u32,
}

fn stats(index: &NoteIndex) -> IndexStats {
    IndexStats {
        notes: index.notes.len() as u32,
        links: index.notes.values().map(|n| n.links.len() as u32).sum(),
        tasks: index.notes.values().map(|n| n.tasks.len() as u32).sum(),
    }
}

/// Rebuild the whole index, reporting progress as it goes.
pub fn index_build(root: String, on_progress: impl Fn(IndexProgress)) -> PattoResult<IndexStats> {
    let metas = store::list_notes(root.clone())?;
    let total = metas.len() as u32;

    let mut fresh = NoteIndex::default();
    on_progress(IndexProgress {
        scanned: 0,
        total,
        done: false,
    });

    for (i, meta) in metas.iter().enumerate() {
        if let Ok(record) = record_for(&root, meta) {
            fresh.insert(record);
        }
        let scanned = i as u32 + 1;
        if scanned.is_multiple_of(20) || scanned == total {
            on_progress(IndexProgress {
                scanned,
                total,
                done: false,
            });
        }
    }

    let result = stats(&fresh);
    *index_for(&root).write() = fresh;
    on_progress(IndexProgress {
        scanned: total,
        total,
        done: true,
    });
    Ok(result)
}

/// Re-index one note, or drop it if the file is gone.
pub fn index_update_file(root: String, rel_path: String) -> PattoResult<()> {
    let path = store::resolve(&root, &rel_path)?;
    let name = rel_path_to_name(&rel_path);
    let index = index_for(&root);

    if !path.is_file() {
        index.write().remove(&name);
        return Ok(());
    }

    let metas = store::list_notes(root.clone())?;
    let Some(meta) = metas.into_iter().find(|m| m.rel_path == rel_path) else {
        index.write().remove(&name);
        return Ok(());
    };

    let record = record_for(&root, &meta)?;
    index.write().insert(record);
    Ok(())
}

/// Re-parse only the notes whose size or mtime changed, and drop deleted ones.
/// Used after a git sync, where a full rebuild would be wasteful.
pub fn index_refresh(root: String) -> PattoResult<IndexStats> {
    let metas = store::list_notes(root.clone())?;
    let index = index_for(&root);

    let stale: Vec<_> = {
        let guard = index.read();
        metas
            .iter()
            .filter(|m| match guard.notes.get(&m.name) {
                Some(old) => old.modified_ms != m.modified_ms || old.size_bytes != m.size_bytes,
                None => true,
            })
            .cloned()
            .collect()
    };

    let refreshed: Vec<NoteRecord> = stale
        .iter()
        .filter_map(|m| record_for(&root, m).ok())
        .collect();

    let live: BTreeSet<String> = metas.iter().map(|m| m.name.clone()).collect();
    let mut guard = index.write();
    let gone: Vec<String> = guard
        .notes
        .keys()
        .filter(|n| !live.contains(*n))
        .cloned()
        .collect();
    for name in gone {
        guard.remove(&name);
    }
    for record in refreshed {
        guard.insert(record);
    }

    Ok(stats(&guard))
}

/// A link pointing at the current note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackLink {
    pub source_name: String,
    pub source_rel_path: String,
    pub row: u32,
    pub context: String,
    pub target_anchor: Option<String>,
}

pub fn backlinks(root: String, rel_path: String) -> PattoResult<Vec<BackLink>> {
    let target = rel_path_to_name(&rel_path);
    with_index(&root, |index| {
        let mut out = Vec::new();
        if let Some(sources) = index.backlinks.get(&target) {
            for source_name in sources {
                let Some(source) = index.notes.get(source_name) else {
                    continue;
                };
                for link in source.links.iter().filter(|l| l.target == target) {
                    out.push(BackLink {
                        source_name: source.name.clone(),
                        source_rel_path: source.rel_path.clone(),
                        row: link.row,
                        context: link.context.clone(),
                        target_anchor: link.anchor.clone(),
                    });
                }
            }
        }
        out.sort_by(|a, b| a.source_name.cmp(&b.source_name).then(a.row.cmp(&b.row)));
        Ok(out)
    })
}

/// Notes reachable in two hops: for each note this one links to, the other notes
/// that also link there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoHop {
    pub via_name: String,
    pub names: Vec<String>,
}

pub fn two_hop_links(root: String, rel_path: String) -> PattoResult<Vec<TwoHop>> {
    let source = rel_path_to_name(&rel_path);
    with_index(&root, |index| {
        let Some(note) = index.notes.get(&source) else {
            return Ok(Vec::new());
        };

        let mut seen_targets = BTreeSet::new();
        let mut out: Vec<TwoHop> = Vec::new();

        for link in &note.links {
            if !seen_targets.insert(link.target.clone()) {
                continue;
            }
            let Some(sources) = index.backlinks.get(&link.target) else {
                continue;
            };
            let names: Vec<String> = sources
                .iter()
                .filter(|n| *n != &source && *n != &link.target)
                .cloned()
                .collect();
            if !names.is_empty() {
                out.push(TwoHop {
                    via_name: link.target.clone(),
                    names,
                });
            }
        }

        out.sort_by(|a, b| {
            b.names
                .len()
                .cmp(&a.names.len())
                .then(a.via_name.cmp(&b.via_name))
        });
        Ok(out)
    })
}

/// How many notes link to each note, for the "most linked" sort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkCount {
    pub name: String,
    pub rel_path: String,
    pub backlinks: u32,
    pub outgoing: u32,
}

pub fn link_counts(root: String) -> PattoResult<Vec<LinkCount>> {
    with_index(&root, |index| {
        let mut out: Vec<LinkCount> = index
            .notes
            .values()
            .map(|n| LinkCount {
                name: n.name.clone(),
                rel_path: n.rel_path.clone(),
                backlinks: index
                    .backlinks
                    .get(&n.name)
                    .map(|s| s.len() as u32)
                    .unwrap_or(0),
                outgoing: n.links.len() as u32,
            })
            .collect();
        out.sort_by(|a, b| b.backlinks.cmp(&a.backlinks).then(a.name.cmp(&b.name)));
        Ok(out)
    })
}

/// Whether a wiki-link target exists, so the UI can flag dangling links.
pub fn note_exists(root: String, name: String) -> bool {
    store::resolve_wiki_link(root, name)
        .map(|p| p.is_some())
        .unwrap_or(false)
}
