//! `textDocument/semanticTokens`: one token per highlighted node span, encoded
//! as the deltas the protocol expects.

use str_indices::utf16::from_byte_idx as utf16_from_byte_idx;
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};

use crate::parser::{AstNode, AstNodeKind, Location, Property};

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

// Indices into LEGEND_TYPE; `legend_indices_match_token_types` keeps them honest.
const FUNCTION: u32 = 0;
const STRING: u32 = 2;
const COMMENT: u32 = 3;
const KEYWORD: u32 = 4;
const OPERATOR: u32 = 5;
const PROPERTY: u32 = 7;
const TYPE: u32 = 8;
const ENUM: u32 = 10;
const MODIFIER: u32 = 11;

#[derive(Debug, Clone)]
struct AbsoluteToken {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
}

pub fn get_semantic_tokens(ast: &AstNode) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    collect_semantic_tokens(ast, &mut tokens);
    delta_encode(tokens)
}

pub fn get_semantic_tokens_range(
    ast: &AstNode,
    start_line: u32,
    end_line: u32,
) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    collect_semantic_tokens(ast, &mut tokens);
    tokens.retain(|token| (start_line..=end_line).contains(&token.line));
    delta_encode(tokens)
}

fn collect_semantic_tokens(node: &AstNode, tokens: &mut Vec<AbsoluteToken>) {
    if let Some(token_type) = node_token_type(node.kind()) {
        tokens.push(span_token(node.location(), token_type));
    }
    if let AstNodeKind::Line { properties } | AstNodeKind::QuoteContent { properties } = node.kind()
    {
        tokens.extend(properties.iter().map(property_token));
    }
    for child in node.children().iter() {
        collect_semantic_tokens(child, tokens);
    }
    for content in node.contents().iter() {
        collect_semantic_tokens(content, tokens);
    }
}

/// `None` leaves the span unhighlighted. `MathContent` is deliberately among
/// them so the editor renders the body as TeX.
fn node_token_type(kind: &AstNodeKind) -> Option<u32> {
    let token_type = match kind {
        AstNodeKind::WikiLink { .. } => OPERATOR,
        AstNodeKind::Link { .. } => FUNCTION,
        AstNodeKind::Code { inline: true, .. } => STRING,
        AstNodeKind::Code { inline: false, .. } => COMMENT,
        AstNodeKind::Math { inline: true } => ENUM,
        AstNodeKind::Math { inline: false } => COMMENT,
        AstNodeKind::Image { .. } => TYPE,
        AstNodeKind::Quote | AstNodeKind::QuoteContent { .. } => COMMENT,
        AstNodeKind::HorizontalLine => COMMENT,
        AstNodeKind::CodeContent => STRING,
        AstNodeKind::Table { .. } => PROPERTY,
        AstNodeKind::Decoration { deleted: true, .. } => COMMENT,
        AstNodeKind::Decoration { .. } => MODIFIER,
        _ => return None,
    };
    Some(token_type)
}

fn property_token(property: &Property) -> AbsoluteToken {
    match property {
        Property::Task { location, .. } => span_token(location, COMMENT),
        Property::Anchor { location, .. } => span_token(location, KEYWORD),
    }
}

fn span_token(location: &Location, token_type: u32) -> AbsoluteToken {
    let line_text: &str = location.input.as_ref();
    let start = utf16_from_byte_idx(line_text, location.span.0) as u32;
    let end = utf16_from_byte_idx(line_text, location.span.1) as u32;
    AbsoluteToken {
        line: location.row as u32,
        start,
        length: end - start,
        token_type,
    }
}

fn delta_encode(mut tokens: Vec<AbsoluteToken>) -> Vec<SemanticToken> {
    tokens.sort_by_key(|token| (token.line, token.start));

    let mut encoded = Vec::with_capacity(tokens.len());
    let (mut prev_line, mut prev_start) = (0, 0);
    for token in tokens {
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line == 0 {
            token.start - prev_start
        } else {
            token.start
        };
        encoded.push(SemanticToken {
            delta_line,
            delta_start,
            length: token.length,
            token_type: token.token_type,
            token_modifiers_bitset: 0,
        });
        prev_line = token.line;
        prev_start = token.start;
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        "Normal text [wikilink] and [note#anchor] 牛乳 [https://example.com title]\n\
#anchor1 {@anchor long} {@task status=todo due=2024-12-31}\n\
inline [` code `] and [$ x^2 $] and [* bold *] [- gone -] [@img ./a.png alt]\n\
[@code rust]\n\
\tfn main() {}\n\
[@math]\n\
\tx = 1\n\
[@quote]\n\
\tquoted {@task status=done due=2024-12-31}\n\
\t\tnested quote\n\
[@table caption]\n\
\ta\tb\n\
last [link]\n";

    fn tuples(tokens: Vec<SemanticToken>) -> Vec<(u32, u32, u32, u32)> {
        tokens
            .into_iter()
            .map(|t| (t.delta_line, t.delta_start, t.length, t.token_type))
            .collect()
    }

    #[test]
    fn legend_indices_match_token_types() {
        assert_eq!(LEGEND_TYPE[FUNCTION as usize], SemanticTokenType::FUNCTION);
        assert_eq!(LEGEND_TYPE[STRING as usize], SemanticTokenType::STRING);
        assert_eq!(LEGEND_TYPE[COMMENT as usize], SemanticTokenType::COMMENT);
        assert_eq!(LEGEND_TYPE[KEYWORD as usize], SemanticTokenType::KEYWORD);
        assert_eq!(LEGEND_TYPE[OPERATOR as usize], SemanticTokenType::OPERATOR);
        assert_eq!(LEGEND_TYPE[PROPERTY as usize], SemanticTokenType::PROPERTY);
        assert_eq!(LEGEND_TYPE[TYPE as usize], SemanticTokenType::TYPE);
        assert_eq!(LEGEND_TYPE[ENUM as usize], SemanticTokenType::ENUM);
        assert_eq!(LEGEND_TYPE[MODIFIER as usize], SemanticTokenType::MODIFIER);
    }

    #[test]
    fn full_document_tokens_are_delta_encoded_in_utf16_units() {
        let ast = crate::parser::parse_text(FIXTURE).ast;
        assert_eq!(
            tuples(get_semantic_tokens(&ast)),
            [
                (0, 12, 10, OPERATOR),
                (0, 15, 13, OPERATOR),
                (0, 18, 25, FUNCTION),
                (1, 0, 8, KEYWORD),
                (0, 9, 14, KEYWORD),
                (0, 15, 34, COMMENT),
                (1, 7, 10, STRING),
                (0, 15, 9, ENUM),
                (0, 14, 10, MODIFIER),
                (0, 11, 10, COMMENT),
                (0, 11, 18, TYPE),
                (1, 0, 12, COMMENT),
                (1, 1, 12, STRING),
                (1, 0, 7, COMMENT),
                (2, 0, 8, COMMENT),
                (1, 1, 41, COMMENT),
                (1, 2, 12, COMMENT),
                (1, 0, 16, PROPERTY),
                (2, 5, 6, OPERATOR),
            ]
        );
    }

    #[test]
    fn range_tokens_start_their_deltas_from_the_document_origin() {
        let ast = crate::parser::parse_text(FIXTURE).ast;
        assert_eq!(
            tuples(get_semantic_tokens_range(&ast, 2, 8)),
            [
                (2, 7, 10, STRING),
                (0, 15, 9, ENUM),
                (0, 14, 10, MODIFIER),
                (0, 11, 10, COMMENT),
                (0, 11, 18, TYPE),
                (1, 0, 12, COMMENT),
                (1, 1, 12, STRING),
                (1, 0, 7, COMMENT),
                (2, 0, 8, COMMENT),
                (1, 1, 41, COMMENT),
            ]
        );
    }

    #[test]
    fn math_body_is_left_unhighlighted() {
        let ast = crate::parser::parse_text("[@math]\n\tx = 1\n").ast;
        assert_eq!(tuples(get_semantic_tokens(&ast)), [(0, 0, 7, COMMENT)]);
    }

    #[test]
    fn empty_document_has_no_tokens() {
        let ast = crate::parser::parse_text("").ast;
        assert!(get_semantic_tokens(&ast).is_empty());
    }
}
