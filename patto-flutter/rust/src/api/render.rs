//! Flatten a parsed note into the block list the Flutter note view scrolls.

use patto::parser::{self, AstNode, AstNodeKind, Location, Property};
use patto::utils::{get_gyazo_img_src, get_youtube_id, is_google_photos_url};
use url::Url;

use crate::api::types::*;

/// Parse note text and flatten it for display.
pub fn render_note(content: String) -> RenderedNote {
    let result = parser::parse_text(&content);

    let mut out = Flattener::default();
    let root = result.ast;
    for child in root.children().iter() {
        out.visit_line(child, 0, 0);
    }

    let errors = result
        .parse_errors
        .iter()
        .map(|e| match e {
            parser::ParserError::InvalidIndentation(loc) => ParseIssue {
                row: loc.row as u32,
                message: "invalid indentation".to_string(),
            },
            parser::ParserError::ParseError(loc, _) => ParseIssue {
                row: loc.row as u32,
                message: "could not parse this line".to_string(),
            },
        })
        .collect();

    RenderedNote {
        blocks: out.blocks,
        anchors: out.anchors,
        errors,
    }
}

#[derive(Default)]
struct Flattener {
    blocks: Vec<Block>,
    anchors: Vec<AnchorRef>,
}

impl Flattener {
    fn push(&mut self, block: Block) {
        for name in &block.anchors {
            self.anchors.push(AnchorRef {
                name: name.clone(),
                block_index: self.blocks.len() as u32,
            });
        }
        self.blocks.push(block);
    }

    /// Emit blocks for one `Line` / `QuoteContent` node and then its children.
    fn visit_line(&mut self, line: &AstNode, depth: u32, quote_depth: u32) {
        let (task, anchors) = line_properties(line);
        let row = line.location().row as u32;
        let contents = line.contents().clone();

        // A line holding a single block container renders as that block.
        let sole = (contents.len() == 1).then(|| &contents[0]);
        match sole.filter(|only| matches!(only.kind(), AstNodeKind::Quote)) {
            // The `[@quote]` marker itself has no content; its children
            // become blocks marked one quote level deeper.
            Some(quote) => self.visit_quote(quote, depth, quote_depth + 1),
            None => {
                let kind = sole
                    .and_then(container_block)
                    .unwrap_or_else(|| line_block(&contents));
                self.push(Block {
                    row,
                    depth,
                    quote_depth,
                    task,
                    anchors,
                    kind,
                });
            }
        }

        self.visit_children(line, depth, quote_depth);
    }

    fn visit_children(&mut self, node: &AstNode, depth: u32, quote_depth: u32) {
        for child in node.children().iter() {
            self.visit_line(child, depth + 1, quote_depth);
        }
    }

    /// A `[@quote]` block: its children are `QuoteContent` lines, or `Line`
    /// nodes that wrap a further nested quote.
    fn visit_quote(&mut self, quote: &AstNode, depth: u32, quote_depth: u32) {
        for child in quote.children().iter() {
            self.visit_line(child, depth, quote_depth);
        }
    }
}

fn line_properties(line: &AstNode) -> (Option<TaskInfo>, Vec<String>) {
    match line.kind() {
        AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } => (
            properties.iter().find_map(task_info),
            properties
                .iter()
                .filter_map(|p| match p {
                    Property::Anchor { name, .. } => Some(name.clone()),
                    _ => None,
                })
                .collect(),
        ),
        // The parser only ever puts Line/QuoteContent here, but stay total.
        _ => (None, Vec::new()),
    }
}

/// The block a line made of one container renders as.
fn container_block(node: &AstNode) -> Option<BlockKind> {
    Some(match node.kind() {
        AstNodeKind::Code {
            lang,
            inline: false,
        } => BlockKind::Code {
            lang: lang.clone(),
            lines: child_texts(node),
        },
        AstNodeKind::Math { inline: false } => BlockKind::Math {
            tex: child_texts(node).join("\n"),
        },
        AstNodeKind::Table { caption } => BlockKind::Table {
            caption: caption.clone(),
            rows: table_rows(node),
        },
        AstNodeKind::HorizontalLine => BlockKind::Rule,
        _ => return None,
    })
}

fn line_block(contents: &[AstNode]) -> BlockKind {
    let visible: Vec<&AstNode> = contents.iter().filter(|c| !is_blank_text(c)).collect();

    // Only a local PDF can be previewed from the device; any other local
    // path stays a link. Every remote URL gets a card that opens in the
    // app's web view.
    let sole_embed = match visible.as_slice() {
        [only] => embed_ref(only).filter(|e| !e.is_local || e.kind == EmbedKind::Pdf),
        _ => None,
    };

    if visible.is_empty() {
        BlockKind::Blank
    } else if let Some(embed) = sole_embed {
        BlockKind::Embed { embed }
    } else if visible
        .iter()
        .all(|c| matches!(c.kind(), AstNodeKind::Image { .. }))
    {
        BlockKind::Images {
            images: visible.iter().filter_map(|c| image_ref(c)).collect(),
        }
    } else {
        BlockKind::Line {
            spans: contents.iter().filter_map(inline_span).collect(),
        }
    }
}

/// The source text a location covers. `Location::as_str` is private to the core
/// crate, but the line text and byte span it holds are public.
fn span_text(location: &Location) -> &str {
    let end = location.span.1.min(location.input.len());
    let start = location.span.0.min(end);
    &location.input[start..end]
}

/// `!2026-12-31` rather than `{@task ...}`.
fn is_shorthand(location: &Location) -> bool {
    !span_text(location).trim_start().starts_with("{@")
}

fn is_blank_text(node: &AstNode) -> bool {
    matches!(node.kind(), AstNodeKind::Text) && node.extract_str().trim().is_empty()
}

fn child_texts(node: &AstNode) -> Vec<String> {
    node.children()
        .iter()
        .map(|c| c.extract_str().to_string())
        .collect()
}

fn contents_text(node: &AstNode) -> String {
    node.contents()
        .iter()
        .map(|c| c.extract_str())
        .collect::<Vec<_>>()
        .join("")
}

fn table_rows(table: &AstNode) -> Vec<NoteTableRow> {
    table
        .children()
        .iter()
        .map(|row| NoteTableRow {
            cells: row
                .contents()
                .iter()
                .map(|col| NoteTableCell {
                    spans: col.contents().iter().filter_map(inline_span).collect(),
                })
                .collect(),
        })
        .collect()
}

fn image_ref(node: &AstNode) -> Option<ImageRef> {
    let AstNodeKind::Image { src, alt } = node.kind() else {
        return None;
    };
    let resolved = get_gyazo_img_src(src).unwrap_or_else(|| src.clone());
    Some(ImageRef {
        is_local: !resolved.contains("://"),
        src: resolved,
        alt: alt.clone(),
    })
}

fn embed_ref(node: &AstNode) -> Option<EmbedRef> {
    let AstNodeKind::Embed { link, title } = node.kind() else {
        return None;
    };
    Some(EmbedRef {
        url: link.clone(),
        title: title.clone(),
        kind: embed_kind(link),
        is_local: !link.contains("://"),
    })
}

fn embed_kind(link: &str) -> EmbedKind {
    if link.to_lowercase().ends_with(".pdf") {
        return EmbedKind::Pdf;
    }
    if let Some(video_id) = get_youtube_id(link) {
        return EmbedKind::Youtube { video_id };
    }
    if is_google_photos_url(link) {
        return EmbedKind::GooglePhotos;
    }
    let Some(host) = Url::parse(link)
        .ok()
        .and_then(|u| u.host_str().map(str::to_lowercase))
    else {
        return EmbedKind::Other;
    };
    match host.as_str() {
        "twitter.com" | "www.twitter.com" | "mobile.twitter.com" | "x.com" | "www.x.com" => {
            EmbedKind::Twitter
        }
        "speakerdeck.com" | "www.speakerdeck.com" => EmbedKind::SpeakerDeck,
        "slideshare.net" | "www.slideshare.net" => EmbedKind::SlideShare,
        _ => EmbedKind::Other,
    }
}

fn inline_span(node: &AstNode) -> Option<NoteSpan> {
    match node.kind() {
        AstNodeKind::Text => Some(NoteSpan::Text {
            text: node.extract_str().to_string(),
        }),
        AstNodeKind::Decoration {
            fontsize,
            italic,
            underline,
            deleted,
        } => Some(NoteSpan::Decoration {
            fontsize: *fontsize as i32,
            italic: *italic,
            underline: *underline,
            deleted: *deleted,
            children: node.contents().iter().filter_map(inline_span).collect(),
        }),
        AstNodeKind::WikiLink { link, anchor } => Some(NoteSpan::WikiLink {
            name: link.clone(),
            anchor: anchor.clone(),
        }),
        AstNodeKind::Link { link, title } => Some(NoteSpan::Url {
            url: link.clone(),
            title: title.clone(),
        }),
        AstNodeKind::Embed { .. } => embed_ref(node).map(|embed| NoteSpan::Embed { embed }),
        AstNodeKind::Code { inline: true, .. } => Some(NoteSpan::InlineCode {
            code: contents_text(node),
        }),
        AstNodeKind::Math { inline: true } => Some(NoteSpan::InlineMath {
            tex: contents_text(node),
        }),
        AstNodeKind::Image { .. } => image_ref(node).map(|image| NoteSpan::Image { image }),
        // Block containers mixed into a line cannot be rendered inline; the
        // caller handles the single-content case and anything else is dropped.
        _ => None,
    }
}

pub(crate) fn task_info(prop: &Property) -> Option<TaskInfo> {
    let Property::Task {
        status,
        due,
        scheduled,
        completed_at,
        started_at,
        time_spent,
        location,
        ..
    } = prop
    else {
        return None;
    };

    Some(TaskInfo {
        status: status.into(),
        due: TaskDate::from_deadline(due),
        scheduled: scheduled.as_ref().and_then(TaskDate::from_deadline),
        completed_at: completed_at.as_ref().and_then(TaskDate::from_deadline),
        started_at: started_at.as_ref().and_then(TaskDate::from_deadline),
        time_spent_minutes: time_spent.as_ref().map(|d| d.hours * 60 + d.minutes),
        is_shorthand: is_shorthand(location),
    })
}
