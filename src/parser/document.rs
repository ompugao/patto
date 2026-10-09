use std::cmp;
use std::sync::Arc;

use pest::Parser;

use super::command::parse_command_line;
use super::inline::transform_statement;
use super::{
    AstNode, AstNodeKind, Location, ParserError, ParserResult, PattoLineParser, Rule, Span,
};
use crate::line_tracker::LineTracker;

/// State for a single quote level in the quote stack
struct QuoteLevel {
    node: AstNode,
    min_indent: usize,
}

/// Quote-specific state with nesting support
struct QuoteState {
    stack: Vec<QuoteLevel>,
}

impl QuoteState {
    fn new(node: AstNode, min_indent: usize) -> Self {
        Self {
            stack: vec![QuoteLevel { node, min_indent }],
        }
    }

    fn current(&self) -> &QuoteLevel {
        self.stack.last().expect("quote stack should not be empty")
    }

    fn current_node(&self) -> &AstNode {
        &self.current().node
    }

    fn current_min_indent(&self) -> usize {
        self.current().min_indent
    }

    fn push(&mut self, node: AstNode, min_indent: usize) {
        self.stack.push(QuoteLevel { node, min_indent });
    }

    fn len(&self) -> usize {
        self.stack.len()
    }
}

/// Block context - carries both state and associated data
/// - `last_nonempty_indent` in Line = "remember this indent so empty lines inherit the right parent depth"
/// - `min_indent` in blocks = "content must be indented at least this much to belong to this block"
enum BlockContext {
    Line { last_nonempty_indent: usize },
    Quote(QuoteState),
    Code { node: AstNode, min_indent: usize },
    Math { node: AstNode, min_indent: usize },
    Table { node: AstNode, min_indent: usize },
}

fn find_parent_line(parent: AstNode, depth: usize) -> Option<AstNode> {
    if depth == 0 {
        return Some(parent);
    }
    let last_child_line = parent
        .children()
        .iter()
        .filter_map(|e| match e.kind() {
            AstNodeKind::Line { .. } => Some(e.clone()),
            _ => None,
        })
        .next_back()?;
    find_parent_line(last_child_line, depth - 1)
}

/// Find parent QuoteContent for nested indentation within a quote block.
/// relative_indent=0 returns the quote node itself.
fn find_parent_quote_content(quote: &AstNode, relative_indent: usize) -> AstNode {
    if relative_indent == 0 {
        return quote.clone();
    }

    let children = quote.children();
    if let Some(last_qc) = children
        .iter()
        .rfind(|c| matches!(c.kind(), AstNodeKind::QuoteContent { .. }))
    {
        find_parent_quote_content(last_qc, relative_indent - 1)
    } else {
        quote.clone() // Fallback if no children yet
    }
}

/// Check if line should exit current block (looking ahead for empty lines)
fn should_exit_block(
    indent: usize,
    min_indent: usize,
    content_len: usize,
    indent_content_len: &[(usize, usize)],
    current_line: usize,
) -> bool {
    // Non-empty line below min_indent exits
    if content_len > 0 && indent < min_indent {
        return true;
    }

    // Empty line: check if next non-empty line is still in block
    if content_len == 0 {
        for &(next_indent, next_content_len) in &indent_content_len[current_line + 1..] {
            if next_content_len > 0 {
                return next_indent < min_indent;
            }
        }
    }

    false
}

pub fn parse_text(text: &str) -> ParserResult {
    let indent_content_len: Vec<_> = text
        .lines()
        .map(|l| {
            let indent = l.chars().take_while(|&c| c == '\t').count();
            let content_len = l.len() - indent;
            (indent, content_len)
        })
        .collect();

    let root = AstNode::new(text, 0, None, Some(AstNodeKind::Dummy));
    let mut lastlinenode = root.clone();

    let mut block_context = BlockContext::Line {
        last_nonempty_indent: 0,
    };

    let mut errors: Vec<ParserError> = Vec::new();
    for (iline, linetext) in text.lines().enumerate() {
        let (indent, content_len) = indent_content_len[iline];

        // Handle block exit first
        match &mut block_context {
            BlockContext::Line { .. } => {}

            BlockContext::Quote(state) => {
                // Pop nested quotes that we've exited
                while state.len() > 1
                    && should_exit_block(
                        indent,
                        state.current_min_indent(),
                        content_len,
                        &indent_content_len,
                        iline,
                    )
                {
                    state.stack.pop();
                }

                // Check if exited quote entirely
                if state.len() == 1
                    && should_exit_block(
                        indent,
                        state.current_min_indent(),
                        content_len,
                        &indent_content_len,
                        iline,
                    )
                {
                    block_context = BlockContext::Line {
                        last_nonempty_indent: indent,
                    };
                }
            }

            BlockContext::Code { min_indent, .. }
            | BlockContext::Math { min_indent, .. }
            | BlockContext::Table { min_indent, .. } => {
                if should_exit_block(indent, *min_indent, content_len, &indent_content_len, iline) {
                    block_context = BlockContext::Line {
                        last_nonempty_indent: indent,
                    };
                }
            }
        }

        // Process line based on context
        match &mut block_context {
            BlockContext::Line {
                last_nonempty_indent,
            } => {
                // Normal line mode - use indent directly for finding parent
                // For empty lines, use last_nonempty_indent to maintain depth context
                let effective_indent = if content_len == 0 {
                    *last_nonempty_indent
                } else {
                    *last_nonempty_indent = indent;
                    indent
                };

                let parent: AstNode = find_parent_line(root.clone(), effective_indent)
                    .unwrap_or_else(|| {
                        log::warn!("Failed to find parent, indent {indent}");
                        errors.push(ParserError::InvalidIndentation(Location {
                            input: Arc::from(linetext),
                            row: iline,
                            span: Span(indent, indent + 1),
                        }));
                        lastlinenode.clone()
                    });

                // Try parsing as command
                let (has_command, props) = parse_command_line(linetext, iline, indent);
                log::trace!("==============================");

                if let Some(command_node) = has_command {
                    log::trace!("parsed command: {:?}", command_node.extract_str());
                    match command_node.kind() {
                        AstNodeKind::Quote => {
                            block_context = BlockContext::Quote(QuoteState::new(
                                command_node.clone(),
                                indent + 1,
                            ));
                        }
                        AstNodeKind::Code { .. } => {
                            block_context = BlockContext::Code {
                                node: command_node.clone(),
                                min_indent: indent + 1,
                            };
                        }
                        AstNodeKind::Math { .. } => {
                            block_context = BlockContext::Math {
                                node: command_node.clone(),
                                min_indent: indent + 1,
                            };
                        }
                        AstNodeKind::Table { .. } => {
                            block_context = BlockContext::Table {
                                node: command_node.clone(),
                                min_indent: indent + 1,
                            };
                        }
                        _ => {}
                    }
                    let newline = AstNode::line(linetext, iline, None, Some(props));
                    newline.add_content(command_node);
                    lastlinenode = newline.clone();
                    parent.add_child(newline);
                } else {
                    // Regular line
                    log::trace!("---- input ----");
                    log::trace!("{}", &linetext[indent..]);
                    match PattoLineParser::parse(Rule::statement, &linetext[indent..]) {
                        Ok(mut parsed) => {
                            log::trace!("---- parsed ----");
                            log::trace!("{:?}", parsed);
                            log::trace!("---- result ----");
                            let (nodes, props) = transform_statement(
                                parsed.next().unwrap(),
                                linetext,
                                iline,
                                indent,
                            );
                            let newline = AstNode::line(linetext, iline, None, Some(props));
                            newline.add_contents(nodes);
                            lastlinenode = newline.clone();
                            log::trace!("{newline}");
                            parent.add_child(newline);
                        }
                        Err(e) => {
                            errors.push(ParserError::ParseError(
                                Location {
                                    input: Arc::from(linetext),
                                    row: iline,
                                    span: Span(indent, linetext.len()),
                                },
                                e.into(),
                            ));
                            let newline = AstNode::line(linetext, iline, None, None);
                            newline.add_content(AstNode::text(linetext, iline, None));
                            lastlinenode = newline.clone();
                            parent.add_child(newline);
                        }
                    }
                }
            }

            BlockContext::Quote(state) => {
                let current_min_indent = state.current_min_indent();
                let relative_indent = indent.saturating_sub(current_min_indent);

                // Check for nested [@quote] command
                let (has_command, props) = parse_command_line(linetext, iline, indent);

                if let Some(command_node) = has_command {
                    if matches!(command_node.kind(), AstNodeKind::Quote) {
                        // Nested quote - add to appropriate parent
                        let parent_qc =
                            find_parent_quote_content(state.current_node(), relative_indent);
                        let newline = AstNode::line(linetext, iline, None, Some(props));
                        newline.add_content(command_node.clone());
                        parent_qc.add_child(newline);

                        state.push(command_node, indent + 1);
                        continue;
                    }
                }

                // Regular quote content - parse from `indent` (clean, no tabs in span)
                match PattoLineParser::parse(Rule::statement_nestable, &linetext[indent..]) {
                    Ok(mut parsed) => {
                        let (nodes, props) =
                            transform_statement(parsed.next().unwrap(), linetext, iline, indent);
                        let quotecontent = AstNode::quotecontent(
                            linetext,
                            iline,
                            Some(Span(indent, linetext.len())), // Clean span
                            Some(props),
                        );
                        quotecontent.add_contents(nodes);

                        let parent_qc =
                            find_parent_quote_content(state.current_node(), relative_indent);
                        parent_qc.add_child(quotecontent);
                    }
                    Err(e) => {
                        errors.push(ParserError::ParseError(
                            Location {
                                input: Arc::from(linetext),
                                row: iline,
                                span: Span(indent, linetext.len()),
                            },
                            e.into(),
                        ));
                        let quotecontent = AstNode::quotecontent(linetext, iline, None, None);
                        quotecontent.add_content(AstNode::text(
                            linetext,
                            iline,
                            Some(Span(indent, linetext.len())),
                        ));
                        let parent_qc =
                            find_parent_quote_content(state.current_node(), relative_indent);
                        parent_qc.add_child(quotecontent);
                    }
                }
            }

            BlockContext::Code { node, min_indent } => {
                let linestart = cmp::min(*min_indent, indent);
                let text_node =
                    AstNode::codecontent(linetext, iline, Some(Span(linestart, linetext.len())));
                node.add_child(text_node);
            }

            BlockContext::Math { node, min_indent } => {
                let linestart = cmp::min(*min_indent, indent);
                let text_node =
                    AstNode::mathcontent(linetext, iline, Some(Span(linestart, linetext.len())));
                node.add_child(text_node);
            }

            BlockContext::Table { node, min_indent } => {
                let linestart = cmp::min(*min_indent, indent);
                let columntexts: Vec<&str> = linetext[linestart..].split('\t').collect();
                let mut span_start = linestart;
                let mut columns = Vec::new();

                for column_text in columntexts {
                    let span_end = span_start + column_text.len();
                    let span = Span(span_start, span_end);

                    match PattoLineParser::parse(Rule::statement_nestable, column_text) {
                        Ok(mut parsed) => {
                            let inner = parsed.next().unwrap();
                            let (nodes, _) =
                                transform_statement(inner, linetext, iline, span_start);
                            let column = AstNode::tablecolumn(linetext, iline, Some(span));
                            column.add_contents(nodes);
                            columns.push(column);
                        }
                        Err(_) => {
                            let column = AstNode::tablecolumn(linetext, iline, Some(span.clone()));
                            column.add_content(AstNode::text(linetext, iline, Some(span)));
                            columns.push(column);
                        }
                    }
                    // Move to next column start position (+1 for tab separator)
                    span_start = span_end + 1;
                }

                let row = AstNode::tablerow(linetext, iline, Some(Span(linestart, linetext.len())));
                row.add_contents(columns);
                node.add_child(row);
            }
        }
    }
    ParserResult {
        ast: root,
        parse_errors: errors,
    }
}

pub fn parse_text_with_persistent_line_tracking(
    text: &str,
    line_tracker: &mut LineTracker,
) -> ParserResult {
    // First, run regular parsing
    let result = parse_text(text);

    let _line_ids = match line_tracker.process_file_content(text) {
        Ok(ids) => ids,
        Err(_) => {
            // Return regular parsing result if line tracking fails
            return result;
        }
    };

    // Apply line IDs to Line and relevant nodes in the AST
    apply_line_ids_to_ast(&result.ast, line_tracker, text);

    result
}

fn apply_line_ids_to_ast(node: &AstNode, line_tracker: &LineTracker, _text: &str) {
    if let AstNodeKind::Line { .. } = node.kind() {
        // Get the line number from the location and assign stable_id
        let row = node.location().row;
        if let Some(line_id) = line_tracker.get_line_id(row + 1) {
            // negative id corresponds to special cases such as empty lines
            if line_id > 0 {
                node.set_stable_id(line_id);
            }
        }
    }

    // Recursively apply to children
    let children = node.children();
    for child in children.iter() {
        apply_line_ids_to_ast(child, line_tracker, _text);
    }
}

#[cfg(test)]
mod tab_indentation_tests {
    use super::*;

    #[test]
    fn test_tab_indentation_with_empty_lines() {
        let input = "Some line
\tIndented Line 1
\tIndented Line 2

\tIndented Line after empty line(s)
\t\tNested Line

\t\tNested Line2 after empty line(s)
";
        let result = parse_text(input);

        // Should parse without errors
        assert!(
            result.parse_errors.is_empty(),
            "Should parse without errors: {:?}",
            result.parse_errors
        );

        let root = result.ast;
        let children = root.children();

        // Debug output
        eprintln!("Root has {} children", children.len());
        for (i, child) in children.iter().enumerate() {
            eprintln!("Child {}: {:?}", i, child.kind());
            let subchildren = child.children();
            for (j, subchild) in subchildren.iter().enumerate() {
                eprintln!("  Subchild {}: {:?}", j, subchild.kind());
            }
        }

        // Should have parsed successfully
        assert!(
            !children.is_empty(),
            "Should have at least one top-level line"
        );
    }
}
