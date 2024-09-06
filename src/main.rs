use pest::Parser;
use pest_derive::Parser;
use std::fs;
use std::fmt;
use thiserror::Error;
//use std::io::{self, BufRead};

#[derive(Parser)]
#[grammar = "markshift.pest"]
pub struct MarkshiftLineParser;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct Location<'a> {
    input: &'a str,
    start: usize,
    end: usize, //exclusive
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
struct Annotation<'a, T> {
    value: T,
    location: Location<'a>,
}

impl<T> fmt::Display for Annotation<'_, T> where T: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n", self.value)?;
        write!(f, "{: <1$}", "", self.location.start)?;
        write!(f, "{:^<1$}", "", self.location.end - self.location.start)
    }
}

#[derive(Debug, Default)]
struct AstNodeInternal<'a> {
    content: Vec<AstNode<'a>>,
    children: Vec<AstNode<'a>>,
    kind: AstNodeKind,
    text: &'a str,
}

#[derive(Debug, Default, PartialEq)]
enum AstNodeKind {
    Line,
    Quote,
    Math,
    Code { lang: String },
    //Table,
    Image { src: String, alt: String },
    Text,
    #[default]
    Dummy,
}

type AstNode<'a> = Annotation<'a, AstNodeInternal<'a>>;

impl<'a> AstNode<'a> {
    fn new(input: &'a str, kind: Option<AstNodeKind>) -> Self {
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
    fn code(input: &'a str, lang: &'a str) -> Self {
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
    fn math(input: &'a str) -> Self {
        Self::new(input, Some(AstNodeKind::Math))
    }
    fn quote(input: &'a str) -> Self {
        Self::new(input, Some(AstNodeKind::Quote))
    }
    fn text(input: &'a str) -> Self {
        Self::new(input, Some(AstNodeKind::Text))
    }
}

#[derive(Error, Debug)]
enum ParserError<'a> {
    #[error("Invalid indent: {0}")]
    InvalidIndentation(Annotation<'a, &'a str>),
    #[error("Invalid command parameter: {0}")]
    InvalidCommandParameter(String),
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
}

fn find_parent_line<'a, 'b>(parent: &'a AstNode<'b>, depth: usize) -> Option<&'a AstNode<'b>> {
    if depth == 0 {
        return Some(parent);
    }
    let Some(ref last_child_line) = parent.value.children.iter().filter_map(|e| match e.value.kind {
        AstNodeKind::Line => Some(e),
        _ => None,
    }).last() else {
        return None;
    };
    return find_parent_line(last_child_line, depth - 1);
}

fn main() {
    // let unparsed_file = fs::read_to_string("./sample.ms").expect("cannot read sample.ms");
    // let parsed = MarkshiftLineParser::parse(Rule::line, &unparsed_file.split("\n").next().unwrap())
    //    .unwrap_or_else(|e| panic!("{}", e));

    use pest::iterators::Pair;
    fn transform_pair(pair: Pair<Rule>) -> AstNode {
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
                unreachable!()
            }
            Rule::statement => {
                transform_pair(pair.into_inner().next().unwrap())
                // TODO handle all inners
                //for pair in pair.into_inner() {
            }
            Rule::raw_sentence => {
                AstNode::text(pair.as_str())
            }
            _ => unreachable!(),
        }
    }

    // TODO memory inefficient
    let filename = "./samples/sample2.ms";
    let text = fs::read_to_string(filename).expect("cannot read {filename}");
    let indent_content_len = (&text).lines().map(|l| {
        let mut itr = l.chars();
        let indent = itr.by_ref().take_while(|&c| c == '\t').count();
        let content_len = itr.count();
        (indent, content_len)
    });

    let root = AstNode::new(&text, Some(AstNodeKind::Dummy));
    //let parent :&AstNode = &root;

    let mut parsing_state = AstNodeKind::Line;
    let mut parsing_depth = 0;

    let mut errors: Vec<ParserError> = Vec::new();
    for (iline, ((indent, content_len), line)) in indent_content_len.zip(text.lines()).enumerate() {
        let mut depth = indent;
        if (parsing_state == AstNodeKind::Line && indent > parsing_depth) || content_len == 0 {
            depth = parsing_depth;
        }
        let parent :&AstNode = find_parent_line(&root, depth).unwrap_or_else(|| {
            errors.push(ParserError::InvalidIndentation(Annotation { value: &line, location: Location {input: &line, start: indent, end: indent+1} }));
            &root //TODO create dummy node(s) to fit the current depth
        });
        let mut newline = AstNode::new(&line, Some(AstNodeKind::Line));
        // TODO gather parsing errors
        let parsed = MarkshiftLineParser::parse(Rule::expr_command, line.trim_start_matches('\t'))
            .unwrap_or_else(|e| panic!("{}", e));
        for pair in parsed {
            println!("{:?}", transform_pair(pair));
        }
    }
    //println!("{:?}", parsed);
    //println!("parsed result:");
    //for pair in parsed.into_inner() {
    //    parse_value
    //}

}
