//! Completion: note names, anchors, papers and `[@command]` snippets.

use ropey::RopeSlice;
use str_indices::utf16::{from_byte_idx as utf16_from_byte_idx, to_byte_idx as utf16_to_byte_idx};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, InsertTextFormat,
    Position, Range, TextEdit, Url,
};
use urlencoding::decode;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::lsp::backend::{gather_anchors, Backend};
use crate::repository::Repository;

/// Lines shown in a completion item's documentation popup.
const PREVIEW_LINES: usize = 5;

/// What the cursor is asking for.
///
/// Built while the repository lock is held. Paper matches are fetched
/// afterwards, since that is an async call and the guard is not `Send`.
enum Completion {
    Items(Vec<CompletionItem>),
    /// Note-name matches, to be merged with matches from the paper catalogue.
    NotesAndPapers {
        notes: Vec<CompletionItem>,
        range: Range,
        query: String,
    },
}

/// The text of the line being completed, up to the cursor.
struct Cursor<'a> {
    line: RopeSlice<'a>,
    line_str: &'a str,
    /// Cursor offset in chars from the start of the line.
    char_col: usize,
    position: Position,
}

impl<'a> Cursor<'a> {
    fn new(line: RopeSlice<'a>, position: Position) -> Option<Self> {
        let line_str = line.as_str()?;
        let char_col = line.byte_to_char(utf16_to_byte_idx(line_str, position.character as usize));
        Some(Self {
            line,
            line_str,
            char_col,
            position,
        })
    }

    fn char_before_cursor(&self) -> Option<char> {
        self.line.get_char(self.char_col.checked_sub(1)?)
    }

    /// Char index of the last `needle` before the cursor.
    fn last_index_of(&self, needle: char) -> Option<usize> {
        let before = self.line.slice(..self.char_col);
        let found = before
            .chars_at(self.char_col)
            .reversed()
            .position(|c| c == needle)?;
        Some(self.char_col - 1 - found)
    }

    fn text(&self, from: usize, to: usize) -> Option<&'a str> {
        self.line.slice(from..to).as_str()
    }

    /// Range from char index `from` to the cursor, in the units LSP expects.
    fn range_from(&self, from: usize) -> Range {
        Range {
            start: Position {
                line: self.position.line,
                character: utf16_from_byte_idx(self.line_str, self.line.char_to_byte(from)) as u32,
            },
            end: self.position,
        }
    }
}

impl Backend {
    pub(super) async fn gather_completion_items(
        &self,
        uri: &Url,
        position: Position,
    ) -> Option<Vec<CompletionItem>> {
        match self.completion_at(uri, position)? {
            Completion::Items(items) => Some(items),
            Completion::NotesAndPapers {
                mut notes,
                range,
                query,
            } => {
                notes.extend(self.paper_completion_items(&query, &range).await);
                Some(notes)
            }
        }
    }

    fn completion_at(&self, uri: &Url, position: Position) -> Option<Completion> {
        let repo_guard = self.repository.lock().unwrap();
        let repo = repo_guard.as_ref()?;
        let rope = repo.document_map.get(uri)?;
        let cursor = Cursor::new(rope.value().get_line(position.line as usize)?, position)?;

        if let Some(items) = self.anchor_completions(repo, uri, &cursor) {
            return Some(Completion::Items(items));
        }

        // Computed before the command check, but only used as a fallback: in
        // `[@code`, both a `[` and an `@` precede the cursor and the snippet wins.
        let notes = self.note_completions(repo, &cursor);

        if let Some(items) = command_completions(&cursor) {
            return Some(Completion::Items(items));
        }

        notes
    }

    /// Anchors of the linked note, offered right after `[note#`.
    fn anchor_completions(
        &self,
        repo: &Repository,
        uri: &Url,
        cursor: &Cursor,
    ) -> Option<Vec<CompletionItem>> {
        if cursor.char_before_cursor()? != '#' {
            return None;
        }
        let bracket = cursor.last_index_of('[')?;
        let link = cursor.text(bracket + 1, cursor.char_col - 1)?;

        let root_uri = self.root_uri.lock().unwrap().as_ref().cloned()?;
        let link_uri = repo.link_to_uri(link, &root_uri).unwrap_or(uri.clone());
        let ast = repo.ast_map.get(&link_uri)?;

        let mut anchors = vec![];
        gather_anchors(ast.value(), &mut anchors);

        let link_rope = repo.document_map.get(&link_uri);
        Some(
            anchors
                .iter()
                .map(|(anchor, row)| CompletionItem {
                    label: format!("#{}", anchor),
                    kind: Some(CompletionItemKind::REFERENCE),
                    filter_text: Some(anchor.to_string()),
                    insert_text: Some(anchor.to_string()),
                    documentation: link_rope
                        .as_ref()
                        .and_then(|rope| preview(rope.value(), *row)),
                    ..Default::default()
                })
                .collect(),
        )
    }

    /// Note names matching what has been typed since the last `[`.
    fn note_completions(&self, repo: &Repository, cursor: &Cursor) -> Option<Completion> {
        let bracket = cursor.last_index_of('[')?;
        let query = cursor.text(bracket + 1, cursor.char_col)?;

        let root_path = self
            .root_uri
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|root_uri| root_uri.to_file_path().ok())?;
        let root_len = root_path.to_string_lossy().len();

        let range = cursor.range_from(bracket + 1);
        let matcher = SkimMatcherV2::default();

        let notes = repo
            .document_map
            .iter()
            .filter_map(|entry| {
                let path = entry.key().to_file_path().ok()?;
                let relative = decode(&path.to_string_lossy()[root_len + 1..])
                    .ok()?
                    .into_owned();
                let name = relative
                    .strip_suffix(".pn")
                    .unwrap_or(&relative)
                    .to_string();

                matcher.fuzzy_match(&name, query)?;
                Some(CompletionItem {
                    label: name.clone(),
                    detail: Some(name.clone()),
                    kind: Some(CompletionItemKind::FILE),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        new_text: name,
                        range,
                    })),
                    documentation: preview(entry.value(), 0),
                    ..Default::default()
                })
            })
            .collect();

        Some(Completion::NotesAndPapers {
            notes,
            range,
            query: query.to_string(),
        })
    }
}

/// Snippet for the `[@command]` being typed, if the text since the last `@`
/// names one.
fn command_completions(cursor: &Cursor) -> Option<Vec<CompletionItem>> {
    let at = cursor.last_index_of('@')?;
    let trigger = cursor.text(at, cursor.char_col)?;
    let (detail, snippet) = command_snippet(trigger)?;

    Some(vec![CompletionItem {
        label: trigger.to_string(),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(detail.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            new_text: snippet,
            range: cursor.range_from(at),
        })),
        ..Default::default()
    }])
}

fn command_snippet(trigger: &str) -> Option<(&'static str, String)> {
    let (detail, snippet) = match trigger {
        "@code" => ("code command", "[@code ${1:lang}]$0".to_string()),
        "@math" => ("math command", "[@math]$0".to_string()),
        "@quote" => ("quote command", "[@quote]$0".to_string()),
        "@img" => (
            "img command",
            "[@img ${1:path} \"${2:alt_text}\"]$0".to_string(),
        ),
        "@task" => (
            "task property",
            format!(
                "{{@task status=${{1:todo}} due=${{2:{}}}}}$0",
                chrono::Local::now().format("%Y-%m-%d")
            ),
        ),
        _ => return None,
    };
    Some((detail, snippet))
}

/// The first few lines from `row`, for a completion item's documentation popup.
fn preview(rope: &ropey::Rope, row: usize) -> Option<Documentation> {
    let total_lines = rope.len_lines();
    if row >= total_lines {
        return None;
    }
    let text: String = (row..(row + PREVIEW_LINES).min(total_lines))
        .filter_map(|line| rope.get_line(line).map(|line| line.to_string()))
        .collect();

    let text = text.trim_end();
    (!text.trim().is_empty()).then(|| Documentation::String(text.to_string()))
}
