use crate::backlinks::BacklinksPanel;
use crate::config;
use crate::image_cache::ImageCache;
use crate::search::{SearchDirection, SearchState};
use crate::wrap::{elem_height, total_height, WrapConfig};
use crossterm::event::{KeyCode, KeyModifiers};
use patto::{
    line_tracker::LineTracker,
    parser,
    repository::Repository,
    tui_renderer::{self, DocElement, FocusableItem, LinkAction, RenderedDoc},
};
use std::path::{Path, PathBuf};

/// Action returned by `App::handle_key()` to signal side-effects to the caller.
///
/// Follows an Elm-like command pattern: `App` mutates its own state and returns
/// a command; `main` executes the side-effecting part (terminal manipulation,
/// spawning processes).
pub(crate) enum AppAction {
    /// No side-effect needed; continue the event loop.
    None,
    /// Exit the event loop.
    Quit,
    /// Launch an external editor. The caller handles terminal suspend/quit/bg.
    LaunchEditor {
        cmd: String,
        action: config::EditorAction,
    },
}

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
    /// Terminal width in columns, updated each frame by `draw_content`.
    pub(crate) viewport_width: u16,
    /// Whether long lines are soft-wrapped.
    pub(crate) wrap: bool,
    /// String prepended to continuation rows when wrap is on (vim `showbreak`).
    pub(crate) showbreak: String,
    pub(crate) line_tracker: LineTracker,
    /// Index into `rendered_doc.focusables` of the currently focused item. None = no focus.
    pub(crate) focused_item_idx: Option<usize>,
    /// Navigation history for back-navigation.
    pub(crate) nav_history: Vec<NavigationEntry>,
    /// Image loading, caching, and display.
    pub(crate) images: ImageCache,
    /// Backlinks/two-hop-links panel.
    pub(crate) backlinks: BacklinksPanel,
    /// syntect theme name for code block syntax highlighting.
    pub(crate) syntax_theme: String,
    /// Active incremental search state. `None` when no search is active.
    pub(crate) search: Option<SearchState>,
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
            viewport_width: 0,
            wrap: true,
            showbreak: "↪ ".to_string(),
            line_tracker: LineTracker::new().expect("Failed to create line tracker"),
            focused_item_idx: None,
            nav_history: Vec::new(),
            images: ImageCache::new(protocol_override),
            backlinks: BacklinksPanel::new(),
            syntax_theme: String::new(),
            search: None,
        }
    }

    // --- Wrap-aware height ---

    /// `WrapConfig` derived from the current app state.
    pub(crate) fn wrap_config(&self) -> Option<WrapConfig> {
        if self.wrap && self.viewport_width > 0 {
            Some(WrapConfig::new(
                self.viewport_width as usize,
                &self.showbreak,
            ))
        } else {
            None
        }
    }

    /// Display height of one element, accounting for soft-wrap and showbreak.
    pub(crate) fn elem_display_height(&self, elem: &DocElement) -> usize {
        elem_height(
            elem,
            self.wrap_config().as_ref(),
            self.images.height_rows,
            Some(&self.images.elem_heights),
        )
    }

    /// Total display height of the document, accounting for soft-wrap.
    pub(crate) fn total_display_height(&self) -> usize {
        total_height(
            &self.rendered_doc.elements,
            self.wrap_config().as_ref(),
            self.images.height_rows,
            Some(&self.images.elem_heights),
        )
    }

    // --- Scrolling ---

    pub(crate) fn scroll_down(&mut self, amount: usize) {
        let max = self.total_display_height().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max);
    }

    pub(crate) fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub(crate) fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.total_display_height().saturating_sub(1);
    }

    // --- Rendering ---

    pub(crate) fn re_render(&mut self, content: &str) {
        let result =
            parser::parse_text_with_persistent_line_tracking(content, &mut self.line_tracker);
        self.rendered_doc = tui_renderer::render_ast(&result.ast, Some(self.syntax_theme.as_str()));
    }

    /// Return a reference to the currently focused item, if any.
    pub(crate) fn focused_item(&self) -> Option<&FocusableItem> {
        self.focused_item_idx
            .and_then(|idx| self.rendered_doc.focusables.get(idx))
    }

    // --- Focus ---

    /// Indices (into `rendered_doc.focusables`) of focusable items visible in the viewport.
    fn visible_focusable_indices(&self) -> Vec<usize> {
        let mut row = 0usize;
        let mut visible_elems = std::collections::HashSet::new();
        for (i, elem) in self.rendered_doc.elements.iter().enumerate() {
            let h = self.elem_display_height(elem);
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
        let mut row = 0usize;
        for elem in &self.rendered_doc.elements {
            if let DocElement::TextLine(line, _) = elem {
                let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                if text.to_lowercase().contains(&anchor_lower) {
                    self.scroll_offset = row;
                    return;
                }
            }
            row += self.elem_display_height(elem);
        }
    }

    /// Scroll to a specific source line number (0-indexed TextLine count).
    pub(crate) fn scroll_to_line(&mut self, target_line: usize) {
        let mut row = 0usize;
        let mut current_source_line = 0usize;
        for elem in &self.rendered_doc.elements {
            if current_source_line >= target_line {
                self.scroll_offset = row;
                return;
            }
            let h = self.elem_display_height(elem);
            if let DocElement::TextLine(_, _) = elem {
                current_source_line += 1;
            }
            row += h;
        }
        self.scroll_offset = row.saturating_sub(self.viewport_height);
    }

    /// Scroll to the element whose stored source row matches the given 1-indexed line number.
    /// Used for `--goto-line` (user-facing, 1-indexed).
    pub(crate) fn scroll_to_source_line(&mut self, line: usize) {
        let target_row = line.saturating_sub(1); // convert to 0-indexed
        let mut display_row = 0usize;
        for elem in &self.rendered_doc.elements {
            if let DocElement::TextLine(_, source_row) = elem {
                if *source_row >= target_row {
                    self.scroll_offset = display_row;
                    return;
                }
            }
            display_row += self.elem_display_height(elem);
        }
        self.scroll_offset = display_row.saturating_sub(self.viewport_height);
    }

    /// Return the 1-indexed source line number visible at the current scroll position.
    /// This is the viewport's top line, used for `{top_line}` in editor commands.
    pub(crate) fn source_line_at_offset(&self) -> usize {
        let mut display_row = 0usize;
        let mut last_source_row = 0usize;
        for elem in &self.rendered_doc.elements {
            if display_row > self.scroll_offset {
                break;
            }
            if let DocElement::TextLine(_, source_row) = elem {
                last_source_row = *source_row;
            }
            display_row += self.elem_display_height(elem);
        }
        last_source_row + 1 // convert to 1-indexed
    }

    /// Return the 1-indexed source line of the currently focused item (Tab-selected link/image),
    /// or `None` if nothing is focused. Used for `{line}` in editor commands.
    pub(crate) fn source_line_of_focused_item(&self) -> Option<usize> {
        let fi = self.focused_item()?;
        if let Some(DocElement::TextLine(_, source_row)) =
            self.rendered_doc.elements.get(fi.elem_idx)
        {
            Some(source_row + 1)
        } else {
            None
        }
    }

    // --- Search ---

    /// Compute per-element cumulative display row offsets.
    /// `result[i]` = display row at which element `i` starts.
    fn elem_display_offsets(&self) -> Vec<usize> {
        let mut offsets = Vec::with_capacity(self.rendered_doc.elements.len());
        let mut row = 0usize;
        for elem in &self.rendered_doc.elements {
            offsets.push(row);
            row += self.elem_display_height(elem);
        }
        offsets
    }

    /// Recompute search matches and jump scroll to the current match.
    fn refresh_search(&mut self) {
        let offsets = self.elem_display_offsets();
        if let Some(search) = &mut self.search {
            search.update_matches(&self.rendered_doc.elements, self.scroll_offset, &offsets);
        }
        self.jump_to_current_match();
    }

    /// Scroll the viewport so the current search match is visible near the top.
    fn jump_to_current_match(&mut self) {
        let match_elem_idx = self
            .search
            .as_ref()
            .and_then(|s| s.current_match())
            .map(|m| m.elem_idx);

        if let Some(elem_idx) = match_elem_idx {
            let offsets = self.elem_display_offsets();
            if let Some(&display_row) = offsets.get(elem_idx) {
                self.scroll_offset = display_row;
            }
        }
    }

    // --- Input handling ---

    /// Unified key dispatcher (Elm-like update function).
    ///
    /// Dispatches to the appropriate sub-handler based on current mode and returns
    /// an `AppAction` describing any side-effect (`main` should perform.
    pub(crate) async fn handle_key(
        &mut self,
        repository: &Repository,
        code: KeyCode,
        modifiers: KeyModifiers,
        viewport_height: usize,
        tui_config: &config::TuiConfig,
    ) -> AppAction {
        // Mode priority: backlinks popup > search input > normal
        if self.backlinks.visible {
            return self
                .handle_backlinks_key(repository, code, modifiers)
                .await;
        }

        if self.search.as_ref().map(|s| s.typing) == Some(true) {
            self.handle_search_key(code, modifiers);
            return AppAction::None;
        }

        self.handle_normal_key(repository, code, modifiers, viewport_height, tui_config)
            .await
    }

    /// Handle a key event while the backlinks popup is open.
    async fn handle_backlinks_key(
        &mut self,
        repository: &Repository,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> AppAction {
        match (code, modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) | (KeyCode::Char('b'), _) => {
                self.backlinks.close();
            }
            (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                self.backlinks.navigate_down();
            }
            (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                self.backlinks.navigate_up();
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
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return AppAction::Quit,
            _ => {}
        }
        AppAction::None
    }

    /// Handle a key event while the user is typing a search query.
    fn handle_search_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match (code, modifiers) {
            (KeyCode::Esc, _) => {
                self.search = None;
            }
            (KeyCode::Enter, _) => {
                if let Some(search) = &mut self.search {
                    search.confirm();
                }
                self.jump_to_current_match();
            }
            // Delete char before cursor: Backspace or C-h
            (KeyCode::Backspace, _) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                if let Some(search) = &mut self.search {
                    search.delete_before_cursor();
                }
                self.refresh_search();
            }
            // Delete char at cursor
            (KeyCode::Delete, _) => {
                if let Some(search) = &mut self.search {
                    search.delete_after_cursor();
                }
                self.refresh_search();
            }
            // Delete word before cursor (C-w)
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                if let Some(search) = &mut self.search {
                    search.delete_word_before_cursor();
                }
                self.refresh_search();
            }
            // Delete to start of query (C-u)
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                if let Some(search) = &mut self.search {
                    search.delete_to_start();
                }
                self.refresh_search();
            }
            // Move cursor left one char
            (KeyCode::Left, KeyModifiers::NONE) => {
                if let Some(search) = &mut self.search {
                    search.move_left();
                }
            }
            // Move cursor right one char
            (KeyCode::Right, KeyModifiers::NONE) => {
                if let Some(search) = &mut self.search {
                    search.move_right();
                }
            }
            // Move cursor left one WORD (C-Left or S-Left)
            (KeyCode::Left, KeyModifiers::CONTROL) | (KeyCode::Left, KeyModifiers::SHIFT) => {
                if let Some(search) = &mut self.search {
                    search.move_word_left();
                }
            }
            // Move cursor right one WORD (C-Right or S-Right)
            (KeyCode::Right, KeyModifiers::CONTROL) | (KeyCode::Right, KeyModifiers::SHIFT) => {
                if let Some(search) = &mut self.search {
                    search.move_word_right();
                }
            }
            // Move to start of query (C-b or Home)
            (KeyCode::Char('b'), KeyModifiers::CONTROL) | (KeyCode::Home, _) => {
                if let Some(search) = &mut self.search {
                    search.move_to_start();
                }
            }
            // Move to end of query (C-e or End)
            (KeyCode::Char('e'), KeyModifiers::CONTROL) | (KeyCode::End, _) => {
                if let Some(search) = &mut self.search {
                    search.move_to_end();
                }
            }
            (KeyCode::Down, _) => {
                if let Some(search) = &mut self.search {
                    search.next_match();
                }
                self.jump_to_current_match();
            }
            (KeyCode::Up, _) => {
                if let Some(search) = &mut self.search {
                    search.prev_match();
                }
                self.jump_to_current_match();
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                if let Some(search) = &mut self.search {
                    search.insert_at_cursor(c);
                }
                self.refresh_search();
            }
            _ => {}
        }
    }

    /// Handle a key event in normal (non-popup, non-search-input) mode.
    /// Returns an `AppAction` describing any required side-effect.
    async fn handle_normal_key(
        &mut self,
        repository: &Repository,
        code: KeyCode,
        modifiers: KeyModifiers,
        viewport_height: usize,
        tui_config: &config::TuiConfig,
    ) -> AppAction {
        match (code, modifiers) {
            // --- Editor shortcut (moved here from main.rs) ---
            (KeyCode::Char('e'), KeyModifiers::NONE) if !self.backlinks.visible => {
                let top_line = self.source_line_at_offset();
                let line = self.source_line_of_focused_item().unwrap_or(top_line);
                let file_str = self.file_path.display().to_string();
                let cmd = crate::build_editor_cmd(&tui_config.editor, &file_str, line, top_line);
                return AppAction::LaunchEditor {
                    cmd,
                    action: tui_config.editor.action.clone(),
                };
            }

            // --- Quit / Esc ---
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                if self.images.fullscreen_src.is_some() {
                    self.images.fullscreen_src = None;
                } else if self.search.is_some() {
                    // Esc clears active search results
                    self.search = None;
                } else {
                    return AppAction::Quit;
                }
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return AppAction::Quit,

            // --- Scrolling ---
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

            // --- Search ---
            (KeyCode::Char('/'), KeyModifiers::NONE) => {
                self.search = Some(SearchState::new(SearchDirection::Forward));
            }
            (KeyCode::Char('?'), _) => {
                self.search = Some(SearchState::new(SearchDirection::Backward));
            }
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                if let Some(search) = &mut self.search {
                    search.next_match();
                }
                self.jump_to_current_match();
            }
            (KeyCode::Char('N'), _) => {
                if let Some(search) = &mut self.search {
                    search.prev_match();
                }
                self.jump_to_current_match();
            }

            // --- Backlinks ---
            (KeyCode::Char('b'), _) => {
                self.backlinks.open();
                self.backlinks.refresh(repository, &self.file_path).await;
            }

            // --- Image size ---
            (KeyCode::Char('+'), _) | (KeyCode::Char('='), _) => {
                self.images.increase_height();
            }
            (KeyCode::Char('-'), _) => {
                self.images.decrease_height();
            }

            // --- Enter: activate focused item or close fullscreen ---
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
                            let target = if url.contains("://") {
                                url.clone()
                            } else {
                                let abs = self.root_dir.join(url.as_str());
                                format!("file://{}", abs.to_string_lossy())
                            };
                            let _ = open::that_detached(&target);
                        }
                    }
                }
            }

            // --- Back navigation ---
            (KeyCode::Backspace, _)
            | (KeyCode::Char('H'), _)
            | (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                if self.go_back() {
                    self.backlinks.refresh(repository, &self.file_path).await;
                }
            }

            // --- Focus ---
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.focus_next_item();
            }
            (KeyCode::BackTab, _) => {
                self.focus_prev_item();
            }

            // --- Reload / wrap ---
            (KeyCode::Char('r'), _) | (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.images.clear();
                let content = std::fs::read_to_string(&self.file_path).unwrap_or_default();
                self.re_render(&content);
            }
            (KeyCode::Char('w'), _) => {
                self.wrap = !self.wrap;
            }
            _ => {}
        }
        AppAction::None
    }
}
