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
    pub row: usize,
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
        assert_eq!(self.row, other.row);
        Self {
            input: self.input,
            row: self.row,
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

    // text will be the string matched with this AstNode.
    // will be used when content.len() == 0
    pub text: &'a str,
}

impl<'a> fmt::Display for AstNodeInternal<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.text)
    }
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
    pub fn new(input: &'a str, row: usize, kind: Option<AstNodeKind>) -> Self {
        AstNode {
            value: AstNodeInternal {
                content: Vec::new(),
                children: Vec::new(),
                kind: kind.unwrap_or(AstNodeKind::Dummy),
                text: input,
            },
            location: Location {
                input,
                row,
                start: 0,
                end: input.len(),
            },
        }
    }
    pub fn line(input: &'a str, row: usize) -> Self {
        Self::new(input, row, Some(AstNodeKind::Line { properties: Vec::new() }))
    }
    pub fn code(input: &'a str, row: usize, lang: &'a str) -> Self {
        AstNode {
            value: AstNodeInternal {
                content: Vec::new(),
                children: Vec::new(),
                kind: AstNodeKind::Code { lang: lang.to_string() },
                text: input,
            },
            location: Location {
                input,
                row,
                start: 0,
                end: input.len(),
            },
        }
    }
    pub fn math(input: &'a str, row: usize) -> Self {
        Self::new(input, row, Some(AstNodeKind::Math))
    }
    pub fn quote(input: &'a str, row: usize) -> Self {
        Self::new(input, row, Some(AstNodeKind::Quote))
    }
    pub fn text(input: &'a str, row: usize) -> Self {
        Self::new(input, row, Some(AstNodeKind::Text))
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
pub fn transform_command(pair: Pair<Rule>, row: usize) -> Option<AstNode> {
    match pair.as_rule() {
        Rule::expr_command => {
            let s = pair.as_str();
            let mut inner = pair.into_inner();
            let builtin_commands = inner.next().unwrap();  // consume the command
            let command = builtin_commands.into_inner().next().unwrap();
            match command.as_rule() {
                Rule::command_math => {
                    return Some(AstNode::math(s, row));
                }
                Rule::command_quote => {
                    return Some(AstNode::quote(s, row));
                }
                Rule::command_code => {
                    // 1st parameter
                    let lang = inner.next().unwrap().as_str();
                    return Some(AstNode::code(s, row, lang));
                }
                Rule::parameter => {
                    println!("parameter must have already been consumed: {}", s);
                    // TODO return text?
                    return Some(AstNode::text(s, row));
                }
                _ => {
                    return None;
                }
            }
            unreachable!()
        }
        _ => { 
            println!("Do not provide other than expr_command to fn transform_command: {:?}", pair.as_rule());
            return None;
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
            Some(AstNode::text(pair.as_str(), line.location.row))
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


#[cfg(test)]
mod tests {
    use super::*;
    use ::pest::Parser;

    #[test]
    fn test_parse_code_command() {
        let input = "[@code rust]";
        let parsed = MarkshiftLineParser::parse(Rule::expr_command, input);
        assert!(parsed.is_ok(), "Failed to parse \"{input}\"");
        let mut pairs = parsed.unwrap();
        assert_eq!(pairs.len(), 1, "must contain only one expr_command");
        let parsed_command = pairs.next().unwrap();
        //                          \- the first pair, which is expr_command
        let Some(astnode) = transform_command(parsed_command, 0) else {
            panic!("Failed to parse code command");
        };
        match astnode.value.kind {
            AstNodeKind::Code{lang} => {
                assert!(lang == "rust");
            }
            _ => {
                panic!{"it is weird"};
            }
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        let input = "[@unknown rust]";
        let parsed = MarkshiftLineParser::parse(Rule::expr_command, input);
        assert!(parsed.is_err(), "Unknown command input has been parsed: \"{input}\"");
        // let parsed_command = parsed.unwrap().next().unwrap();
        // assert!(transform_command(parsed_command, 0).is_none(), "Unknown command has been successfully parsed");
    }
}
