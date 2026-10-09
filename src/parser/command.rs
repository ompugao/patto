use pest::iterators::{Pair, Pairs};
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
    let mut parts = pairs.next().unwrap().into_inner();
    let command = transform_command(parts.next().unwrap(), line, row, indent);
    let properties = parts
        .next()
        .map(|trailing| {
            trailing
                .into_inner()
                .filter_map(|prop| transform_property(prop, line, row, indent))
                .collect()
        })
        .unwrap_or_default();
    (command, properties)
}

fn transform_command<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    if pair.as_rule() != Rule::expr_command {
        log::warn!(
            "Do you provide other than expr_command to fn transform_command: {:?}",
            pair.as_rule()
        );
        return None;
    }
    let span = Span::from(pair.as_span()) + indent;
    let mut inner = pair.into_inner();
    let command = inner.next().unwrap().into_inner().next().unwrap();
    match command.as_rule() {
        Rule::command_math => Some(AstNode::math(line, row, Some(span), false)),
        Rule::command_quote => Some(AstNode::quote(line, row, Some(span))),
        Rule::command_code => Some(AstNode::code(
            line,
            row,
            Some(span),
            code_language(inner),
            false,
        )),
        Rule::command_table => Some(AstNode::table(
            line,
            row,
            Some(span),
            table_caption(inner).as_deref(),
        )),
        other => {
            log::warn!("Unhandled builtin command: {:?}", other);
            None
        }
    }
}

fn code_language<'a>(mut params: Pairs<'a, Rule>) -> &'a str {
    match params.next() {
        Some(lang) => lang.as_str(),
        None => {
            log::warn!("No language specified for code block");
            ""
        }
    }
}

/// A bare parameter is still read as the caption because older notes wrote
/// `[@table "Caption"]` before the `caption=` form existed.
fn table_caption(params: Pairs<Rule>) -> Option<String> {
    let mut caption = None;
    for param in params.filter(|param| param.as_rule() == Rule::parameter) {
        let text = param.as_str();
        match text.split_once('=') {
            Some(("caption", value)) => caption = Some(unquote(value).to_string()),
            Some(_) => {}
            None => caption = Some(unquote(text).to_string()),
        }
    }
    caption
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{AstNodeKind, Deadline, TaskStatus};

    #[test]
    fn test_parse_code_command() {
        let input = "[@code rust]";
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
