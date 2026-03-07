//! Incremental vim-like search state for the TUI preview.
//!
//! This module is self-contained (mirrors the `BacklinksPanel` pattern):
//! all search state and logic live here; the rest of the app reads it as props.

use patto::tui_renderer::DocElement;

/// Direction of a search initiated with `/` (forward) or `?` (backward).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchDirection {
    Forward,
    Backward,
}

/// A single match found while scanning `DocElement::TextLine` elements.
#[derive(Debug, Clone)]
pub(crate) struct SearchMatch {
    /// Index into `RenderedDoc.elements`.
    pub elem_idx: usize,
    /// Start char offset within the line's plain text (inclusive).
    pub char_start: usize,
    /// End char offset within the line's plain text (exclusive).
    pub char_end: usize,
}

/// Self-contained search state.
///
/// Separation of concerns (tui-react props/state pattern):
/// - **State** (persists across renders): `query`, `cursor`, `matches`, `match_idx`, `typing`
/// - **Props** (read by rendering code): all public fields
pub(crate) struct SearchState {
    pub direction: SearchDirection,
    /// The query string the user is typing.
    pub query: String,
    /// Cursor position as a **byte offset** into `query` (always on a char boundary).
    /// Ranges from `0` (before first char) to `query.len()` (after last char).
    pub cursor: usize,
    /// All matches found in the document.
    pub matches: Vec<SearchMatch>,
    /// Index into `matches` of the current (highlighted) match.
    pub match_idx: Option<usize>,
    /// `true` while the user is still entering the query (search prompt visible).
    /// `false` after `Enter` — results stay, `n`/`N` navigation is available.
    pub typing: bool,
}

impl SearchState {
    /// Create a new, empty search state.
    pub(crate) fn new(direction: SearchDirection) -> Self {
        Self {
            direction,
            query: String::new(),
            cursor: 0,
            matches: Vec::new(),
            match_idx: None,
            typing: true,
        }
    }

    /// Recompute matches against `elements`.
    ///
    /// `elem_display_offsets[i]` is the cumulative display row at which element `i` starts
    /// (used to pick the closest match to the current `scroll_offset`).
    pub(crate) fn update_matches(
        &mut self,
        elements: &[DocElement],
        scroll_offset: usize,
        elem_display_offsets: &[usize],
    ) {
        self.matches.clear();

        if self.query.is_empty() {
            self.match_idx = None;
            return;
        }

        let query_lower = self.query.to_lowercase();

        for (elem_idx, elem) in elements.iter().enumerate() {
            if let DocElement::TextLine(line, _) = elem {
                // Extract plain text by concatenating all span content.
                let plain: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                let plain_lower = plain.to_lowercase();

                let mut search_start = 0;
                while let Some(byte_pos) = plain_lower[search_start..].find(&query_lower) {
                    let abs_byte = search_start + byte_pos;
                    let char_start = plain[..abs_byte].chars().count();
                    let char_end = char_start + self.query.chars().count();
                    self.matches.push(SearchMatch {
                        elem_idx,
                        char_start,
                        char_end,
                    });
                    // Advance past this match (by bytes, to handle multi-byte chars correctly).
                    search_start = abs_byte + query_lower.len().max(1);
                    if search_start >= plain_lower.len() {
                        break;
                    }
                }
            }
        }

        // Pick the closest match to the current scroll position.
        self.match_idx = if self.matches.is_empty() {
            None
        } else {
            let best = self
                .matches
                .iter()
                .enumerate()
                .min_by_key(|(_, m)| {
                    let offset = elem_display_offsets
                        .get(m.elem_idx)
                        .copied()
                        .unwrap_or(usize::MAX);
                    offset.abs_diff(scroll_offset)
                })
                .map(|(i, _)| i);
            best
        };
    }

    /// Advance to the next match (wraps around).
    pub(crate) fn next_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.match_idx = Some(match self.match_idx {
            Some(idx) => (idx + 1) % self.matches.len(),
            None => 0,
        });
    }

    /// Retreat to the previous match (wraps around).
    pub(crate) fn prev_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.match_idx = Some(match self.match_idx {
            Some(idx) => {
                if idx == 0 {
                    self.matches.len() - 1
                } else {
                    idx - 1
                }
            }
            None => self.matches.len().saturating_sub(1),
        });
    }

    /// Insert a character at the cursor position and advance the cursor.
    pub(crate) fn insert_at_cursor(&mut self, c: char) {
        self.query.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Delete the character immediately before the cursor (Backspace / C-h).
    pub(crate) fn delete_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        // Walk back to the start of the previous char.
        let prev = self.query[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.query.remove(prev);
        self.cursor = prev;
    }

    /// Delete the character at the cursor position (Delete key).
    pub(crate) fn delete_after_cursor(&mut self) {
        if self.cursor >= self.query.len() {
            return;
        }
        self.query.remove(self.cursor);
        // cursor byte position stays the same (now points to next char).
    }

    /// Delete from the cursor back to the start of the previous WORD (C-w).
    ///
    /// Mirrors vim's `c_CTRL-W`: skip trailing whitespace, then delete the
    /// preceding non-whitespace word.
    pub(crate) fn delete_word_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let before = &self.query[..self.cursor];
        // Find start of the word to delete: skip whitespace, then skip non-whitespace.
        let new_end = before.len();
        let chars: Vec<(usize, char)> = before.char_indices().collect();
        let mut i = chars.len();
        // Skip trailing whitespace.
        while i > 0 && chars[i - 1].1.is_whitespace() {
            i -= 1;
        }
        // Skip non-whitespace word.
        while i > 0 && !chars[i - 1].1.is_whitespace() {
            i -= 1;
        }
        let new_cursor = if i == 0 { 0 } else { chars[i].0 };
        self.query.drain(new_cursor..new_end);
        self.cursor = new_cursor;
    }

    /// Delete everything from the start of the query to the cursor (C-u).
    pub(crate) fn delete_to_start(&mut self) {
        self.query.drain(..self.cursor);
        self.cursor = 0;
    }

    /// Move cursor one character to the left.
    pub(crate) fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor = self.query[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    /// Move cursor one character to the right.
    pub(crate) fn move_right(&mut self) {
        if self.cursor >= self.query.len() {
            return;
        }
        let c = self.query[self.cursor..].chars().next().unwrap();
        self.cursor += c.len_utf8();
    }

    /// Move cursor one WORD to the left (C-Left / S-Left).
    ///
    /// Mirrors vim `c_<C-Left>`: skip whitespace backwards, then skip
    /// non-whitespace backwards.
    pub(crate) fn move_word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let chars: Vec<(usize, char)> = self.query[..self.cursor].char_indices().collect();
        let mut i = chars.len();
        // Skip whitespace.
        while i > 0 && chars[i - 1].1.is_whitespace() {
            i -= 1;
        }
        // Skip non-whitespace word.
        while i > 0 && !chars[i - 1].1.is_whitespace() {
            i -= 1;
        }
        self.cursor = if i == 0 { 0 } else { chars[i].0 };
    }

    /// Move cursor one WORD to the right (C-Right / S-Right).
    ///
    /// Mirrors vim `c_<C-Right>`: skip non-whitespace, then skip whitespace.
    pub(crate) fn move_word_right(&mut self) {
        if self.cursor >= self.query.len() {
            return;
        }
        let chars: Vec<(usize, char)> = self.query[self.cursor..].char_indices().collect();
        let mut i = 0;
        // Skip non-whitespace.
        while i < chars.len() && !chars[i].1.is_whitespace() {
            i += 1;
        }
        // Skip whitespace.
        while i < chars.len() && chars[i].1.is_whitespace() {
            i += 1;
        }
        self.cursor += if i < chars.len() {
            chars[i].0
        } else {
            self.query.len() - self.cursor
        };
    }

    /// Move cursor to the start of the query (C-b / Home).
    pub(crate) fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to the end of the query (C-e / End).
    pub(crate) fn move_to_end(&mut self) {
        self.cursor = self.query.len();
    }

    /// Confirm the search: stop typing, keep results for `n`/`N` navigation.
    pub(crate) fn confirm(&mut self) {
        self.typing = false;
    }

    /// Return the currently highlighted match, if any.
    pub(crate) fn current_match(&self) -> Option<&SearchMatch> {
        self.match_idx.and_then(|i| self.matches.get(i))
    }
}
