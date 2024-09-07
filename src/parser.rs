use pest_derive::Parser;
use std::fmt;
use thiserror::Error;
use chrono;

#[derive(Parser)]
#[grammar = "markshift.pest"]
pub struct MarkshiftLineParser;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Location<'a> {
    pub input: &'a str,
    pub start: usize,
    pub end: usize, //exclusive
}

impl fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}) - ({})", self.start, self.end)
    }
}

impl Location<'_> {
    fn merge(&self, other: &Self) -> Self {
        use std::cmp::{max, min};
        assert_eq!(self.input, other.input);
        Self {
            input: self.input,
            start: min(self.start, other.start),
            end: max(self.end, other.end),
        }
    }
}

#[derive(Debug, Default)]
pub struct Annotation<'a, T> {
    pub value: T,
    pub location: Location<'a>,
}

impl<T> fmt::Display for Annotation<'_, T> where T: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n", self.value)?;
        write!(f, "{: <1$}", "", self.location.start)?;
        write!(f, "{:^<1$}", "", self.location.end - self.location.start)
    }
}

#[derive(Debug, Default)]
pub struct AstNodeInternal<'a> {
    pub content: Vec<AstNode<'a>>,
    pub children: Vec<AstNode<'a>>,
    pub kind: AstNodeKind,
    pub text: &'a str,
}

#[derive(Debug)]
pub enum Deadline {
    DateTime(chrono::NaiveDateTime),
    Date(chrono::NaiveDate),
    Uninterpretable(String),
}

#[derive(Debug)]
pub enum Property {
    Task { status: String, until: Deadline },
    Anchor { name: String },
}

#[derive(Debug, Default)]
pub enum AstNodeKind {
    Line { properties: Vec<Property> },
    Quote,
    Math,
    Code { lang: String },
    //Table,
    Image { src: String, alt: String },
    Text,
    #[default]
    Dummy,
}

pub type AstNode<'a> = Annotation<'a, AstNodeInternal<'a>>;

impl<'a> AstNode<'a> {
    pub fn new(input: &'a str, kind: Option<AstNodeKind>) -> Self {
        AstNode {
            value: AstNodeInternal {
                content: Vec::new(),
                children: Vec::new(),
                kind: kind.unwrap_or(AstNodeKind::Dummy),
                text: input,
            },
            location: Location {
                input,
                start: 0,
                end: input.len(),
            },
        }
    }
    pub fn line(input: &'a str) -> Self {
        Self::new(input, Some(AstNodeKind::Line { properties: Vec::new() }))
    }
    pub fn code(input: &'a str, lang: &'a str) -> Self {
        AstNode {
            value: AstNodeInternal {
                content: Vec::new(),
                children: Vec::new(),
                kind: AstNodeKind::Code { lang: lang.to_string() },
                text: input,
            },
            location: Location {
                input,
                start: 0,
                end: input.len(),
            },
        }
    }
    pub fn math(input: &'a str) -> Self {
        Self::new(input, Some(AstNodeKind::Math))
    }
    pub fn quote(input: &'a str) -> Self {
        Self::new(input, Some(AstNodeKind::Quote))
    }
    pub fn text(input: &'a str) -> Self {
        Self::new(input, Some(AstNodeKind::Text))
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

use pest::iterators::Pair;
pub fn transform_command(pair: Pair<Rule>) -> AstNode {
    match pair.as_rule() {
        Rule::expr_command => {
            let s = pair.as_str();
            let mut inner = pair.into_inner();
            let builtin_commands = inner.next().unwrap();  // consume the command
            let command = builtin_commands.into_inner().next().unwrap();
            match command.as_rule() {
                Rule::command_math => {
                    return AstNode::math(s);
                }
                Rule::command_quote => {
                    return AstNode::quote(s);
                }
                Rule::command_code => {
                    // 1st parameter
                    let lang = inner.next().unwrap().as_str();
                    return AstNode::code(s, lang);
                }
                Rule::parameter => {
                    println!("parameter must have already been consumed: {}", s);
                    // TODO return text?
                    return AstNode::text(s);
                }
                _ => {
                    println!("unknown command: {:?}", command);
                    unreachable!()
                }
            }
            println!("parsed command: {:?}", command);
            unreachable!()
        }
        _ => { 
            println!("Do not provide other than expr_command to fn transform_command: {:?}", pair.as_rule());
            unreachable!()
        }
    }
}

pub fn transform_statement<'a, 'b>(pair: Pair<'a, Rule>, line: &'b mut AstNode<'a>) -> Option<AstNode<'a>> {
    match pair.as_rule() {
        Rule::statement => {
            transform_statement(pair.into_inner().next().unwrap(), line)
            // TODO handle all inners
            //for pair in pair.into_inner() {
        }
        Rule::raw_sentence => {
            Some(AstNode::text(pair.as_str()))
        }
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
                properties.push(Property::Anchor { name: pair.as_str().to_string() });
            }
            None
        }
        Rule::expr_code_inline => {
            todo!("expr_code_inline");
            None
        }
        _ => { 
            println!("{:?} not implemented", pair.as_rule());
            unreachable!()
        }
    }
}

