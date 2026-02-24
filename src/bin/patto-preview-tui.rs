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
    tui_renderer::{self, DocElement, RenderedDoc},
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

struct App {
    file_path: PathBuf,
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
    /// Element index of the currently focused image. None = no focus.
    focused_image_elem: Option<usize>,
}

enum CachedImage {
    Loaded(StatefulProtocol),
    Failed(String),
}

impl App {
    fn new(file_path: PathBuf) -> Self {
        let picker = Picker::from_query_stdio().ok();

        Self {
            file_path,
            rendered_doc: RenderedDoc {
                elements: Vec::new(),
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
            focused_image_elem: None,
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
        let path = if src.starts_with("http://") || src.starts_with("https://") {
            // For remote images, store a placeholder — async fetch would be needed
            self.image_cache.insert(
                src.to_string(),
                CachedImage::Failed("remote images not yet supported in TUI".to_string()),
            );
            return;
        } else {
            root_dir.join(src)
        };

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

    /// Collect element indices of Image elements visible in the current viewport.
    fn visible_image_elem_indices(&self) -> Vec<usize> {
        let img_h = self.image_height_rows;
        let mut row = 0usize;
        let mut result = Vec::new();
        for (i, elem) in self.rendered_doc.elements.iter().enumerate() {
            let h = elem.height(img_h) as usize;
            let elem_top = row;
            let elem_bot = row + h;
            row = elem_bot;
            // Element is visible if it overlaps [scroll_offset, scroll_offset + viewport_height)
            if elem_bot <= self.scroll_offset {
                continue;
            }
            if elem_top >= self.scroll_offset + self.viewport_height {
                break;
            }
            if matches!(elem, DocElement::Image { .. }) {
                result.push(i);
            }
        }
        result
    }

    /// Focus the next visible image (wrap around within viewport).
    fn focus_next_image(&mut self) {
        let visible = self.visible_image_elem_indices();
        if visible.is_empty() {
            return;
        }
        let next = match self.focused_image_elem {
            Some(cur) => {
                // Find the next visible image after cur
                visible
                    .iter()
                    .find(|&&ei| ei > cur)
                    .copied()
                    .unwrap_or(visible[0])
            }
            None => visible[0],
        };
        self.focused_image_elem = Some(next);
    }

    /// Focus the previous visible image (wrap around within viewport).
    fn focus_prev_image(&mut self) {
        let visible = self.visible_image_elem_indices();
        if visible.is_empty() {
            return;
        }
        let prev = match self.focused_image_elem {
            Some(cur) => {
                visible
                    .iter()
                    .rev()
                    .find(|&&ei| ei < cur)
                    .copied()
                    .unwrap_or(*visible.last().unwrap())
            }
            None => *visible.last().unwrap(),
        };
        self.focused_image_elem = Some(prev);
    }

    /// Return the src of the currently focused image, if any.
    fn focused_image_src(&self) -> Option<String> {
        let ei = self.focused_image_elem?;
        if let Some(DocElement::Image { src, .. }) = self.rendered_doc.elements.get(ei) {
            Some(src.clone())
        } else {
            None
        }
    }

    /// Clear focus if the focused image is no longer visible.
    fn clear_stale_focus(&mut self) {
        if let Some(ei) = self.focused_image_elem {
            let visible = self.visible_image_elem_indices();
            if !visible.contains(&ei) {
                self.focused_image_elem = None;
            }
        }
    }
}

// --- Drawing ---

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
            Constraint::Min(1),   // content
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
        Span::styled(
            pos,
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(title), area);
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

    // Pre-load images that will be visible
    let image_srcs: Vec<String> = app
        .rendered_doc
        .elements
        .iter()
        .skip(start_elem)
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
    let focused_elem = app.focused_image_elem;
    let mut y = 0usize;
    for (elem_idx, elem) in app.rendered_doc.elements.iter().enumerate().skip(start_elem) {
        if y >= height {
            break;
        }
        let is_focused = focused_elem == Some(elem_idx);
        match elem {
            DocElement::TextLine(line) => {
                let line_area = Rect::new(area.x, area.y + y as u16, area.width, 1);
                frame.render_widget(Paragraph::new(line.clone()), line_area);
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
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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
    let on_image = app.focused_image_src().is_some();
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
        Span::styled("n/Tab", Style::default().fg(Color::Yellow)),
        Span::styled(":sel img ", Style::default().fg(Color::DarkGray)),
    ];
    if on_image {
        hints.push(Span::styled("Enter", Style::default().fg(Color::Yellow)));
        hints.push(Span::styled(":fullscreen ", Style::default().fg(Color::DarkGray)));
    }
    hints.push(Span::styled("r", Style::default().fg(Color::Yellow)));
    hints.push(Span::styled(":reload", Style::default().fg(Color::DarkGray)));

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
                let context = loc
                    .context
                    .as_deref()
                    .unwrap_or("");
                lines.push(Line::from(vec![
                    Span::styled("  • ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{} (L{})", bl.source_file, loc.line + 1),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("  {}", context),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
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
                lines.push(Line::from(vec![
                    Span::styled("    → ", Style::default().fg(Color::Yellow)),
                    Span::styled(target.clone(), Style::default().fg(Color::White)),
                ]));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Press b or Esc to close",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Backlinks & Two-hop Links ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
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
        Span::styled(
            src.to_string(),
            Style::default().fg(Color::White),
        ),
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
    let mut app = App::new(file_path.clone());
    app.re_render(&initial_content);

    // Compute initial backlinks
    app.back_links = repository.calculate_back_links(&file_path);
    app.two_hop_links = repository.calculate_two_hop_links(&file_path).await;

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
                        match (code, modifiers) {
                            (KeyCode::Char('q'), _) => {
                                if app.fullscreen_image.is_some() {
                                    app.fullscreen_image = None;
                                } else if app.show_backlinks {
                                    app.show_backlinks = false;
                                } else {
                                    break;
                                }
                            }
                            (KeyCode::Esc, _) => {
                                if app.fullscreen_image.is_some() {
                                    app.fullscreen_image = None;
                                } else if app.show_backlinks {
                                    app.show_backlinks = false;
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
                                app.show_backlinks = !app.show_backlinks;
                                if app.show_backlinks {
                                    // Refresh backlinks data
                                    app.back_links = repository.calculate_back_links(&file_path);
                                    app.two_hop_links = repository.calculate_two_hop_links(&file_path).await;
                                }
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
                                } else if let Some(src) = app.focused_image_src() {
                                    app.fullscreen_image = Some(src);
                                }
                            }
                            (KeyCode::Char('n'), _) | (KeyCode::Tab, KeyModifiers::NONE) => {
                                app.focus_next_image();
                            }
                            (KeyCode::Char('N'), _) | (KeyCode::BackTab, _) => {
                                app.focus_prev_image();
                            }
                            (KeyCode::Char('r'), _) => {
                                app.clear_image_cache();
                                let content = std::fs::read_to_string(&file_path)
                                    .unwrap_or_default();
                                app.re_render(&content);
                            }
                            _ => {}
                        }
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
                        if path == file_path {
                            app.re_render(&content);
                            // Refresh backlinks
                            app.back_links = repository.calculate_back_links(&file_path);
                            app.two_hop_links = repository.calculate_two_hop_links(&file_path).await;
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

    Ok(())
}
