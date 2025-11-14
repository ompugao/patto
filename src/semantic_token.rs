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

#[derive(Debug, Clone)]
struct ImCompleteSemanticToken {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
}

fn collect_semantic_tokens(node: &AstNode, tokens: &mut Vec<ImCompleteSemanticToken>, text: &str) {
    let location = node.location();
    let row = location.row as u32;
    let span = &location.span;
    
    // Get the line text for UTF-16 conversion
    let lines: Vec<&str> = text.lines().collect();
    if row as usize >= lines.len() {
        // Skip children and contents for this node
        for child in node.value().children.lock().unwrap().iter() {
            collect_semantic_tokens(child, tokens, text);
        }
        for content in node.value().contents.lock().unwrap().iter() {
            collect_semantic_tokens(content, tokens, text);
        }
        return;
    }
    let line_text = lines[row as usize];
    
    match node.kind() {
        AstNodeKind::WikiLink { .. } => {
            // Highlight the entire wikilink as PARAMETER
            let start = utf16_from_byte_idx(line_text, span.0) as u32;
            let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
            tokens.push(ImCompleteSemanticToken {
                line: row,
                start,
                length,
                token_type: TOKEN_TYPE_PARAMETER,
            });
        }
        AstNodeKind::Link { .. } => {
            // Highlight link URL as STRING
            let start = utf16_from_byte_idx(line_text, span.0) as u32;
            let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
            tokens.push(ImCompleteSemanticToken {
                line: row,
                start,
                length,
                token_type: TOKEN_TYPE_STRING,
            });
        }
        AstNodeKind::Code { .. } => {
            // Highlight @code command as PROPERTY
            let start = utf16_from_byte_idx(line_text, span.0) as u32;
            let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
            tokens.push(ImCompleteSemanticToken {
                line: row,
                start,
                length,
                token_type: TOKEN_TYPE_PROPERTY,
            });
        }
        AstNodeKind::Math { .. } => {
            // Highlight @math command as PROPERTY
            let start = utf16_from_byte_idx(line_text, span.0) as u32;
            let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
            tokens.push(ImCompleteSemanticToken {
                line: row,
                start,
                length,
                token_type: TOKEN_TYPE_PROPERTY,
            });
        }
        AstNodeKind::Image { .. } => {
            // Highlight @img command as PROPERTY
            let start = utf16_from_byte_idx(line_text, span.0) as u32;
            let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
            tokens.push(ImCompleteSemanticToken {
                line: row,
                start,
                length,
                token_type: TOKEN_TYPE_PROPERTY,
            });
        }
        AstNodeKind::Quote => {
            // Highlight quote as COMMENT
            let start = utf16_from_byte_idx(line_text, span.0) as u32;
            let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
            tokens.push(ImCompleteSemanticToken {
                line: row,
                start,
                length,
                token_type: TOKEN_TYPE_COMMENT,
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
        AstNodeKind::Decoration { .. } => {
            // Highlight decoration markers as OPERATOR
            let start = utf16_from_byte_idx(line_text, span.0) as u32;
            let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
            tokens.push(ImCompleteSemanticToken {
                line: row,
                start,
                length,
                token_type: TOKEN_TYPE_OPERATOR,
            });
        }
        AstNodeKind::Line { properties } => {
            // Handle properties (tasks, anchors)
            for prop in properties {
                match prop {
                    Property::Task { .. } => {
                        // Highlight @task as PROPERTY
                        let start = utf16_from_byte_idx(line_text, span.0) as u32;
                        let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                        tokens.push(ImCompleteSemanticToken {
                            line: row,
                            start,
                            length,
                            token_type: TOKEN_TYPE_COMMENT,
                        });
                    }
                    Property::Anchor { .. } => {
                        // Highlight anchor as STRING
                        let start = utf16_from_byte_idx(line_text, span.0) as u32;
                        let length = (utf16_from_byte_idx(line_text, span.1) - utf16_from_byte_idx(line_text, span.0)) as u32;
                        tokens.push(ImCompleteSemanticToken {
                            line: row,
                            start,
                            length,
                            token_type: TOKEN_TYPE_KEYWORD,
                        });
                    }
                }
            }
        }
        _ => {}
    }
    
    // Recursively process children and contents
    for child in node.value().children.lock().unwrap().iter() {
        collect_semantic_tokens(child, tokens, text);
    }
    for content in node.value().contents.lock().unwrap().iter() {
        collect_semantic_tokens(content, tokens, text);
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

pub fn get_semantic_tokens(ast: &AstNode, text: &str) -> Vec<SemanticToken> {
    let mut incomplete_tokens = Vec::new();
    collect_semantic_tokens(ast, &mut incomplete_tokens, text);
    build_semantic_tokens(incomplete_tokens)
}

pub fn get_semantic_tokens_range(ast: &AstNode, text: &str, start_line: u32, end_line: u32) -> Vec<SemanticToken> {
    let mut incomplete_tokens = Vec::new();
    collect_semantic_tokens(ast, &mut incomplete_tokens, text);
    
    // Filter tokens within the requested range
    let filtered_tokens: Vec<ImCompleteSemanticToken> = incomplete_tokens
        .into_iter()
        .filter(|token| token.line >= start_line && token.line <= end_line)
        .collect();
    
    build_semantic_tokens(filtered_tokens)
}
