use crossterm::event::{KeyCode, KeyModifiers};
use patto::{
    line_tracker::LineTracker,
    parser,
    repository::Repository,
    tui_renderer::{self, DocElement, FocusableItem, LinkAction, RenderedDoc},
};
use std::path::{Path, PathBuf};

use crate::backlinks::BacklinksPanel;
use crate::image_cache::ImageCache;

/// Saved navigation state for back-navigation.
pub(crate) struct NavigationEntry {
    pub(crate) file_path: PathBuf,
    pub(crate) scroll_offset: usize,
}

pub(crate) struct App {
    pub(crate) file_path: PathBuf,
    /// Workspace root directory.
    pub(crate) root_dir: PathBuf,
    pub(crate) rendered_doc: RenderedDoc,
    pub(crate) scroll_offset: usize,
    pub(crate) viewport_height: usize,
    pub(crate) line_tracker: LineTracker,
    /// Index into `rendered_doc.focusables` of the currently focused item. None = no focus.
    pub(crate) focused_item_idx: Option<usize>,
    /// Navigation history for back-navigation.
    pub(crate) nav_history: Vec<NavigationEntry>,
    /// Image loading, caching, and display.
    pub(crate) images: ImageCache,
    /// Backlinks/two-hop-links panel.
    pub(crate) backlinks: BacklinksPanel,
}

impl App {
    pub(crate) fn new(
        file_path: PathBuf,
        root_dir: PathBuf,
        protocol_override: Option<&str>,
    ) -> Self {
        Self {
            file_path,
            root_dir,
            rendered_doc: RenderedDoc {
                elements: Vec::new(),
                focusables: Vec::new(),
            },
            scroll_offset: 0,
            viewport_height: 24,
            line_tracker: LineTracker::new().expect("Failed to create line tracker"),
            focused_item_idx: None,
            nav_history: Vec::new(),
            images: ImageCache::new(protocol_override),
            backlinks: BacklinksPanel::new(),
        }
    }

    // --- Scrolling ---

    pub(crate) fn scroll_down(&mut self, amount: usize) {
        let max = self
            .rendered_doc
            .total_height(self.images.height_rows)
            .saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max);
    }

    pub(crate) fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub(crate) fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self
            .rendered_doc
            .total_height(self.images.height_rows)
            .saturating_sub(1);
    }

    // --- Rendering ---

    pub(crate) fn re_render(&mut self, content: &str) {
        let result =
            parser::parse_text_with_persistent_line_tracking(content, &mut self.line_tracker);
        self.rendered_doc = tui_renderer::render_ast(&result.ast);
    }

    /// Return a reference to the currently focused item, if any.
    pub(crate) fn focused_item(&self) -> Option<&FocusableItem> {
        self.focused_item_idx
            .and_then(|idx| self.rendered_doc.focusables.get(idx))
    }

    // --- Focus ---

    /// Indices (into `rendered_doc.focusables`) of focusable items visible in the viewport.
    fn visible_focusable_indices(&self) -> Vec<usize> {
        let img_h = self.images.height_rows;
        let mut row = 0usize;
        let mut visible_elems = std::collections::HashSet::new();
        for (i, elem) in self.rendered_doc.elements.iter().enumerate() {
            let h = elem.height(img_h) as usize;
            let elem_top = row;
            let elem_bot = row + h;
            row = elem_bot;
            if elem_bot <= self.scroll_offset {
                continue;
            }
            if elem_top >= self.scroll_offset + self.viewport_height {
                break;
            }
            visible_elems.insert(i);
        }
        self.rendered_doc
            .focusables
            .iter()
            .enumerate()
            .filter(|(_, fi)| visible_elems.contains(&fi.elem_idx))
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Focus the next visible focusable item (wrap around).
    pub(crate) fn focus_next_item(&mut self) {
        let visible = self.visible_focusable_indices();
        if visible.is_empty() {
            return;
        }
        let next = match self.focused_item_idx {
            Some(cur) => visible
                .iter()
                .find(|&&fi| fi > cur)
                .copied()
                .unwrap_or(visible[0]),
            None => visible[0],
        };
        self.focused_item_idx = Some(next);
    }

    /// Focus the previous visible focusable item (wrap around).
    pub(crate) fn focus_prev_item(&mut self) {
        let visible = self.visible_focusable_indices();
        if visible.is_empty() {
            return;
        }
        let prev = match self.focused_item_idx {
            Some(cur) => visible
                .iter()
                .rev()
                .find(|&&fi| fi < cur)
                .copied()
                .unwrap_or(*visible.last().unwrap()),
            None => *visible.last().unwrap(),
        };
        self.focused_item_idx = Some(prev);
    }

    /// Clear focus if the focused item is no longer visible.
    pub(crate) fn clear_stale_focus(&mut self) {
        if let Some(idx) = self.focused_item_idx {
            let visible = self.visible_focusable_indices();
            if !visible.contains(&idx) {
                self.focused_item_idx = None;
            }
        }
    }

    // --- Navigation ---

    /// Navigate to a wiki-linked note. Saves current state in history.
    pub(crate) fn open_note(&mut self, name: &str, anchor: Option<&str>) -> bool {
        let target_path = if name.ends_with(".pn") {
            self.root_dir.join(name)
        } else {
            self.root_dir.join(format!("{}.pn", name))
        };

        if !target_path.exists() || !target_path.is_file() {
            return false;
        }

        let content = match std::fs::read_to_string(&target_path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        self.nav_history.push(NavigationEntry {
            file_path: self.file_path.clone(),
            scroll_offset: self.scroll_offset,
        });

        self.file_path = target_path;
        self.scroll_offset = 0;
        self.focused_item_idx = None;
        self.images.fullscreen_src = None;
        self.re_render(&content);

        if let Some(anchor_text) = anchor {
            self.scroll_to_anchor(anchor_text);
        }

        true
    }

    /// Navigate to a file by path. Saves current state in history.
    #[allow(dead_code)]
    pub(crate) fn open_file(&mut self, path: &Path) -> bool {
        if !path.exists() || !path.is_file() {
            return false;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        self.nav_history.push(NavigationEntry {
            file_path: self.file_path.clone(),
            scroll_offset: self.scroll_offset,
        });

        self.file_path = path.to_path_buf();
        self.scroll_offset = 0;
        self.focused_item_idx = None;
        self.images.fullscreen_src = None;
        self.re_render(&content);
        true
    }

    /// Go back in navigation history.
    pub(crate) fn go_back(&mut self) -> bool {
        if let Some(entry) = self.nav_history.pop() {
            let content = match std::fs::read_to_string(&entry.file_path) {
                Ok(c) => c,
                Err(_) => return false,
            };
            self.file_path = entry.file_path;
            self.scroll_offset = entry.scroll_offset;
            self.focused_item_idx = None;
            self.images.fullscreen_src = None;
            self.re_render(&content);
            true
        } else {
            false
        }
    }

    /// Try to scroll to a heading/anchor matching the given text.
    fn scroll_to_anchor(&mut self, anchor: &str) {
        let anchor_lower = anchor.to_lowercase();
        let img_h = self.images.height_rows;
        let mut row = 0usize;
        for elem in &self.rendered_doc.elements {
            if let DocElement::TextLine(line) = elem {
                let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                if text.to_lowercase().contains(&anchor_lower) {
                    self.scroll_offset = row;
                    return;
                }
            }
            row += elem.height(img_h) as usize;
        }
    }

    /// Scroll to a specific source line number (0-indexed).
    fn scroll_to_line(&mut self, target_line: usize) {
        let img_h = self.images.height_rows;
        let mut row = 0usize;
        let mut current_source_line = 0usize;
        for elem in &self.rendered_doc.elements {
            if current_source_line >= target_line {
                self.scroll_offset = row;
                return;
            }
            let h = elem.height(img_h) as usize;
            if let DocElement::TextLine(_) = elem {
                current_source_line += 1;
            }
            row += h;
        }
        self.scroll_offset = row.saturating_sub(self.viewport_height);
    }

    // --- Input handling ---

    /// Handle a key event while the backlinks popup is open.
    /// Returns `true` if the app should quit.
    pub(crate) async fn handle_backlinks_key(
        &mut self,
        repository: &Repository,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> bool {
        match (code, modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) | (KeyCode::Char('b'), _) => {
                self.backlinks.close();
            }
            (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                self.backlinks.cursor_down();
            }
            (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                self.backlinks.cursor_up();
            }
            (KeyCode::Enter, _) => {
                if let Some((name, line)) = self.backlinks.resolve_cursor() {
                    self.backlinks.close();
                    if self.open_note(&name, None) {
                        if line > 0 {
                            self.scroll_to_line(line);
                        }
                        self.backlinks.refresh(repository, &self.file_path).await;
                    }
                }
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return true,
            _ => {}
        }
        false
    }

    /// Handle a key event in the normal (non-popup) view.
    /// Returns `true` if the app should quit.
    /// `viewport_height` is the current terminal height in rows.
    pub(crate) async fn handle_normal_key(
        &mut self,
        repository: &Repository,
        code: KeyCode,
        modifiers: KeyModifiers,
        viewport_height: usize,
    ) -> bool {
        match (code, modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                if self.images.fullscreen_src.is_some() {
                    self.images.fullscreen_src = None;
                } else {
                    return true;
                }
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return true,
            (KeyCode::Char('j'), _) | (KeyCode::Down, _) => self.scroll_down(1),
            (KeyCode::Char('k'), _) | (KeyCode::Up, _) => self.scroll_up(1),
            (KeyCode::PageDown, _)
            | (KeyCode::Char(' '), _)
            | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.scroll_down(viewport_height);
            }
            (KeyCode::PageUp, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.scroll_up(viewport_height);
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.scroll_down(viewport_height / 2);
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.scroll_up(viewport_height / 2);
            }
            (KeyCode::Char('g'), _) | (KeyCode::Home, _) => self.scroll_to_top(),
            (KeyCode::Char('G'), _) | (KeyCode::End, _) => self.scroll_to_bottom(),
            (KeyCode::Char('b'), _) => {
                self.backlinks.open();
                self.backlinks.refresh(repository, &self.file_path).await;
            }
            (KeyCode::Char('+'), _) | (KeyCode::Char('='), _) => {
                self.images.increase_height();
            }
            (KeyCode::Char('-'), _) => {
                self.images.decrease_height();
            }
            (KeyCode::Enter, _) => {
                if self.images.fullscreen_src.is_some() {
                    self.images.fullscreen_src = None;
                } else if let Some(fi) = self.focused_item().cloned() {
                    match &fi.action {
                        LinkAction::ViewImage(src) => {
                            self.images.fullscreen_src = Some(src.clone());
                        }
                        LinkAction::OpenNote { name, anchor } => {
                            if self.open_note(name, anchor.as_deref()) {
                                self.backlinks.refresh(repository, &self.file_path).await;
                            }
                        }
                        LinkAction::OpenUrl(url) => {
                            let _ = std::process::Command::new("xdg-open")
                                .arg(url)
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                        }
                    }
                }
            }
            (KeyCode::Backspace, _)
            | (KeyCode::Char('H'), _)
            | (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                if self.go_back() {
                    self.backlinks.refresh(repository, &self.file_path).await;
                }
            }
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.focus_next_item();
            }
            (KeyCode::BackTab, _) => {
                self.focus_prev_item();
            }
            (KeyCode::Char('r'), _) | (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.images.clear();
                let content = std::fs::read_to_string(&self.file_path).unwrap_or_default();
                self.re_render(&content);
            }
            _ => {}
        }
        false
    }
}
