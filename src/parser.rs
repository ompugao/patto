use chrono;
use pest::Parser;
use pest_derive::Parser;
use std::fmt;
use std::ops;
use thiserror::Error;

use pest::iterators::Pair;

#[derive(Parser)]
#[grammar = "markshift.pest"]
pub struct MarkshiftLineParser;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span (pub usize, pub usize);

impl ops::Add<usize> for Span {
    type Output = Self;

    fn add(self, offset: usize) -> Self {
        Span(self.0 + offset, self.1 + offset)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Location<'a> {
    pub row: usize,
    pub input: &'a str,
    pub span: Span, //TODO span がindent分ずれるのがわかりづらい
}

impl fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({} - {})", self.span.0, self.span.1)
    }
}

impl From<pest::Span<'_>> for Span {
    fn from(from: pest::Span<'_>) -> Span {
        Self(from.start(), from.end())
    }
}

impl Location<'_> {
    fn merge(&self, other: &Self) -> Self {
        use std::cmp::{max, min};
        assert_eq!(self.input, other.input);
        assert_eq!(self.row, other.row);
        Self {
            row: self.row,
            input: self.input,
            span: Span(min(self.span.0, other.span.0), max(self.span.1, other.span.1)),
        }
    }

    fn as_str(&self) -> &str {
        &self.input[self.span.0..self.span.1]
    }
}

#[derive(Debug, Default)]
pub struct Annotation<'a, T> {
    pub value: T,
    pub location: Location<'a>,
}

impl<T> fmt::Display for Annotation<'_, T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n", self.value)?;
        write!(f, "{: <1$}", "", self.location.span.0)?;
        write!(f, "{:^<1$}", "", self.location.span.1 - self.location.span.0)
    }
}

#[derive(Debug, Default)]
pub struct AstNodeInternal<'a> {
    pub content: Vec<AstNode<'a>>,
    pub children: Vec<AstNode<'a>>,
    pub kind: AstNodeKind,

    // text will be the string matched with this AstNode.
    // will be used when content.len() == 0
    // pub text: &'a str,
}

// impl<'a> fmt::Display for AstNodeInternal<'a> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.content)
//     }
// }

#[derive(Debug)]
pub enum Deadline {
    DateTime(chrono::NaiveDateTime),
    Date(chrono::NaiveDate),
    Uninterpretable(String),
}

#[derive(Debug)]
pub enum TaskStatus {
    Todo,
    Doing,
    Done,
}

#[derive(Debug)]
pub enum Property {
    Task { status: TaskStatus, until: Deadline },
    Anchor { name: String },
}

#[derive(Debug, Default)]
pub enum AstNodeKind {
    Line {
        //indent: usize,
        properties: Vec<Property>,
    },
    Quote,
    Math,
    Code {
        lang: String,
        inline: bool,
    },
    //Table,
    Image {
        src: String,
        alt: String,
    },
    Text,
    #[default]
    Dummy,
}

pub type AstNode<'a> = Annotation<'a, AstNodeInternal<'a>>;

impl<'a> AstNode<'a> {
    pub fn new(input: &'a str, row: usize, span: Option<Span>, kind: Option<AstNodeKind>) -> Self {
        AstNode {
            value: AstNodeInternal {
                content: Vec::new(),
                children: Vec::new(),
                kind: kind.unwrap_or(AstNodeKind::Dummy),
            },
            location: Location {
                row,
                input,
                span: span.unwrap_or(Span(0, input.len())),
            },
        }
    }
    pub fn line(input: &'a str, row: usize, span: Option<Span>) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Line {
                properties: Vec::new(),
            }),
        )
    }
    pub fn code(input: &'a str, row: usize, span: Option<Span>, lang: &'a str, inline: bool) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Code {
                lang: lang.to_string(),
                inline: inline,
            }),
        )
    }
    pub fn math(input: &'a str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Math))
    }
    pub fn quote(input: &'a str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Quote))
    }
    pub fn text(input: &'a str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Text))
    }

    pub fn extract_str(&self) -> &str {
        self.location.as_str()
    }
}

#[derive(Error, Debug)]
pub enum ParserError<'a> {
    #[error("Invalid indent: {0}")]
    InvalidIndentation(Annotation<'a, &'a str>),
    #[error("Invalid command parameter: {0}")]
    InvalidCommandParameter(String),
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
}


pub fn parse_command(line: &str, row: usize, indent: usize) -> (Option<AstNode>, Vec<Property>) {
    let Ok(mut pairs) = MarkshiftLineParser::parse(Rule::expr_command, &line[indent..]) else {
        return (None, vec![]);
    };
    assert_eq!(pairs.len(), 1, "must contain only one expr_command");
    let parsed_command = pairs.next().unwrap();
    //                          \- the first pair, which is expr_command
    return transform_command(parsed_command, line, row, indent);
}

fn transform_command<'a>(pair: Pair<'a, Rule>, line: &'a str, row: usize, indent: usize) -> (Option<AstNode<'a>>, Vec<Property>) {
    let span = Into::<Span>::into(pair.as_span()) + indent;
    let mut node: Option<AstNode<'a>> = None;
    let mut props: Vec<Property> = vec![];
    match pair.as_rule() {
        Rule::expr_command => {
            let mut inner = pair.into_inner();
            let builtin_commands = inner.next().unwrap(); // consume the command
            let command = builtin_commands.into_inner().next().unwrap();
            match command.as_rule() {
                Rule::command_math => {
                    node = Some(AstNode::math(line, row, Some(span)));
                }
                Rule::command_quote => {
                    node = Some(AstNode::quote(line, row, Some(span)));
                }
                Rule::command_code => {
                    // 1st parameter
                    let lang = inner.next().unwrap().as_str();
                    node = Some(AstNode::code(line, row, Some(span), lang, false));
                }
                Rule::parameter => {
                    println!("parameter must have already been consumed: {}", command.as_str());
                    // TODO return text?
                    node = Some(AstNode::text(line, row, Some(span)));
                }
                _ => {
                }
            }
        }
        Rule::trailing_properties => {
            for inner in pair.into_inner() {
                if let Some(prop) = transform_property(inner) {
                    props.push(prop);
                }
            }
        }
        _ => {
            println!(
                "Do not provide other than expr_command to fn transform_command: {:?}",
                pair.as_rule()
            );
        }
    }
    (node, props)
}

fn transform_property(pair: Pair<Rule>) -> Option<Property> {
    match pair.as_rule() {
        Rule::expr_anchor => {
            let anchor = Property::Anchor{name: pair.into_inner().next().unwrap().as_str().to_string()};
            return Some(anchor);
        }
        Rule::expr_property => {
            let mut inner = pair.into_inner();
            let property_name = inner.next().unwrap().as_str();
            if property_name != "task" {
                println!("Unknown property: {}", property_name);
                return None;
            }

            let mut status = TaskStatus::Todo;
            let mut until = Deadline::Uninterpretable("".to_string());
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
                                println!("Unknown task status: '{}', interpreted as 'todo'", value);
                            }
                        } else if current_key == "until" {
                            if let Ok(datetime) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M") {
                                until = Deadline::DateTime(datetime);
                            } else if let Ok(date) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                                until = Deadline::Date(date);
                            } else {
                                until = Deadline::Uninterpretable(value.to_string());
                            }
                        } else {
                            println!("Unknown property value: {}", value);
                        }
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
            return Some(Property::Task{status, until});
        }
        _ => {
            println!("????? {:?}", pair);
            return None;
        }
    }
    None
}

fn parse_trailing_properties(s: &str) -> Option<Vec<Property>> {
    let Ok(mut trailing_properties) = MarkshiftLineParser::parse(Rule::trailing_properties, s) else {
        return None;
    };
    let mut properties: Vec<Property> = vec![];
    for pair in trailing_properties.next().unwrap().into_inner() {
        if let Some(prop) = transform_property(pair) {
            properties.push(prop);
        }
    }
    Some(properties)
}

pub fn transform_statement<'a, 'b>(
    pair: Pair<'a, Rule>,
    line: &'b mut AstNode<'a>,
) -> Option<AstNode<'a>> {
    match pair.as_rule() {
        Rule::statement => {
            transform_statement(pair.into_inner().next().unwrap(), line)
            // TODO handle all inners
            //for pair in pair.into_inner() {
        }
        Rule::raw_sentence => Some(AstNode::text(pair.get_input(), line.location.row, Some(pair.as_span().into()))),
        Rule::line => {
            // `line' contains only one element, either expr_command or statement
            // be careful when you change the grammar
            for inner in pair.into_inner() {
                if let Some(parsed) = transform_statement(inner, line) {
                    line.value.content.push(parsed);
                }
            }
            None
        }
        Rule::expr_anchor => {
            assert!(matches!(line.value.kind, AstNodeKind::Line { .. }));
            if let AstNodeKind::Line { properties } = &mut line.value.kind {
                properties.push(Property::Anchor {
                    name: pair.as_str().to_string(),
                });
            }
            None
        }
        Rule::expr_code_inline => {
            assert!(matches!(line.value.kind, AstNodeKind::Line { .. }));
            let s = pair.get_input();
            let mut code = AstNode::code(s, line.location.row, Some(pair.as_span().into()), "", true);
            if let Some(code_inline) = transform_statement(pair.into_inner().next().unwrap(), line) {
                code.value.content.push(code_inline);
            }
            Some(code)
        }
        Rule::code_inline => {
            Some(AstNode::text(pair.get_input(), line.location.row, Some(pair.as_span().into())))
        }
        _ => {
            println!("{:?} not implemented", pair.as_rule());
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::pest::Parser;

    #[test]
    fn test_parse_code_command() {
        let input = "[@code rust]";
        // let parsed = MarkshiftLineParser::parse(Rule::expr_command, input);
        // assert!(parsed.is_ok(), "Failed to parse \"{input}\"");
        // let mut pairs = parsed.unwrap();
        // assert_eq!(pairs.len(), 1, "must contain only one expr_command");
        // let parsed_command = pairs.next().unwrap();
        // //                          \- the first pair, which is expr_command
        let (astnode, props) = parse_command(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        match node.value.kind {
            AstNodeKind::Code { lang, inline } => {
                assert!(lang == "rust");
                assert!(inline == false);
            }
            _ => {
                panic! {"it is weird"};
            }
        }
    }

    #[test]
    fn test_parse_indented_code_command() {
        let input = "		[@code cpp]   #anchor1 {@task status=todo until=2024-09-24}";
        let indent = input.chars().take_while(|&c| c == '\t').count();
        let (astnode, props) = parse_command(input, 0, indent);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        //println!("{:?}", node);
        assert_eq!(node.location.span, Span(indent, input.len()));
        match node.value.kind {
            AstNodeKind::Code { lang, inline } => {
                assert!(lang == "cpp");
                assert!(inline == false);
            }
            _ => {
                panic! {"it is weird"};
            }
        }
        for prop in props {
            match prop {
                Property::Task{status, until} => {
                    let TaskStatus::Todo = status else {
                        panic!("task is not in todo state!");
                    };
                    if let Deadline::Date(date) = until {
                        assert_eq!(date, chrono::NaiveDate::from_ymd(2024, 9, 24));
                    } else {
                        panic!("date is not correctly parsed");
                    }
                }
                Property::Anchor{name} => {
                    assert_eq!(name, "anchor1");
                }
            }
        }
    }
    #[test]
    fn test_parse_trailing_properties() {
        let input = "   #anchor1 {@task status=todo until=2024-09-24} ";
        println!("{:?}", parse_trailing_properties(input));
        assert!(true);
    }

    #[test]
    fn test_parse_math() {
        let input = "[@math  ]";
        let indent = input.chars().take_while(|&c| c == '\t').count();
        let (astnode, props) = parse_command(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        println!("{:?}", node);
        assert_eq!(node.location.span, Span(indent, input.len()));
        match node.value.kind {
            AstNodeKind::Math => {
            }
            _ => {
                panic! {"Math command could not be parsed"};
            }
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        let input = "[@unknown rust]";
        assert!(parse_command(input, 0, 0).0.is_none(), 
            "Unknown command input has been parsed: \"{input}\""
        );
    }

    #[test]
    fn test_parse_code_inline() {
        let input = "[` inline code 123`]";
        if let Ok(mut parsed) = MarkshiftLineParser::parse(Rule::expr_code_inline, input) {
            let mut newline = AstNode::line(&input, 0, None);
            if let Some(code) = transform_statement(parsed.next().unwrap(), &mut newline) {
                //assert_eq!(code.extract_str(), "inline code 123");
                match code.value.kind {
                    AstNodeKind::Code { lang, inline } => {
                        assert_eq!(lang, "");
                        assert_eq!(inline, true);
                    }
                    _ => {
                        println!("{:?}", code);
                        panic! {"it is weird"};
                    }
                }
                println!("{:?}", code.value.content[0].extract_str());
            }
        }
        // let parsed_command = parsed.unwrap().next().unwrap();
        // assert!(transform_command(parsed_command, 0).is_none(), "Unknown command has been successfully parsed");
    }
}
