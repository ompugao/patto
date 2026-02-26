use patto::repository::{BackLinkData, Repository};
use std::path::Path;
use tui_widget_list::ListState;

/// A single entry in the flat list shown in the backlinks panel.
#[derive(Clone)]
pub(crate) enum FlatEntry {
    /// Section header, e.g. "Backlinks:" or "Two-hop Links:".
    SectionHeader(String),
    /// A single backlink location.
    BacklinkItem {
        source_file: String,
        line: usize,
        context: Option<String>,
    },
    /// Sub-header for a two-hop "via" grouping.
    ViaHeader(String),
    /// A two-hop target file name.
    TwoHopItem(String),
    /// Informational "(none)" placeholder – not selectable.
    Placeholder(String),
}

impl FlatEntry {
    /// Returns true if this entry can be jumped to (i.e. is a navigable target).
    pub(crate) fn is_selectable(&self) -> bool {
        matches!(
            self,
            FlatEntry::BacklinkItem { .. } | FlatEntry::TwoHopItem(_)
        )
    }
}

/// Self-contained backlinks panel state.
///
/// Manages backlink/two-hop-link data and cursor navigation
/// without any knowledge of the wider application.
pub(crate) struct BacklinksPanel {
    pub(crate) visible: bool,
    pub(crate) back_links: Vec<BackLinkData>,
    pub(crate) two_hop_links: Vec<(String, Vec<String>)>,
    /// Flat list of all display entries (headers + selectable items).
    pub(crate) entries: Vec<FlatEntry>,
    /// tui-widget-list selection state.
    pub(crate) list_state: ListState,
}

impl BacklinksPanel {
    pub(crate) fn new() -> Self {
        Self {
            visible: false,
            back_links: Vec::new(),
            two_hop_links: Vec::new(),
            entries: Vec::new(),
            list_state: ListState::default(),
        }
    }

    /// Show the panel, resetting selection.
    pub(crate) fn open(&mut self) {
        self.visible = true;
        self.list_state = ListState::default();
    }

    /// Hide the panel, resetting selection.
    pub(crate) fn close(&mut self) {
        self.visible = false;
        self.list_state = ListState::default();
    }

    /// Recompute backlinks and two-hop links for the given file, then rebuild the flat entry list.
    pub(crate) async fn refresh(&mut self, repository: &Repository, file_path: &Path) {
        self.back_links = repository.calculate_back_links(file_path);
        self.two_hop_links = repository.calculate_two_hop_links(file_path).await;
        self.rebuild_entries();
        self.list_state = ListState::default();
    }

    /// Rebuild the flat `entries` vec from `back_links` + `two_hop_links`.
    fn rebuild_entries(&mut self) {
        let mut entries = Vec::new();

        entries.push(FlatEntry::SectionHeader("Backlinks:".to_string()));
        if self.back_links.is_empty() {
            entries.push(FlatEntry::Placeholder("  (none)".to_string()));
        } else {
            for bl in &self.back_links {
                for loc in &bl.locations {
                    entries.push(FlatEntry::BacklinkItem {
                        source_file: bl.source_file.clone(),
                        line: loc.line,
                        context: loc.context.clone(),
                    });
                }
            }
        }

        entries.push(FlatEntry::SectionHeader(String::new())); // blank separator

        entries.push(FlatEntry::SectionHeader("Two-hop Links:".to_string()));
        if self.two_hop_links.is_empty() {
            entries.push(FlatEntry::Placeholder("  (none)".to_string()));
        } else {
            for (via, targets) in &self.two_hop_links {
                entries.push(FlatEntry::ViaHeader(via.clone()));
                for target in targets {
                    entries.push(FlatEntry::TwoHopItem(target.clone()));
                }
            }
        }

        self.entries = entries;
    }

    /// Move selection down, skipping non-selectable entries.
    pub(crate) fn navigate_down(&mut self) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let start = self.list_state.selected.unwrap_or(0);
        let mut next = (start + 1) % len;
        // Skip non-selectable entries; give up after a full pass.
        for _ in 0..len {
            if self.entries[next].is_selectable() {
                break;
            }
            next = (next + 1) % len;
        }
        if self.entries[next].is_selectable() {
            self.list_state.select(Some(next));
        }
    }

    /// Move selection up, skipping non-selectable entries.
    pub(crate) fn navigate_up(&mut self) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let start = self.list_state.selected.unwrap_or(0);
        let mut prev = if start == 0 { len - 1 } else { start - 1 };
        for _ in 0..len {
            if self.entries[prev].is_selectable() {
                break;
            }
            prev = if prev == 0 { len - 1 } else { prev - 1 };
        }
        if self.entries[prev].is_selectable() {
            self.list_state.select(Some(prev));
        }
    }

    /// Resolve the current selection to a navigation target (file_name, line).
    pub(crate) fn resolve_cursor(&self) -> Option<(String, usize)> {
        let idx = self.list_state.selected?;
        match self.entries.get(idx)? {
            FlatEntry::BacklinkItem {
                source_file, line, ..
            } => Some((source_file.clone(), *line)),
            FlatEntry::TwoHopItem(name) => Some((name.clone(), 0)),
            _ => None,
        }
    }
}
