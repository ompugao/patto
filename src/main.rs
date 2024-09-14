use pest::Parser;
use std::cmp;
use std::fs;
use std::io::BufWriter;

mod parser;
mod renderer;
use crate::renderer::Renderer;
//use crate::parser;
//use std::io::{self, BufRead};

#[derive(Debug, PartialEq)]
enum ParsingState {
    Line,
    Quote,
    Code,
    Math,
    Table,
}

fn find_parent_line<'b>(parent: parser::AstNode<'b>, depth: usize) -> Option<parser::AstNode<'b>> {
    if depth == 0 {
        return Some(parent);
    }
    let Some(last_child_line) = parent
        .value()
        .children
        .borrow()
        .iter()
        .filter_map(|e| match e.value().kind {
            parser::AstNodeKind::Line { .. } => Some(e.clone()),
            _ => None,
        })
        .last()
    else {
        return None;
    };
    return find_parent_line(last_child_line, depth - 1);
}

// fn create_dummy_line<'a, 'b>(
//     parent: &'a mut parser::AstNode<'b>,
//     depth: usize,
// ) -> Option<&'a mut parser::AstNode<'b>> {
//     if depth == 0 {
//         return Some(&mut parser::AstNode::line("", 0, None));
//     }
//     if let Some(ref mut last_child_line) = parent
//         .value
//         .children.borrow()
//         .iter()
//         .filter_map(|e| match e.value.kind {
//             parser::AstNodeKind::Line { .. } => Some(e),
//             _ => None,
//         })
//         .last() {
//         return create_dummy_line(last_child_line, depth - 1);
//     } else {
//         let mut newline = parser::AstNode::line("", 0, None);
//         let mut ret = create_dummy_line(&mut newline, depth-1);
//         parent.value.children.borrow_mut().push(newline);
//         return ret;
//     };
// }

fn main() {
    // let unparsed_file = fs::read_to_string("./sample.ms").expect("cannot read sample.ms");
    // let parsed = parser::MarkshiftLineParser::parse(Rule::line, &unparsed_file.split("\n").next().unwrap())
    //    .unwrap_or_else(|e| panic!("{}", e));

    // TODO memory inefficient
    let filename = "./samples/sample2.ms";
    let text = fs::read_to_string(filename).expect("cannot read {filename}");
    let indent_content_len: Vec<_> = (&text).lines().map(|l| {
        let mut itr = l.chars();
        let indent = itr.by_ref().take_while(|&c| c == '\t').count();
        let content_len = itr.count();
        (indent, content_len)
    }).collect();
    let numlines = indent_content_len.len();

    let root = parser::AstNode::new(&text, 0, None, Some(parser::AstNodeKind::Dummy));

    let mut parsing_state: ParsingState = ParsingState::Line;
    let mut parsing_depth = 0;

    let mut errors: Vec<parser::ParserError> = Vec::new();
    // for (iline, ((indent, content_len), linetext)) in
    //     indent_content_len.zip(text.lines()).enumerate()
    for (iline, linetext) in text.lines().enumerate()
    {
        let (indent, content_len) = indent_content_len[iline];
        // let depth = if (parsing_state != ParsingState::Line && indent > parsing_depth) {
        //     parsing_depth
        // } else {
        //     indent
        // };
        
        // TODO can be O(n^2) where n = numlines
        let mut depth = indent;
        if parsing_state != ParsingState::Line {
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
                depth = parsing_depth;
            } else {
                parsing_state = ParsingState::Line;
                parsing_depth = indent;
                depth = indent;
            }
        }
        let parent: parser::AstNode<'_> =
            find_parent_line(root.clone(), depth).unwrap_or_else(|| {
                println!("Failed to find parent");
                errors.push(parser::ParserError::InvalidIndentation(parser::Location {
                    input: &linetext,
                    row: iline,
                    span: parser::Span(depth, depth + 1),
                }));
                //TODO create_dummy_line(&mut root, depth).unwrap()
                root.clone()
            });

        if parsing_state != ParsingState::Line && depth == parsing_depth {
            if parsing_state == ParsingState::Quote  {
                let quote = parent.value().contents.borrow().last().expect("no way! should be quote block").clone();
                match parser::MarkshiftLineParser::parse(parser::Rule::statement_nestable, &linetext[depth..]) {
                    Ok(mut parsed) => {
                        let (nodes, props) = parser::transform_statement(
                            parsed.next().unwrap(),
                            linetext,
                            iline,
                            depth,
                        );
                        // TODO should be text rather than line?
                        let newline = parser::AstNode::line(&linetext, iline, Some(parser::Span(cmp::min(depth, linetext.len()), linetext.len())), props);
                        newline.add_contents(nodes);
                        quote.add_child(newline);
                    }
                    Err(e) => {
                        // TODO accumulate error
                        let newline = parser::AstNode::line(&linetext, iline, None, None);
                        newline.add_content(parser::AstNode::text(&linetext, iline, Some(parser::Span(cmp::min(depth, linetext.len()), linetext.len()))));
                        quote.add_child(newline);
                    }
                }
                continue;
            } else if parsing_state == ParsingState::Code || parsing_state == ParsingState::Math {
                let block = parent.value().contents.borrow().last().expect("no way! should be code or math block").clone();
                let text = parser::AstNode::text(&linetext, iline, Some(parser::Span(cmp::min(depth, linetext.len()), linetext.len())));
                block.add_child(text);
                continue;
            } else {
                let table = parent.value().contents.borrow().last().expect("no way! should be table block").clone();
                todo!("table rows might have empty lines, do not start from `depth'");
                let columntexts = &linetext[depth..].split('\t');
                let span_starts = columntexts.to_owned().scan(depth, |cum, x| {*cum += x.len() + 1; Some(*cum)}/* +1 for seperator*/);
                let columns = columntexts.to_owned().zip(span_starts)
                    .map(|(t, c)| (parser::MarkshiftLineParser::parse(parser::Rule::statement_nestable, t), c))
                    .map(|(ret, c)| {
                        match ret {
                            Ok(mut parsed) => {
                                let inner = parsed.next().unwrap();
                                let span = Into::<parser::Span>::into(inner.as_span()) + c;
                                let (nodes, _) = parser::transform_statement(
                                    inner,
                                    linetext,
                                    iline,
                                    depth,
                                );
                                let column = parser::AstNode::tablecolumn(&linetext, iline, Some(span));
                                column.add_contents(nodes);
                                return column;
                            }
                            Err(e) => {
                                let span = parser::Span(c, c + e.line().len());  // TODO is this correct span?
                                let column = parser::AstNode::tablecolumn(&linetext, iline, Some(span.clone()));
                                column.add_content(parser::AstNode::text(&linetext, iline, Some(span)));
                                return column;
                            }}
                    }).collect();
                let newline = parser::AstNode::line(&linetext, iline, Some(parser::Span(depth, linetext.len())), None);
                newline.add_contents(columns);
                table.add_child(newline);
                continue;
            }
        }

        // TODO gather parsing errors
        let (has_command, props) = parser::parse_command_line(&linetext, 0, depth);
        println!("==============================");
        if let Some(command_node) = has_command {
            println!("parsed command: {:?}", command_node.extract_str());
            match &command_node.value().kind {
                parser::AstNodeKind::Quote => {
                    parsing_state = ParsingState::Quote;
                    parsing_depth = depth + 1;
                }
                parser::AstNodeKind::Code{..} => {
                    parsing_state = ParsingState::Code;
                    parsing_depth = depth + 1;
                }
                parser::AstNodeKind::Math => {
                    parsing_state = ParsingState::Math;
                    parsing_depth = depth + 1;
                }
                parser::AstNodeKind::Table => {
                    parsing_state = ParsingState::Table;
                    parsing_depth = depth + 1;
                }
                _ => {
                    parsing_state = ParsingState::Line;
                    parsing_depth = depth;
                }
            }
            let newline = parser::AstNode::line(&linetext, iline, None, Some(props));
            newline.add_content(command_node);
            parent.add_child(newline);
        } else {
            println!("---- input ----");
            println!("depth: {depth}");
            println!("{}", &linetext[depth..]);
            // TODO error will never happen since raw_sentence will match finally(...?)
            match parser::MarkshiftLineParser::parse(parser::Rule::statement, &linetext[depth..]) {
                Ok(mut parsed) => {
                    println!("---- parsed ----");
                    println!("{:?}", parsed);
                    println!("---- result ----");
                    let (nodes, props) = parser::transform_statement(
                        parsed.next().unwrap(),
                        linetext,
                        iline,
                        depth,
                    );
                    let newline = parser::AstNode::line(&linetext, iline, None, props);
                    newline.add_contents(nodes);
                    println!("{newline}");
                    parent.add_child(newline);
                }
                Err(e) => {
                    // TODO accumulate error
                    println!("{}", e);
                    let newline = parser::AstNode::line(&linetext, iline, None, None);
                    newline.add_content(parser::AstNode::text(&linetext, iline, None));
                    parent.add_child(newline);
                }
            }
        }
    }
    let mut writer = BufWriter::new(fs::File::create("output.html").unwrap());
    let renderer = renderer::HtmlRenderer::new(renderer::Options::default());
    renderer.format(&root, &mut writer).unwrap();
    //println!("{:?}", parsed);
    //println!("parsed result:");
    //for pair in parsed.into_inner() {
    //    parse_value
    //}
}
