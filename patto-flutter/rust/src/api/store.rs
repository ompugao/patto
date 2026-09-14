//! The on-device notes directory.
//!
//! A note's wiki-link name is its path relative to the notes root without the
//! `.pn` extension, so `[dir/note]` resolves to `<root>/dir/note.pn`. This
//! matches `Repository::link_to_path` on the desktop side.

use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use walkdir::WalkDir;

use crate::api::error::{PattoError, PattoResult};
use crate::api::types::NoteMeta;

pub const NOTE_EXT: &str = "pn";

/// Reject names that would escape the notes root or collide with git internals.
fn validate_name(name: &str) -> PattoResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(PattoError::InvalidName("empty".to_string()));
    }
    if name.starts_with('/') || name.contains('\\') {
        return Err(PattoError::InvalidName(name.to_string()));
    }
    if name.split('/').any(|seg| {
        seg.is_empty() || seg == "." || seg == ".." || seg.starts_with('.') || seg.ends_with(' ')
    }) {
        return Err(PattoError::InvalidName(name.to_string()));
    }
    Ok(())
}

pub fn name_to_rel_path(name: &str) -> PattoResult<String> {
    validate_name(name)?;
    Ok(format!("{}.{}", name.trim(), NOTE_EXT))
}

pub fn rel_path_to_name(rel_path: &str) -> String {
    rel_path
        .strip_suffix(&format!(".{}", NOTE_EXT))
        .unwrap_or(rel_path)
        .replace('\\', "/")
}

/// Join a repo-relative path onto the root, refusing anything that escapes it.
pub fn resolve(root: &str, rel_path: &str) -> PattoResult<PathBuf> {
    let rel = Path::new(rel_path);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(PattoError::InvalidName(rel_path.to_string()));
    }
    Ok(Path::new(root).join(rel))
}

fn meta_for(root: &Path, path: &Path) -> Option<NoteMeta> {
    let rel = path.strip_prefix(root).ok()?;
    let rel_path = rel.to_string_lossy().replace('\\', "/");
    let meta = std::fs::metadata(path).ok()?;
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    Some(NoteMeta {
        name: rel_path_to_name(&rel_path),
        rel_path,
        modified_ms,
        size_bytes: meta.len(),
    })
}

/// Every `.pn` file under `root`, newest first. Hidden directories (`.git`) are
/// skipped.
pub fn list_notes(root: String) -> PattoResult<Vec<NoteMeta>> {
    let root_path = Path::new(&root);
    if !root_path.is_dir() {
        return Err(PattoError::NotFound(root));
    }

    let mut notes: Vec<NoteMeta> = WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !(e.depth() > 0
                && e.file_type().is_dir()
                && e.file_name().to_string_lossy().starts_with('.'))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|x| x == NOTE_EXT).unwrap_or(false))
        .filter_map(|e| meta_for(root_path, e.path()))
        .collect();

    notes.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms).then(a.name.cmp(&b.name)));
    Ok(notes)
}

pub fn read_note(root: String, rel_path: String) -> PattoResult<String> {
    let path = resolve(&root, &rel_path)?;
    std::fs::read_to_string(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => PattoError::NotFound(rel_path),
        _ => PattoError::Io(e.to_string()),
    })
}

/// Write via a temporary file in the same directory, so an interrupted write
/// never leaves a truncated note behind.
pub fn write_note(root: String, rel_path: String, content: String) -> PattoResult<()> {
    let path = resolve(&root, &rel_path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp = path.with_extension("pn.tmp");
    std::fs::write(&tmp, content.as_bytes())?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn create_note(root: String, name: String, initial_content: String) -> PattoResult<NoteMeta> {
    let rel_path = name_to_rel_path(&name)?;
    let path = resolve(&root, &rel_path)?;
    if path.exists() {
        return Err(PattoError::AlreadyExists(rel_path));
    }

    write_note(root.clone(), rel_path.clone(), initial_content)?;
    meta_for(Path::new(&root), &path).ok_or(PattoError::NotFound(rel_path))
}

pub fn delete_note(root: String, rel_path: String) -> PattoResult<()> {
    let path = resolve(&root, &rel_path)?;
    std::fs::remove_file(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => PattoError::NotFound(rel_path),
        _ => PattoError::Io(e.to_string()),
    })
}

/// Resolve a wiki-link target to a note path, or `None` if no such note exists.
pub fn resolve_wiki_link(root: String, name: String) -> PattoResult<Option<String>> {
    let Ok(rel_path) = name_to_rel_path(&name) else {
        return Ok(None);
    };
    let path = resolve(&root, &rel_path)?;
    Ok(path.is_file().then_some(rel_path))
}

/// Fuzzy match over note names. An empty query lists everything, newest first.
pub fn search_notes(root: String, query: String, limit: u32) -> PattoResult<Vec<NoteMeta>> {
    let notes = list_notes(root)?;
    let query = query.trim();
    let limit = limit as usize;

    if query.is_empty() {
        let mut notes = notes;
        notes.truncate(limit);
        return Ok(notes);
    }

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, NoteMeta)> = notes
        .into_iter()
        .filter_map(|n| matcher.fuzzy_match(&n.name, query).map(|score| (score, n)))
        .collect();

    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(b.1.modified_ms.cmp(&a.1.modified_ms))
            .then(a.1.name.cmp(&b.1.name))
    });
    scored.truncate(limit);

    Ok(scored.into_iter().map(|(_, n)| n).collect())
}
