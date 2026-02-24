use clap::Parser;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use patto::{
    line_tracker::LineTracker,
    parser,
    repository::{BackLinkData, Repository, RepositoryMessage},
    tui_renderer::{self, DocElement, FocusableItem, LinkAction, RenderedDoc},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// --- CLI args ---

#[derive(Parser, Debug)]
#[command(author, version, about = "Terminal preview for .pn (patto) files")]
struct Args {
    /// Path to the .pn file to preview
    file: String,

    /// Workspace directory (defaults to file's parent directory)
    #[arg(short, long)]
    dir: Option<String>,
}

// --- App state ---

/// Saved navigation state for back-navigation.
struct NavigationEntry {
    file_path: PathBuf,
    scroll_offset: usize,
}

struct App {
    file_path: PathBuf,
    /// Workspace root directory.
    root_dir: PathBuf,
    rendered_doc: RenderedDoc,
    scroll_offset: usize,
    viewport_height: usize,
    show_backlinks: bool,
    back_links: Vec<BackLinkData>,
    two_hop_links: Vec<(String, Vec<String>)>,
    image_cache: HashMap<String, CachedImage>,
    picker: Option<Picker>,
    line_tracker: LineTracker,
    image_height_rows: u16,
    /// Source of the image currently shown fullscreen (None = normal view).
    fullscreen_image: Option<String>,
    /// Index into `rendered_doc.focusables` of the currently focused item. None = no focus.
    focused_item_idx: Option<usize>,
    /// Navigation history for back-navigation.
    nav_history: Vec<NavigationEntry>,
    /// Cursor position within the backlinks popup. None = no selection.
    backlink_cursor: Option<usize>,
}

enum CachedImage {
    Loaded(StatefulProtocol),
    Failed(String),
}

impl App {
    fn new(file_path: PathBuf, root_dir: PathBuf) -> Self {
        let picker = Picker::from_query_stdio().ok();

        Self {
            file_path,
            root_dir,
            rendered_doc: RenderedDoc {
                elements: Vec::new(),
                focusables: Vec::new(),
            },
            scroll_offset: 0,
            viewport_height: 24,
            show_backlinks: false,
            back_links: Vec::new(),
            two_hop_links: Vec::new(),
            image_cache: HashMap::new(),
            picker,
            line_tracker: LineTracker::new().expect("Failed to create line tracker"),
            image_height_rows: 10,
            fullscreen_image: None,
            focused_item_idx: None,
            nav_history: Vec::new(),
            backlink_cursor: None,
        }
    }

    fn scroll_down(&mut self, amount: usize) {
        let max = self
            .rendered_doc
            .total_height(self.image_height_rows)
            .saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max);
    }

    fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self
            .rendered_doc
            .total_height(self.image_height_rows)
            .saturating_sub(1);
    }

    fn re_render(&mut self, content: &str) {
        let result =
            parser::parse_text_with_persistent_line_tracking(content, &mut self.line_tracker);
        self.rendered_doc = tui_renderer::render_ast(&result.ast);
    }

    fn load_image(&mut self, src: &str, root_dir: &Path) {
        if self.image_cache.contains_key(src) || self.picker.is_none() {
            return;
        }
        if src.starts_with("http://") || src.starts_with("https://") {
            match reqwest::blocking::get(src) {
                Ok(resp) => match resp.bytes() {
                    Ok(bytes) => match image::load_from_memory(&bytes) {
                        Ok(img) => {
                            let protocol = self.picker.as_mut().unwrap().new_resize_protocol(img);
                            self.image_cache
                                .insert(src.to_string(), CachedImage::Loaded(protocol));
                        }
                        Err(e) => {
                            self.image_cache.insert(
                                src.to_string(),
                                CachedImage::Failed(format!("decode error: {}", e)),
                            );
                        }
                    },
                    Err(e) => {
                        self.image_cache.insert(
                            src.to_string(),
                            CachedImage::Failed(format!("fetch error: {}", e)),
                        );
                    }
                },
                Err(e) => {
                    self.image_cache.insert(
                        src.to_string(),
                        CachedImage::Failed(format!("fetch error: {}", e)),
                    );
                }
            }
            return;
        }

        let path = root_dir.join(src);

        match image::open(&path) {
            Ok(img) => {
                let protocol = self.picker.as_mut().unwrap().new_resize_protocol(img);
                self.image_cache
                    .insert(src.to_string(), CachedImage::Loaded(protocol));
            }
            Err(e) => {
                self.image_cache
                    .insert(src.to_string(), CachedImage::Failed(e.to_string()));
            }
        }
    }

    fn clear_image_cache(&mut self) {
        self.image_cache.clear();
    }

    fn increase_image_height(&mut self) {
        self.image_height_rows = (self.image_height_rows + 5).min(60);
        self.image_cache.clear();
    }

    fn decrease_image_height(&mut self) {
        self.image_height_rows = (self.image_height_rows.saturating_sub(5)).max(5);
        self.image_cache.clear();
    }

    /// Indices (into `rendered_doc.focusables`) of focusable items visible in the viewport.
    fn visible_focusable_indices(&self) -> Vec<usize> {
        let img_h = self.image_height_rows;
        // Build a set of visible element indices
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
    fn focus_next_item(&mut self) {
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
    fn focus_prev_item(&mut self) {
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

    /// Return a reference to the currently focused item, if any.
    fn focused_item(&self) -> Option<&FocusableItem> {
        self.focused_item_idx
            .and_then(|idx| self.rendered_doc.focusables.get(idx))
    }

    /// Clear focus if the focused item is no longer visible.
    fn clear_stale_focus(&mut self) {
        if let Some(idx) = self.focused_item_idx {
            let visible = self.visible_focusable_indices();
            if !visible.contains(&idx) {
                self.focused_item_idx = None;
            }
        }
    }

    /// Navigate to a wiki-linked note. Saves current state in history.
    fn open_note(&mut self, name: &str, anchor: Option<&str>) -> bool {
        // Resolve the note name to a file path
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

        // Save current state
        self.nav_history.push(NavigationEntry {
            file_path: self.file_path.clone(),
            scroll_offset: self.scroll_offset,
        });

        // Switch to new file
        self.file_path = target_path;
        self.scroll_offset = 0;
        self.focused_item_idx = None;
        self.fullscreen_image = None;
        self.re_render(&content);

        // If anchor specified, try to scroll to it
        if let Some(anchor_text) = anchor {
            self.scroll_to_anchor(anchor_text);
        }

        true
    }

    /// Navigate to a file by path. Saves current state in history.
    #[allow(dead_code)]
    fn open_file(&mut self, path: &Path) -> bool {
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
        self.fullscreen_image = None;
        self.re_render(&content);
        true
    }

    /// Go back in navigation history.
    fn go_back(&mut self) -> bool {
        if let Some(entry) = self.nav_history.pop() {
            let content = match std::fs::read_to_string(&entry.file_path) {
                Ok(c) => c,
                Err(_) => return false,
            };
            self.file_path = entry.file_path;
            self.scroll_offset = entry.scroll_offset;
            self.focused_item_idx = None;
            self.fullscreen_image = None;
            self.re_render(&content);
            true
        } else {
            false
        }
    }

    /// Try to scroll to a heading/anchor matching the given text.
    fn scroll_to_anchor(&mut self, anchor: &str) {
        let anchor_lower = anchor.to_lowercase();
        let img_h = self.image_height_rows;
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
        let img_h = self.image_height_rows;
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
        // If target line beyond content, scroll to end
        self.scroll_offset = row.saturating_sub(self.viewport_height);
    }

    /// Count total selectable entries in the backlinks popup.
    fn backlink_entry_count(&self) -> usize {
        let bl_count: usize = self.back_links.iter().map(|bl| bl.locations.len()).sum();
        let th_count: usize = self
            .two_hop_links
            .iter()
            .map(|(_, links)| links.len())
            .sum();
        bl_count + th_count
    }

    /// Move the backlink cursor down.
    fn backlink_cursor_down(&mut self) {
        let total = self.backlink_entry_count();
        if total == 0 {
            return;
        }
        self.backlink_cursor = Some(match self.backlink_cursor {
            Some(c) => (c + 1).min(total - 1),
            None => 0,
        });
    }

    /// Move the backlink cursor up.
    fn backlink_cursor_up(&mut self) {
        let total = self.backlink_entry_count();
        if total == 0 {
            return;
        }
        self.backlink_cursor = Some(match self.backlink_cursor {
            Some(c) => c.saturating_sub(1),
            None => 0,
        });
    }

    /// Resolve the current backlink cursor to a navigation target (file_name, line).
    fn resolve_backlink_cursor(&self) -> Option<(String, usize)> {
        let cursor = self.backlink_cursor?;
        let mut idx = 0;
        // First: backlink entries
        for bl in &self.back_links {
            for loc in &bl.locations {
                if idx == cursor {
                    return Some((bl.source_file.clone(), loc.line));
                }
                idx += 1;
            }
        }
        // Second: two-hop link entries
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

fn draw(frame: &mut Frame, app: &mut App, root_dir: &Path) {
    // Fullscreen image overlay
    if let Some(ref src) = app.fullscreen_image.clone() {
        draw_fullscreen_image(frame, app, root_dir, src);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(1),    // content
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    draw_title_bar(frame, chunks[0], app);
    draw_content(frame, chunks[1], app, root_dir);
    draw_status_bar(frame, chunks[2], app);

    if app.show_backlinks {
        draw_backlinks_popup(frame, app);
    }
}

fn draw_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let file_name = app
        .file_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let total = app.rendered_doc.total_height(app.image_height_rows);
    let pos = if total > 0 {
        format!("{}/{}", app.scroll_offset + 1, total)
    } else {
        "0/0".to_string()
    };

    let title = Line::from(vec![
        Span::styled(
            " patto-preview-tui ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(file_name, Style::default().fg(Color::White)),
        Span::raw(" "),
        Span::styled(pos, Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(title), area);
}

/// Produce a new Line with chars in [char_start, char_end) highlighted with reverse video.
fn highlight_line_range(line: &Line<'static>, char_start: usize, char_end: usize) -> Line<'static> {
    let mut new_spans: Vec<Span<'static>> = Vec::new();
    let mut pos = 0usize;
    for span in line.spans.iter() {
        let span_len = span.content.chars().count();
        let span_start = pos;
        let span_end = pos + span_len;
        pos = span_end;

        if span_end <= char_start || span_start >= char_end {
            // Entirely outside highlight range
            new_spans.push(span.clone());
        } else if span_start >= char_start && span_end <= char_end {
            // Entirely inside highlight range
            new_spans.push(Span::styled(
                span.content.clone(),
                span.style.bg(Color::Yellow).fg(Color::Black),
            ));
        } else {
            // Partially overlapping — split the span
            let chars: Vec<char> = span.content.chars().collect();
            let hl_start = char_start.saturating_sub(span_start);
            let hl_end = (char_end - span_start).min(span_len);

            if hl_start > 0 {
                let before: String = chars[..hl_start].iter().collect();
                new_spans.push(Span::styled(before, span.style));
            }
            let mid: String = chars[hl_start..hl_end].iter().collect();
            new_spans.push(Span::styled(
                mid,
                span.style.bg(Color::Yellow).fg(Color::Black),
            ));
            if hl_end < span_len {
                let after: String = chars[hl_end..].iter().collect();
                new_spans.push(Span::styled(after, span.style));
            }
        }
    }
    Line::from(new_spans)
}

fn draw_content(frame: &mut Frame, area: Rect, app: &mut App, root_dir: &Path) {
    let height = area.height as usize;
    let img_h = app.image_height_rows;

    // Update viewport height for visible-image logic
    app.viewport_height = height;
    app.clear_stale_focus();

    // Skip elements until we reach scroll_offset rows
    let mut skip_rows = app.scroll_offset;
    let mut start_elem = 0usize;
    for (i, elem) in app.rendered_doc.elements.iter().enumerate() {
        let h = elem.height(img_h) as usize;
        if skip_rows >= h {
            skip_rows -= h;
            start_elem = i + 1;
        } else {
            start_elem = i;
            break;
        }
    }

    // Pre-load images that will be visible (only scan viewport-worth of elements)
    let mut scan_rows = 0usize;
    let image_srcs: Vec<String> = app
        .rendered_doc
        .elements
        .iter()
        .skip(start_elem)
        .take_while(|elem| {
            let h = elem.height(img_h) as usize;
            scan_rows += h;
            scan_rows <= height + img_h as usize
        })
        .filter_map(|elem| {
            if let DocElement::Image { src, .. } = elem {
                Some(src.clone())
            } else {
                None
            }
        })
        .collect();
    for src in &image_srcs {
        app.load_image(src, root_dir);
    }

    // Render visible elements
    // Determine which element index is focused and get char range for text highlights
    let (focused_elem_idx, focused_char_range) = match app.focused_item() {
        Some(fi) => (Some(fi.elem_idx), Some((fi.char_start, fi.char_end))),
        None => (None, None),
    };
    let mut y = 0usize;
    for (elem_idx, elem) in app
        .rendered_doc
        .elements
        .iter()
        .enumerate()
        .skip(start_elem)
    {
        if y >= height {
            break;
        }
        let is_focused = focused_elem_idx == Some(elem_idx);
        match elem {
            DocElement::TextLine(line) => {
                let line_area = Rect::new(area.x, area.y + y as u16, area.width, 1);
                if is_focused {
                    if let Some((cs, ce)) = focused_char_range {
                        // Highlight the focused span within the line
                        let highlighted = highlight_line_range(line, cs, ce);
                        frame.render_widget(Paragraph::new(highlighted), line_area);
                    } else {
                        frame.render_widget(Paragraph::new(line.clone()), line_area);
                    }
                } else {
                    frame.render_widget(Paragraph::new(line.clone()), line_area);
                }
                y += 1;
            }
            DocElement::Spacer => {
                y += 1;
            }
            DocElement::Image { src, alt } => {
                let elem_h = elem.height(img_h).min((height - y) as u16);
                let img_area = Rect::new(area.x, area.y + y as u16, area.width, elem_h);

                if is_focused && elem_h >= 3 {
                    // Draw a highlight border around the focused image
                    let border = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow))
                        .title(Span::styled(
                            " Enter:fullscreen ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ));
                    let inner = border.inner(img_area);
                    frame.render_widget(border, img_area);

                    match app.image_cache.get_mut(src.as_str()) {
                        Some(CachedImage::Loaded(protocol)) => {
                            let image_widget = StatefulImage::default();
                            frame.render_stateful_widget(image_widget, inner, protocol);
                        }
                        Some(CachedImage::Failed(err)) => {
                            let placeholder = Paragraph::new(Line::from(vec![Span::styled(
                                format!("[Image: {} — {}]", alt.as_deref().unwrap_or(src), err),
                                Style::default().fg(Color::Red),
                            )]));
                            frame.render_widget(placeholder, inner);
                        }
                        None => {
                            let placeholder = Paragraph::new(Line::from(vec![Span::styled(
                                format!("[Image: {}]", alt.as_deref().unwrap_or(src)),
                                Style::default().fg(Color::DarkGray),
                            )]));
                            frame.render_widget(placeholder, inner);
                        }
                    }
                } else {
                    match app.image_cache.get_mut(src.as_str()) {
                        Some(CachedImage::Loaded(protocol)) => {
                            let image_widget = StatefulImage::default();
                            frame.render_stateful_widget(image_widget, img_area, protocol);
                        }
                        Some(CachedImage::Failed(err)) => {
                            let placeholder = Paragraph::new(Line::from(vec![Span::styled(
                                format!("[Image: {} — {}]", alt.as_deref().unwrap_or(src), err),
                                Style::default().fg(Color::Red),
                            )]));
                            frame.render_widget(placeholder, img_area);
                        }
                        None => {
                            let placeholder = Paragraph::new(Line::from(vec![Span::styled(
                                format!("[Image: {}]", alt.as_deref().unwrap_or(src)),
                                Style::default().fg(Color::DarkGray),
                            )]));
                            frame.render_widget(placeholder, img_area);
                        }
                    }
                }
                y += elem_h as usize;
            }
        }
    }
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let focused_action = app.focused_item().map(|fi| &fi.action);
    let mut hints = vec![
        Span::styled(" q", Style::default().fg(Color::Yellow)),
        Span::styled(":quit ", Style::default().fg(Color::DarkGray)),
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::styled(":scroll ", Style::default().fg(Color::DarkGray)),
        Span::styled("PgDn/PgUp", Style::default().fg(Color::Yellow)),
        Span::raw(" "),
        Span::styled("b", Style::default().fg(Color::Yellow)),
        Span::styled(":backlinks ", Style::default().fg(Color::DarkGray)),
        Span::styled("+/-", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!(":img({}rows) ", app.image_height_rows),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::styled(":focus ", Style::default().fg(Color::DarkGray)),
    ];
    if let Some(action) = focused_action {
        let action_hint = match action {
            LinkAction::OpenNote { .. } => "Enter:open note ",
            LinkAction::OpenUrl(_) => "Enter:open url ",
            LinkAction::ViewImage(_) => "Enter:fullscreen ",
        };
        hints.push(Span::styled("Enter", Style::default().fg(Color::Yellow)));
        hints.push(Span::styled(
            action_hint,
            Style::default().fg(Color::DarkGray),
        ));
    }
    hints.push(Span::styled("r/^L", Style::default().fg(Color::Yellow)));
    hints.push(Span::styled(
        ":reload ",
        Style::default().fg(Color::DarkGray),
    ));
    if !app.nav_history.is_empty() {
        hints.push(Span::styled("BS/^O", Style::default().fg(Color::Yellow)));
        hints.push(Span::styled(":back ", Style::default().fg(Color::DarkGray)));
    }

    let status = Line::from(hints);
    frame.render_widget(
        Paragraph::new(status).style(Style::default().bg(Color::DarkGray)),
        area,
    );
}

fn draw_backlinks_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_width = (area.width * 60 / 100).max(30).min(area.width - 4);
    let popup_height = (area.height * 60 / 100).max(10).min(area.height - 4);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let cursor = app.backlink_cursor;
    let mut entry_idx: usize = 0;
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Backlinks section
    lines.push(Line::from(Span::styled(
        "Backlinks:",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    if app.back_links.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for bl in &app.back_links {
            for loc in &bl.locations {
                let context = loc.context.as_deref().unwrap_or("");
                let is_selected = cursor == Some(entry_idx);
                let (bullet_style, text_style, ctx_style) = if is_selected {
                    (
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    )
                } else {
                    (
                        Style::default().fg(Color::Yellow),
                        Style::default().fg(Color::White),
                        Style::default().fg(Color::DarkGray),
                    )
                };
                lines.push(Line::from(vec![
                    Span::styled("  • ", bullet_style),
                    Span::styled(
                        format!("{} (L{})", bl.source_file, loc.line + 1),
                        text_style,
                    ),
                    Span::styled(format!("  {}", context), ctx_style),
                ]));
                entry_idx += 1;
            }
        }
    }

    lines.push(Line::from(""));

    // Two-hop links section
    lines.push(Line::from(Span::styled(
        "Two-hop Links:",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    if app.two_hop_links.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (via, targets) in &app.two_hop_links {
            lines.push(Line::from(vec![
                Span::styled("  via ", Style::default().fg(Color::DarkGray)),
                Span::styled(via.clone(), Style::default().fg(Color::White)),
                Span::styled(":", Style::default().fg(Color::DarkGray)),
            ]));
            for target in targets {
                let is_selected = cursor == Some(entry_idx);
                let (arrow_style, name_style) = if is_selected {
                    (
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    )
                } else {
                    (
                        Style::default().fg(Color::Yellow),
                        Style::default().fg(Color::White),
                    )
                };
                lines.push(Line::from(vec![
                    Span::styled("    → ", arrow_style),
                    Span::styled(target.clone(), name_style),
                ]));
                entry_idx += 1;
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k:select  Enter:jump  b/Esc:close",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Backlinks & Two-hop Links ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup_area);
}

fn draw_fullscreen_image(frame: &mut Frame, app: &mut App, root_dir: &Path, src: &str) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    // Load image if needed
    app.load_image(src, root_dir);

    match app.image_cache.get_mut(src) {
        Some(CachedImage::Loaded(protocol)) => {
            let image_widget = StatefulImage::default();
            frame.render_stateful_widget(image_widget, chunks[0], protocol);
        }
        Some(CachedImage::Failed(err)) => {
            let msg = Paragraph::new(Line::from(vec![Span::styled(
                format!("[Failed to load image: {}]", err),
                Style::default().fg(Color::Red),
            )]));
            frame.render_widget(msg, chunks[0]);
        }
        None => {
            let msg = Paragraph::new(Line::from(vec![Span::styled(
                format!("[Loading: {}]", src),
                Style::default().fg(Color::DarkGray),
            )]));
            frame.render_widget(msg, chunks[0]);
        }
    }

    // Status hint
    let hint = Line::from(vec![
        Span::styled(" Esc", Style::default().fg(Color::Yellow)),
        Span::styled(":close ", Style::default().fg(Color::DarkGray)),
        Span::styled(src.to_string(), Style::default().fg(Color::White)),
    ]);
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().bg(Color::DarkGray)),
        chunks[1],
    );
}

// --- Main ---

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let file_path = std::fs::canonicalize(PathBuf::from(&args.file)).unwrap_or_else(|_| {
        eprintln!("Cannot find file: {}", args.file);
        std::process::exit(1);
    });

    if !file_path.exists() || !file_path.is_file() {
        eprintln!("Not a file: {}", file_path.display());
        std::process::exit(1);
    }

    let dir = if let Some(d) = &args.dir {
        std::fs::canonicalize(PathBuf::from(d)).unwrap_or_else(|_| {
            eprintln!("Cannot find directory: {}", d);
            std::process::exit(1);
        })
    } else {
        file_path
            .parent()
            .expect("File must have a parent directory")
            .to_path_buf()
    };

    // Create repository
    let repository = Arc::new(Repository::new(dir.clone()));
    let mut rx = repository.subscribe();

    // Start file watcher
    let repository_clone = repository.clone();
    tokio::spawn(async move {
        if let Err(e) = repository_clone.start_watcher().await {
            eprintln!("Failed to start file watcher: {}", e);
        }
    });

    // Read initial content
    let initial_content = std::fs::read_to_string(&file_path)?;

    // Set up app
    let mut app = App::new(file_path.clone(), dir.clone());
    app.re_render(&initial_content);

    // Compute initial backlinks
    app.back_links = repository.calculate_back_links(&app.file_path);
    app.two_hop_links = repository.calculate_two_hop_links(&app.file_path).await;

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut event_stream = EventStream::new();

    // Main loop
    loop {
        terminal.draw(|f| draw(f, &mut app, &dir))?;

        tokio::select! {
            event = event_stream.next() => {
                match event {
                    Some(Ok(Event::Key(KeyEvent { code, modifiers, .. }))) => {
                        // When backlinks popup is open, intercept navigation keys
                        if app.show_backlinks {
                            match (code, modifiers) {
                                (KeyCode::Char('q'), _) | (KeyCode::Esc, _) | (KeyCode::Char('b'), _) => {
                                    app.show_backlinks = false;
                                    app.backlink_cursor = None;
                                }
                                (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                                    app.backlink_cursor_down();
                                }
                                (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                                    app.backlink_cursor_up();
                                }
                                (KeyCode::Enter, _) => {
                                    if let Some((name, line)) = app.resolve_backlink_cursor() {
                                        app.show_backlinks = false;
                                        app.backlink_cursor = None;
                                        if app.open_note(&name, None) {
                                            if line > 0 {
                                                app.scroll_to_line(line);
                                            }
                                            app.back_links = repository.calculate_back_links(&app.file_path);
                                            app.two_hop_links = repository.calculate_two_hop_links(&app.file_path).await;
                                        }
                                    }
                                }
                                (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                                _ => {}
                            }
                        } else {
                        match (code, modifiers) {
                            (KeyCode::Char('q'), _) => {
                                if app.fullscreen_image.is_some() {
                                    app.fullscreen_image = None;
                                } else {
                                    break;
                                }
                            }
                            (KeyCode::Esc, _) => {
                                if app.fullscreen_image.is_some() {
                                    app.fullscreen_image = None;
                                } else {
                                    break;
                                }
                            }
                            (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                            (KeyCode::Char('j'), _) | (KeyCode::Down, _) => app.scroll_down(1),
                            (KeyCode::Char('k'), _) | (KeyCode::Up, _) => app.scroll_up(1),
                            (KeyCode::PageDown, _) | (KeyCode::Char(' '), _) => {
                                let half = (terminal.size()?.height as usize) / 2;
                                app.scroll_down(half);
                            }
                            (KeyCode::PageUp, _) => {
                                let half = (terminal.size()?.height as usize) / 2;
                                app.scroll_up(half);
                            }
                            (KeyCode::Char('g'), _) | (KeyCode::Home, _) => app.scroll_to_top(),
                            (KeyCode::Char('G'), _) | (KeyCode::End, _) => app.scroll_to_bottom(),
                            (KeyCode::Char('b'), _) => {
                                app.show_backlinks = true;
                                app.backlink_cursor = None;
                                // Refresh backlinks data
                                app.back_links = repository.calculate_back_links(&app.file_path);
                                app.two_hop_links = repository.calculate_two_hop_links(&app.file_path).await;
                            }
                            (KeyCode::Char('+'), _) | (KeyCode::Char('='), _) => {
                                app.increase_image_height();
                            }
                            (KeyCode::Char('-'), _) => {
                                app.decrease_image_height();
                            }
                            (KeyCode::Enter, _) => {
                                if app.fullscreen_image.is_some() {
                                    app.fullscreen_image = None;
                                } else if let Some(fi) = app.focused_item().cloned() {
                                    match &fi.action {
                                        LinkAction::ViewImage(src) => {
                                            app.fullscreen_image = Some(src.clone());
                                        }
                                        LinkAction::OpenNote { name, anchor } => {
                                            if app.open_note(name, anchor.as_deref()) {
                                                // Refresh backlinks for new file
                                                app.back_links = repository.calculate_back_links(&app.file_path);
                                                app.two_hop_links = repository.calculate_two_hop_links(&app.file_path).await;
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
                            (KeyCode::Backspace, _) | (KeyCode::Char('H'), _) | (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                                if app.go_back() {
                                    app.back_links = repository.calculate_back_links(&app.file_path);
                                    app.two_hop_links = repository.calculate_two_hop_links(&app.file_path).await;
                                }
                            }
                            (KeyCode::Tab, KeyModifiers::NONE) => {
                                app.focus_next_item();
                            }
                            (KeyCode::BackTab, _) => {
                                app.focus_prev_item();
                            }
                            (KeyCode::Char('r'), _) | (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                                app.clear_image_cache();
                                let content = std::fs::read_to_string(&app.file_path)
                                    .unwrap_or_default();
                                app.re_render(&content);
                            }
                            _ => {}
                        }
                        } // end else (not show_backlinks)
                    }
                    Some(Ok(Event::Resize(_, _))) => {
                        // Terminal resized — redraw handled by the loop
                    }
                    Some(Err(_)) => break,
                    None => break,
                    _ => {}
                }
            }
            msg = rx.recv() => {
                match msg {
                    Ok(RepositoryMessage::FileChanged(path, _metadata, content)) => {
                        if path == app.file_path {
                            app.re_render(&content);
                            // Refresh backlinks
                            app.back_links = repository.calculate_back_links(&app.file_path);
                            app.two_hop_links = repository.calculate_two_hop_links(&app.file_path).await;
                        }
                    }
                    Ok(_) => {
                        // Other messages: ignore for single-file mode
                    }
                    Err(_) => {
                        // Channel lagged or closed
                    }
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Force exit to stop background file watcher task
    std::process::exit(0);
}
