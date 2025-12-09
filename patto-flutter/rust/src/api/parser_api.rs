//! Parser API for Flutter
//!
//! This module provides parsing functionality for .pn files via flutter_rust_bridge.
//! It wraps the main patto crate's parser and converts the AST to a format suitable for Dart.

use flutter_rust_bridge::frb;
use patto::parser::{parse_text, AstNode, AstNodeKind, Deadline, Property, TaskStatus};

/// Result of parsing a document
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub struct ParseResult {
    pub success: bool,
    pub ast: Option<DartAstNode>,
    pub errors: Vec<ParseError>,
}

/// Parser error information
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: u32,
    pub column_start: u32,
    pub column_end: u32,
}

/// Location information for AST nodes
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub struct DartLocation {
    pub line: u32,
    pub column_start: u32,
    pub column_end: u32,
}

/// Property types that can be attached to lines
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub enum DartProperty {
    Task {
        status: String,
        due: Option<String>,
    },
    Anchor {
        name: String,
    },
}

/// AST Node representation for Dart consumption
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub enum DartAstNode {
    /// Root node containing children
    Dummy {
        children: Vec<DartAstNode>,
    },
    /// A line of text with optional properties
    Line {
        contents: Vec<DartAstNode>,
        properties: Vec<DartProperty>,
        children: Vec<DartAstNode>,
        depth: u32,
        location: Option<DartLocation>,
    },
    /// Plain text content
    Text {
        text: String,
        location: Option<DartLocation>,
    },
    /// Wiki link to another note: [PageName] or [PageName#anchor]
    WikiLink {
        name: String,
        anchor: Option<String>,
        location: Option<DartLocation>,
    },
    /// URL link: [title url] or [url]
    UrlLink {
        url: String,
        title: Option<String>,
        location: Option<DartLocation>,
    },
    /// Image: [@img "alt" path]
    Image {
        src: String,
        alt: Option<String>,
        location: Option<DartLocation>,
    },
    /// Code block: [@code lang]
    CodeBlock {
        language: String,
        content: String,
        location: Option<DartLocation>,
    },
    /// Inline code: [`code`]
    CodeInline {
        content: String,
        location: Option<DartLocation>,
    },
    /// Math block: [@math]
    MathBlock {
        content: String,
        location: Option<DartLocation>,
    },
    /// Inline math: [$formula$]
    MathInline {
        content: String,
        location: Option<DartLocation>,
    },
    /// Quote block: [@quote]
    QuoteBlock {
        cite: Option<String>,
        children: Vec<DartAstNode>,
        location: Option<DartLocation>,
    },
    /// Table block: [@table]
    TableBlock {
        caption: Option<String>,
        children: Vec<DartAstNode>,
        location: Option<DartLocation>,
    },
    /// Table row
    TableRow {
        children: Vec<DartAstNode>,
        location: Option<DartLocation>,
    },
    /// Table column/cell
    TableColumn {
        contents: Vec<DartAstNode>,
        location: Option<DartLocation>,
    },
    /// Text decoration: bold, italic, underline, deleted
    Decoration {
        fontsize: i32,
        italic: bool,
        underline: bool,
        deleted: bool,
        contents: Vec<DartAstNode>,
        location: Option<DartLocation>,
    },
    /// Horizontal rule: -----
    HorizontalRule {
        location: Option<DartLocation>,
    },
}

/// Wiki link information extracted from a document
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub struct LinkInfo {
    pub name: String,
    pub line: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub anchor: Option<String>,
}

/// Anchor information extracted from a document
#[frb(dart_metadata=("freezed"))]
#[derive(Debug, Clone)]
pub struct AnchorInfo {
    pub name: String,
    pub line: u32,
}

/// Parse a Patto document and return the AST
///
/// This function parses the given content string as a .pn file and returns
/// the resulting AST along with any parse errors.
#[frb(sync)]
pub fn parse_document(content: String) -> ParseResult {
    let result = parse_text(&content);

    let errors: Vec<ParseError> = result
        .parse_errors
        .iter()
        .map(|e| match e {
            patto::parser::ParserError::ParseError(loc, _) => ParseError {
                message: "Parse error".to_string(),
                line: loc.row as u32,
                column_start: loc.span.0 as u32,
                column_end: loc.span.1 as u32,
            },
            patto::parser::ParserError::InvalidIndentation(loc) => ParseError {
                message: "Invalid indentation".to_string(),
                line: loc.row as u32,
                column_start: loc.span.0 as u32,
                column_end: loc.span.1 as u32,
            },
        })
        .collect();

    ParseResult {
        success: errors.is_empty(),
        ast: Some(convert_ast_node(&result.ast, 0)),
        errors,
    }
}

/// Extract wiki links from content
///
/// Returns a list of all wiki links found in the document,
/// including their location information.
#[frb(sync)]
pub fn get_links(content: String) -> Vec<LinkInfo> {
    let result = parse_text(&content);
    let mut links = Vec::new();
    collect_links(&result.ast, &mut links);
    links
}

/// Extract anchors from content
///
/// Returns a list of all anchors defined in the document.
#[frb(sync)]
pub fn get_anchors(content: String) -> Vec<AnchorInfo> {
    let result = parse_text(&content);
    let mut anchors = Vec::new();
    collect_anchors(&result.ast, &mut anchors);
    anchors
}

// Internal helper functions

fn convert_ast_node(node: &AstNode, depth: u32) -> DartAstNode {
    let location = Some(DartLocation {
        line: node.location().row as u32,
        column_start: node.location().span.0 as u32,
        column_end: node.location().span.1 as u32,
    });

    match node.kind() {
        AstNodeKind::Dummy => {
            let children = node.value().children.lock().unwrap();
            DartAstNode::Dummy {
                children: children.iter().map(|c| convert_ast_node(c, 0)).collect(),
            }
        }
        AstNodeKind::Line { properties } => {
            let contents = node.value().contents.lock().unwrap();
            let children = node.value().children.lock().unwrap();

            // Calculate depth from indentation
            let input = &node.location().input;
            let indent = input.len() - input.trim_start().len();
            let line_depth = (indent / 2) as u32;

            DartAstNode::Line {
                contents: contents
                    .iter()
                    .map(|c| convert_ast_node(c, line_depth))
                    .collect(),
                properties: properties.iter().map(convert_property).collect(),
                children: children
                    .iter()
                    .map(|c| convert_ast_node(c, line_depth + 1))
                    .collect(),
                depth: line_depth,
                location,
            }
        }
        AstNodeKind::Text => {
            let input = &node.location().input;
            let span = &node.location().span;
            let text = if span.1 <= input.len() {
                input[span.0..span.1].to_string()
            } else {
                input.to_string()
            };
            DartAstNode::Text { text, location }
        }
        AstNodeKind::WikiLink { link, anchor } => DartAstNode::WikiLink {
            name: link.clone(),
            anchor: anchor.clone(),
            location,
        },
        AstNodeKind::Link { link, title } => DartAstNode::UrlLink {
            url: link.clone(),
            title: title.clone(),
            location,
        },
        AstNodeKind::Image { src, alt } => DartAstNode::Image {
            src: src.clone(),
            alt: alt.clone(),
            location,
        },
        AstNodeKind::Code { lang, inline } => {
            if *inline {
                let contents = node.value().contents.lock().unwrap();
                let content = contents
                    .iter()
                    .map(|c| {
                        let loc = c.location();
                        if loc.span.1 <= loc.input.len() {
                            loc.input[loc.span.0..loc.span.1].to_string()
                        } else {
                            loc.input.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("");
                DartAstNode::CodeInline { content, location }
            } else {
                let children = node.value().children.lock().unwrap();
                let content = children
                    .iter()
                    .map(|c| {
                        let loc = c.location();
                        if loc.span.1 <= loc.input.len() {
                            loc.input[loc.span.0..loc.span.1].to_string()
                        } else {
                            loc.input.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                DartAstNode::CodeBlock {
                    language: lang.clone(),
                    content,
                    location,
                }
            }
        }
        AstNodeKind::CodeContent => {
            let input = &node.location().input;
            let span = &node.location().span;
            let text = if span.1 <= input.len() {
                input[span.0..span.1].to_string()
            } else {
                input.to_string()
            };
            DartAstNode::Text { text, location }
        }
        AstNodeKind::Math { inline } => {
            if *inline {
                let contents = node.value().contents.lock().unwrap();
                let content = contents
                    .iter()
                    .map(|c| {
                        let loc = c.location();
                        if loc.span.1 <= loc.input.len() {
                            loc.input[loc.span.0..loc.span.1].to_string()
                        } else {
                            loc.input.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("");
                DartAstNode::MathInline { content, location }
            } else {
                let children = node.value().children.lock().unwrap();
                let content = children
                    .iter()
                    .map(|c| {
                        let loc = c.location();
                        if loc.span.1 <= loc.input.len() {
                            loc.input[loc.span.0..loc.span.1].to_string()
                        } else {
                            loc.input.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                DartAstNode::MathBlock { content, location }
            }
        }
        AstNodeKind::MathContent => {
            let input = &node.location().input;
            let span = &node.location().span;
            let text = if span.1 <= input.len() {
                input[span.0..span.1].to_string()
            } else {
                input.to_string()
            };
            DartAstNode::Text { text, location }
        }
        AstNodeKind::Quote => {
            let children = node.value().children.lock().unwrap();
            DartAstNode::QuoteBlock {
                cite: None,
                children: children.iter().map(|c| convert_ast_node(c, depth)).collect(),
                location,
            }
        }
        AstNodeKind::QuoteContent { properties: _ } => {
            let contents = node.value().contents.lock().unwrap();
            DartAstNode::Line {
                contents: contents.iter().map(|c| convert_ast_node(c, depth)).collect(),
                properties: vec![],
                children: vec![],
                depth,
                location,
            }
        }
        AstNodeKind::Table { caption } => {
            let children = node.value().children.lock().unwrap();
            DartAstNode::TableBlock {
                caption: caption.clone(),
                children: children.iter().map(|c| convert_ast_node(c, depth)).collect(),
                location,
            }
        }
        AstNodeKind::TableRow => {
            let children = node.value().children.lock().unwrap();
            DartAstNode::TableRow {
                children: children.iter().map(|c| convert_ast_node(c, depth)).collect(),
                location,
            }
        }
        AstNodeKind::TableColumn => {
            let contents = node.value().contents.lock().unwrap();
            DartAstNode::TableColumn {
                contents: contents.iter().map(|c| convert_ast_node(c, depth)).collect(),
                location,
            }
        }
        AstNodeKind::Decoration {
            fontsize,
            italic,
            underline,
            deleted,
        } => {
            let contents = node.value().contents.lock().unwrap();
            DartAstNode::Decoration {
                fontsize: *fontsize as i32,
                italic: *italic,
                underline: *underline,
                deleted: *deleted,
                contents: contents.iter().map(|c| convert_ast_node(c, depth)).collect(),
                location,
            }
        }
        AstNodeKind::HorizontalLine => DartAstNode::HorizontalRule { location },
    }
}

fn convert_property(prop: &Property) -> DartProperty {
    match prop {
        Property::Task { status, due, .. } => {
            let status_str = match status {
                TaskStatus::Todo => "Todo",
                TaskStatus::Doing => "Doing",
                TaskStatus::Done => "Done",
            };
            let due_str = match due {
                Deadline::Date(date) => Some(date.format("%Y-%m-%d").to_string()),
                Deadline::DateTime(datetime) => Some(datetime.format("%Y-%m-%dT%H:%M").to_string()),
                Deadline::Uninterpretable(s) => Some(s.clone()),
            };
            DartProperty::Task {
                status: status_str.to_string(),
                due: due_str,
            }
        }
        Property::Anchor { name, .. } => DartProperty::Anchor { name: name.clone() },
    }
}

fn collect_links(node: &AstNode, links: &mut Vec<LinkInfo>) {
    if let AstNodeKind::WikiLink { link, anchor } = node.kind() {
        links.push(LinkInfo {
            name: link.clone(),
            line: node.location().row as u32,
            column_start: node.location().span.0 as u32,
            column_end: node.location().span.1 as u32,
            anchor: anchor.clone(),
        });
    }

    // Recursively collect from contents
    {
        let contents = node.value().contents.lock().unwrap();
        for content in contents.iter() {
            collect_links(content, links);
        }
    }

    // Recursively collect from children
    {
        let children = node.value().children.lock().unwrap();
        for child in children.iter() {
            collect_links(child, links);
        }
    }
}

fn collect_anchors(node: &AstNode, anchors: &mut Vec<AnchorInfo>) {
    match node.kind() {
        AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } => {
            for prop in properties {
                if let Property::Anchor { name, .. } = prop {
                    anchors.push(AnchorInfo {
                        name: name.clone(),
                        line: node.location().row as u32,
                    });
                }
            }
        }
        _ => {}
    }

    // Recursively collect from children
    {
        let children = node.value().children.lock().unwrap();
        for child in children.iter() {
            collect_anchors(child, anchors);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_line() {
        let result = parse_document("Hello world".to_string());
        assert!(result.success);
        assert!(result.ast.is_some());
    }

    #[test]
    fn test_parse_wiki_link() {
        let result = parse_document("Check out [SomePage] for more".to_string());
        assert!(result.success);

        let links = get_links("Check out [SomePage] for more".to_string());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].name, "SomePage");
    }

    #[test]
    fn test_parse_wiki_link_with_anchor() {
        let links = get_links("See [PageName#section] for details".to_string());
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].name, "PageName");
        assert_eq!(links[0].anchor, Some("section".to_string()));
    }

    #[test]
    fn test_parse_task() {
        let result = parse_document("Do something !2024-12-31".to_string());
        assert!(result.success);
    }

    #[test]
    fn test_get_anchors() {
        let anchors = get_anchors("This is a section #myanchor".to_string());
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].name, "myanchor");
    }
}
