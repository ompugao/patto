use patto::repository::{BackLinkData, Repository};
use std::path::Path;
use tui_widget_list::ListState;

use crate::selection::{step_list, Step};

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
    pub(crate) fn is_selectable(&self) -> bool {
        matches!(
            self,
            FlatEntry::BacklinkItem { .. } | FlatEntry::TwoHopItem(_)
        )
    }
}

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

    pub(crate) fn open(&mut self) {
        self.visible = true;
        self.list_state = ListState::default();
    }

    pub(crate) fn close(&mut self) {
        self.visible = false;
        self.list_state = ListState::default();
    }

    pub(crate) async fn refresh(&mut self, repository: &Repository, file_path: &Path) {
        self.back_links = repository.calculate_back_links(file_path);
        self.two_hop_links = repository.calculate_two_hop_links(file_path).await;
        self.rebuild_entries();
        self.list_state = ListState::default();
    }

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

    pub(crate) fn navigate_down(&mut self) {
        step_list(
            &mut self.list_state,
            &self.entries,
            FlatEntry::is_selectable,
            Step::Next,
        );
    }

    pub(crate) fn navigate_up(&mut self) {
        step_list(
            &mut self.list_state,
            &self.entries,
            FlatEntry::is_selectable,
            Step::Prev,
        );
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
