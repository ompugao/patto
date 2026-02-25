use patto::repository::{BackLinkData, Repository};
use std::path::Path;

/// Self-contained backlinks panel state.
///
/// Manages backlink/two-hop-link data and cursor navigation
/// without any knowledge of the wider application.
pub(crate) struct BacklinksPanel {
    pub(crate) visible: bool,
    pub(crate) back_links: Vec<BackLinkData>,
    pub(crate) two_hop_links: Vec<(String, Vec<String>)>,
    pub(crate) cursor: Option<usize>,
}

impl BacklinksPanel {
    pub(crate) fn new() -> Self {
        Self {
            visible: false,
            back_links: Vec::new(),
            two_hop_links: Vec::new(),
            cursor: None,
        }
    }

    /// Show the panel, resetting cursor.
    pub(crate) fn open(&mut self) {
        self.visible = true;
        self.cursor = None;
    }

    /// Hide the panel, resetting cursor.
    pub(crate) fn close(&mut self) {
        self.visible = false;
        self.cursor = None;
    }

    /// Recompute backlinks and two-hop links for the given file.
    pub(crate) async fn refresh(&mut self, repository: &Repository, file_path: &Path) {
        self.back_links = repository.calculate_back_links(file_path);
        self.two_hop_links = repository.calculate_two_hop_links(file_path).await;
    }

    /// Count total selectable entries in the panel.
    pub(crate) fn entry_count(&self) -> usize {
        let bl_count: usize = self.back_links.iter().map(|bl| bl.locations.len()).sum();
        let th_count: usize = self
            .two_hop_links
            .iter()
            .map(|(_, links)| links.len())
            .sum();
        bl_count + th_count
    }

    /// Move the cursor down.
    pub(crate) fn cursor_down(&mut self) {
        let total = self.entry_count();
        if total == 0 {
            return;
        }
        self.cursor = Some(match self.cursor {
            Some(c) => (c + 1).min(total - 1),
            None => 0,
        });
    }

    /// Move the cursor up.
    pub(crate) fn cursor_up(&mut self) {
        let total = self.entry_count();
        if total == 0 {
            return;
        }
        self.cursor = Some(match self.cursor {
            Some(c) => c.saturating_sub(1),
            None => 0,
        });
    }

    /// Resolve the current cursor to a navigation target (file_name, line).
    pub(crate) fn resolve_cursor(&self) -> Option<(String, usize)> {
        let cursor = self.cursor?;
        let mut idx = 0;
        for bl in &self.back_links {
            for loc in &bl.locations {
                if idx == cursor {
                    return Some((bl.source_file.clone(), loc.line));
                }
                idx += 1;
            }
        }
        for (_via, links) in &self.two_hop_links {
            for link_name in links {
                if idx == cursor {
                    return Some((link_name.clone(), 0));
                }
                idx += 1;
            }
        }
        None
    }
}
