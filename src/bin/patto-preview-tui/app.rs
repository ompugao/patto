use crate::backlinks::BacklinksPanel;
use crate::config;
use crate::image_cache::ImageCache;
use crate::search::{SearchDirection, SearchState};
use crate::tasks::TasksPanel;
use crate::tui_renderer::{self, DocElement, FocusableItem, LinkAction, RenderedDoc};
use crate::wrap::{elem_height, total_height, WrapConfig};
use crossterm::event::{KeyCode, KeyModifiers};
use patto::{line_tracker::LineTracker, parser, repository::Repository};
use std::path::{Path, PathBuf};

/// Side effect requested by `App::handle_key()`.
///
/// `App` mutates its own state and returns a command; `main` performs the
/// part that touches the terminal or spawns processes.
pub(crate) enum AppAction {
    None,
    Quit,
    /// Launch an external editor. The caller handles terminal suspend/quit/bg.
    LaunchEditor {
        cmd: String,
        action: config::EditorAction,
    },
}

pub(crate) struct NavigationEntry {
    pub(crate) file_path: PathBuf,
    pub(crate) scroll_offset: usize,
}

/// The view as it was when the tasks panel opened, so that `Esc` can restore
/// it without committing a navigation history entry.
pub(crate) struct TaskPreviewState {
    pub(crate) file_path: PathBuf,
    pub(crate) scroll_offset: usize,
    /// Raw file content; re-rendered on restore to avoid requiring Clone on RenderedDoc.
    pub(crate) content: String,
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
    /// Index into `rendered_doc.focusables` of the currently focused item.
    pub(crate) focused_item_idx: Option<usize>,
    pub(crate) nav_history: Vec<NavigationEntry>,
    pub(crate) images: ImageCache,
    pub(crate) backlinks: BacklinksPanel,
    pub(crate) tasks: TasksPanel,
    pub(crate) task_preview_state: Option<TaskPreviewState>,
    pub(crate) tui_config: config::TuiConfig,
    /// Active incremental search state. `None` when no search is active.
    pub(crate) search: Option<SearchState>,
}

enum Scroll {
    Down(usize),
    Up(usize),
    Top,
    Bottom,
}

fn scroll_for_key(code: KeyCode, modifiers: KeyModifiers, page: usize) -> Option<Scroll> {
    Some(match (code, modifiers) {
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => Scroll::Down(1),
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => Scroll::Up(1),
        (KeyCode::PageDown, _)
        | (KeyCode::Char(' '), _)
        | (KeyCode::Char('f'), KeyModifiers::CONTROL) => Scroll::Down(page),
        (KeyCode::PageUp, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => Scroll::Up(page),
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => Scroll::Down(page / 2),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => Scroll::Up(page / 2),
        (KeyCode::Char('g'), _) | (KeyCode::Home, _) => Scroll::Top,
        (KeyCode::Char('G'), _) | (KeyCode::End, _) => Scroll::Bottom,
        _ => return None,
    })
}

/// What a key does to the search prompt, and what has to follow it.
enum SearchKey {
    Cancel,
    Confirm,
    /// Changes the query, so the matches are recomputed.
    EditQuery(fn(&mut SearchState)),
    MoveCursor(fn(&mut SearchState)),
    /// Moves between matches, so the view jumps to the current one.
    StepMatch(fn(&mut SearchState)),
    Insert(char),
}

fn search_key(code: KeyCode, modifiers: KeyModifiers) -> Option<SearchKey> {
    use SearchKey::*;
    Some(match (code, modifiers) {
        (KeyCode::Esc, _) => Cancel,
        (KeyCode::Enter, _) => Confirm,
        (KeyCode::Backspace, _) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
            EditQuery(SearchState::delete_before_cursor)
        }
        (KeyCode::Delete, _) => EditQuery(SearchState::delete_after_cursor),
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
            EditQuery(SearchState::delete_word_before_cursor)
        }
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => EditQuery(SearchState::delete_to_start),
        (KeyCode::Left, KeyModifiers::NONE) => MoveCursor(SearchState::move_left),
        (KeyCode::Right, KeyModifiers::NONE) => MoveCursor(SearchState::move_right),
        (KeyCode::Left, KeyModifiers::CONTROL) | (KeyCode::Left, KeyModifiers::SHIFT) => {
            MoveCursor(SearchState::move_word_left)
        }
        (KeyCode::Right, KeyModifiers::CONTROL) | (KeyCode::Right, KeyModifiers::SHIFT) => {
            MoveCursor(SearchState::move_word_right)
        }
        (KeyCode::Char('b'), KeyModifiers::CONTROL) | (KeyCode::Home, _) => {
            MoveCursor(SearchState::move_to_start)
        }
        (KeyCode::Char('e'), KeyModifiers::CONTROL) | (KeyCode::End, _) => {
            MoveCursor(SearchState::move_to_end)
        }
        (KeyCode::Down, _) => StepMatch(SearchState::next_match),
        (KeyCode::Up, _) => StepMatch(SearchState::prev_match),
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            Insert(c)
        }
        _ => return None,
    })
}

fn open_url(root_dir: &Path, url: &str) {
    let target = if url.contains("://") {
        url.to_string()
    } else {
        format!("file://{}", root_dir.join(url).to_string_lossy())
    };
    let _ = open::that_detached(&target);
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
                anchors: std::collections::HashMap::new(),
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
            tasks: TasksPanel::new(),
            task_preview_state: None,
            tui_config: config::TuiConfig::default(),
            search: None,
        }
    }

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

    fn scroll(&mut self, scroll: Scroll) {
        match scroll {
            Scroll::Down(rows) => self.scroll_down(rows),
            Scroll::Up(rows) => self.scroll_up(rows),
            Scroll::Top => self.scroll_to_top(),
            Scroll::Bottom => self.scroll_to_bottom(),
        }
    }

    pub(crate) fn re_render(&mut self, content: &str) {
        let result =
            parser::parse_text_with_persistent_line_tracking(content, &mut self.line_tracker);
        self.rendered_doc =
            tui_renderer::render_ast(&result.ast, Some(self.tui_config.syntax_theme.as_str()));
    }

    pub(crate) fn focused_item(&self) -> Option<&FocusableItem> {
        self.focused_item_idx
            .and_then(|idx| self.rendered_doc.focusables.get(idx))
    }

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

    fn push_history(&mut self) {
        self.nav_history.push(NavigationEntry {
            file_path: self.file_path.clone(),
            scroll_offset: self.scroll_offset,
        });
    }

    /// Show `content` as `path`, scrolled to the top.
    fn show_file(&mut self, path: PathBuf, content: &str) {
        self.file_path = path;
        self.scroll_offset = 0;
        self.focused_item_idx = None;
        self.images.fullscreen_src = None;
        self.re_render(content);
    }

    /// Navigate to a wiki-linked note, recording the current view in history.
    pub(crate) fn open_note(&mut self, name: &str, anchor: Option<&str>) -> bool {
        let target_path = if name.ends_with(".pn") {
            self.root_dir.join(name)
        } else {
            self.root_dir.join(format!("{}.pn", name))
        };

        if !target_path.exists() || !target_path.is_file() {
            return false;
        }

        let Ok(content) = std::fs::read_to_string(&target_path) else {
            return false;
        };

        self.push_history();
        self.show_file(target_path, &content);

        if let Some(anchor_text) = anchor {
            self.scroll_to_anchor(anchor_text);
        }

        true
    }

    pub(crate) fn go_back(&mut self) -> bool {
        let Some(entry) = self.nav_history.pop() else {
            return false;
        };
        let Ok(content) = std::fs::read_to_string(&entry.file_path) else {
            return false;
        };
        self.show_file(entry.file_path, &content);
        self.scroll_offset = entry.scroll_offset;
        true
    }

    /// Scroll to the element defining `anchor`, or failing that to the first
    /// line that starts with the anchor text.
    fn scroll_to_anchor(&mut self, anchor: &str) {
        let anchor_lower = anchor.to_lowercase();

        if let Some(&elem_idx) = self.rendered_doc.anchors.get(&anchor_lower) {
            let mut row = 0usize;
            for (i, elem) in self.rendered_doc.elements.iter().enumerate() {
                if i == elem_idx {
                    self.scroll_offset = row;
                    return;
                }
                row += self.elem_display_height(elem);
            }
        }

        // Requiring the anchor at the start of the content skips the
        // self-links that mention it.
        let mut row = 0usize;
        for elem in &self.rendered_doc.elements {
            if let DocElement::TextLine(line, _) = elem {
                let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                if text.trim_start().to_lowercase().starts_with(&anchor_lower) {
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
        let target_row = line.saturating_sub(1);
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

    /// The 1-indexed source line at the top of the viewport, used for
    /// `{top_line}` in editor commands.
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
        last_source_row + 1
    }

    /// The 1-indexed source line of the focused item, used for `{line}` in
    /// editor commands.
    pub(crate) fn source_line_of_focused_item(&self) -> Option<usize> {
        let fi = self.focused_item()?;
        match self.rendered_doc.elements.get(fi.elem_idx) {
            Some(DocElement::TextLine(_, source_row)) => Some(source_row + 1),
            _ => None,
        }
    }

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

    fn edit_search(&mut self, edit: impl FnOnce(&mut SearchState)) {
        if let Some(search) = &mut self.search {
            edit(search);
        }
    }

    /// Dispatch a key to the handler for the current mode. Modes take
    /// priority in the order tasks panel, backlinks popup, search prompt,
    /// normal view.
    pub(crate) async fn handle_key(
        &mut self,
        repository: &Repository,
        code: KeyCode,
        modifiers: KeyModifiers,
        viewport_height: usize,
    ) -> AppAction {
        if self.tasks.visible {
            return self.handle_tasks_key(repository, code, modifiers);
        }

        if self.backlinks.visible {
            return self.handle_backlinks_key(repository, code, modifiers).await;
        }

        if self.search.as_ref().map(|s| s.typing) == Some(true) {
            self.handle_search_key(code, modifiers);
            return AppAction::None;
        }

        self.handle_normal_key(repository, code, modifiers, viewport_height)
            .await
    }

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

    fn handle_tasks_key(
        &mut self,
        repository: &Repository,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> AppAction {
        match (code, modifiers) {
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) | (KeyCode::Char('T'), _) => {
                self.close_tasks_panel();
            }
            (KeyCode::Char('r'), KeyModifiers::NONE) | (KeyCode::Char('R'), _) => {
                self.tasks.toggle_view();
                self.tasks.refresh_review(repository);
            }
            (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                self.tasks.navigate_down();
                self.load_task_preview();
            }
            (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                self.tasks.navigate_up();
                self.load_task_preview();
            }
            (KeyCode::Enter, _) => self.commit_task_selection(),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return AppAction::Quit,
            _ => {}
        }
        AppAction::None
    }

    /// Open the tasks panel over a snapshot of the current view, and preview
    /// the first task's file behind it.
    pub(crate) async fn open_tasks_panel(&mut self, repository: &Repository) {
        self.backlinks.close();
        let saved_content = std::fs::read_to_string(&self.file_path).unwrap_or_default();
        self.task_preview_state = Some(TaskPreviewState {
            file_path: self.file_path.clone(),
            scroll_offset: self.scroll_offset,
            content: saved_content,
        });
        self.tasks.refresh(repository);
        self.tasks.open();
        self.load_task_preview();
    }

    /// Restore the view snapshot saved when the tasks panel was opened.
    fn close_tasks_panel(&mut self) {
        if let Some(state) = self.task_preview_state.take() {
            self.file_path = state.file_path;
            self.scroll_offset = state.scroll_offset;
            self.re_render(&state.content);
        }
        self.tasks.close();
    }

    /// Show the selected task's file behind the panel, without touching the
    /// navigation history.
    fn load_task_preview(&mut self) {
        let Some((uri, line)) = self.tasks.resolve_cursor() else {
            return;
        };
        let Ok(path) = uri.to_file_path() else {
            return;
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            return;
        };
        self.file_path = path;
        self.re_render(&content);
        self.scroll_to_line(line);
    }

    /// Leave the panel on the selected task, keeping it as the current view.
    fn commit_task_selection(&mut self) {
        let Some((uri, line)) = self.tasks.resolve_cursor() else {
            return;
        };
        self.task_preview_state = None;
        self.tasks.close();
        let Ok(path) = uri.to_file_path() else {
            return;
        };
        if path != self.file_path {
            let Ok(content) = std::fs::read_to_string(&path) else {
                return;
            };
            self.push_history();
            self.file_path = path;
            self.re_render(&content);
        }
        self.scroll_to_line(line);
    }

    fn handle_search_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let Some(key) = search_key(code, modifiers) else {
            return;
        };
        match key {
            SearchKey::Cancel => self.search = None,
            SearchKey::Confirm => {
                self.edit_search(SearchState::confirm);
                self.jump_to_current_match();
            }
            SearchKey::EditQuery(edit) => {
                self.edit_search(edit);
                self.refresh_search();
            }
            SearchKey::MoveCursor(edit) => self.edit_search(edit),
            SearchKey::StepMatch(edit) => {
                self.edit_search(edit);
                self.jump_to_current_match();
            }
            SearchKey::Insert(c) => {
                self.edit_search(|search| search.insert_at_cursor(c));
                self.refresh_search();
            }
        }
    }

    async fn handle_normal_key(
        &mut self,
        repository: &Repository,
        code: KeyCode,
        modifiers: KeyModifiers,
        viewport_height: usize,
    ) -> AppAction {
        if let Some(scroll) = scroll_for_key(code, modifiers, viewport_height) {
            self.scroll(scroll);
            return AppAction::None;
        }
        match (code, modifiers) {
            (KeyCode::Char('e'), KeyModifiers::NONE) => return self.launch_editor(),
            (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => return self.escape_or_quit(),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return AppAction::Quit,
            (KeyCode::Char('/'), KeyModifiers::NONE) => {
                self.search = Some(SearchState::new(SearchDirection::Forward));
            }
            (KeyCode::Char('?'), _) => {
                self.search = Some(SearchState::new(SearchDirection::Backward));
            }
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                self.edit_search(SearchState::next_match);
                self.jump_to_current_match();
            }
            (KeyCode::Char('N'), _) => {
                self.edit_search(SearchState::prev_match);
                self.jump_to_current_match();
            }
            (KeyCode::Char('b'), _) => {
                self.backlinks.open();
                self.backlinks.refresh(repository, &self.file_path).await;
            }
            (KeyCode::Char('T'), _) => self.open_tasks_panel(repository).await,
            (KeyCode::Char('+'), _) | (KeyCode::Char('='), _) => self.images.increase_height(),
            (KeyCode::Char('-'), _) => self.images.decrease_height(),
            (KeyCode::Enter, _) => self.activate_focused_item(repository).await,
            (KeyCode::Backspace, _)
            | (KeyCode::Char('H'), _)
            | (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                if self.go_back() {
                    self.backlinks.refresh(repository, &self.file_path).await;
                }
            }
            (KeyCode::Tab, KeyModifiers::NONE) => self.focus_next_item(),
            (KeyCode::BackTab, _) => self.focus_prev_item(),
            (KeyCode::Char('r'), _) | (KeyCode::Char('l'), KeyModifiers::CONTROL) => self.reload(),
            (KeyCode::Char('w'), _) => self.wrap = !self.wrap,
            _ => {}
        }
        AppAction::None
    }

    fn launch_editor(&self) -> AppAction {
        let top_line = self.source_line_at_offset();
        let line = self.source_line_of_focused_item().unwrap_or(top_line);
        let file = self.file_path.display().to_string();
        AppAction::LaunchEditor {
            cmd: crate::build_editor_cmd(&self.tui_config.editor, &file, line, top_line),
            action: self.tui_config.editor.action.clone(),
        }
    }

    /// Esc closes the fullscreen image first, then clears the search, and
    /// only then quits.
    fn escape_or_quit(&mut self) -> AppAction {
        if self.images.fullscreen_src.is_some() {
            self.images.fullscreen_src = None;
        } else if self.search.is_some() {
            self.search = None;
        } else {
            return AppAction::Quit;
        }
        AppAction::None
    }

    async fn activate_focused_item(&mut self, repository: &Repository) {
        if self.images.fullscreen_src.is_some() {
            self.images.fullscreen_src = None;
            return;
        }
        let Some(item) = self.focused_item().cloned() else {
            return;
        };
        match item.action {
            LinkAction::ViewImage(src) => self.images.fullscreen_src = Some(src),
            LinkAction::OpenNote { name, anchor } => {
                if self.open_note(&name, anchor.as_deref()) {
                    self.backlinks.refresh(repository, &self.file_path).await;
                }
            }
            LinkAction::JumpToAnchor { anchor } => {
                self.push_history();
                self.scroll_to_anchor(&anchor);
            }
            LinkAction::OpenUrl(url) => open_url(&self.root_dir, &url),
        }
    }

    fn reload(&mut self) {
        self.images.clear();
        let content = std::fs::read_to_string(&self.file_path).unwrap_or_default();
        self.re_render(&content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::{TaskEntry, TaskItem};
    use chrono::TimeDelta;
    use patto::parser::TaskStatus;
    use patto::tasks_view::PendingGroup;
    use tempfile::TempDir;
    use tower_lsp::lsp_types::Url;

    struct Workspace {
        dir: TempDir,
        repository: Repository,
    }

    impl Workspace {
        fn with_notes(notes: &[(&str, &str)]) -> Self {
            let dir = tempfile::tempdir().unwrap();
            for (name, content) in notes {
                std::fs::write(dir.path().join(name), content).unwrap();
            }
            let repository = Repository::new(dir.path().to_path_buf());
            Self { dir, repository }
        }

        fn open(&self, name: &str) -> App {
            let path = self.dir.path().join(name);
            let content = std::fs::read_to_string(&path).unwrap();
            let mut app = App::new(path, self.dir.path().to_path_buf(), None);
            app.viewport_width = 60;
            app.viewport_height = 10;
            app.re_render(&content);
            app
        }

        async fn press(&self, app: &mut App, code: KeyCode) -> AppAction {
            self.key(app, code, KeyModifiers::NONE).await
        }

        async fn key(&self, app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> AppAction {
            app.handle_key(&self.repository, code, modifiers, app.viewport_height)
                .await
        }
    }

    fn forty_lines() -> String {
        (0..40).map(|i| format!("line {i}\n")).collect()
    }

    #[tokio::test]
    async fn j_and_k_scroll_one_row() {
        let ws = Workspace::with_notes(&[("a.pn", &forty_lines())]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Char('j')).await;
        ws.press(&mut app, KeyCode::Char('j')).await;
        assert_eq!(app.scroll_offset, 2);
        ws.press(&mut app, KeyCode::Char('k')).await;
        assert_eq!(app.scroll_offset, 1);
    }

    #[tokio::test]
    async fn page_and_half_page_keys_scroll_by_the_viewport() {
        let ws = Workspace::with_notes(&[("a.pn", &forty_lines())]);
        let mut app = ws.open("a.pn");
        ws.key(&mut app, KeyCode::Char('f'), KeyModifiers::CONTROL)
            .await;
        assert_eq!(app.scroll_offset, 10);
        ws.key(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL)
            .await;
        assert_eq!(app.scroll_offset, 5);
    }

    #[tokio::test]
    async fn g_and_shift_g_jump_to_the_ends() {
        let ws = Workspace::with_notes(&[("a.pn", &forty_lines())]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Char('G')).await;
        assert_eq!(app.scroll_offset, app.total_display_height() - 1);
        ws.press(&mut app, KeyCode::Char('g')).await;
        assert_eq!(app.scroll_offset, 0);
    }

    #[tokio::test]
    async fn q_quits_and_ctrl_c_quits() {
        let ws = Workspace::with_notes(&[("a.pn", "x\n")]);
        let mut app = ws.open("a.pn");
        assert!(matches!(
            ws.press(&mut app, KeyCode::Char('q')).await,
            AppAction::Quit
        ));
        assert!(matches!(
            ws.key(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL)
                .await,
            AppAction::Quit
        ));
    }

    #[tokio::test]
    async fn slash_opens_a_search_and_typing_jumps_to_the_first_match() {
        let ws = Workspace::with_notes(&[("a.pn", &forty_lines())]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Char('/')).await;
        for c in "line 2".chars() {
            ws.press(&mut app, KeyCode::Char(c)).await;
        }
        let search = app.search.as_ref().unwrap();
        assert!(search.typing);
        assert_eq!(search.query, "line 2");
        assert_eq!(app.scroll_offset, 2);
    }

    #[tokio::test]
    async fn enter_confirms_the_search_and_n_steps_through_matches() {
        let ws = Workspace::with_notes(&[("a.pn", &forty_lines())]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Char('/')).await;
        for c in "line 2".chars() {
            ws.press(&mut app, KeyCode::Char(c)).await;
        }
        ws.press(&mut app, KeyCode::Enter).await;
        assert!(!app.search.as_ref().unwrap().typing);
        ws.press(&mut app, KeyCode::Char('n')).await;
        assert_eq!(app.scroll_offset, 20);
        ws.press(&mut app, KeyCode::Char('N')).await;
        assert_eq!(app.scroll_offset, 2);
    }

    #[tokio::test]
    async fn escape_clears_a_finished_search_before_it_quits() {
        let ws = Workspace::with_notes(&[("a.pn", "x\n")]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Char('/')).await;
        ws.press(&mut app, KeyCode::Char('x')).await;
        ws.press(&mut app, KeyCode::Enter).await;
        assert!(matches!(
            ws.press(&mut app, KeyCode::Esc).await,
            AppAction::None
        ));
        assert!(app.search.is_none());
        assert!(matches!(
            ws.press(&mut app, KeyCode::Esc).await,
            AppAction::Quit
        ));
    }

    #[tokio::test]
    async fn backspace_in_the_prompt_edits_the_query() {
        let ws = Workspace::with_notes(&[("a.pn", "x\n")]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Char('/')).await;
        ws.press(&mut app, KeyCode::Char('a')).await;
        ws.press(&mut app, KeyCode::Char('b')).await;
        ws.press(&mut app, KeyCode::Backspace).await;
        assert_eq!(app.search.as_ref().unwrap().query, "a");
    }

    #[tokio::test]
    async fn w_toggles_wrapping_and_plus_minus_resize_images() {
        let ws = Workspace::with_notes(&[("a.pn", "x\n")]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Char('w')).await;
        assert!(!app.wrap);
        let rows = app.images.height_rows;
        ws.press(&mut app, KeyCode::Char('+')).await;
        assert_eq!(app.images.height_rows, rows + 5);
        ws.press(&mut app, KeyCode::Char('-')).await;
        assert_eq!(app.images.height_rows, rows);
    }

    #[tokio::test]
    async fn tab_focuses_a_link_and_enter_opens_the_note() {
        let ws = Workspace::with_notes(&[("a.pn", "see [b]\n"), ("b.pn", "target\n")]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Tab).await;
        assert!(matches!(
            app.focused_item().map(|f| &f.action),
            Some(LinkAction::OpenNote { name, .. }) if name == "b"
        ));
        ws.press(&mut app, KeyCode::Enter).await;
        assert_eq!(app.file_path, ws.dir.path().join("b.pn"));
        assert_eq!(app.nav_history.len(), 1);
    }

    #[tokio::test]
    async fn backspace_returns_to_the_previous_note_and_scroll_position() {
        let ws = Workspace::with_notes(&[
            ("a.pn", &format!("{}[b]\n", forty_lines())),
            ("b.pn", "target\n"),
        ]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Char('G')).await;
        let bottom = app.scroll_offset;
        ws.press(&mut app, KeyCode::Tab).await;
        ws.press(&mut app, KeyCode::Enter).await;
        assert_eq!(app.file_path, ws.dir.path().join("b.pn"));
        ws.press(&mut app, KeyCode::Backspace).await;
        assert_eq!(app.file_path, ws.dir.path().join("a.pn"));
        assert_eq!(app.scroll_offset, bottom);
    }

    #[tokio::test]
    async fn e_asks_for_the_editor_at_the_top_line() {
        let ws = Workspace::with_notes(&[("a.pn", &forty_lines())]);
        let mut app = ws.open("a.pn");
        app.tui_config.editor.cmd = Some("edit {file}:{line}:{top_line}".to_string());
        ws.press(&mut app, KeyCode::Char('j')).await;
        ws.press(&mut app, KeyCode::Char('j')).await;
        let AppAction::LaunchEditor { cmd, .. } = ws.press(&mut app, KeyCode::Char('e')).await
        else {
            panic!("expected an editor launch");
        };
        assert_eq!(
            cmd,
            format!("edit {}:3:3", ws.dir.path().join("a.pn").display())
        );
    }

    fn task_in(ws: &Workspace, name: &str, line: usize) -> TaskEntry {
        TaskEntry::Item(TaskItem {
            text: "task".to_string(),
            file_name: name.to_string(),
            uri: Url::from_file_path(ws.dir.path().join(name)).unwrap(),
            line,
            due_str: String::new(),
            group: PendingGroup::Later,
            status: TaskStatus::Todo,
            base_time_spent: TimeDelta::zero(),
            started_at_dt: None,
        })
    }

    #[tokio::test]
    async fn the_tasks_panel_previews_the_selected_task_and_escape_restores_the_view() {
        let ws = Workspace::with_notes(&[("a.pn", &forty_lines()), ("b.pn", "x\ntask here\n")]);
        let mut app = ws.open("a.pn");
        ws.press(&mut app, KeyCode::Char('j')).await;
        ws.press(&mut app, KeyCode::Char('T')).await;
        app.tasks.entries = vec![task_in(&ws, "b.pn", 1)];
        app.tasks.list_state.select(Some(0));
        ws.press(&mut app, KeyCode::Char('j')).await;
        assert_eq!(app.file_path, ws.dir.path().join("b.pn"));
        ws.press(&mut app, KeyCode::Esc).await;
        assert_eq!(app.file_path, ws.dir.path().join("a.pn"));
        assert_eq!(app.scroll_offset, 1);
        assert!(!app.tasks.visible);
    }
}
