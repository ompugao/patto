use pest::iterators::Pair;
use pest::Parser;

use super::property::transform_property;
use super::{AstNode, PattoLineParser, Property, Rule, Span};

pub(super) fn parse_command_line(
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
            if let Some(prop) = transform_property(pair, line, row, indent) {
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
                Rule::command_table => {
                    // Parse parameters for table command
                    let mut caption: Option<String> = None;

                    for param in inner {
                        if param.as_rule() == Rule::parameter {
                            let param_str = param.as_str();

                            // Check if this is a key=value parameter
                            if let Some(eq_pos) = param_str.find('=') {
                                let key = &param_str[..eq_pos];
                                let value = &param_str[eq_pos + 1..];

                                if key == "caption" {
                                    // Handle quoted strings by removing quotes
                                    if value.starts_with('"') && value.ends_with('"') {
                                        caption = Some(value[1..value.len() - 1].to_string());
                                    } else {
                                        caption = Some(value.to_string());
                                    }
                                }
                            } else {
                                // Handle quoted parameter as caption (for backward compatibility)
                                if param_str.starts_with('"') && param_str.ends_with('"') {
                                    caption = Some(param_str[1..param_str.len() - 1].to_string());
                                } else {
                                    caption = Some(param_str.to_string());
                                }
                            }
                        }
                    }

                    return Some(AstNode::table(line, row, Some(span), caption.as_deref()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{AstNodeKind, Deadline, TaskStatus};

    #[test]
    fn test_parse_code_command() {
        let input = "[@code rust]";
        // assert!(parsed.is_ok(), "Failed to parse \"{input}\"");
        // assert_eq!(pairs.len(), 1, "must contain only one expr_command");
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
        // assert!(parsed.is_ok(), "Failed to parse \"{input}\"");
        // assert_eq!(pairs.len(), 1, "must contain only one expr_command");
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
                Property::Task { status, due, .. } => {
                    let TaskStatus::Todo = status else {
                        panic!("task is not in todo state!");
                    };
                    if let Deadline::Date(date) = due {
                        assert_eq!(date, chrono::NaiveDate::from_ymd_opt(2024, 9, 24).unwrap());
                    } else {
                        panic!("date is not correctly parsed");
                    }
                }
                Property::Anchor { name, .. } => {
                    assert_eq!(name, "anchor1");
                }
            }
        }
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
    fn test_parse_table() {
        let input = "[@table caption=\"test caption\"]";
        let indent = input.chars().take_while(|&c| c == '\t').count();
        let (astnode, _props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse table command");
        };
        println!("{:?}", node);
        assert_eq!(node.location().span, Span(indent, input.len()));
        match node.kind() {
            AstNodeKind::Table { ref caption } => {
                if let Some(caption) = caption {
                    assert_eq!(caption, "test caption");
                } else {
                    panic! {"caption not parsed"};
                }
            }
            _ => {
                panic! {"Math command could not be parsed"};
            }
        }
    }

    #[test]
    fn test_parse_table2() {
        let input = "[@table \"test caption\"]";
        let indent = input.chars().take_while(|&c| c == '\t').count();
        let (astnode, _props) = parse_command_line(input, 0, 0);
        let Some(node) = astnode else {
            panic!("Failed to parse table command");
        };
        println!("{:?}", node);
        assert_eq!(node.location().span, Span(indent, input.len()));
        match node.kind() {
            AstNodeKind::Table { ref caption } => {
                if let Some(caption) = caption {
                    assert_eq!(caption, "test caption");
                } else {
                    panic! {"caption not parsed"};
                }
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
}
