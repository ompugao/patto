use pest::Parser;
use std::fs;
mod parser;
//use crate::parser;
//use std::io::{self, BufRead};

#[derive(Debug, PartialEq)]
enum ParsingState {
    Line,
    Command,
}

fn find_parent_line<'a, 'b>(
    parent: &'a parser::AstNode<'b>,
    depth: usize,
) -> Option<&'a parser::AstNode<'b>> {
    if depth == 0 {
        return Some(parent);
    }
    let Some(ref last_child_line) = parent
        .value
        .children
        .iter()
        .filter_map(|e| match e.value.kind {
            parser::AstNodeKind::Line { .. } => Some(e),
            _ => None,
        })
        .last()
    else {
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

    let root = parser::AstNode::new(&text, 0, None, Some(parser::AstNodeKind::Dummy));

    let mut parsing_state: ParsingState = ParsingState::Line;
    let mut parsing_depth = 0;

    let mut errors: Vec<parser::ParserError> = Vec::new();
    for (iline, ((indent, content_len), linetext)) in indent_content_len.zip(text.lines()).enumerate() {
        let mut depth = indent;
        if (parsing_state == ParsingState::Line && indent > parsing_depth) || content_len == 0 {
            depth = parsing_depth;
        }
        let parent: &parser::AstNode = find_parent_line(&root, depth).unwrap_or_else(|| {
            // errors.push(parser::ParserError::InvalidIndentation(
            //     parser::Annotation {
            //         value: &linetext,
            //         location: parser::Location {
            //             input: &linetext,
            //             row: iline,
            //             span: parser::Span(indent, indent+1)
            //         },
            //     },
            // ));
            &root //TODO create dummy node(s) to fit the current depth
        });
        let mut newline = parser::AstNode::line(&linetext, iline, None);
        // TODO gather parsing errors
        let (has_command, props) = parser::parse_command_line(&linetext, 0, indent);
        println!("==============================");
        if let Some(command_node) = has_command {
            println!("parsed command: {:?}", command_node.extract_str());
        } else {
            println!("---- input ----");
            println!("{}", &linetext[indent..]);
            // TODO error will never happen since raw_sentence will match finally(...?)
            match parser::MarkshiftLineParser::parse(parser::Rule::statement, &linetext[indent..]) {
                Ok(mut parsed) => {
                    println!("---- parsed ----");
                    println!("{:?}", parsed);
                    match parser::transform_statement(parsed.next().unwrap(), linetext, iline, indent) {
                        Ok((mut nodes, None)) => {
                            // elements in nodes are moved and the nodes will become empty. therefore,
                            // mut is required.
                            newline.value.content.append(&mut nodes);
                        }
                        Ok((mut nodes, Some(mut props))) => {
                            newline.value.content.append(&mut nodes);
                            match newline.value.kind {
                                parser::AstNodeKind::Line { ref mut properties } => {
                                    properties.append(&mut props);
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            println!("{}", e);
                        }
                    }
                    println!("---- result ----");
                    println!("{newline}");
                }
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
    }
    //println!("{:?}", parsed);
    //println!("parsed result:");
    //for pair in parsed.into_inner() {
    //    parse_value
    //}
}
