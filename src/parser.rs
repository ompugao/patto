use chrono;
use pest::Parser;
use pest_derive::Parser;
use std::cell::RefCell;
use std::cmp;
use std::fmt;
use std::ops;
use std::rc::Rc;
use thiserror::Error;
use log;

use pest;
use pest::iterators::Pair;

#[derive(Parser)]
#[grammar = "tabton.pest"]
struct TabtonLineParser;

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
    pub span: Span,
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
    pub fn math(input: &'a str, row: usize, span: Option<Span>, inline: bool) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Math { inline }))
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
    pub fn link(
        input: &'a str,
        row: usize,
        span: Option<Span>,
        link: &'a str,
        title: Option<&'a str>,
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
    pub fn text(input: &'a str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Text))
    }
    pub fn image(input: &'a str, row: usize, span: Option<Span>, src: &'a str, alt: Option<&'a str>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::Image{src: src.to_string(), alt: alt.map(str::to_string)}))
    }
    pub fn decoration(
        input: &'a str,
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
    //pub fn bold(input: &'a str, row: usize, span: Option<Span>, fontsize: isize) -> Self {
    //    Self::new(input, row, span, Some(AstNodeKind::Bold{isize}))
    //}
    //pub fn italic(input: &'a str, row: usize, span: Option<Span>) -> Self {
    //    Self::new(input, row, span, Some(AstNodeKind::Italic))
    //}
    //pub fn underline(input: &'a str, row: usize, span: Option<Span>) -> Self {
    //    Self::new(input, row, span, Some(AstNodeKind::Underline))
    //}
    //pub fn deleted(input: &'a str, row: usize, span: Option<Span>) -> Self {
    //    Self::new(input, row, span, Some(AstNodeKind::Deleted))
    //}
    pub fn tablecolumn(input: &'a str, row: usize, span: Option<Span>) -> Self {
        Self::new(input, row, span, Some(AstNodeKind::TableColumn))
    }

    pub fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
    pub fn value(&self) -> &AstNodeInternal<'a> {
        &self.0.value
    }
    pub fn add_content(&self, content: AstNode<'a>) {
        self.value().contents.borrow_mut().push(content);
    }
    pub fn add_contents(&self, contents: Vec<AstNode<'a>>) {
        self.value().contents.borrow_mut().extend(contents);
    }
    pub fn add_child(&self, child: AstNode<'a>) {
        self.value().children.borrow_mut().push(child);
    }
    pub fn location(&self) -> &Location<'a> {
        &self.0.location
    }
    pub fn extract_str(&self) -> &str {
        self.location().as_str()
    }
}

impl<'a> fmt::Display for AstNode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "extracted: {}\n", self.extract_str())?;
        for (i, content) in self.value().contents.borrow().iter().enumerate() {
            write!(f, "-- {}", content)?;
        }
        if let AstNodeKind::Line { properties } = &self.value().kind {
            for prop in properties {
                write!(f, "property -- {:?}\n", prop)?;
            }
        }
        for (i, child) in self.value().children.borrow().iter().enumerate() {
            write!(f, "\t{i}child -- {:?}\n", child)?;
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

fn find_parent_line<'b>(parent: AstNode<'b>, depth: usize) -> Option<AstNode<'b>> {
    if depth == 0 {
        return Some(parent);
    }
    let Some(last_child_line) = parent
        .value()
        .children
        .borrow()
        .iter()
        .filter_map(|e| match e.value().kind {
            AstNodeKind::Line { .. } => Some(e.clone()),
            _ => None,
        })
        .last()
    else {
        return None;
    };
    return find_parent_line(last_child_line, depth - 1);
}

// fn create_dummy_line<'a, 'b>(
//     parent: &'a mut AstNode<'b>,
//     depth: usize,
// ) -> Option<&'a mut AstNode<'b>> {
//     if depth == 0 {
//         return Some(&mut AstNode::line("", 0, None));
//     }
//     if let Some(ref mut last_child_line) = parent
//         .value
//         .children.borrow()
//         .iter()
//         .filter_map(|e| match e.value.kind {
//             AstNodeKind::Line { .. } => Some(e),
//             _ => None,
//         })
//         .last() {
//         return create_dummy_line(last_child_line, depth - 1);
//     } else {
//         let mut newline = AstNode::line("", 0, None);
//         let mut ret = create_dummy_line(&mut newline, depth-1);
//         parent.value.children.borrow_mut().push(newline);
//         return ret;
//     };
// }
#[derive(Error, Debug)]
pub enum ParserError<'a> {
    #[error("Invalid indent: {0}")]
    InvalidIndentation(Location<'a>),
    #[error("Invalid command parameter: {0}")]
    InvalidCommandParameter(String),
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
}

pub fn parse_text<'a>(text: &'a str) -> AstNode<'a> {
    let indent_content_len: Vec<_> = (&text).lines().map(|l| {
        let mut itr = l.chars();
        let indent = itr.by_ref().take_while(|&c| c == '\t').count();
        let content_len = itr.count();
        (indent, content_len)
    }).collect();
    let numlines = indent_content_len.len();

    let root = AstNode::new(&text, 0, None, Some(AstNodeKind::Dummy));

    let mut parsing_state: ParsingState = ParsingState::Line;
    let mut parsing_depth = 0;

    let mut errors: Vec<ParserError> = Vec::new();
    // for (iline, ((indent, content_len), linetext)) in
    //     indent_content_len.zip(text.lines()).enumerate()
    for (iline, linetext) in text.lines().enumerate() {
        let (indent, content_len) = indent_content_len[iline];
        // let depth = if (parsing_state != ParsingState::Line && indent > parsing_depth) {
        //     parsing_depth
        // } else {
        //     indent
        // };
        
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
                // this line is not in block, translating to line-parsing mode
                parsing_state = ParsingState::Line;
                parsing_depth = indent;
                depth = indent;
            }
        } else {
            log::debug!("content_len: {content_len}");
            if content_len == 0 {
                depth = parsing_depth;
            }
        }
        log::debug!("depth: {depth}, parsing_depth: {parsing_depth}");

        let parent: AstNode<'_> =
            find_parent_line(root.clone(), depth).unwrap_or_else(|| {
                log::warn!("Failed to find parent, depth {depth}");
                errors.push(ParserError::InvalidIndentation(Location {
                    input: &linetext,
                    row: iline,
                    span: Span(depth, depth + 1),
                }));
                //TODO create_dummy_line(&mut root, depth).unwrap()
                root.clone()
            });

        if parsing_state != ParsingState::Line {
            if parsing_state == ParsingState::Quote  {
                let quote = parent.value().contents.borrow().last().expect("no way! should be quote block").clone();
                match TabtonLineParser::parse(Rule::statement_nestable, &linetext[cmp::min(depth, indent)..]) {
                    Ok(mut parsed) => {
                        let (nodes, props) = transform_statement(
                            parsed.next().unwrap(),
                            linetext,
                            iline,
                            depth,
                        );
                        // TODO should be text rather than line?
                        let newline = AstNode::line(&linetext, iline, Some(Span(cmp::min(depth, indent), linetext.len())), props);
                        newline.add_contents(nodes);
                        quote.add_child(newline);
                    }
                    Err(e) => {
                        // TODO accumulate error
                        let newline = AstNode::line(&linetext, iline, None, None);
                        newline.add_content(AstNode::text(&linetext, iline, Some(Span(cmp::min(depth, indent), linetext.len()))));
                        quote.add_child(newline);
                    }
                }
                continue;
            } else if parsing_state == ParsingState::Code || parsing_state == ParsingState::Math {
                let block = parent.value().contents.borrow().last().expect("no way! should be code or math block").clone();
                let text = AstNode::text(&linetext, iline, Some(Span(cmp::min(depth, indent), linetext.len())));
                block.add_child(text);
                continue;
            } else {
                let table = parent.value().contents.borrow().last().expect("no way! should be table block").clone();
                todo!("table rows might have empty lines, do not start from `depth'");
                let columntexts = &linetext[depth..].split('\t');
                let span_starts = columntexts.to_owned().scan(depth, |cum, x| {*cum += x.len() + 1; Some(*cum)}/* +1 for seperator*/);
                let columns = columntexts.to_owned().zip(span_starts)
                    .map(|(t, c)| (TabtonLineParser::parse(Rule::statement_nestable, t), c))
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
                                let column = AstNode::tablecolumn(&linetext, iline, Some(span));
                                column.add_contents(nodes);
                                return column;
                            }
                            Err(e) => {
                                let span = Span(c, c + e.line().len());  // TODO is this correct span?
                                let column = AstNode::tablecolumn(&linetext, iline, Some(span.clone()));
                                column.add_content(AstNode::text(&linetext, iline, Some(span)));
                                return column;
                            }}
                    }).collect();
                let newline = AstNode::line(&linetext, iline, Some(Span(depth, linetext.len())), None);
                newline.add_contents(columns);
                table.add_child(newline);
                continue;
            }
        }

        // TODO gather parsing errors
        let (has_command, props) = parse_command_line(&linetext, 0, cmp::min(depth, indent));
        log::debug!("==============================");
        if let Some(command_node) = has_command {
            log::debug!("parsed command: {:?}", command_node.extract_str());
            match &command_node.value().kind {
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
            let newline = AstNode::line(&linetext, iline, None, Some(props));
            newline.add_content(command_node);
            parent.add_child(newline);
        } else {
            log::debug!("---- input ----");
            log::debug!("{}", &linetext[cmp::min(depth,indent)..]);
            // TODO error will never happen since raw_sentence will match finally(...?)
            match TabtonLineParser::parse(Rule::statement, &linetext[cmp::min(depth, indent)..]) {
                Ok(mut parsed) => {
                    log::debug!("---- parsed ----");
                    log::debug!("{:?}", parsed);
                    log::debug!("---- result ----");
                    let (nodes, props) = transform_statement(
                        parsed.next().unwrap(),
                        linetext,
                        iline,
                        cmp::min(depth, indent),
                    );
                    let newline = AstNode::line(&linetext, iline, None, props);
                    newline.add_contents(nodes);
                    log::debug!("{newline}");
                    parent.add_child(newline);
                }
                Err(e) => {
                    // TODO accumulate error
                    log::warn!("parsing statement error!: {}", e);
                    log::warn!("{:?}", e);
                    let newline = AstNode::line(&linetext, iline, None, None);
                    newline.add_content(AstNode::text(&linetext, iline, None));
                    parent.add_child(newline);
                }
            }
            // parsing_state = ParsingState::Line;
            parsing_depth = depth;
        }
    }
    root
}


fn parse_command_line(
    line: &str,
    row: usize,
    indent: usize,
) -> (Option<AstNode>, Vec<Property>) {
    let Ok(mut pairs) = TabtonLineParser::parse(Rule::expr_command_line, &line[indent..]) else {
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
) -> Option<AstNode<'a>> {
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
            return Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                Some(alt_img),
            ));
        }
        Rule::img_path_alt_opts => {
            let mut inner2 = inner.into_inner();
            let img_path = inner2.next().unwrap().into_inner().next().unwrap().as_str();
            let alt_img = inner2.next().unwrap().into_inner().next().unwrap().into_inner().next().unwrap().as_str();
            return Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                Some(alt_img),
            ));
        }
        Rule::img_path_opts => {
            let mut inner2 = inner.into_inner();
            let img_path = inner2.next().unwrap().into_inner().next().unwrap().as_str();
            return Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                None,
            ));
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

/// assuming input pair is url stuff
fn transform_url_link<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode<'a>> {
    let inner = pair.into_inner().next().unwrap();
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::expr_url_title => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            let title = inner2.next().unwrap();
            return Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(title.as_str()),
            ));
        }
        Rule::expr_title_url => {
            let mut inner2 = inner.into_inner();
            let title = inner2.next().unwrap();
            let url = inner2.next().unwrap();
            return Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(title.as_str()),
            ));
        }
        Rule::expr_url_only => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            return Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                None,
            ));
        }
        Rule::expr_url_url => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            let url2 = inner2.next().unwrap();
            return Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(url2.as_str()),
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
                log::warn!("Unknown property: {}", property_name);
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
                                log::warn!("Unknown task status: '{}', interpreted as 'todo'", value);
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
                            log::warn!("Unknown property value: {}", value);
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
            panic!("Unhandled token: {:?}", pair.as_rule());
        }
    }
}

fn parse_trailing_properties(s: &str) -> Option<Vec<Property>> {
    let Ok(mut trailing_properties) = TabtonLineParser::parse(Rule::trailing_properties, s) else {
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

fn transform_statement<'a, 'b>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> (Vec<AstNode<'a>>, Option<Vec<Property>>) {
    let mut nodes: Vec<AstNode<'a>> = vec![];
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
                let mut inner2 = inner.into_inner().into_iter();
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
                props.extend(
                    inner
                        .into_inner()
                        .into_iter()
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
    return (nodes, Some(props));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::pest::Parser;

    #[test]
    fn test_parse_code_command() {
        let input = "[@code rust]";
        // let parsed = TabtonLineParser::parse(Rule::expr_command, input);
        // assert!(parsed.is_ok(), "Failed to parse \"{input}\"");
        // let mut pairs = parsed.unwrap();
        // assert_eq!(pairs.len(), 1, "must contain only one expr_command");
        // let parsed_command = pairs.next().unwrap();
        // //                          \- the first pair, which is expr_command
        let (astnode, props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        match &node.value().kind {
            AstNodeKind::Code { lang, inline } => {
                assert_eq!(lang, "rust");
                assert_eq!(*inline, false);
            }
            _ => {
                panic! {"it is weird"};
            }
        }
    }

    #[test]
    fn test_parse_code_emtpy_lang() {
        let input = "[@code   ]";
        // let parsed = TabtonLineParser::parse(Rule::expr_command, input);
        // assert!(parsed.is_ok(), "Failed to parse \"{input}\"");
        // let mut pairs = parsed.unwrap();
        // assert_eq!(pairs.len(), 1, "must contain only one expr_command");
        // let parsed_command = pairs.next().unwrap();
        // //                          \- the first pair, which is expr_command
        let (astnode, props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse code command");
        };
        match &node.value().kind {
            AstNodeKind::Code { lang, inline } => {
                assert_eq!(lang, "");
                assert_eq!(*inline, false);
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
        assert_eq!(node.location().span, Span(indent, end_code + 1));
        match &node.value().kind {
            AstNodeKind::Code { lang, inline } => {
                assert_eq!(lang, "なでしこ");
                assert_eq!(*inline, false);
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
        assert_eq!(node.location().span, Span(indent, input.len()));
        match node.value().kind {
            AstNodeKind::Math { inline } => {
                assert_eq!(inline, false);
            }
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
    fn test_parse_code_inline_text_anchor() -> Result<(), Box<dyn std::error::Error>>{
        let input = "[` inline ![] code 123`] raw text #anchor";
        let mut parsed = TabtonLineParser::parse(Rule::statement, input)?;
        let (nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
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
                &raw_text.location().input
                    [raw_text.location().span.0..raw_text.location().span.1],
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
        Ok(())
    }

    #[test]
    fn test_parse_wiki_link() {
        let input = "[test wiki_page]";
        if let Ok(mut parsed) = TabtonLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match &wiki_link.value().kind {
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
        if let Ok(mut parsed) = TabtonLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match &wiki_link.value().kind {
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
    fn test_parse_img() -> Result<(), Box<dyn std::error::Error>>{
        for (input, g_path, g_alt) in vec![
            ("[@img \"img alt title\" https://gyazo.com/path/to/icon.png]", "https://gyazo.com/path/to/icon.png", Some("img alt title".to_string())),
            ("[@img https://gyazo.com/path/to/icon.png \"img alt title\"]", "https://gyazo.com/path/to/icon.png", Some("img alt title".to_string())),
            ("[@img https://gyazo.com/path/to/icon.png]", "https://gyazo.com/path/to/icon.png", None),
            (r##"[@img ./local/path/to/icon.png "img escaped \"alt title"]"##, "./local/path/to/icon.png", Some(r##"img escaped \"alt title"##.to_string()))] {
            match TabtonLineParser::parse(Rule::expr_img, input) {
                Ok(mut parsed) => {
                    let node = transform_img(parsed.next().unwrap(), input, 0, 0).ok_or("transform_img failed")?;
                    if let AstNodeKind::Image{src, alt} = &node.value().kind {
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
                match TabtonLineParser::parse(Rule::expr_url_link, input) {
                    Ok(mut parsed) => {
                        if let Some(link) = transform_url_link(parsed.next().unwrap(), input, 0, 0) {
                            match &link.value().kind {
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

    // #[test]
    // fn test_parse_error() {
    //     let err = TabtonLineParser::parse(Rule::expr_command, "[@  ] #anchor").unwrap_err();
    //     println!("{:?}", err);
    //     log::debug!("{:?}", err.variant.message());
    //     todo!();
    //     ()
    // }
}
