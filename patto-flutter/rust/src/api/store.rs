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
use crate::api::types::{NoteMeta, TextMatch, TextSearchHit};

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

pub(crate) fn meta_for(root: &Path, path: &Path) -> Option<NoteMeta> {
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

/// How many characters of a matching line are kept around the match.
const SNIPPET_CHARS: usize = 160;

/// Case-insensitive substring search over note names and contents.
///
/// An empty query finds nothing. Hits are ranked by [`rank_text_hits`].
pub fn search_text(
    root: String,
    query: String,
    max_notes: u32,
    max_lines_per_note: u32,
) -> PattoResult<Vec<TextSearchHit>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }

    let root_path = Path::new(&root);
    let mut hits = Vec::new();
    for note in list_notes(root.clone())? {
        // A note that cannot be read as text is simply not a match.
        let Ok(content) = std::fs::read_to_string(root_path.join(&note.rel_path)) else {
            continue;
        };

        let mut matches = Vec::new();
        let mut total_matches = 0u32;
        for (row, line) in content.lines().enumerate() {
            let lower = line.to_lowercase();
            let Some(at) = lower.find(&needle) else {
                continue;
            };
            total_matches += 1;
            if matches.len() < max_lines_per_note as usize {
                let match_char = lower[..at].chars().count();
                matches.push(TextMatch {
                    row: row as u32,
                    line: snippet(line, match_char),
                });
            }
        }

        let name_matches = note.name.to_lowercase().contains(&needle);
        if name_matches || total_matches > 0 {
            hits.push(TextSearchHit {
                note,
                name_matches,
                matches,
                total_matches,
            });
        }
    }

    rank_text_hits(&mut hits, max_notes);
    Ok(hits)
}

/// Notes whose name matches first, then those with the most matching lines,
/// then the most recently changed.
pub fn rank_text_hits(hits: &mut Vec<TextSearchHit>, max_notes: u32) {
    hits.sort_by(|a, b| {
        b.name_matches
            .cmp(&a.name_matches)
            .then(b.total_matches.cmp(&a.total_matches))
            .then(b.note.modified_ms.cmp(&a.note.modified_ms))
            .then(a.note.name.cmp(&b.note.name))
    });
    hits.truncate(max_notes as usize);
}

/// Trim a line and, when it is long, keep a window around the match at
/// `match_char` (a char index into the untrimmed, lowercased line).
fn snippet(line: &str, match_char: usize) -> String {
    let leading = line.chars().take_while(|c| c.is_whitespace()).count();
    let chars: Vec<char> = line.trim().chars().collect();
    if chars.len() <= SNIPPET_CHARS {
        return chars.into_iter().collect();
    }

    // Lowercasing can change the char count slightly, so the index is only a
    // close estimate; keep a margin before it.
    let at = match_char.saturating_sub(leading).min(chars.len());
    let start = at.saturating_sub(SNIPPET_CHARS / 3);
    let start = start.min(chars.len() - SNIPPET_CHARS);
    let end = start + SNIPPET_CHARS;

    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(&chars[start..end]);
    if end < chars.len() {
        out.push('…');
    }
    out
}
