use std::cmp;
use std::sync::Arc;

use pest::Parser;

use super::command::parse_command_line;
use super::inline::transform_statement;
use super::{
    AstNode, AstNodeKind, Location, ParserError, ParserResult, PattoLineParser, Property, Rule,
    Span,
};
use crate::line_tracker::LineTracker;

pub fn parse_text(text: &str) -> ParserResult {
    let mut document = DocumentBuilder::new(text);
    for (row, line) in text.lines().enumerate() {
        document.leave_finished_blocks(row);
        document.push_line(row, line);
    }
    document.finish()
}

pub fn parse_text_with_persistent_line_tracking(
    text: &str,
    line_tracker: &mut LineTracker,
) -> ParserResult {
    let result = parse_text(text);
    if line_tracker.process_file_content(text).is_ok() {
        apply_line_ids(&result.ast, line_tracker);
    }
    result
}

fn apply_line_ids(node: &AstNode, line_tracker: &LineTracker) {
    if let AstNodeKind::Line { .. } = node.kind() {
        // Non-positive ids mark special cases such as empty lines.
        let line_id = line_tracker
            .get_line_id(node.location().row + 1)
            .filter(|id| *id > 0);
        if let Some(line_id) = line_id {
            node.set_stable_id(line_id);
        }
    }
    for child in node.children().iter() {
        apply_line_ids(child, line_tracker);
    }
}

#[derive(Clone, Copy)]
struct LineShape {
    indent: usize,
    content_len: usize,
}

impl LineShape {
    fn of(line: &str) -> Self {
        let indent = line.chars().take_while(|&c| c == '\t').count();
        Self {
            indent,
            content_len: line.len() - indent,
        }
    }

    fn is_empty(&self) -> bool {
        self.content_len == 0
    }
}

struct QuoteLevel {
    node: AstNode,
    min_indent: usize,
}

/// `[@quote]` lines inside a quote open nested quotes, so the open quotes form
/// a stack and the innermost one receives the content.
struct QuoteStack {
    levels: Vec<QuoteLevel>,
}

impl QuoteStack {
    fn new(node: AstNode, min_indent: usize) -> Self {
        Self {
            levels: vec![QuoteLevel { node, min_indent }],
        }
    }

    fn current(&self) -> &QuoteLevel {
        self.levels.last().expect("quote stack should not be empty")
    }

    fn push(&mut self, node: AstNode, min_indent: usize) {
        self.levels.push(QuoteLevel { node, min_indent });
    }

    fn pop(&mut self) {
        self.levels.pop();
    }

    fn depth(&self) -> usize {
        self.levels.len()
    }
}

/// `min_indent` is the indentation a line needs to still belong to the block.
enum BlockContext {
    Line,
    Quote(QuoteStack),
    Code { node: AstNode, min_indent: usize },
    Math { node: AstNode, min_indent: usize },
    Table { node: AstNode, min_indent: usize },
}

struct DocumentBuilder {
    shapes: Vec<LineShape>,
    root: AstNode,
    last_line: AstNode,
    context: BlockContext,
    /// Empty lines inherit this depth so they attach to the same parent as
    /// the line before them.
    last_nonempty_indent: usize,
    errors: Vec<ParserError>,
}

impl DocumentBuilder {
    fn new(text: &str) -> Self {
        let root = AstNode::new(text, 0, None, Some(AstNodeKind::Dummy));
        Self {
            shapes: text.lines().map(LineShape::of).collect(),
            last_line: root.clone(),
            root,
            context: BlockContext::Line,
            last_nonempty_indent: 0,
            errors: Vec::new(),
        }
    }

    fn finish(self) -> ParserResult {
        ParserResult {
            ast: self.root,
            parse_errors: self.errors,
        }
    }

    fn leave_finished_blocks(&mut self, row: usize) {
        let shapes = &self.shapes;
        let ends_here = |min_indent: usize| block_ends_at(shapes, row, min_indent);
        let left = match &mut self.context {
            BlockContext::Line => false,
            BlockContext::Quote(quotes) => {
                while quotes.depth() > 1 && ends_here(quotes.current().min_indent) {
                    quotes.pop();
                }
                quotes.depth() == 1 && ends_here(quotes.current().min_indent)
            }
            BlockContext::Code { min_indent, .. }
            | BlockContext::Math { min_indent, .. }
            | BlockContext::Table { min_indent, .. } => ends_here(*min_indent),
        };
        if left {
            self.context = BlockContext::Line;
            self.last_nonempty_indent = self.shapes[row].indent;
        }
    }

    fn push_line(&mut self, row: usize, line: &str) {
        let indent = self.shapes[row].indent;
        match &self.context {
            BlockContext::Line => self.push_plain_line(row, line),
            BlockContext::Quote(_) => self.push_quote_line(row, line),
            BlockContext::Code { node, min_indent } => {
                let span = block_body_span(line, indent, *min_indent);
                node.add_child(AstNode::codecontent(line, row, Some(span)));
            }
            BlockContext::Math { node, min_indent } => {
                let span = block_body_span(line, indent, *min_indent);
                node.add_child(AstNode::mathcontent(line, row, Some(span)));
            }
            BlockContext::Table { node, min_indent } => {
                let span = block_body_span(line, indent, *min_indent);
                node.add_child(table_row(line, row, span));
            }
        }
    }

    fn push_plain_line(&mut self, row: usize, line: &str) {
        let shape = self.shapes[row];
        if !shape.is_empty() {
            self.last_nonempty_indent = shape.indent;
        }
        let parent = self.parent_at_depth(self.last_nonempty_indent, row, line);

        let (command, props) = parse_command_line(line, row, shape.indent);
        let new_line = match command {
            Some(command_node) => {
                self.enter_block(&command_node, shape.indent);
                let new_line = AstNode::line(line, row, None, Some(props));
                new_line.add_content(command_node);
                new_line
            }
            None => self.statement_line(row, line, shape.indent),
        };
        self.last_line = new_line.clone();
        parent.add_child(new_line);
    }

    fn parent_at_depth(&mut self, depth: usize, row: usize, line: &str) -> AstNode {
        let indent = self.shapes[row].indent;
        find_parent_line(self.root.clone(), depth).unwrap_or_else(|| {
            log::warn!("Failed to find parent, indent {indent}");
            self.errors.push(ParserError::InvalidIndentation(Location {
                input: Arc::from(line),
                row,
                span: Span(indent, indent + 1),
            }));
            self.last_line.clone()
        })
    }

    fn enter_block(&mut self, command: &AstNode, indent: usize) {
        let node = command.clone();
        let min_indent = indent + 1;
        self.context = match command.kind() {
            AstNodeKind::Quote => BlockContext::Quote(QuoteStack::new(node, min_indent)),
            AstNodeKind::Code { .. } => BlockContext::Code { node, min_indent },
            AstNodeKind::Math { .. } => BlockContext::Math { node, min_indent },
            AstNodeKind::Table { .. } => BlockContext::Table { node, min_indent },
            _ => return,
        };
    }

    fn statement_line(&mut self, row: usize, line: &str, indent: usize) -> AstNode {
        match parse_statement(Rule::statement, line, row, indent, &mut self.errors) {
            Some((nodes, props)) => {
                let new_line = AstNode::line(line, row, None, Some(props));
                new_line.add_contents(nodes);
                new_line
            }
            None => {
                let new_line = AstNode::line(line, row, None, None);
                new_line.add_content(AstNode::text(line, row, None));
                new_line
            }
        }
    }

    fn push_quote_line(&mut self, row: usize, line: &str) {
        let indent = self.shapes[row].indent;
        let BlockContext::Quote(quotes) = &mut self.context else {
            unreachable!("push_quote_line is only called inside a quote block");
        };
        let relative_indent = indent.saturating_sub(quotes.current().min_indent);
        let parent = find_parent_quote_content(&quotes.current().node, relative_indent);

        let (command, props) = parse_command_line(line, row, indent);
        if let Some(nested_quote) = command.filter(|node| matches!(node.kind(), AstNodeKind::Quote))
        {
            let new_line = AstNode::line(line, row, None, Some(props));
            new_line.add_content(nested_quote.clone());
            parent.add_child(new_line);
            quotes.push(nested_quote, indent + 1);
            return;
        }

        let content_span = Span(indent, line.len());
        let content = match parse_statement(
            Rule::statement_nestable,
            line,
            row,
            indent,
            &mut self.errors,
        ) {
            Some((nodes, props)) => {
                let content = AstNode::quotecontent(line, row, Some(content_span), Some(props));
                content.add_contents(nodes);
                content
            }
            None => {
                let content = AstNode::quotecontent(line, row, None, None);
                content.add_content(AstNode::text(line, row, Some(content_span)));
                content
            }
        };
        parent.add_child(content);
    }
}

/// A block also ends at an empty line when the next non-empty line is dedented
/// past it, so trailing blank lines are not swallowed into the block.
fn block_ends_at(shapes: &[LineShape], row: usize, min_indent: usize) -> bool {
    let shape = shapes[row];
    if !shape.is_empty() {
        return shape.indent < min_indent;
    }
    shapes[row + 1..]
        .iter()
        .find(|next| !next.is_empty())
        .is_some_and(|next| next.indent < min_indent)
}

fn block_body_span(line: &str, indent: usize, min_indent: usize) -> Span {
    Span(cmp::min(min_indent, indent), line.len())
}

fn parse_statement(
    rule: Rule,
    line: &str,
    row: usize,
    indent: usize,
    errors: &mut Vec<ParserError>,
) -> Option<(Vec<AstNode>, Vec<Property>)> {
    match PattoLineParser::parse(rule, &line[indent..]) {
        Ok(mut parsed) => Some(transform_statement(
            parsed.next().unwrap(),
            line,
            row,
            indent,
        )),
        Err(error) => {
            errors.push(ParserError::ParseError(
                Location {
                    input: Arc::from(line),
                    row,
                    span: Span(indent, line.len()),
                },
                error.into(),
            ));
            None
        }
    }
}

fn table_row(line: &str, row: usize, span: Span) -> AstNode {
    let table_row = AstNode::tablerow(line, row, Some(span.clone()));
    let mut column_start = span.0;
    for column_text in line[span.0..].split('\t') {
        let column_span = Span(column_start, column_start + column_text.len());
        column_start = column_span.1 + 1;
        table_row.add_content(table_column(line, row, column_span, column_text));
    }
    table_row
}

fn table_column(line: &str, row: usize, span: Span, column_text: &str) -> AstNode {
    let column = AstNode::tablecolumn(line, row, Some(span.clone()));
    match PattoLineParser::parse(Rule::statement_nestable, column_text) {
        Ok(mut parsed) => {
            let (nodes, _) = transform_statement(parsed.next().unwrap(), line, row, span.0);
            column.add_contents(nodes);
        }
        Err(_) => column.add_content(AstNode::text(line, row, Some(span))),
    }
    column
}

fn find_parent_line(parent: AstNode, depth: usize) -> Option<AstNode> {
    if depth == 0 {
        return Some(parent);
    }
    let last_child_line = parent
        .children()
        .iter()
        .rfind(|child| matches!(child.kind(), AstNodeKind::Line { .. }))?
        .clone();
    find_parent_line(last_child_line, depth - 1)
}

/// `relative_indent` 0 is the quote node itself; each further level descends
/// into the last QuoteContent, stopping early when there is none yet.
fn find_parent_quote_content(quote: &AstNode, relative_indent: usize) -> AstNode {
    if relative_indent == 0 {
        return quote.clone();
    }
    let children = quote.children();
    match children
        .iter()
        .rfind(|child| matches!(child.kind(), AstNodeKind::QuoteContent { .. }))
    {
        Some(last_content) => find_parent_quote_content(last_content, relative_indent - 1),
        None => quote.clone(),
    }
}
