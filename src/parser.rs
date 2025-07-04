use chrono;
use pest::Parser;
use pest_derive::Parser;
use std::cmp;
use std::fmt;
use std::ops;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
//use std::time::{Instant};
use thiserror::Error;
use log;

use pest;
use pest::iterators::Pair;
use crate::line_tracker::LineTracker;

//use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[grammar = "patto.pest"]
struct PattoLineParser;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span(pub usize, pub usize);

impl ops::Add<usize> for Span {
    type Output = Self;

    fn add(self, offset: usize) -> Self {
        Span(self.0 + offset, self.1 + offset)
    }
}

impl Span {
    pub fn contains(&self, col: usize) -> bool {
        self.0 <= col && col < self.1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Location {
    pub row: usize,
    pub input: Arc<str>,
    pub span: Span,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.input)?;
        // if self.span.0 >= self.input.len() {
        //     log::warn!("input: {}, span: {:?}", self.input, self.span);
        // }
        write!(
            f,
            "{}",
            self.input[..self.span.0]
                .chars()
                .map(|c| {
                    if c != '\t' {
                        ' '
                    } else {
                        c
                    }
                })
                .collect::<String>()
        )?;
        write!(f, "{:^<1$}", "", self.span.1 - self.span.0)
    }
}

impl From<pest::Span<'_>> for Span {
    fn from(from: pest::Span<'_>) -> Span {
        Self(from.start(), from.end())
    }
}

impl From<pest::error::InputLocation> for Span {
    fn from(from: pest::error::InputLocation) -> Span {
        match from {
            pest::error::InputLocation::Pos(pos) => Span(pos, pos + 1),
            pest::error::InputLocation::Span(span) => Span(span.0, span.1),
        }
    }
}

impl Location {
    #[allow(dead_code)]
    fn merge(&self, other: &Self) -> Self {
        use std::cmp::{max, min};
        assert_eq!(self.input, other.input);
        assert_eq!(self.row, other.row);
        Self {
            row: self.row,
            input: Arc::clone(&self.input),
            span: Span(
                min(self.span.0, other.span.0),
                max(self.span.1, other.span.1),
            ),
        }
    }

    fn as_str(&self) -> &str {
        &self.input[self.span.0..self.span.1]
    }
}

#[derive(Debug, Default)]
pub struct Annotation<T> {
    pub value: T,
    pub location: Location,
}

//impl<T> fmt::Display for Annotation<'_, T>
// where
//     T: fmt::Display,
//{
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        write!(f, "{}", self.location)
//    }
//}

#[derive(Debug, Default)]
pub struct AstNodeInternal {
    pub contents: Mutex<Vec<AstNode>>,
    pub children: Mutex<Vec<AstNode>>,
    pub kind: AstNodeKind,
    pub stable_id: Mutex<Option<i64>>,
    // text will be the string matched with this AstNode.
    // will be used when contents.len() == 0
    // pub text: &'a str,
}

// impl<'a> fmt::Display for AstNodeInternal<'a> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.contents)
//     }
// }

#[derive(PartialEq, Eq, Debug, Clone)] //, Deserialize, Serialize)]
pub enum Deadline {
    DateTime(chrono::NaiveDateTime),
    Date(chrono::NaiveDate),
    Uninterpretable(String),
}

impl fmt::Display for Deadline {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Deadline::DateTime(dt) => {
                write!(f, "{}", dt)?;
            }
            Deadline::Date(d) => {
                write!(f, "{}", d)?;
            }
            Deadline::Uninterpretable(s) => {
                write!(f, "{}", s)?;
            }
        }
        Ok(())
    }
}

impl PartialOrd for Deadline {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Deadline {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Deadline::Date(d1), Deadline::Date(d2)) => d1.cmp(d2),
            (Deadline::DateTime(dt1), Deadline::DateTime(dt2)) => dt1.cmp(dt2),
            (Deadline::Uninterpretable(t1), Deadline::Uninterpretable(t2)) => t1.cmp(t2),
            (Deadline::Date(d1), Deadline::DateTime(dt2)) => {
                d1.and_hms_opt(0, 0, 0).map_or(Ordering::Less, |dt1| {
                    dt1.cmp(dt2).then(Ordering::Less)
                })
            }
            (Deadline::DateTime(dt1), Deadline::Date(d2)) => {
                d2.and_hms_opt(0, 0, 0).map_or(Ordering::Greater, |dt2| {
                    dt1.cmp(&dt2).then(Ordering::Greater)
                })
            }
            (Deadline::Date(_), _) => Ordering::Less,
            (Deadline::DateTime(_), Deadline::Uninterpretable(_)) => Ordering::Less,
            (Deadline::Uninterpretable(_), _) => Ordering::Greater,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TaskStatus {
    Todo,
    Doing,
    Done,
}

#[derive(Debug)]
pub enum Property {
    Task { status: TaskStatus, due: Deadline },
    Anchor { name: String },
}

#[derive(Debug, Default)]
pub enum AstNodeKind {
    Line {
        //indent: usize,
        properties: Vec<Property>,
    },
    Quote,
    Math {
        inline: bool,
    },
    Code {
        lang: String,
        inline: bool,
    },
    Table,
    Image {
        src: String,
        alt: Option<String>,
    },
    WikiLink {
        link: String,
        anchor: Option<String>,
    },
    Link {
        link: String,
        title: Option<String>,
    },

    //Bold {
    //    size: usize
    //},
    //Italic,
    //Underline,
    //Deleted,
    Decoration {
        fontsize: isize,
        italic: bool,
        underline: bool,
        deleted: bool,
    },
    TableColumn,

    Text,
    HorizontalLine,
    #[default]
    Dummy,
}

type AstNodeImpl = Annotation<AstNodeInternal>;
#[derive(Debug)]
pub struct AstNode(Arc<Annotation<AstNodeInternal>>);

impl AstNode {
    pub fn new(input: &str, row: usize, span: Option<Span>, kind: Option<AstNodeKind>) -> Self {
        AstNode(Arc::new(AstNodeImpl {
            value: AstNodeInternal {
                contents: Mutex::new(vec![]),
                children: Mutex::new(vec![]),
                kind: kind.unwrap_or(AstNodeKind::Dummy),
                stable_id: Mutex::new(None),
            },
            location: Location {
                row,
                input: Arc::from(input),
                span: span.unwrap_or(Span(0, input.len())),
            },
        }))
    }

    pub fn with_line_id(_input: &str, location: Location, kind: Option<AstNodeKind>, line_id: Option<i64>) -> Self {
        AstNode(Arc::new(AstNodeImpl {
            value: AstNodeInternal {
                contents: Mutex::new(vec![]),
                children: Mutex::new(vec![]),
                kind: kind.unwrap_or(AstNodeKind::Dummy),
                stable_id: Mutex::new(line_id),
            },
            location,
        }))
    }
    pub fn line(
        input: &str,
        row: usize,
        span: Option<Span>,
        props: Option<Vec<Property>>,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Line {
                properties: props.unwrap_or_default(),
            }),
        )
    }
    pub fn code(
        input: &str,
        row: usize,
        span: Option<Span>,
        lang: &str,
        inline: bool,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Code {
                lang: lang.to_string(),
                inline,
            }),
        )
    }
    pub fn math(input: &str, row: usize, span: Option<Span>, inline: bool) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Math { inline }))
    }
    pub fn quote(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Quote))
    }
    pub fn wikilink(
        input: &str,
        row: usize,
        span: Option<Span>,
        link: &str,
        anchor: Option<&str>,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::WikiLink {
                link: link.to_string(),
                anchor: anchor.map(str::to_string),
            }),
        )
    }
    pub fn link(
        input: &str,
        row: usize,
        span: Option<Span>,
        link: &str,
        title: Option<&str>,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Link {
                link: link.to_string(),
                title: title.map(str::to_string),
            }),
        )
    }
    pub fn text(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Text))
    }
    pub fn horizontal_line(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::HorizontalLine))
    }
    pub fn image(input: &str, row: usize, span: Option<Span>, src: &str, alt: Option<&str>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Image{src: src.to_string(), alt: alt.map(str::to_string)}))
    }
    pub fn decoration(
        input: &str,
        row: usize,
        span: Option<Span>,
        fontsize: isize,
        italic: bool,
        underline: bool,
        deleted: bool,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Decoration {
                fontsize,
                italic,
                underline,
                deleted,
            }),
        )
    }
    pub fn tablecolumn(input: &str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::TableColumn))
    }

    pub fn value(&self) -> &AstNodeInternal {
        &self.0.value
    }
    pub fn kind(&self) -> &AstNodeKind {
        &self.value().kind
    }
    pub fn add_content(&self, content: AstNode) {
        self.value().contents.lock().unwrap().push(content);
    }
    pub fn add_contents(&self, contents: Vec<AstNode>) {
        self.value().contents.lock().unwrap().extend(contents);
    }
    pub fn add_child(&self, child: AstNode) {
        self.value().children.lock().unwrap().push(child);
    }
    pub fn location(&self) -> &Location {
        &self.0.location
    }
    pub fn extract_str(&self) -> &str {
        self.location().as_str()
    }
}

impl Clone for AstNode {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl fmt::Display for AstNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "extracted: {}", self.extract_str())?;
        for content in self.value().contents.lock().unwrap().iter() {
            write!(f, "-- {}", content)?;
        }
        if let AstNodeKind::Line { properties } = &self.kind() {
            for prop in properties {
                writeln!(f, "property -- {:?}", prop)?;
            }
        }
        for (i, child) in self.value().children.lock().unwrap().iter().enumerate() {
            writeln!(f, "\t{i}child -- {:?}", child)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
enum ParsingState {
    Line,
    Quote,
    Code,
    Math,
    Table,
}

fn find_parent_line(parent: AstNode, depth: usize) -> Option<AstNode> {
    if depth == 0 {
        return Some(parent);
    }
    let last_child_line = parent
        .value()
        .children
        .lock()
        .unwrap()
        .iter()
        .filter_map(|e| match e.kind() {
            AstNodeKind::Line { .. } => Some(e.clone()),
            _ => None,
        })
        .next_back()?;
    find_parent_line(last_child_line, depth - 1)
}

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Invalid indentation:\n{0}")]
    InvalidIndentation(Location),
    #[error("Failed to parse:\n{1}")]
    ParseError(Location, String),
    // #[error("Invalid command parameter: {0}")]
    // InvalidCommandParameter(String),
    // #[error("Unexpected token: {0}")]
    // UnexpectedToken(String),
}

#[derive(Debug)]
pub struct ParserResult {
    pub ast: AstNode,
    pub parse_errors: Vec<ParserError>,
}

pub fn parse_text(text: &str) -> ParserResult {
    let indent_content_len: Vec<_> = text.lines().map(|l| {
        let indent = l.chars().take_while(|&c| c == '\t').count();
        let content_len = l.len() - indent;
        (indent, content_len)
    }).collect();
    let numlines = indent_content_len.len();

    let root = AstNode::new(text, 0, None, Some(AstNodeKind::Dummy));
    let mut lastlinenode = root.clone();

    let mut parsing_state: ParsingState = ParsingState::Line;
    let mut parsing_depth = 0;

    let mut errors: Vec<ParserError> = Vec::new();
    for (iline, linetext) in text.lines().enumerate() {
        let (indent, content_len) = indent_content_len[iline];

        // TODO can be O(n^2) where n = numlines
        let mut depth = indent;
        if parsing_state != ParsingState::Line {
            // search which line code/math/quote/table block will continue until
            let mut inblock = false;
            for jline in iline..numlines {
                let (jindent, jcontent_len) = indent_content_len[jline];
                if jindent >= parsing_depth {
                    inblock = true;
                    break;
                }
                if jindent == 0 && jcontent_len == 0 {
                    continue;
                } else {
                    inblock = false;
                    break;
                }
            }
            if inblock {
                // this line is in block
                depth = parsing_depth;
            } else {
                // this line is not in block, transitioning to line-parsing mode
                parsing_state = ParsingState::Line;
                parsing_depth = indent;
                depth = indent;
            }
        } else {
            log::trace!("content_len: {content_len}");
            if content_len == 0 {
                depth = parsing_depth;
            }
        }
        log::trace!("depth: {depth}, parsing_depth: {parsing_depth}");

        let parent: AstNode =
            find_parent_line(root.clone(), depth).unwrap_or_else(|| {
                log::warn!("Failed to find parent, depth {depth}");
                errors.push(ParserError::InvalidIndentation(Location {
                    input: Arc::from(linetext),
                    row: iline,
                    span: Span(indent, indent + 1),
                }));
                lastlinenode.clone()
            });

        let linestart = cmp::min(depth, indent);

        if parsing_state != ParsingState::Line {
            if parsing_state == ParsingState::Quote  {
                let quote = parent.value().contents.lock().unwrap().last().expect("no way! should be quote block").clone();
                match PattoLineParser::parse(Rule::statement_nestable, &linetext[linestart..]) {
                    Ok(mut parsed) => {
                        let (nodes, props) = transform_statement(
                            parsed.next().unwrap(),
                            linetext,
                            iline,
                            depth,
                        );
                        // TODO should be text rather than line?
                        let newline = AstNode::line(linetext, iline, Some(Span(linestart, linetext.len())), Some(props));
                        newline.add_contents(nodes);
                        quote.add_child(newline);
                    }
                    Err(e) => {
                        errors.push(ParserError::ParseError(Location {
                            input: Arc::from(linetext),
                            row: iline,
                            span: Span(linestart, linetext.len()),
                        }, e.variant.message().to_string()));
                        let newline = AstNode::line(linetext, iline, None, None);
                        newline.add_content(AstNode::text(linetext, iline, Some(Span(linestart, linetext.len()))));
                        quote.add_child(newline);
                    }
                }
                continue;
            } else if parsing_state == ParsingState::Code || parsing_state == ParsingState::Math {
                let block = parent.value().contents.lock().unwrap().last().expect("no way! should be code or math block").clone();
                let text = AstNode::text(linetext, iline, Some(Span(linestart, linetext.len())));
                block.add_child(text);
                continue;
            } else {
                #[allow(unused_variables)]
                let table = parent.value().contents.lock().unwrap().last().expect("no way! should be table block").clone();
                todo!("table rows might have empty lines, do not start from `depth'");
                #[allow(unreachable_code)]
                {
                let columntexts = &linetext[depth..].split('\t');
                let span_starts = columntexts.to_owned().scan(depth, |cum, x| {*cum += x.len() + 1; Some(*cum)}/* +1 for seperator*/);
                let columns = columntexts.to_owned().zip(span_starts)
                    .map(|(t, c)| (PattoLineParser::parse(Rule::statement_nestable, t), c))
                    .map(|(ret, c)| {
                        match ret {
                            Ok(mut parsed) => {
                                let inner = parsed.next().unwrap();
                                let span = Into::<Span>::into(inner.as_span()) + c;
                                let (nodes, _) = transform_statement(
                                    inner,
                                    linetext,
                                    iline,
                                    depth,
                                );
                                let column = AstNode::tablecolumn(linetext, iline, Some(span));
                                column.add_contents(nodes);
                                column
                            }
                            Err(e) => {
                                let span = Span(c, c + e.line().len());  // TODO is this correct span?
                                let column = AstNode::tablecolumn(linetext, iline, Some(span.clone()));
                                column.add_content(AstNode::text(linetext, iline, Some(span)));
                                column
                            }}
                    }).collect();
                let newline = AstNode::line(linetext, iline, Some(Span(depth, linetext.len())), None);
                newline.add_contents(columns);
                table.add_child(newline);
                continue;
                }
            }
        }

        // TODO gather parsing errors
        let (has_command, props) = parse_command_line(linetext, iline, linestart);
        log::trace!("==============================");
        if let Some(command_node) = has_command {
            log::trace!("parsed command: {:?}", command_node.extract_str());
            match &command_node.kind() {
                AstNodeKind::Quote => {
                    parsing_state = ParsingState::Quote;
                    parsing_depth = depth + 1;
                }
                AstNodeKind::Code{..} => {
                    parsing_state = ParsingState::Code;
                    parsing_depth = depth + 1;
                }
                AstNodeKind::Math{..} => {
                    parsing_state = ParsingState::Math;
                    parsing_depth = depth + 1;
                }
                AstNodeKind::Table => {
                    parsing_state = ParsingState::Table;
                    parsing_depth = depth + 1;
                }
                _ => {
                    parsing_state = ParsingState::Line;
                    parsing_depth = depth;
                }
            }
            let newline = AstNode::line(linetext, iline, None, Some(props));
            newline.add_content(command_node);
            lastlinenode = newline.clone();
            parent.add_child(newline);
        } else {
            log::trace!("---- input ----");
            log::trace!("{}", &linetext[cmp::min(depth,indent)..]);
            // TODO error will never happen since raw_sentence will match finally(...?)
            match PattoLineParser::parse(Rule::statement, &linetext[cmp::min(depth, indent)..]) {
                Ok(mut parsed) => {
                    log::trace!("---- parsed ----");
                    log::trace!("{:?}", parsed);
                    log::trace!("---- result ----");
                    let (nodes, props) = transform_statement(
                        parsed.next().unwrap(),
                        linetext,
                        iline,
                        cmp::min(depth, indent),
                    );
                    let newline = AstNode::line(linetext, iline, None, Some(props));
                    newline.add_contents(nodes);
                    lastlinenode = newline.clone();
                    log::trace!("{newline}");
                    parent.add_child(newline);
                }
                Err(e) => {
                    // TODO accumulate error
                    // log::warn!("parsing statement error!: {}", e);
                    errors.push(ParserError::ParseError(Location {
                        input: Arc::from(linetext),
                        row: iline,
                        span: Span(linestart, linetext.len()),
                    }, e.to_string()));
                    let newline = AstNode::line(linetext, iline, None, None);
                    newline.add_content(AstNode::text(linetext, iline, None));
                    lastlinenode = newline.clone();
                    parent.add_child(newline);
                }
            }
            // parsing_state = ParsingState::Line;
            parsing_depth = depth;
        }
    }
    ParserResult{ast: root, parse_errors: errors}
}

pub fn parse_text_with_persistent_line_tracking(text: &str, line_tracker: &mut LineTracker) -> ParserResult {
    // First, run regular parsing
    //let start = Instant::now();
    let result = parse_text(text);
    //println!("-- {} ms for parsing", start.elapsed().as_millis());

    //let start = Instant::now();
    let _line_ids = match line_tracker.process_file_content(text) {
        Ok(ids) => ids,
        Err(_) => {
            // Return regular parsing result if line tracking fails
            return result;
        }
    };
    //println!("-- {} ms for processing file", start.elapsed().as_millis());

    // Apply line IDs to Line and relevant nodes in the AST
    apply_line_ids_to_ast(&result.ast, line_tracker, text);
    
    result
}

fn apply_line_ids_to_ast(node: &AstNode, line_tracker: &LineTracker, _text: &str) {
    if let AstNodeKind::Line { .. } = node.kind() {
        // Get the line number from the location and assign stable_id
        let row = node.0.location.row;
        if let Some(line_id) = line_tracker.get_line_id(row + 1) {
            // negative id corresponds to special cases such as empty lines
            if line_id > 0 {
                // We need to access the internal mutable state to set stable_id
                // This is tricky due to Arc wrapping, but we can modify the approach
                set_node_stable_id(node, line_id);
            }
        }
    }
    
    // Recursively apply to children
    let children = node.value().children.lock().unwrap();
    for child in children.iter() {
        apply_line_ids_to_ast(child, line_tracker, _text);
    }
}

fn set_node_stable_id(node: &AstNode, stable_id: i64) {
    *node.value().stable_id.lock().unwrap() = Some(stable_id);
}

fn parse_command_line(
    line: &str,
    row: usize,
    indent: usize,
) -> (Option<AstNode>, Vec<Property>) {
    let Ok(mut pairs) = PattoLineParser::parse(Rule::expr_command_line, &line[indent..]) else {
        return (None, vec![]);
    };
    let parsed_command_line = pairs.next().unwrap();
    let mut pairs = parsed_command_line.into_inner();
    let parsed_command = pairs.next().unwrap();
    let command_node = transform_command(parsed_command, line, row, indent);

    let mut properties: Vec<Property> = vec![];

    if let Some(parsed_props) = pairs.next() {
        for pair in parsed_props.into_inner() {
            if let Some(prop) = transform_property(pair) {
                properties.push(prop);
            }
        }
    };
    (command_node, properties)
}

fn transform_command<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let span = Into::<Span>::into(pair.as_span()) + indent;
    match pair.as_rule() {
        Rule::expr_command => {
            let mut inner = pair.into_inner();
            let builtin_commands = inner.next().unwrap(); // consume the command
            let command = builtin_commands.into_inner().next().unwrap();
            match command.as_rule() {
                Rule::command_math => {
                    return Some(AstNode::math(line, row, Some(span), false));
                }
                Rule::command_quote => {
                    return Some(AstNode::quote(line, row, Some(span)));
                }
                Rule::command_code => {
                    // 1st parameter
                    let mut lang = "";
                    if let Some(lang_part) = inner.next() {
                        lang = lang_part.as_str();
                    } else {
                        log::warn!("No language specified for code block");
                    }
                    return Some(AstNode::code(line, row, Some(span), lang, false));
                }
                Rule::parameter => {
                    log::warn!(
                        "parameter must have already been consumed: {}",
                        command.as_str()
                    );
                    // TODO return text?
                    return Some(AstNode::text(line, row, Some(span)));
                }
                _ => {
                    return None;
                }
            }
        }
        _ => {
            log::warn!(
                "Do you provide other than expr_command to fn transform_command: {:?}",
                pair.as_rule()
            );
        }
    }
    None
}

fn transform_img<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let span = Into::<Span>::into(pair.as_span()) + indent;
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::img_alt_path_opts => {
            let mut inner2 = inner.into_inner();
            let alt_img = inner2.next().unwrap().into_inner().next().unwrap().into_inner().next().unwrap().as_str();
            let img_path = inner2.next().unwrap().into_inner().next().unwrap().as_str();
            // inner2.chunks(2).map(|(k,v)| {
            //     match k.unwrap().as_str() {
            //         "width" => {
            //             match v.parse::<isize>() {
            //                 Ok(v) =>
            //     }
            // }
            Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                Some(alt_img),
            ))
        }
        Rule::img_path_alt_opts => {
            let mut inner2 = inner.into_inner();
            let img_path = inner2.next().unwrap().into_inner().next().unwrap().as_str();
            let alt_img = inner2.next().unwrap().into_inner().next().unwrap().into_inner().next().unwrap().as_str();
            Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                Some(alt_img),
            ))
        }
        Rule::img_path_opts => {
            let mut inner2 = inner.into_inner();
            let img_path = inner2.next().unwrap().into_inner().next().unwrap().as_str();
            Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                None,
            ))
        }
        _ => {
            unreachable!();
        }
    }
}


/// assuming pair is expr_wiki_link
fn transform_wiki_link<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let inner = pair.into_inner().next().unwrap(); // wiki_link or wiki_link_anchored
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::wiki_link_anchored => {
            let mut inner2 = inner.into_inner();
            let wiki_link = inner2.next().unwrap();
            let expr_anchor = inner2.next().unwrap();
            Some(AstNode::wikilink(
                line,
                row,
                Some(span),
                wiki_link.as_str(),
                Some(expr_anchor.into_inner().next().unwrap().as_str()),
            ))
        }
        Rule::wiki_link => {
            Some(AstNode::wikilink(
                line,
                row,
                Some(span),
                inner.as_str(),
                None,
            ))
        }
        Rule::self_link_anchored => {
            Some(AstNode::wikilink(
                line,
                row,
                Some(span),
                "",
                Some(inner.into_inner().next().unwrap().into_inner().next().unwrap().as_str()),
            ))
        }
        _ => {
            unreachable!();
        }
    }
}

/// assuming input pair is url stuff
fn transform_url_link<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let inner = pair.into_inner().next().unwrap();
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::expr_url_title => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            let title = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_title_url => {
            let mut inner2 = inner.into_inner();
            let title = inner2.next().unwrap();
            let url = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_url_only => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                None,
            ))
        }
        Rule::expr_url_url => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            let url2 = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(url2.as_str()),
            ))
        }
        _ => {
            unreachable!();
        }
    }
}

fn transform_local_file_link<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let inner = pair.into_inner().next().unwrap();
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::expr_local_file_title => {
            let mut inner2 = inner.into_inner();
            let local_file = inner2.next().unwrap();
            let title = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                local_file.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_title_local_file => {
            let mut inner2 = inner.into_inner();
            let title = inner2.next().unwrap();
            let local_file = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                local_file.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_local_file_only => {
            let mut inner2 = inner.into_inner();
            let local_file = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                local_file.as_str(),
                None,
            ))
        }
        _ => {
            unreachable!();
        }
    }
}

fn transform_mail_link<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let inner = pair.into_inner().next().unwrap();
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::expr_mail_title => {
            let mut inner2 = inner.into_inner();
            let mail = inner2.next().unwrap();
            let title = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                mail.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_title_mail => {
            let mut inner2 = inner.into_inner();
            let title = inner2.next().unwrap();
            let mail = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                mail.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_mail_only => {
            let mut inner2 = inner.into_inner();
            let mail = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                mail.as_str(),
                None,
            ))
        }
        Rule::expr_mail_mail => {
            let mut inner2 = inner.into_inner();
            let mail = inner2.next().unwrap();
            let mail2 = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                mail.as_str(),
                Some(mail2.as_str()),
            ))
        }
        _ => {
            unreachable!();
        }
    }
}

fn transform_property(pair: Pair<Rule>) -> Option<Property> {
    match pair.as_rule() {
        Rule::expr_anchor => {
            let anchor = Property::Anchor {
                name: pair.into_inner().next().unwrap().as_str().to_string(),
            };
            Some(anchor)
        }
        Rule::expr_property => {
            let mut inner = pair.into_inner();
            let property_name = inner.next().unwrap().as_str();
            if property_name != "task" {
                log::warn!("Unknown property: {}", property_name);
                return None;
            }

            let mut status = TaskStatus::Todo;
            let mut due = Deadline::Uninterpretable("".to_string());
            let mut current_key = "";
            for kv in inner {
                match kv.as_rule() {
                    Rule::property_keyword_arg => {
                        current_key = kv.as_str();
                    }
                    Rule::property_keyword_value => {
                        let value = kv.as_str();
                        if current_key == "status" {
                            if value == "todo" {
                                status = TaskStatus::Todo;
                            } else if value == "doing" {
                                status = TaskStatus::Doing;
                            } else if value == "done" {
                                status = TaskStatus::Done;
                            } else {
                                log::warn!("Unknown task status: '{}', interpreted as 'todo'", value);
                            }
                        } else if current_key == "due" {
                            if let Ok(datetime) =
                                chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
                            {
                                due = Deadline::DateTime(datetime);
                            } else if let Ok(date) =
                                chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                            {
                                due = Deadline::Date(date);
                            } else {
                                due = Deadline::Uninterpretable(value.to_string());
                            }
                        } else {
                            log::warn!("Unknown property value: {}", value);
                        }
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
            Some(Property::Task { status, due })
        }
        Rule::expr_task => {
            let mut inner = pair.into_inner();
            let symbol = inner.by_ref().next().unwrap();
            let status = match symbol.as_rule() {
                Rule::symbol_task_done => {
                    TaskStatus::Done
                }
                Rule::symbol_task_doing => {
                    TaskStatus::Doing
                }
                Rule::symbol_task_todo => {
                    TaskStatus::Todo
                }
                _ => unreachable!(),
            };
            let due_str = inner.as_str();
            let due = if let Ok(datetime) =
                chrono::NaiveDateTime::parse_from_str(due_str, "%Y-%m-%dT%H:%M")
            {
                Deadline::DateTime(datetime)
            } else if let Ok(date) =
                chrono::NaiveDate::parse_from_str(due_str, "%Y-%m-%d")
            {
                Deadline::Date(date)
            } else {
                Deadline::Uninterpretable(due_str.to_string())
            };
            Some(Property::Task { status, due })
        }
        _ => {
            panic!("Unhandled token: {:?}", pair.as_rule());
        }
    }
}

fn transform_statement<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> (Vec<AstNode>, Vec<Property>) {
    let mut nodes: Vec<AstNode> = vec![];
    let mut props: Vec<Property> = vec![];

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::expr_img => {
                if let Some(node) = transform_img(inner, line, row, indent) {
                    nodes.push(node);
                }
            }
            Rule::expr_builtin_symbols => {
                let s = Some(Into::<Span>::into(inner.as_span()) + indent);
                let mut inner2 = inner.into_inner();
                let symbols = inner2.by_ref().next().unwrap();
                let mut boldsize = 0;
                let mut italic = false;
                let mut underline = false;
                let mut deleted = false;
                for symbol in symbols.into_inner() {
                    match symbol.as_rule() {
                        Rule::symbol_bold => {
                            boldsize += 1;
                        }
                        Rule::symbol_italic => {
                            italic = true;
                        }
                        Rule::symbol_underline => {
                            underline = true;
                        }
                        Rule::symbol_deleted => {
                            deleted = true;
                        }
                        _ => unreachable!(),
                    }
                }

                let node = AstNode::decoration(line, row, s, boldsize, italic, underline, deleted);
                // WARN `statement_nestable' must be the subset of `statement'
                let (inner_nodes, _) =
                    transform_statement(inner2.next().unwrap(), line, row, indent);
                // elements in nodes are moved and the nodes will become empty. therefore,
                // mut is required.
                node.add_contents(inner_nodes);
                nodes.push(node);
            }
            Rule::expr_wiki_link => {
                if let Some(node) = transform_wiki_link(inner, line, row, indent) {
                    nodes.push(node);
                }
            }
            Rule::expr_url_link => {
                if let Some(node) = transform_url_link(inner, line, row, indent) {
                    nodes.push(node);
                }
            }
            Rule::expr_local_file_link => {
                if let Some(node) = transform_local_file_link(inner, line, row, indent) {
                    nodes.push(node);
                }
            }
            Rule::expr_mail_link => {
                if let Some(node) = transform_mail_link(inner, line, row, indent) {
                    nodes.push(node);
                }
            }
            Rule::expr_code_inline => {
                //assert!(matches!(line.value.kind, AstNodeKind::Line { .. }));
                let code = AstNode::code(
                    line,
                    row,
                    Some(Into::<Span>::into(inner.as_span()) + indent),
                    "",
                    true,
                );
                let code_inline = inner.into_inner().next().unwrap();
                code.add_content(AstNode::text(
                    line,
                    row,
                    Some(Into::<Span>::into(code_inline.as_span()) + indent),
                ));
                nodes.push(code);
            }
            Rule::expr_math_inline => {
                let math = AstNode::math(
                    line,
                    row,
                    Some(Into::<Span>::into(inner.as_span()) + indent),
                    true,
                );
                let math_inline = inner.into_inner().next().unwrap();
                math.add_content(AstNode::text(
                    line,
                    row,
                    Some(Into::<Span>::into(math_inline.as_span()) + indent),
                ));
                nodes.push(math);
            }
            Rule::expr_property => {
                if let Some(prop) = transform_property(inner) {
                    props.push(prop);
                }
            }
            Rule::expr_anchor => {
                //println!("non-trailing anchor will be treated as a text");
                //nodes.push(AstNode::text(line, row, Some(Into::<Span>::into(inner.as_span()) + indent)));
                if let Some(prop) = transform_property(inner) {
                    props.push(prop);
                }
            }
            Rule::expr_task => {
                if let Some(prop) = transform_property(inner) {
                    props.push(prop);
                }
            }
            Rule::raw_sentence => {
                nodes.push(AstNode::text(
                    line,
                    row,
                    Some(Into::<Span>::into(inner.as_span()) + indent),
                ));
            }
            Rule::expr_hr => {
                nodes.push(AstNode::horizontal_line(
                    line,
                    row,
                    Some(Into::<Span>::into(inner.as_span()) + indent),
                ));
            }
            Rule::trailing_properties => {
                props.extend(
                    inner
                        .into_inner()
                        .filter_map(|e| transform_property(e)),
                );
            }
            Rule::EOI => {
                continue;
            }
            _ => {
                log::warn!("{:?} not implemented", inner.as_rule());
                unreachable!()
            }
        }
    }
    (nodes, props)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::pest::Parser;

    #[test]
    fn test_parse_code_command() {
        let input = "[@code rust]";
        // let parsed = PattoLineParser::parse(Rule::expr_command, input);
        // assert!(parsed.is_ok(), "Failed to parse \"{input}\"");
        // let mut pairs = parsed.unwrap();
        // assert_eq!(pairs.len(), 1, "must contain only one expr_command");
        // let parsed_command = pairs.next().unwrap();
        // //                          \- the first pair, which is expr_command
        let (astnode, _props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        match &node.kind() {
            AstNodeKind::Code { lang, inline } => {
                assert_eq!(lang, "rust");
                assert!(!(*inline));
            }
            _ => {
                panic! {"it is weird"};
            }
        }
    }

    #[test]
    fn test_parse_code_emtpy_lang() {
        let input = "[@code   ]";
        // let parsed = PattoLineParser::parse(Rule::expr_command, input);
        // assert!(parsed.is_ok(), "Failed to parse \"{input}\"");
        // let mut pairs = parsed.unwrap();
        // assert_eq!(pairs.len(), 1, "must contain only one expr_command");
        // let parsed_command = pairs.next().unwrap();
        // //                          \- the first pair, which is expr_command
        let (astnode, _props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        match &node.kind() {
            AstNodeKind::Code { lang, inline } => {
                assert_eq!(lang, "");
                assert!(!*inline);
            }
            _ => {
                panic! {"it is weird"};
            }
        }
    }

    #[test]
    fn test_parse_indented_code_command() {
        let input = "		[@code なでしこ]   #anchor1 {@task status=todo due=2024-09-24}";
        let indent = input.chars().take_while(|&c| c == '\t').count();
        let (astnode, props) = parse_command_line(input, 0, indent);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        println!("{}", node);
        let Some(end_code) = input.find("]") else {
            panic!("no way!");
        };
        assert_eq!(node.location().span, Span(indent, end_code + 1));
        match &node.kind() {
            AstNodeKind::Code { lang, inline } => {
                assert_eq!(lang, "なでしこ");
                assert!(!*inline);
            }
            _ => {
                panic! {"it is weird"};
            }
        }
        for prop in props {
            match prop {
                Property::Task { status, due } => {
                    let TaskStatus::Todo = status else {
                        panic!("task is not in todo state!");
                    };
                    if let Deadline::Date(date) = due {
                        assert_eq!(date, chrono::NaiveDate::from_ymd_opt(2024, 9, 24).unwrap());
                    } else {
                        panic!("date is not correctly parsed");
                    }
                }
                Property::Anchor { name } => {
                    assert_eq!(name, "anchor1");
                }
            }
        }
    }
    #[test]
    fn test_parse_trailing_properties() -> Result<(), Box<dyn std::error::Error>>  {
        let input = "   #anchor1 {@task status=todo due=2024-09-24} #anchor2";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (_nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        let anchor1 = &props[0];
        if let Property::Anchor { name } = anchor1 {
            assert_eq!(name, "anchor1");
        } else {
            panic!("anchor1 is not extracted properly");
        };

        let task = &props[1];
        if let Property::Task { status, due } = task {
            let TaskStatus::Todo = status else {
                panic!("task is not in todo state!");
            };
            assert_eq!(
                due,
                &Deadline::Date(chrono::NaiveDate::from_ymd_opt(2024, 9, 24).unwrap())
            );
        } else {
            panic!("anchor1 is not extracted properly");
        };

        let anchor2 = &props[2];
        if let Property::Anchor { name } = anchor2 {
            assert_eq!(name, "anchor2");
        } else {
            panic!("anchor2 is not extracted properly");
        };
        Ok(())
    }

    #[test]
    fn test_parse_math() {
        let input = "[@math  ]";
        let indent = input.chars().take_while(|&c| c == '\t').count();
        let (astnode, _props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        println!("{:?}", node);
        assert_eq!(node.location().span, Span(indent, input.len()));
        match node.kind() {
            AstNodeKind::Math { ref inline } => {
                assert!(!*inline);
            }
            _ => {
                panic! {"Math command could not be parsed"};
            }
        }
    }

    #[test]
    fn test_parse_math_inline() -> Result<(), Box<dyn std::error::Error>> {
        let input = "[$ math = a * b * c$]";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (nodes, _props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        let math = &nodes[0];
        if let AstNodeKind::Math { ref inline } = math.kind() {
            assert!(*inline);
        } else {
            panic! {"Inline math could not be parsed"};
        }
        assert_eq!(math.value().contents.lock().unwrap()[0].extract_str(), "math = a * b * c");
        Ok(())
    }


    #[test]
    fn test_parse_unknown_command() {
        let input = "[@unknown rust]";
        assert!(
            parse_command_line(input, 0, 0).0.is_none(),
            "Unknown command input has been parsed: \"{input}\""
        );
    }

    #[test]
    fn test_parse_code_inline_text_anchor() -> Result<(), Box<dyn std::error::Error>>{
        let input = "[` inline ![] code 123`] raw text    #anchor";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        //assert_eq!(code.extract_str(), "inline code 123");
        let code = &nodes[0];
        match code.kind() {
            AstNodeKind::Code { ref lang, ref inline } => {
                assert_eq!(lang, "");
                assert!(*inline);
            }
            _ => {
                println!("{:?}", code);
                panic! {"it is weird"};
            }
        }
        //println!("{:?}", code.value.contents[0].extract_str());
        //
        let raw_text = &nodes[1];
        if let AstNodeKind::Text = raw_text.kind() {
            assert_eq!(
                &raw_text.location().input
                    [raw_text.location().span.0..raw_text.location().span.1],
                " raw text"
            );
        } else {
            panic!("text not extracted");
        }

        assert_eq!(props.len(), 1);
        if let Property::Anchor { ref name } = props[0] {
            assert_eq!(name, "anchor");
        } else {
            panic!("anchor is not extracted properly");
        }
        Ok(())
    }

    #[test]
    fn test_parse_wiki_link() {
        let input = "[test wiki_page]";
        if let Ok(mut parsed) = PattoLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match &wiki_link.kind() {
                    AstNodeKind::WikiLink { link, anchor } => {
                        assert_eq!(link, "test wiki_page");
                        assert!(anchor.is_none());
                    }
                    _ => {
                        println!("{:?}", wiki_link);
                        panic! {"wiki_link is not correctly parse"};
                    }
                }
            }
        }
    }

    #[test]
    fn test_parse_wiki_link_anchored() {
        let input = "[test wiki_page#anchored]";
        if let Ok(mut parsed) = PattoLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match &wiki_link.kind() {
                    AstNodeKind::WikiLink { link, anchor } => {
                        assert_eq!(link, "test wiki_page");
                        assert!(anchor.is_some());
                        if let Some(anchor) = anchor {
                            assert_eq!(anchor, "anchored");
                        }
                    }
                    _ => {
                        println!("{:?}", wiki_link);
                        panic! {"wiki_link is not correctly parse"};
                    }
                }
            }
        }
    }

    #[test]
    fn test_parse_self_link_anchored() {
        let input = "[#anchored]";
        if let Ok(mut parsed) = PattoLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match &wiki_link.kind() {
                    AstNodeKind::WikiLink { link, anchor } => {
                        assert_eq!(link, "");
                        assert!(anchor.is_some());
                        if let Some(anchor) = anchor {
                            assert_eq!(anchor, "anchored");
                        }
                    }
                    _ => {
                        println!("{:?}", wiki_link);
                        panic! {"wiki_link is not correctly parse"};
                    }
                }
            }
        }
    }

    #[test]
    fn test_parse_img() -> Result<(), Box<dyn std::error::Error>>{
        for (input, g_path, g_alt) in vec![
            ("[@img \"img alt title\" https://gyazo.com/path/to/icon.png]", "https://gyazo.com/path/to/icon.png", Some("img alt title".to_string())),
            ("[@img https://gyazo.com/path/to/icon.png \"img alt title\"]", "https://gyazo.com/path/to/icon.png", Some("img alt title".to_string())),
            ("[@img ./path/to/image.png.png \"alt title\"]", "./path/to/image.png.png", Some("alt title".to_string())),
            ("[@img https://gyazo.com/path/to/icon.png]", "https://gyazo.com/path/to/icon.png", None),
            (r##"[@img ./local/path/to/icon.png "img escaped \"alt title"]"##, "./local/path/to/icon.png", Some(r##"img escaped \"alt title"##.to_string())),
            (r##"[@img ./local/with space/path/to/icon.png "img escaped \"alt title"]"##, "./local/with space/path/to/icon.png", Some(r##"img escaped \"alt title"##.to_string()))] {

            match PattoLineParser::parse(Rule::expr_img, input) {
                Ok(mut parsed) => {
                    let node = transform_img(parsed.next().unwrap(), input, 0, 0).ok_or("transform_img failed")?;
                    if let AstNodeKind::Image{src, alt} = &node.kind() {
                        assert_eq!(src, g_path);
                        assert_eq!(*alt, g_alt);
                    } else {
                        panic!{"image is not correctly transformed"};
                    }
                }
                Err(e) => {
                    println!("{e}");
                    return Err(Box::new(e));
                }
            }
        }
        Ok(())
    }


    #[test]
    fn test_parse_urls() -> Result<(), Box<dyn std::error::Error>> {
        for (input, g_url, g_title) in vec![
            ("[https://username@example.com google]", "https://username@example.com", Some("google".to_string())),
            ("[google https://username@example.com]", "https://username@example.com", Some("google".to_string())),
            ("[https://google.com]", "https://google.com", None),
            ("[日本語の タイトル https://username@example.com]", "https://username@example.com", Some("日本語の タイトル".to_string())),
            ("[https://lutpub.lut.fi/bitstream/handle/10024/167844/masterthesis_khaire_shubham.pdf pdfへのリンク]", "https://lutpub.lut.fi/bitstream/handle/10024/167844/masterthesis_khaire_shubham.pdf", Some("pdfへのリンク".to_string())),
            ("[pdfへのリンク https://lutpub.lut.fi/bitstream/handle/10024/167844/masterthesis_khaire_shubham.pdf]", "https://lutpub.lut.fi/bitstream/handle/10024/167844/masterthesis_khaire_shubham.pdf", Some("pdfへのリンク".to_string())),
            ("[https://username@example.com/path/to/url?param=1&newparam=2]", "https://username@example.com/path/to/url?param=1&newparam=2", None),
            ("[https://google.com https://google.com]", "https://google.com", Some("https://google.com".to_string()))] {
                println!("parsing {input}");
                match PattoLineParser::parse(Rule::expr_url_link, input) {
                    Ok(mut parsed) => {
                        if let Some(link) = transform_url_link(parsed.next().unwrap(), input, 0, 0) {
                            match &link.kind() {
                                AstNodeKind::Link { link, title } => {
                                    assert_eq!(link, g_url);
                                    //assert!(title.is_some());
                                    assert_eq!(*title, g_title);
                                }
                                _ => {
                                    panic! {"link is not correctly parse {:?}", link};
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("{e}");
                        return Err(Box::new(e));
                    }
                }
            }
        Ok(())
    }
    #[test]
    fn test_parse_local_files() -> Result<(), Box<dyn std::error::Error>> {
        for (input, g_local_file, g_title) in [
            ("[./asset/to/image.png path to image]", "./asset/to/image.png", Some("path to image".to_string())),
            ("[./nested/path/image.png path to image]", "./nested/path/image.png", Some("path to image".to_string())),
            ("[title of file ./path/to/file.pdf]", "./path/to/file.pdf", Some("title of file".to_string())),
            ("[./path/to/only_local_file.pdf]", "./path/to/only_local_file.pdf", None),] {
                println!("parsing {input}");
                match PattoLineParser::parse(Rule::expr_local_file_link, input) {
                    Ok(mut parsed) => {
                        if let Some(link) = transform_local_file_link(parsed.next().unwrap(), input, 0, 0) {
                            match &link.kind() {
                                AstNodeKind::Link { link, title } => {
                                    assert_eq!(link, g_local_file);
                                    //assert!(title.is_some());
                                    assert_eq!(*title, g_title);
                                }
                                _ => {
                                    panic! {"link is not correctly parse {:?}", link};
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("{e}");
                        return Err(Box::new(e));
                    }
                }
            }
        Ok(())
    }

    #[test]
    fn test_parse_mails() -> Result<(), Box<dyn std::error::Error>> {
        for (input, g_mail, g_title) in [
            ("[mailto:hoge@example.com example email]", "mailto:hoge@example.com", Some("example email".to_string())),
        ]{
                println!("parsing {input}");
                match PattoLineParser::parse(Rule::expr_mail_link, input) {
                    Ok(mut parsed) => {
                        if let Some(link) = transform_mail_link(parsed.next().unwrap(), input, 0, 0) {
                            match &link.kind() {
                                AstNodeKind::Link { link, title } => {
                                    assert_eq!(link, g_mail);
                                    //assert!(title.is_some());
                                    assert_eq!(*title, g_title);
                                }
                                _ => {
                                    panic! {"link is not correctly parse {:?}", link};
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("{e}");
                        return Err(Box::new(e));
                    }
                }
            }
        Ok(())
    }

    #[test]
    fn test_parse_horizontal_line() -> Result<(), Box<dyn std::error::Error>> {
        let input = "-----";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (nodes, _props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        let hr = &nodes[0];
        if !matches!(hr.kind(), AstNodeKind::HorizontalLine) {
            panic! {"HorizontalLine could not be parsed"};
        }
        Ok(())
    }

    #[test]
    fn test_parse_abbrev_task() -> Result<(), Box<dyn std::error::Error>>  {
        let input = "!2024-10-10 #anchor2 -2024-10-11T20:00";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (_nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        let task = &props[0];
        if let Property::Task { status, due } = task {
            assert_eq!(*status, TaskStatus::Todo);
            assert_eq!(*due, Deadline::Date(chrono::NaiveDate::from_ymd_opt(2024, 10, 10).unwrap()));
        } else {
            panic!("task could not be parsed");
        };

        let task = &props[2];
        if let Property::Task { status, due } = task {
            assert_eq!(*status, TaskStatus::Done);
            assert_eq!(*due, Deadline::DateTime(chrono::NaiveDateTime::parse_from_str("2024-10-11T20:00", "%Y-%m-%dT%H:%M").unwrap()));
        } else {
            panic!("task could not be parsed");
        };

        Ok(())
    }

    #[test]
    #[allow(deprecated)]
    fn test_deadline_sorting_order() -> Result<(), Box<dyn std::error::Error>>  {
        let mut values = [
            Deadline::Date(chrono::NaiveDate::from_ymd(2022, 1, 1)),
            Deadline::DateTime(chrono::NaiveDateTime::from_timestamp(1672531199, 0)),
            Deadline::Uninterpretable(String::from("Hello")),
            Deadline::Date(chrono::NaiveDate::from_ymd(2023, 1, 1)),
            Deadline::Date(chrono::NaiveDate::from_ymd(2022, 12, 31)),
            Deadline::DateTime(chrono::NaiveDateTime::from_timestamp(1672531200, 0)),
            Deadline::Uninterpretable(String::from("World")),
        ];

        let gt = [
            Deadline::Date(chrono::NaiveDate::from_ymd(2022, 1, 1)),
            Deadline::Date(chrono::NaiveDate::from_ymd(2022, 12, 31)),
            Deadline::DateTime(chrono::NaiveDateTime::from_timestamp(1672531199, 0)),
            Deadline::Date(chrono::NaiveDate::from_ymd(2023, 1, 1)),
            Deadline::DateTime(chrono::NaiveDateTime::from_timestamp(1672531200, 0)),
            Deadline::Uninterpretable(String::from("Hello")),
            Deadline::Uninterpretable(String::from("World")),
        ];

        values.sort();

        for (v, x) in values.iter().zip(gt.iter()) {
            assert_eq!(v, x);
        }
        Ok(())
    }

    // #[test]
    // fn test_parse_error() {
    //     let err = PattoLineParser::parse(Rule::expr_command, "[@  ] #anchor").unwrap_err();
    //     println!("{:?}", err);
    //     log::debug!("{:?}", err.variant.message());
    //     todo!();
    //     ()
    // }
}
