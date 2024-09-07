use std::fs;
use pest::Parser;
mod parser;
//use crate::parser;
//use std::io::{self, BufRead};

#[derive(Debug, PartialEq)]
enum ParsingState {
    Line,
    Command,
}

fn find_parent_line<'a, 'b>(parent: &'a parser::AstNode<'b>, depth: usize) -> Option<&'a parser::AstNode<'b>> {
    if depth == 0 {
        return Some(parent);
    }
    let Some(ref last_child_line) = parent.value.children.iter().filter_map(|e| match e.value.kind {
        parser::AstNodeKind::Line{..} => Some(e),
        _ => None,
    }).last() else {
        return None;
    };
    return find_parent_line(last_child_line, depth - 1);
}

fn main() {
    // let unparsed_file = fs::read_to_string("./sample.ms").expect("cannot read sample.ms");
    // let parsed = parser::MarkshiftLineParser::parse(Rule::line, &unparsed_file.split("\n").next().unwrap())
    //    .unwrap_or_else(|e| panic!("{}", e));

    // TODO memory inefficient
    let filename = "./samples/sample2.ms";
    let text = fs::read_to_string(filename).expect("cannot read {filename}");
    let indent_content_len = (&text).lines().map(|l| {
        let mut itr = l.chars();
        let indent = itr.by_ref().take_while(|&c| c == '\t').count();
        let content_len = itr.count();
        (indent, content_len)
    });

    let root = parser::AstNode::new(&text, Some(parser::AstNodeKind::Dummy));

    let mut parsing_state: ParsingState = ParsingState::Line;
    let mut parsing_depth = 0;

    let mut errors: Vec<parser::ParserError> = Vec::new();
    for (iline, ((indent, content_len), line)) in indent_content_len.zip(text.lines()).enumerate() {
        let mut depth = indent;
        if (parsing_state == ParsingState::Line && indent > parsing_depth) || content_len == 0 {
            depth = parsing_depth;
        }
        let parent :&parser::AstNode = find_parent_line(&root, depth).unwrap_or_else(|| {
            errors.push(parser::ParserError::InvalidIndentation(parser::Annotation { value: &line, location: parser::Location {input: &line, start: indent, end: indent+1} }));
            &root //TODO create dummy node(s) to fit the current depth
        });
        let mut newline = parser::AstNode::line(&line);
        // TODO gather parsing errors
        if let Ok(parsed) = parser::MarkshiftLineParser::parse(parser::Rule::expr_command, line.trim_start_matches('\t')) {
            for pair in parsed {
                println!("command parsed! {:?}", parser::transform_command(pair));
            }
        } else {
            // TODO error will never happen since raw_sentence will match finally(...?)
            let parsed = parser::MarkshiftLineParser::parse(parser::Rule::statement, line.trim_start_matches('\t')).unwrap();
            for node in parsed.map(|pair| parser::transform_statement(pair, &mut newline)) {
                println!("{:?}", node);
            }
            println!("{newline:?}");
        }
    }
    //println!("{:?}", parsed);
    //println!("parsed result:");
    //for pair in parsed.into_inner() {
    //    parse_value
    //}

}
