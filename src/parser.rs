use chrono;
use pest::Parser;
use pest_derive::Parser;
use std::cell::RefCell;
use std::fmt;
use std::ops;
use std::rc::Rc;
use thiserror::Error;

use pest;
use pest::iterators::Pair;

#[derive(Parser)]
#[grammar = "markshift.pest"]
pub struct MarkshiftLineParser;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Span(pub usize, pub usize);

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
        write!(f, "{}\n", self.input)?;
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
        //write!(f, "{: <1$}", "", self.span.0)?;
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

impl Location<'_> {
    fn merge(&self, other: &Self) -> Self {
        use std::cmp::{max, min};
        assert_eq!(self.input, other.input);
        assert_eq!(self.row, other.row);
        Self {
            row: self.row,
            input: self.input,
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
pub struct Annotation<'a, T> {
    pub value: T,
    pub location: Location<'a>,
}

//impl<T> fmt::Display for Annotation<'_, T>
//// where
////     T: fmt::Display,
//{
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        write!(f, "{}", self.location)
//    }
//}

#[derive(Debug, Default)]
pub struct AstNodeInternal<'a> {
    pub contents: RefCell<Vec<AstNode<'a>>>,
    pub children: RefCell<Vec<AstNode<'a>>>,
    pub kind: AstNodeKind,
    // text will be the string matched with this AstNode.
    // will be used when contents.len() == 0
    // pub text: &'a str,
}

// impl<'a> fmt::Display for AstNodeInternal<'a> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.contents)
//     }
// }

#[derive(PartialEq, Debug)]
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
    WikiLink {
        link: String,
        anchor: Option<String>,
    },
    Text,
    #[default]
    Dummy,
}

type AstNodeImpl<'a> = Annotation<'a, AstNodeInternal<'a>>;
#[derive(Debug)]
pub struct AstNode<'a>(Rc<Annotation<'a, AstNodeInternal<'a>>>);

impl<'a> AstNode<'a> {
    pub fn new(input: &'a str, row: usize, span: Option<Span>, kind: Option<AstNodeKind>) -> Self {
        AstNode(Rc::new(AstNodeImpl {
            value: AstNodeInternal {
                contents: RefCell::new(vec![]),
                children: RefCell::new(vec![]),
                kind: kind.unwrap_or(AstNodeKind::Dummy),
            },
            location: Location {
                row,
                input,
                span: span.unwrap_or(Span(0, input.len())),
            },
        }))
    }
    pub fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }

    pub fn value(&self) -> &AstNodeInternal<'a> {
        &self.0.value
    }
    pub fn location(&self) -> &Location<'a> {
        &self.0.location
    }
    pub fn line(
        input: &'a str,
        row: usize,
        span: Option<Span>,
        props: Option<Vec<Property>>,
    ) -> Self {
        Self::new(
            input,
            row,
            span,
            Some(AstNodeKind::Line {
                properties: props.unwrap_or(vec![]),
            }),
        )
    }
    pub fn code(
        input: &'a str,
        row: usize,
        span: Option<Span>,
        lang: &'a str,
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
    pub fn math(input: &'a str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Math))
    }
    pub fn quote(input: &'a str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Quote))
    }
    pub fn wikilink(
        input: &'a str,
        row: usize,
        span: Option<Span>,
        link: &'a str,
        anchor: Option<&'a str>,
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
    pub fn text(input: &'a str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Text))
    }

    pub fn extract_str(&self) -> &str {
        self.location().as_str()
    }
}

impl<'a> fmt::Display for AstNode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "extracted: {}\n", self.extract_str())?;
        for (i, content) in self.value().contents.borrow().iter().enumerate() {
            write!(f, "{i} -- {}", content)?;
        }
        //for child in &self.value.children {
        //    write!(f, "\tchild -- {:?}\n", child)?;
        //}
        if let AstNodeKind::Line { properties } = &self.value().kind {
            for prop in properties {
                write!(f, "property -- {:?}\n", prop)?;
            }
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ParserError<'a> {
    #[error("Invalid indent: {0}")]
    InvalidIndentation(Location<'a>),
    #[error("Invalid command parameter: {0}")]
    InvalidCommandParameter(String),
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
}

pub fn parse_command_line(
    line: &str,
    row: usize,
    indent: usize,
) -> (Option<AstNode>, Vec<Property>) {
    let Ok(mut pairs) = MarkshiftLineParser::parse(Rule::expr_command_line, &line[indent..]) else {
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
    return (command_node, properties);
}

fn transform_command<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode<'a>> {
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
                    return Some(AstNode::math(line, row, Some(span)));
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
                        println!("No language specified for code block");
                    }
                    return Some(AstNode::code(line, row, Some(span), lang, false));
                }
                Rule::parameter => {
                    println!(
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
            println!(
                "Do you provide other than expr_command to fn transform_command: {:?}",
                pair.as_rule()
            );
        }
    }
    None
}

/// assuming pair is expr_wiki_link
fn transform_wiki_link<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode<'a>> {
    let inner = pair.into_inner().next().unwrap(); // wiki_link or wiki_link_anchored
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::wiki_link_anchored => {
            let mut inner2 = inner.into_inner();
            let wiki_link = inner2.next().unwrap();
            let expr_anchor = inner2.next().unwrap();
            return Some(AstNode::wikilink(
                line,
                row,
                Some(span),
                wiki_link.as_str(),
                Some(expr_anchor.into_inner().next().unwrap().as_str()),
            ));
        }
        Rule::wiki_link => {
            return Some(AstNode::wikilink(
                line,
                row,
                Some(span),
                inner.as_str(),
                None,
            ));
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
                            if let Ok(datetime) =
                                chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
                            {
                                until = Deadline::DateTime(datetime);
                            } else if let Ok(date) =
                                chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                            {
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
            return Some(Property::Task { status, until });
        }
        _ => {
            println!("????? {:?}", pair);
            return None;
        }
    }
    None
}

fn parse_trailing_properties(s: &str) -> Option<Vec<Property>> {
    let Ok(mut trailing_properties) = MarkshiftLineParser::parse(Rule::trailing_properties, s)
    else {
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
    line: &'a str,
    row: usize,
    indent: usize,
) -> Result<(Vec<AstNode<'a>>, Option<Vec<Property>>), ParserError<'a>> {
    let mut nodes: Vec<AstNode<'a>> = vec![];
    let mut props: Vec<Property> = vec![];

    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Rule::statement => {
            //     let transform_statement(inner.into_inner().next().unwrap(), line)?;
            //     // TODO handle all inners
            //     //for inner in inner.into_inner() {
            // }
            //Rule::line => {
            //    // `line' contains only one element, either expr_command or statement
            //    // be careful when you change the grammar
            //    for inner in inner.into_inner() {
            //        if let Some(parsed) = transform_statement(inner, line) {
            //            line.value.content.push(parsed);
            //        }
            //    }
            //    None
            //}
            Rule::expr_builtin_symbols => {
                //todo!("TODO!");
                continue;
            }
            Rule::expr_wiki_link => {
                if let Some(node) = transform_wiki_link(inner, line, row, indent) {
                    nodes.push(node);
                }
            }
            Rule::expr_code_inline => {
                //assert!(matches!(line.value.kind, AstNodeKind::Line { .. }));
                let mut code = AstNode::code(
                    line,
                    row,
                    Some(Into::<Span>::into(inner.as_span()) + indent),
                    "",
                    true,
                );
                let code_inline = inner.into_inner().next().unwrap();
                code.value().contents.borrow_mut().push(AstNode::text(
                    line,
                    row,
                    Some(Into::<Span>::into(code_inline.as_span()) + indent),
                ));
                nodes.push(code);
            }
            Rule::expr_property => {
                if let Some(prop) = transform_property(inner) {
                    props.push(prop);
                }
            }
            // Rule::expr_anchor => {
            //     //println!("non-trailing anchor will be treated as a text");
            //     //nodes.push(AstNode::text(line, row, Some(Into::<Span>::into(inner.as_span()) + indent)));
            //     if let Some(prop) = transform_property(inner) {
            //         props.push(prop);
            //     }
            // }
            Rule::raw_sentence => {
                nodes.push(AstNode::text(
                    line,
                    row,
                    Some(Into::<Span>::into(inner.as_span()) + indent),
                ));
            }
            Rule::trailing_properties => {
                let mut properties: Vec<Property> = vec![];
                for (i, inner) in inner.into_inner().enumerate() {
                    if let Some(prop) = transform_property(inner) {
                        props.push(prop);
                    }
                }
            }
            Rule::EOI => {
                continue;
            }
            _ => {
                println!("{:?} not implemented", inner.as_rule());
                unreachable!()
            }
        }
    }
    return Ok((nodes, Some(props)));
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
        let (astnode, props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        match node.value().kind {
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
    fn test_parse_code_emtpy_lang() {
        let input = "[@code   ]";
        // let parsed = MarkshiftLineParser::parse(Rule::expr_command, input);
        // assert!(parsed.is_ok(), "Failed to parse \"{input}\"");
        // let mut pairs = parsed.unwrap();
        // assert_eq!(pairs.len(), 1, "must contain only one expr_command");
        // let parsed_command = pairs.next().unwrap();
        // //                          \- the first pair, which is expr_command
        let (astnode, props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        match node.value().kind {
            AstNodeKind::Code { lang, inline } => {
                assert!(lang == "");
                assert!(inline == false);
            }
            _ => {
                panic! {"it is weird"};
            }
        }
    }

    #[test]
    fn test_parse_indented_code_command() {
        let input = "		[@code なでしこ]   #anchor1 {@task status=todo until=2024-09-24}";
        let indent = input.chars().take_while(|&c| c == '\t').count();
        let (astnode, props) = parse_command_line(input, 0, indent);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        println!("{}", node);
        let Some(end_code) = input.find("]") else {
            panic!("no way!");
        };
        assert_eq!(node.location.span, Span(indent, end_code + 1));
        match node.value().kind {
            AstNodeKind::Code { lang, inline } => {
                assert!(lang == "なでしこ");
                assert!(inline == false);
            }
            _ => {
                panic! {"it is weird"};
            }
        }
        for prop in props {
            match prop {
                Property::Task { status, until } => {
                    let TaskStatus::Todo = status else {
                        panic!("task is not in todo state!");
                    };
                    if let Deadline::Date(date) = until {
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
    fn test_parse_trailing_properties() {
        let input = "   #anchor1 {@task status=todo until=2024-09-24} #anchor2";
        let Some(props) = parse_trailing_properties(input) else {
            panic!("Failed to parse trailing properties");
        };
        let anchor1 = &props[0];
        if let Property::Anchor { name } = anchor1 {
            assert_eq!(name, "anchor1");
        } else {
            panic!("anchor1 is not extracted properly");
        };

        let task = &props[1];
        if let Property::Task { status, until } = task {
            let TaskStatus::Todo = status else {
                panic!("task is not in todo state!");
            };
            assert_eq!(
                until,
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
    }

    #[test]
    fn test_parse_math() {
        let input = "[@math  ]";
        let indent = input.chars().take_while(|&c| c == '\t').count();
        let (astnode, props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        println!("{:?}", node);
        assert_eq!(node.location.span, Span(indent, input.len()));
        match node.value().kind {
            AstNodeKind::Math => {}
            _ => {
                panic! {"Math command could not be parsed"};
            }
        }
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
    fn test_parse_code_inline_text_anchor() {
        let input = "[` inline ![] code 123`] raw text #anchor";
        if let Ok(mut parsed) = MarkshiftLineParser::parse(Rule::statement, input) {
            if let Ok((nodes, props)) = transform_statement(parsed.next().unwrap(), input, 0, 0) {
                //assert_eq!(code.extract_str(), "inline code 123");
                let code = &nodes[0];
                match code.value().kind {
                    AstNodeKind::Code { ref lang, inline } => {
                        assert_eq!(lang, "");
                        assert_eq!(inline, true);
                    }
                    _ => {
                        println!("{:?}", code);
                        panic! {"it is weird"};
                    }
                }
                //println!("{:?}", code.value.contents[0].extract_str());
                //
                let raw_text = &nodes[1];
                if let AstNodeKind::Text = raw_text.value().kind {
                    assert_eq!(
                        &raw_text.location.input
                            [raw_text.location.span.0..raw_text.location.span.1],
                        " raw text "
                    );
                } else {
                    panic!("text not extracted");
                }

                assert!(props.is_some());
                if let Property::Anchor { ref name } = props.unwrap()[0] {
                    assert_eq!(name, "anchor");
                } else {
                    panic!("anchor is not extracted properly");
                }
            }
        }
    }

    #[test]
    fn test_parse_wiki_link() {
        let input = "[test wiki_page]";
        if let Ok(mut parsed) = MarkshiftLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match wiki_link.value().kind {
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
        if let Ok(mut parsed) = MarkshiftLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match wiki_link.value().kind {
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

    // #[test]
    // fn test_parse_error() {
    //     let err = MarkshiftLineParser::parse(Rule::expr_command, "[@  ] #anchor").unwrap_err();
    //     println!("{:?}", err);
    //     println!("{:?}", err.variant.message());
    //     todo!();
    //     ()
    // }
}
