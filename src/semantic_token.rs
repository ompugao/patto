use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};
use str_indices::utf16::from_byte_idx as utf16_from_byte_idx;

use crate::parser::{AstNode, AstNodeKind, Property};

pub const LEGEND_TYPE: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::STRING,
    SemanticTokenType::COMMENT,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::TYPE,
    SemanticTokenType::METHOD,
    SemanticTokenType::ENUM,
    SemanticTokenType::MODIFIER,
];

// Token type indices (must match LEGEND_TYPE order)
const TOKEN_TYPE_FUNCTION: u32 = 0;
const TOKEN_TYPE_VARIABLE: u32 = 1;
const TOKEN_TYPE_STRING: u32 = 2;
const TOKEN_TYPE_COMMENT: u32 = 3;
const TOKEN_TYPE_KEYWORD: u32 = 4;
const TOKEN_TYPE_OPERATOR: u32 = 5;
const TOKEN_TYPE_PARAMETER: u32 = 6;
const TOKEN_TYPE_PROPERTY: u32 = 7;
const TOKEN_TYPE_TYPE: u32 = 8;
const TOKEN_TYPE_METHOD: u32 = 9;
const TOKEN_TYPE_ENUM: u32 = 10;
const TOKEN_TYPE_MODIFIER: u32 = 11;

#[derive(Debug, Clone)]
struct ImCompleteSemanticToken {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
}

fn properties_to_tokens(properties: &Vec<Property>, tokens: &mut Vec<ImCompleteSemanticToken>) {
    for prop in properties {
        match prop {
            Property::Task { location, .. } => {
                // Highlight @task as COMMENT
                let line_text: &str = location.input.as_ref();
                let start = utf16_from_byte_idx(line_text, location.span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, location.span.1) - utf16_from_byte_idx(line_text, location.span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: location.row as u32,
                    start,
                    length,
                    token_type: TOKEN_TYPE_COMMENT,
                });
            }
            Property::Anchor { location, .. } => {
                // Highlight anchor as KEYWORD
                let line_text: &str = location.input.as_ref();
                let start = utf16_from_byte_idx(line_text, location.span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, location.span.1) - utf16_from_byte_idx(line_text, location.span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: location.row as u32,
                    start,
                    length,
                    token_type: TOKEN_TYPE_KEYWORD,
                });
            }
        }
    }
}

fn collect_semantic_tokens(node: &AstNode, tokens: &mut Vec<ImCompleteSemanticToken>, line_range: Option<(u32, u32)>) {
    let location = node.location();
    let row = location.row as u32;
    let span = &location.span;
    
    let mut b_process: bool = true;
    if let Some((start_line, end_line)) = line_range {
        if start_line > row || row > end_line {
            b_process = false;
        }
    }
    if b_process {
        let line_text: &str = location.input.as_ref();
        match node.kind() {
            AstNodeKind::WikiLink { .. } => {
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type: TOKEN_TYPE_OPERATOR,
                });
            }
            AstNodeKind::Link { .. } => {
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type: TOKEN_TYPE_FUNCTION,
                });
            }
            AstNodeKind::Code { lang: _lang, inline } => {
                let token_type = if *inline {
                    TOKEN_TYPE_STRING
                } else {
                    TOKEN_TYPE_COMMENT
                };
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type,
                });
            }
            AstNodeKind::Math { inline } => {
                let token_type = if *inline {
                    TOKEN_TYPE_ENUM
                } else {
                    TOKEN_TYPE_COMMENT
                };
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type,
                });
            }
            AstNodeKind::Image { .. } => {
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type: TOKEN_TYPE_VARIABLE,
                });
            }
            AstNodeKind::Quote => {
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type: TOKEN_TYPE_COMMENT,
                });
            }
            AstNodeKind::MathContent => {
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type: TOKEN_TYPE_ENUM,
                });
            }
            AstNodeKind::CodeContent => {
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type: TOKEN_TYPE_STRING,
                });
            }
            AstNodeKind::Table { .. } => {
                // Highlight @table command as PROPERTY
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type: TOKEN_TYPE_PROPERTY,
                });
            }
            AstNodeKind::Decoration { fontsize: _, italic: _, underline: _, deleted } => {
                // Highlight decoration based on type
                // Deleted text should be highlighted as COMMENT (indicates removed/deprecated)
                // Other decorations (bold, italic, underline) as MODIFIER
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                let token_type = if *deleted {
                    TOKEN_TYPE_COMMENT
                } else {
                    TOKEN_TYPE_MODIFIER
                };
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type,
                });
            }
            AstNodeKind::HorizontalLine => {
                // Highlight horizontal line as COMMENT (visual separator)
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type: TOKEN_TYPE_COMMENT,
                });
            }
            AstNodeKind::Line { properties } => {
                properties_to_tokens(properties, tokens);
            }
            AstNodeKind::QuoteContent { properties } => {
                let start = utf16_from_byte_idx(line_text, span.0) as u32;
                let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                tokens.push(ImCompleteSemanticToken {
                    line: row,
                    start,
                    length,
                    token_type: TOKEN_TYPE_COMMENT,
                });
                properties_to_tokens(properties, tokens);
            }
            _ => {}
        }
    }

    // Recursively process children and contents
    for child in node.value().children.lock().unwrap().iter() {
        collect_semantic_tokens(child, tokens, line_range);
    }
    for content in node.value().contents.lock().unwrap().iter() {
        collect_semantic_tokens(content, tokens, line_range);
    }
}

fn build_semantic_tokens(tokens: Vec<ImCompleteSemanticToken>) -> Vec<SemanticToken> {
    let mut sorted_tokens = tokens;
    sorted_tokens.sort_by(|a, b| {
        if a.line != b.line {
            a.line.cmp(&b.line)
        } else {
            a.start.cmp(&b.start)
        }
    });
    
    let mut result = Vec::new();
    let mut prev_line = 0;
    let mut prev_start = 0;
    
    for token in sorted_tokens {
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line == 0 {
            token.start - prev_start
        } else {
            token.start
        };
        
        result.push(SemanticToken {
            delta_line,
            delta_start,
            length: token.length,
            token_type: token.token_type,
            token_modifiers_bitset: 0,
        });
        
        prev_line = token.line;
        prev_start = token.start;
    }
    
    result
}

pub fn get_semantic_tokens(ast: &AstNode) -> Vec<SemanticToken> {
    let mut incomplete_tokens = Vec::new();
    collect_semantic_tokens(ast, &mut incomplete_tokens, None);
    build_semantic_tokens(incomplete_tokens)
}

pub fn get_semantic_tokens_range(ast: &AstNode, start_line: u32, end_line: u32) -> Vec<SemanticToken> {
    let mut incomplete_tokens = Vec::new();
    collect_semantic_tokens(ast, &mut incomplete_tokens, Some((start_line, end_line)));

    // Filter tokens within the requested range
    let filtered_tokens: Vec<ImCompleteSemanticToken> = incomplete_tokens
        .into_iter()
        .filter(|token| token.line >= start_line && token.line <= end_line)
        .collect();

    build_semantic_tokens(filtered_tokens)
}
