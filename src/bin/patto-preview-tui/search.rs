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
/// - **State** (persists across renders): `query`, `matches`, `match_idx`, `typing`
/// - **Props** (read by rendering code): all public fields
pub(crate) struct SearchState {
    pub direction: SearchDirection,
    /// The query string the user is typing.
    pub query: String,
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

    /// Confirm the search: stop typing, keep results for `n`/`N` navigation.
    pub(crate) fn confirm(&mut self) {
        self.typing = false;
    }

    /// Return the currently highlighted match, if any.
    pub(crate) fn current_match(&self) -> Option<&SearchMatch> {
        self.match_idx.and_then(|i| self.matches.get(i))
    }
}
