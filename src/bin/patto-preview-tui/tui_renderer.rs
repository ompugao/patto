use std::collections::HashMap;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use patto::parser::{AstNode, AstNodeKind, Property, TaskStatus};
use patto::utils::get_gyazo_img_src;

/// Action to perform when a focusable item is activated.
#[derive(Debug, Clone)]
pub enum LinkAction {
    /// Open a wiki-linked note, optionally jumping to an anchor.
    OpenNote {
        name: String,
        anchor: Option<String>,
    },
    /// Jump to an anchor within the current document (self-link).
    JumpToAnchor { anchor: String },
    /// Open a URL in the system browser.
    OpenUrl(String),
    /// View an image fullscreen.
    ViewImage(String),
}

/// A focusable item in the rendered document (link, image, etc.).
#[derive(Debug, Clone)]
pub struct FocusableItem {
    /// Index into `RenderedDoc.elements` that contains this item.
    pub elem_idx: usize,
    /// Character offset where the focusable span starts within the text line.
    /// For images, this is 0.
    pub char_start: usize,
    /// Character offset where the focusable span ends (exclusive).
    /// For images, this equals 0.
    pub char_end: usize,
    /// Action to perform on Enter.
    pub action: LinkAction,
}

/// A single element in the rendered document.
#[derive(Debug, Clone)]
pub enum DocElement {
    /// A styled text line. The `usize` is the 0-indexed source row from the AST.
    TextLine(Line<'static>, usize),
    /// An image to render via kitty/sixel.
    Image {
        src: String,
        alt: Option<String>,
        indent: usize,
    },
    /// Multiple images on the same line, rendered side by side.
    /// The second field is the indentation level.
    ImageRow(Vec<(String, Option<String>)>, usize),
    /// A math block to render as an image (LaTeX source).
    Math { content: String, indent: usize },
}

/// A fully rendered document ready for display.
pub struct RenderedDoc {
    pub elements: Vec<DocElement>,
    /// All focusable items (links, images) in document order.
    pub focusables: Vec<FocusableItem>,
    /// Map from anchor name to element index.
    pub anchors: HashMap<String, usize>,
}

impl RenderedDoc {}

/// Render an AST root node into a flat list of DocElements.
pub fn render_ast(ast: &AstNode, syntax_theme: Option<&str>) -> RenderedDoc {
    let mut builder = DocBuilder::new(syntax_theme);
    builder.node(ast, 0);
    builder.finish()
}

/// Result of inline rendering — may contain image blocks that need to be
/// emitted between text line fragments.
enum InlineResult {
    /// Pure inline content (appended to current spans).
    Inline,
    /// An image block that must be emitted as a separate DocElement.
    ImageBlock { src: String, alt: Option<String> },
}

/// Returns true if `spans` contains any non-whitespace text.
fn spans_have_content(spans: &[Span<'_>]) -> bool {
    spans.iter().any(|s| !s.content.trim().is_empty())
}

/// Builds the flat element list the TUI draws from.
///
/// The three outputs — elements, focusables and the anchor map — are filled in
/// together as the tree is walked, so they live here rather than being threaded
/// through every function as `&mut` parameters.
struct DocBuilder<'a> {
    elements: Vec<DocElement>,
    focusables: Vec<FocusableItem>,
    anchors: HashMap<String, usize>,
    syntax_theme: Option<&'a str>,
}

impl<'a> DocBuilder<'a> {
    fn new(syntax_theme: Option<&'a str>) -> Self {
        Self {
            elements: Vec::new(),
            focusables: Vec::new(),
            anchors: HashMap::new(),
            syntax_theme,
        }
    }

    fn finish(self) -> RenderedDoc {
        RenderedDoc {
            elements: self.elements,
            focusables: self.focusables,
            anchors: self.anchors,
        }
    }

    fn push_text(&mut self, spans: Vec<Span<'static>>, row: usize) {
        self.elements
            .push(DocElement::TextLine(Line::from(spans), row));
    }

    /// Images are focusable, so that Enter can open them fullscreen.
    fn push_image(&mut self, src: String, alt: Option<String>, indent: usize) {
        self.focusables.push(FocusableItem {
            elem_idx: self.elements.len(),
            char_start: 0,
            char_end: 0,
            action: LinkAction::ViewImage(src.clone()),
        });
        self.elements.push(DocElement::Image { src, alt, indent });
    }

    /// Flush `buf` as a single `Image` (len == 1) or `ImageRow` (len > 1).
    fn flush_image_row(&mut self, buf: &mut Vec<(String, Option<String>)>, indent: usize) {
        if buf.is_empty() {
            return;
        }
        if buf.len() == 1 {
            let (src, alt) = buf.remove(0);
            self.push_image(src, alt, indent);
        } else {
            for (src, _alt) in buf.iter() {
                self.focusables.push(FocusableItem {
                    elem_idx: self.elements.len(),
                    char_start: 0,
                    char_end: 0,
                    action: LinkAction::ViewImage(src.clone()),
                });
            }
            self.elements
                .push(DocElement::ImageRow(std::mem::take(buf), indent));
        }
        buf.clear();
    }

    fn children(&mut self, ast: &AstNode, indent: usize) {
        for child in ast.children().iter() {
            self.node(child, indent);
        }
    }

    fn node(&mut self, ast: &AstNode, indent: usize) {
        match ast.kind() {
            AstNodeKind::Dummy | AstNodeKind::Quote => self.children(ast, indent),
            AstNodeKind::Line { properties } => self.line(ast, properties, false, indent),
            AstNodeKind::QuoteContent { properties } => self.line(ast, properties, true, indent),
            AstNodeKind::Math { inline: false } => self.math_block(ast, indent),
            AstNodeKind::Code {
                lang,
                inline: false,
            } => self.code_block(ast, lang, indent),
            AstNodeKind::Image { src, alt } => {
                let src = get_gyazo_img_src(src).unwrap_or_else(|| src.clone());
                self.push_image(src, alt.clone(), indent);
                if let Some(alt) = alt {
                    self.push_text(
                        vec![Span::styled(
                            format!("  {}", alt),
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC),
                        )],
                        ast.location().row,
                    );
                }
            }
            AstNodeKind::HorizontalLine => {
                self.push_text(
                    vec![Span::styled(
                        "─".repeat(40),
                        Style::default().fg(Color::DarkGray),
                    )],
                    ast.location().row,
                );
            }
            AstNodeKind::Table { caption } => self.table(ast, caption.as_deref(), indent),
            // Inline kinds are rendered by `render_inline` from their line, and
            // table rows and columns by `table`.
            _ => {}
        }
    }

    fn line(&mut self, ast: &AstNode, properties: &[Property], is_quote: bool, indent: usize) {
        // A line whose only content is a block delegates to that block.
        if let Some(block) = block_container(ast) {
            self.node(&block, indent);
            self.children(ast, indent + 1);
            return;
        }

        for property in properties {
            if let Property::Anchor { name, .. } = property {
                self.anchors
                    .insert(name.to_lowercase(), self.elements.len());
            }
        }

        let task_status = properties.iter().rev().find_map(|property| match property {
            Property::Task { status, .. } => Some(status),
            _ => None,
        });
        let is_done = matches!(task_status, Some(TaskStatus::Done));

        let mut spans = self.line_prefix(ast, task_status, is_quote, indent);
        let base_style = if is_done {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::CROSSED_OUT)
        } else if is_quote {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::ITALIC)
        } else {
            Style::default()
        };

        // Images that follow each other with no text between them share a row.
        let mut image_row: Vec<(String, Option<String>)> = Vec::new();
        for content in ast.contents().iter() {
            let elem_idx = self.elements.len();
            match render_inline(
                content,
                &mut spans,
                base_style,
                &mut self.focusables,
                elem_idx,
            ) {
                InlineResult::ImageBlock { src, alt } => {
                    if spans_have_content(&spans) {
                        let row = ast.location().row;
                        self.push_text(std::mem::take(&mut spans), row);
                        // Text between images breaks the group.
                        self.flush_image_row(&mut image_row, indent);
                    } else {
                        spans = vec![Span::raw("  ".repeat(indent + 1))];
                    }
                    image_row.push((src, alt));
                }
                InlineResult::Inline => self.flush_image_row(&mut image_row, indent),
            }
        }
        self.flush_image_row(&mut image_row, indent);

        for property in properties {
            if let Property::Task { status, due, .. } = property {
                if !matches!(status, TaskStatus::Done) {
                    spans.push(Span::styled(
                        format!(" [{}]", due),
                        Style::default().fg(Color::Red),
                    ));
                }
            }
        }

        // Always emitted, so that blank lines keep their height.
        self.push_text(spans, ast.location().row);
        self.children(ast, indent + 1);
    }

    /// Indent, quote bar and task icon or bullet.
    fn line_prefix(
        &self,
        ast: &AstNode,
        task_status: Option<&TaskStatus>,
        is_quote: bool,
        indent: usize,
    ) -> Vec<Span<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        if indent > 0 {
            spans.push(Span::raw("  ".repeat(indent)));
        }
        if is_quote {
            spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
        }

        if let Some(status) = task_status {
            let (icon, color) = match status {
                TaskStatus::Done => ("✓ ", Color::Green),
                TaskStatus::Doing => ("◑ ", Color::Yellow),
                TaskStatus::Paused => ("⏸ ", Color::Cyan),
                _ => ("○ ", Color::White),
            };
            spans.push(Span::styled(icon.to_string(), Style::default().fg(color)));
        } else if !is_quote && indent > 0 && !is_blank(ast) {
            spans.push(Span::raw("• "));
        }
        spans
    }

    fn math_block(&mut self, ast: &AstNode, indent: usize) {
        let content = ast
            .children()
            .iter()
            .map(|child| child.extract_str().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        self.elements.push(DocElement::Math { content, indent });
    }

    fn code_block(&mut self, ast: &AstNode, lang: &str, indent: usize) {
        let prefix = if indent > 0 {
            "  ".repeat(indent)
        } else {
            "  ".to_string()
        };
        let row = ast.location().row;

        if !lang.is_empty() {
            self.push_text(
                vec![
                    Span::raw(prefix.clone()),
                    Span::styled(
                        format!(" {} ", lang),
                        Style::default()
                            .fg(Color::Cyan)
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                ],
                row,
            );
        }

        let raw_lines: Vec<String> = ast
            .children()
            .iter()
            .map(|child| child.extract_str().replace('\t', "    "))
            .collect();
        let raw_refs: Vec<&str> = raw_lines.iter().map(|s| s.as_str()).collect();

        for line_spans in
            crate::syntax_highlight::highlight_code(lang, &raw_refs, self.syntax_theme)
        {
            let mut spans = vec![Span::raw(prefix.clone())];
            if line_spans.is_empty() {
                // Keep the row's height on an empty line.
                spans.push(Span::raw(""));
            } else {
                spans.extend(line_spans);
            }
            self.push_text(spans, row);
        }
    }

    fn table(&mut self, ast: &AstNode, caption: Option<&str>, indent: usize) {
        if let Some(caption) = caption {
            self.push_text(
                vec![
                    Span::raw("  ".repeat(indent)),
                    Span::styled(
                        caption.to_string(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ],
                ast.location().row,
            );
        }
        for child in ast.children().iter() {
            self.table_row(child, indent);
        }
    }

    fn table_row(&mut self, ast: &AstNode, indent: usize) {
        let separator = Style::default().fg(Color::DarkGray);
        let mut spans = vec![
            Span::raw("  ".repeat(indent)),
            Span::styled("│ ", separator),
        ];

        let elem_idx = self.elements.len();
        for (i, column) in ast.contents().iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", separator));
            }
            for content in column.contents().iter() {
                render_inline(
                    content,
                    &mut spans,
                    Style::default(),
                    &mut self.focusables,
                    elem_idx,
                );
            }
        }
        spans.push(Span::styled(" │", separator));
        self.push_text(spans, ast.location().row);
    }
}

/// The block this line exists only to hold, if it is such a line.
fn block_container(ast: &AstNode) -> Option<AstNode> {
    let contents = ast.contents();
    let [only] = contents.as_slice() else {
        return None;
    };
    matches!(
        only.kind(),
        AstNodeKind::Quote
            | AstNodeKind::Code { inline: false, .. }
            | AstNodeKind::Math { inline: false }
            | AstNodeKind::Table { .. }
    )
    .then(|| only.clone())
}

/// A line with nothing but whitespace on it.
fn is_blank(ast: &AstNode) -> bool {
    let contents = ast.contents();
    contents.is_empty()
        || contents
            .iter()
            .all(|c| matches!(c.kind(), AstNodeKind::Text) && c.extract_str().trim().is_empty())
}

/// Count total character width of accumulated spans.
fn spans_char_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

fn render_inline(
    ast: &AstNode,
    spans: &mut Vec<Span<'static>>,
    base_style: Style,
    focusables: &mut Vec<FocusableItem>,
    current_elem_idx: usize,
) -> InlineResult {
    match ast.kind() {
        AstNodeKind::Text => {
            spans.push(Span::styled(ast.extract_str().to_string(), base_style));
        }
        AstNodeKind::WikiLink { link, anchor } => {
            let display = if let Some(anchor) = anchor {
                if link.is_empty() {
                    format!("#{}", anchor)
                } else {
                    format!("{}#{}", link, anchor)
                }
            } else {
                link.clone()
            };
            let text = format!("[{}]", display);
            let char_start = spans_char_width(spans);
            let char_end = char_start + text.chars().count();
            spans.push(Span::styled(
                text,
                base_style
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED),
            ));
            // Self-link: empty link name with anchor -> jump within current doc
            let action = if link.is_empty() {
                if let Some(anc) = anchor {
                    LinkAction::JumpToAnchor {
                        anchor: anc.clone(),
                    }
                } else {
                    // Edge case: empty link with no anchor (shouldn't happen normally)
                    LinkAction::OpenNote {
                        name: link.clone(),
                        anchor: anchor.clone(),
                    }
                }
            } else {
                LinkAction::OpenNote {
                    name: link.clone(),
                    anchor: anchor.clone(),
                }
            };
            focusables.push(FocusableItem {
                elem_idx: current_elem_idx,
                char_start,
                char_end,
                action,
            });
        }
        AstNodeKind::Link { link, title } => {
            let display = title.as_deref().unwrap_or(link.as_str());
            let char_start = spans_char_width(spans);
            let char_end = char_start + display.chars().count();
            spans.push(Span::styled(
                display.to_string(),
                base_style
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED),
            ));
            focusables.push(FocusableItem {
                elem_idx: current_elem_idx,
                char_start,
                char_end,
                action: LinkAction::OpenUrl(link.clone()),
            });
        }
        AstNodeKind::Embed { link, title } => {
            let is_pdf = link.to_lowercase().ends_with(".pdf");
            let display = title.as_deref().unwrap_or(link.as_str());
            let text = if is_pdf {
                format!("[PDF: {}]", display)
            } else {
                format!("[embed: {}]", display)
            };
            let char_start = spans_char_width(spans);
            let char_end = char_start + text.chars().count();
            let style = if is_pdf {
                base_style
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                base_style
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED)
            };
            spans.push(Span::styled(text, style));
            focusables.push(FocusableItem {
                elem_idx: current_elem_idx,
                char_start,
                char_end,
                action: LinkAction::OpenUrl(link.clone()),
            });
        }
        AstNodeKind::Code { inline: true, .. } => {
            let contents = ast.contents();
            for content in contents.iter() {
                spans.push(Span::styled(
                    content.extract_str().to_string(),
                    base_style.fg(Color::Yellow).bg(Color::DarkGray),
                ));
            }
        }
        AstNodeKind::Math { inline: true } => {
            let contents = ast.contents();
            for content in contents.iter() {
                spans.push(Span::styled(
                    content.extract_str().to_string(),
                    base_style.fg(Color::Magenta),
                ));
            }
        }
        AstNodeKind::Decoration {
            fontsize,
            italic,
            underline,
            deleted,
        } => {
            let mut style = base_style;
            if *fontsize > 0 {
                style = style.add_modifier(Modifier::BOLD);
            }
            if *italic {
                style = style.add_modifier(Modifier::ITALIC);
            }
            if *underline {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            if *deleted {
                style = style.add_modifier(Modifier::CROSSED_OUT);
            }
            let contents = ast.contents();
            for content in contents.iter() {
                let result = render_inline(content, spans, style, focusables, current_elem_idx);
                if matches!(result, InlineResult::ImageBlock { .. }) {
                    return result;
                }
            }
        }
        AstNodeKind::Image { src, alt } => {
            let mut src_resolved = src.clone();
            if let Some(gyazo_src) = get_gyazo_img_src(src) {
                src_resolved = gyazo_src;
            }
            return InlineResult::ImageBlock {
                src: src_resolved,
                alt: alt.clone(),
            };
        }
        AstNodeKind::Quote => {
            let children = ast.children();
            for child in children.iter() {
                render_inline(
                    child,
                    spans,
                    base_style.fg(Color::DarkGray),
                    focusables,
                    current_elem_idx,
                );
            }
        }
        _ => {
            // Fallback: raw text
            let text = ast.extract_str();
            if !text.is_empty() {
                spans.push(Span::styled(text.to_string(), base_style));
            }
        }
    }
    InlineResult::Inline
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(text: &str) -> RenderedDoc {
        let result = patto::parser::parse_text(text);
        assert!(
            result.parse_errors.is_empty(),
            "parse errors: {:?}",
            result.parse_errors
        );
        render_ast(&result.ast, None)
    }

    /// Plain text of every `TextLine`, in order.
    fn text_lines(doc: &RenderedDoc) -> Vec<String> {
        doc.elements
            .iter()
            .filter_map(|element| match element {
                DocElement::TextLine(line, _) => Some(
                    line.spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>(),
                ),
                _ => None,
            })
            .collect()
    }

    fn assert_any_line_contains(doc: &RenderedDoc, needle: &str) {
        let lines = text_lines(doc);
        assert!(
            lines.iter().any(|line| line.contains(needle)),
            "expected a line containing {needle:?} in {lines:#?}"
        );
    }

    #[test]
    fn plain_lines_keep_their_source_rows() {
        let doc = render("first\nsecond\n");
        let rows: Vec<usize> = doc
            .elements
            .iter()
            .filter_map(|element| match element {
                DocElement::TextLine(_, row) => Some(*row),
                _ => None,
            })
            .collect();
        assert_eq!(rows, vec![0, 1]);
        assert_eq!(text_lines(&doc), vec!["first", "second"]);
    }

    #[test]
    fn nested_lines_get_a_bullet_and_indent() {
        let doc = render("parent\n\tchild\n");
        assert_eq!(text_lines(&doc), vec!["parent", "  • child"]);
    }

    #[test]
    fn blank_nested_lines_get_no_bullet() {
        let doc = render("parent\n\t\n");
        assert_eq!(text_lines(&doc), vec!["parent", ""]);
    }

    #[test]
    fn task_status_picks_an_icon_and_shows_the_deadline() {
        for (status, icon) in [
            ("todo", "○ "),
            ("doing", "◑ "),
            ("paused", "⏸ "),
            ("done", "✓ "),
        ] {
            let doc = render(&format!("{{@task status={status} due=2024-12-31}} item\n"));
            assert_any_line_contains(&doc, icon);
        }

        let doc = render("{@task status=todo due=2024-12-31} item\n");
        assert_any_line_contains(&doc, "[2024-12-31]");

        // A finished task no longer shows its deadline.
        let doc = render("{@task status=done due=2024-12-31} item\n");
        assert!(
            !text_lines(&doc).iter().any(|l| l.contains("2024-12-31")),
            "a done task should not show its deadline"
        );
    }

    #[test]
    fn done_tasks_are_struck_through() {
        let doc = render("{@task status=done due=2024-12-31} item\n");
        let DocElement::TextLine(line, _) = &doc.elements[0] else {
            panic!("expected a text line, got {:?}", doc.elements[0]);
        };
        assert!(
            line.spans
                .iter()
                .any(|span| span.style.add_modifier.contains(Modifier::CROSSED_OUT)),
            "no struck-through span in {line:?}"
        );
    }

    #[test]
    fn quote_content_is_prefixed() {
        let doc = render("[@quote]\n\tquoted\n");
        assert_any_line_contains(&doc, "│ ");
        assert_any_line_contains(&doc, "quoted");
    }

    #[test]
    fn code_block_shows_its_language_then_its_body() {
        let doc = render("[@code python]\n\tprint(1)\n");
        let lines = text_lines(&doc);
        assert!(
            lines.iter().any(|l| l.contains(" python ")),
            "no language label in {lines:#?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("print(1)")),
            "no code body in {lines:#?}"
        );
    }

    #[test]
    fn math_block_becomes_a_math_element() {
        let doc = render("[@math]\n\tx = 1\n");
        let math: Vec<_> = doc
            .elements
            .iter()
            .filter_map(|element| match element {
                DocElement::Math { content, .. } => Some(content.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(math, vec!["x = 1".to_string()]);
    }

    #[test]
    fn an_image_is_an_element_and_a_focusable() {
        let doc = render("[@img ./cat.png]\n");
        assert!(
            doc.elements
                .iter()
                .any(|e| matches!(e, DocElement::Image { src, .. } if src == "./cat.png")),
            "no image element in {:?}",
            doc.elements
        );
        assert!(
            doc.focusables
                .iter()
                .any(|f| matches!(&f.action, LinkAction::ViewImage(src) if src == "./cat.png")),
            "image is not focusable"
        );
    }

    #[test]
    fn text_before_an_image_is_flushed_as_its_own_line() {
        let doc = render("text then [@img ./x.png] image\n");
        let kinds: Vec<&str> = doc
            .elements
            .iter()
            .map(|element| match element {
                DocElement::TextLine(..) => "text",
                DocElement::Image { .. } => "image",
                DocElement::ImageRow(..) => "image-row",
                DocElement::Math { .. } => "math",
            })
            .collect();
        assert_eq!(kinds, vec!["text", "image", "text"]);
        assert_eq!(text_lines(&doc)[0], "text then ");
    }

    #[test]
    fn consecutive_images_share_a_row() {
        let doc = render("[@img ./a.png][@img ./b.png]\n");
        let rows: Vec<usize> = doc
            .elements
            .iter()
            .filter_map(|element| match element {
                DocElement::ImageRow(images, _) => Some(images.len()),
                _ => None,
            })
            .collect();
        assert_eq!(rows, vec![2], "expected one row of two images");
    }

    /// A horizontal line is inline content of a line, so `render_inline`
    /// handles it — through its raw-text fallback. `render_node`'s
    /// `HorizontalLine` arm, which draws a box-drawing rule, is never reached.
    #[test]
    fn horizontal_line_is_shown_as_its_source_text() {
        let doc = render("------\n");
        assert_eq!(text_lines(&doc), vec!["------"]);
    }

    #[test]
    fn table_renders_its_caption_and_cells() {
        let doc = render("[@table cap]\n\ta\tb\n");
        assert_any_line_contains(&doc, "cap");
        assert_any_line_contains(&doc, "a");
        assert_any_line_contains(&doc, " │ ");
    }

    #[test]
    fn anchors_map_to_the_element_that_defines_them() {
        let doc = render("first\nsecond #here\n");
        assert_eq!(doc.anchors.get("here"), Some(&1));
    }

    #[test]
    fn wikilinks_and_urls_become_focusables() {
        let doc = render("[other note] and [https://example.com Site] and [#anchor]\n");
        let actions: Vec<String> = doc
            .focusables
            .iter()
            .map(|f| format!("{:?}", f.action))
            .collect();

        assert!(
            doc.focusables.iter().any(
                |f| matches!(&f.action, LinkAction::OpenNote { name, .. } if name == "other note")
            ),
            "no OpenNote in {actions:#?}"
        );
        assert!(
            doc.focusables.iter().any(
                |f| matches!(&f.action, LinkAction::OpenUrl(url) if url == "https://example.com")
            ),
            "no OpenUrl in {actions:#?}"
        );
        assert!(
            doc.focusables.iter().any(
                |f| matches!(&f.action, LinkAction::JumpToAnchor { anchor } if anchor == "anchor")
            ),
            "no JumpToAnchor in {actions:#?}"
        );
    }

    #[test]
    fn a_focusable_spans_the_characters_it_covers() {
        let doc = render("see [other note] here\n");
        let link = doc
            .focusables
            .iter()
            .find(|f| matches!(&f.action, LinkAction::OpenNote { .. }))
            .expect("no wikilink focusable");

        let DocElement::TextLine(line, _) = &doc.elements[link.elem_idx] else {
            panic!("focusable does not point at a text line");
        };
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let covered: String = text
            .chars()
            .skip(link.char_start)
            .take(link.char_end - link.char_start)
            .collect();
        assert_eq!(covered, "[other note]");
    }
}
