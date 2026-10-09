use pest::iterators::Pair;

use super::property::transform_property;
use super::{AstNode, Property, Rule, Span};

fn transform_img<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let span = Into::<Span>::into(pair.as_span()) + indent;
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::img_alt_path_opts => {
            let mut inner2 = inner.into_inner();
            let alt_img = inner2
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str();
            let img_path = inner2.next().unwrap().into_inner().next().unwrap().as_str();
            // inner2.chunks(2).map(|(k,v)| {
            Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                Some(alt_img),
            ))
        }
        Rule::img_path_alt_opts => {
            let mut inner2 = inner.into_inner();
            let img_path = inner2.next().unwrap().into_inner().next().unwrap().as_str();
            let alt_img = inner2
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap()
                .as_str();
            Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                Some(alt_img),
            ))
        }
        Rule::img_unquoted_alt_path_opts => {
            let mut inner2 = inner.into_inner();
            let alt_img = inner2.next().unwrap().as_str(); // url_title
            let img_path = inner2.next().unwrap().as_str(); // local_file
            Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                Some(alt_img),
            ))
        }
        Rule::img_path_unquoted_alt_opts => {
            let mut inner2 = inner.into_inner();
            let img_path = inner2.next().unwrap().into_inner().next().unwrap().as_str();
            let alt_img = inner2.next().unwrap().as_str(); // url_title
            Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                Some(alt_img),
            ))
        }
        Rule::img_unquoted_alt_url_opts => {
            let mut inner2 = inner.into_inner();
            let alt_img = inner2.next().unwrap().as_str(); // url_title
            let img_path = inner2.next().unwrap().as_str(); // URL
            Some(AstNode::image(
                line,
                row,
                Some(span),
                img_path,
                Some(alt_img),
            ))
        }
        Rule::img_path_opts => {
            let mut inner2 = inner.into_inner();
            let img_path = inner2.next().unwrap().into_inner().next().unwrap().as_str();
            Some(AstNode::image(line, row, Some(span), img_path, None))
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
) -> Option<AstNode> {
    let span = Into::<Span>::into(pair.as_span()) + indent;
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::wiki_link_anchored => {
            let mut inner2 = inner.into_inner();
            let wiki_link = inner2.next().unwrap();
            let expr_anchor = inner2.next().unwrap();
            Some(AstNode::wikilink(
                line,
                row,
                Some(span),
                wiki_link.as_str(),
                Some(expr_anchor.into_inner().next().unwrap().as_str()),
            ))
        }
        Rule::wiki_link => Some(AstNode::wikilink(
            line,
            row,
            Some(span),
            inner.as_str(),
            None,
        )),
        Rule::self_link_anchored => Some(AstNode::wikilink(
            line,
            row,
            Some(span),
            "",
            Some(
                inner
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            ),
        )),
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
) -> Option<AstNode> {
    let inner = pair.into_inner().next().unwrap();
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::expr_url_title => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            let title = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_title_url => {
            let mut inner2 = inner.into_inner();
            let title = inner2.next().unwrap();
            let url = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_url_only => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            Some(AstNode::link(line, row, Some(span), url.as_str(), None))
        }
        Rule::expr_url_url => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            let url2 = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(url2.as_str()),
            ))
        }
        _ => {
            unreachable!();
        }
    }
}

fn transform_local_file_link<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let inner = pair.into_inner().next().unwrap();
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::expr_local_file_title => {
            let mut inner2 = inner.into_inner();
            let local_file = inner2.next().unwrap();
            let title = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                local_file.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_title_local_file => {
            let mut inner2 = inner.into_inner();
            let title = inner2.next().unwrap();
            let local_file = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                local_file.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_local_file_only => {
            let mut inner2 = inner.into_inner();
            let local_file = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                local_file.as_str(),
                None,
            ))
        }
        _ => {
            unreachable!();
        }
    }
}

fn transform_mail_link<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let inner = pair.into_inner().next().unwrap();
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::expr_mail_title => {
            let mut inner2 = inner.into_inner();
            let mail = inner2.next().unwrap();
            let title = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                mail.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_title_mail => {
            let mut inner2 = inner.into_inner();
            let title = inner2.next().unwrap();
            let mail = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                mail.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::expr_mail_only => {
            let mut inner2 = inner.into_inner();
            let mail = inner2.next().unwrap();
            Some(AstNode::link(line, row, Some(span), mail.as_str(), None))
        }
        Rule::expr_mail_mail => {
            let mut inner2 = inner.into_inner();
            let mail = inner2.next().unwrap();
            let mail2 = inner2.next().unwrap();
            Some(AstNode::link(
                line,
                row,
                Some(span),
                mail.as_str(),
                Some(mail2.as_str()),
            ))
        }
        _ => {
            unreachable!();
        }
    }
}

fn transform_embed<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> Option<AstNode> {
    let inner = pair.into_inner().next().unwrap();
    let span = Into::<Span>::into(inner.as_span()) + indent;
    match inner.as_rule() {
        Rule::embed_url_title => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            let title = inner2.next().unwrap();
            Some(AstNode::embed(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::embed_title_url => {
            let mut inner2 = inner.into_inner();
            let title = inner2.next().unwrap();
            let url = inner2.next().unwrap();
            Some(AstNode::embed(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::embed_url_url => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            let url2 = inner2.next().unwrap();
            Some(AstNode::embed(
                line,
                row,
                Some(span),
                url.as_str(),
                Some(url2.as_str()),
            ))
        }
        Rule::embed_url_only => {
            let mut inner2 = inner.into_inner();
            let url = inner2.next().unwrap();
            Some(AstNode::embed(line, row, Some(span), url.as_str(), None))
        }
        Rule::embed_local_only => {
            let mut inner2 = inner.into_inner();
            let path = inner2.next().unwrap();
            Some(AstNode::embed(line, row, Some(span), path.as_str(), None))
        }
        Rule::embed_title_local => {
            let mut inner2 = inner.into_inner();
            let title = inner2.next().unwrap(); // url_title
            let path = inner2.next().unwrap(); // local_file
            Some(AstNode::embed(
                line,
                row,
                Some(span),
                path.as_str(),
                Some(title.as_str()),
            ))
        }
        Rule::embed_local_title => {
            let mut inner2 = inner.into_inner();
            let path = inner2.next().unwrap();
            let title = inner2.next().unwrap();
            Some(AstNode::embed(
                line,
                row,
                Some(span),
                path.as_str(),
                Some(title.as_str()),
            ))
        }
        _ => {
            unreachable!();
        }
    }
}

pub(super) fn transform_statement<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> (Vec<AstNode>, Vec<Property>) {
    let mut nodes: Vec<AstNode> = vec![];
    let mut props: Vec<Property> = vec![];

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::expr_img => {
                if let Some(node) = transform_img(inner, line, row, indent) {
                    nodes.push(node);
                }
            }
            Rule::expr_embed => {
                if let Some(node) = transform_embed(inner, line, row, indent) {
                    nodes.push(node);
                }
            }
            Rule::expr_builtin_symbols => {
                let s = Some(Into::<Span>::into(inner.as_span()) + indent);
                let mut inner2 = inner.into_inner();
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
            Rule::expr_local_file_link => {
                if let Some(node) = transform_local_file_link(inner, line, row, indent) {
                    nodes.push(node);
                }
            }
            Rule::expr_mail_link => {
                if let Some(node) = transform_mail_link(inner, line, row, indent) {
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
            Rule::expr_math_inline => {
                let math = AstNode::math(
                    line,
                    row,
                    Some(Into::<Span>::into(inner.as_span()) + indent),
                    true,
                );
                let math_inline = inner.into_inner().next().unwrap();
                math.add_content(AstNode::text(
                    line,
                    row,
                    Some(Into::<Span>::into(math_inline.as_span()) + indent),
                ));
                nodes.push(math);
            }
            Rule::expr_property => {
                if let Some(prop) = transform_property(inner, line, row, indent) {
                    props.push(prop);
                }
            }
            Rule::expr_anchor => {
                //nodes.push(AstNode::text(line, row, Some(Into::<Span>::into(inner.as_span()) + indent)));
                if let Some(prop) = transform_property(inner, line, row, indent) {
                    props.push(prop);
                }
            }
            Rule::expr_task => {
                if let Some(prop) = transform_property(inner, line, row, indent) {
                    props.push(prop);
                }
            }
            Rule::raw_sentence => {
                nodes.push(AstNode::text(
                    line,
                    row,
                    Some(Into::<Span>::into(inner.as_span()) + indent),
                ));
            }
            Rule::expr_hr => {
                nodes.push(AstNode::horizontal_line(
                    line,
                    row,
                    Some(Into::<Span>::into(inner.as_span()) + indent),
                ));
            }
            Rule::trailing_properties => {
                props.extend(
                    inner
                        .into_inner()
                        .filter_map(|e| transform_property(e, line, row, indent)),
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
    (nodes, props)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{AstNodeKind, Deadline, PattoLineParser, TaskStatus};
    use ::pest::Parser;

    #[test]
    fn test_parse_trailing_properties() -> Result<(), Box<dyn std::error::Error>> {
        let input = "   #anchor1 {@task status=todo due=2024-09-24} #anchor2";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (_nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        let anchor1 = &props[0];
        if let Property::Anchor { name, .. } = anchor1 {
            assert_eq!(name, "anchor1");
        } else {
            panic!("anchor1 is not extracted properly");
        };

        let task = &props[1];
        if let Property::Task { status, due, .. } = task {
            let TaskStatus::Todo = status else {
                panic!("task is not in todo state!");
            };
            assert_eq!(
                due,
                &Deadline::Date(chrono::NaiveDate::from_ymd_opt(2024, 9, 24).unwrap())
            );
        } else {
            panic!("anchor1 is not extracted properly");
        };

        let anchor2 = &props[2];
        if let Property::Anchor { name, .. } = anchor2 {
            assert_eq!(name, "anchor2");
        } else {
            panic!("anchor2 is not extracted properly");
        };

        Ok(())
    }

    #[test]
    fn test_parse_anchor_long_form() -> Result<(), Box<dyn std::error::Error>> {
        let input = "{@anchor myanchor}";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (_nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);

        assert_eq!(props.len(), 1, "Should have one anchor property");
        if let Property::Anchor {
            ref name,
            ref location,
        } = props[0]
        {
            assert_eq!(name, "myanchor");
            // The location should cover the entire {@anchor myanchor} span
            assert_eq!(location.span.0, 0);
            assert_eq!(location.span.1, input.len());
        } else {
            panic!("Expected anchor property");
        }
        Ok(())
    }

    #[test]
    fn test_parse_anchor_long_form_trailing() -> Result<(), Box<dyn std::error::Error>> {
        let input = "Some text {@anchor section1}";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);

        // Should have text node and anchor property
        assert_eq!(nodes.len(), 1, "Should have one text node");
        assert_eq!(props.len(), 1, "Should have one anchor property");

        if let Property::Anchor { ref name, .. } = props[0] {
            assert_eq!(name, "section1");
        } else {
            panic!("Expected anchor property");
        }
        Ok(())
    }

    #[test]
    fn test_parse_anchor_both_forms() -> Result<(), Box<dyn std::error::Error>> {
        // Test that both short and long forms work in trailing position
        let input = "Text #short {@anchor long1}";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (_nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);

        assert_eq!(props.len(), 2, "Should have two anchor properties");

        if let Property::Anchor { ref name, .. } = props[0] {
            assert_eq!(name, "short");
        } else {
            panic!("Expected short anchor property");
        }

        if let Property::Anchor { ref name, .. } = props[1] {
            assert_eq!(name, "long1");
        } else {
            panic!("Expected long anchor property");
        }
        Ok(())
    }

    #[test]
    fn test_parse_math_inline() -> Result<(), Box<dyn std::error::Error>> {
        let input = "[$ math = a * b * c$]";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (nodes, _props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        let math = &nodes[0];
        if let AstNodeKind::Math { ref inline } = math.kind() {
            assert!(*inline);
        } else {
            panic! {"Inline math could not be parsed"};
        }
        assert_eq!(math.contents()[0].extract_str(), "math = a * b * c");
        Ok(())
    }

    #[test]
    fn test_parse_embed() -> Result<(), Box<dyn std::error::Error>> {
        let input_url_title = "[@embed https://example.com/embed title]";
        let mut parsed_url_title = PattoLineParser::parse(Rule::statement, input_url_title)?;
        let (nodes_url_title, _props_url_title) =
            transform_statement(parsed_url_title.next().unwrap(), input_url_title, 0, 0);

        assert_eq!(nodes_url_title.len(), 1);
        if let AstNodeKind::Embed { link, title } = nodes_url_title[0].kind() {
            assert_eq!(link, "https://example.com/embed");
            assert_eq!(title, &Some("title".to_string()));
        } else {
            panic!(
                "Expected AstNodeKind::Embed, got {:?}",
                nodes_url_title[0].kind()
            );
        }

        let input_title_url = "[@embed title https://example.com/embed]";
        let mut parsed_title_url = PattoLineParser::parse(Rule::statement, input_title_url)?;
        let (nodes_title_url, _props_title_url) =
            transform_statement(parsed_title_url.next().unwrap(), input_title_url, 0, 0);

        assert_eq!(nodes_title_url.len(), 1);
        if let AstNodeKind::Embed { link, title } = nodes_title_url[0].kind() {
            assert_eq!(link, "https://example.com/embed");
            assert_eq!(title, &Some("title".to_string()));
        } else {
            panic!(
                "Expected AstNodeKind::Embed, got {:?}",
                nodes_title_url[0].kind()
            );
        }

        let input_url_url = "[@embed https://example.com/embed https://example.com/embed]";
        let mut parsed_url_url = PattoLineParser::parse(Rule::statement, input_url_url)?;
        let (nodes_url_url, _props_url_url) =
            transform_statement(parsed_url_url.next().unwrap(), input_url_url, 0, 0);

        assert_eq!(nodes_url_url.len(), 1);
        if let AstNodeKind::Embed { link, title } = nodes_url_url[0].kind() {
            assert_eq!(link, "https://example.com/embed");
            assert_eq!(title, &Some("https://example.com/embed".to_string()));
        } else {
            panic!(
                "Expected AstNodeKind::Embed, got {:?}",
                nodes_url_url[0].kind()
            );
        }

        let input_url_only = "[@embed https://example.com/embed]";
        let mut parsed_url_only = PattoLineParser::parse(Rule::statement, input_url_only)?;
        let (nodes_url_only, _props_url_only) =
            transform_statement(parsed_url_only.next().unwrap(), input_url_only, 0, 0);

        assert_eq!(nodes_url_only.len(), 1);
        if let AstNodeKind::Embed { link, title } = nodes_url_only[0].kind() {
            assert_eq!(link, "https://example.com/embed");
            assert_eq!(title, &None);
        } else {
            panic!(
                "Expected AstNodeKind::Embed, got {:?}",
                nodes_url_only[0].kind()
            );
        }

        Ok(())
    }

    #[test]
    fn test_parse_embed_local_pdf() -> Result<(), Box<dyn std::error::Error>> {
        // local path only — requires ./
        let input = "[@embed ./docs/report.pdf]";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (nodes, _) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        assert_eq!(nodes.len(), 1);
        if let AstNodeKind::Embed { link, title } = nodes[0].kind() {
            assert_eq!(link, "./docs/report.pdf");
            assert_eq!(title, &None);
        } else {
            panic!("Expected AstNodeKind::Embed, got {:?}", nodes[0].kind());
        }

        // local path with title after path
        let input2 = "[@embed ./docs/report.pdf My PDF]";
        let mut parsed2 = PattoLineParser::parse(Rule::statement, input2)?;
        let (nodes2, _) = transform_statement(parsed2.next().unwrap(), input2, 0, 0);
        assert_eq!(nodes2.len(), 1);
        if let AstNodeKind::Embed { link, title } = nodes2[0].kind() {
            assert_eq!(link, "./docs/report.pdf");
            assert_eq!(title, &Some("My PDF".to_string()));
        } else {
            panic!("Expected AstNodeKind::Embed, got {:?}", nodes2[0].kind());
        }

        // bare path without ./ → parse fails, not Embed
        let input3 = "[@embed docs/report.pdf]";
        let mut parsed3 = PattoLineParser::parse(Rule::statement, input3).unwrap();
        let (nodes3, _) = transform_statement(parsed3.next().unwrap(), input3, 0, 0);
        assert!(
            nodes3
                .iter()
                .all(|n| !matches!(n.kind(), AstNodeKind::Embed { .. })),
            "bare path without ./ should not parse as Embed"
        );

        Ok(())
    }

    #[test]
    fn test_parse_embed_title_local() -> Result<(), Box<dyn std::error::Error>> {
        // unquoted title before local path — safe now because ./ is unambiguous
        for (input, exp_link, exp_title) in [
            (
                "[@embed My PDF ./docs/report.pdf]",
                "./docs/report.pdf",
                Some("My PDF"),
            ),
            (
                "[@embed My PDF Report ./docs/report.pdf]",
                "./docs/report.pdf",
                Some("My PDF Report"),
            ),
            (
                "[@embed Title ./subdir/nested/file.pdf]",
                "./subdir/nested/file.pdf",
                Some("Title"),
            ),
        ] {
            let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
            let (nodes, _) = transform_statement(parsed.next().unwrap(), input, 0, 0);
            assert_eq!(nodes.len(), 1, "input: {input}");
            if let AstNodeKind::Embed { link, title } = nodes[0].kind() {
                assert_eq!(link, exp_link, "link mismatch for: {input}");
                assert_eq!(
                    title,
                    &exp_title.map(|s| s.to_string()),
                    "title mismatch for: {input}"
                );
            } else {
                panic!("Expected Embed, got {:?} for: {input}", nodes[0].kind());
            }
        }
        Ok(())
    }

    #[test]
    fn test_parse_embed_ambiguous_not_parsed() {
        // bare filename → becomes raw text, not Embed
        let input = "[@embed report.pdf]";
        let mut parsed = PattoLineParser::parse(Rule::statement, input).unwrap();
        let (nodes, _) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        assert!(
            nodes
                .iter()
                .all(|n| !matches!(n.kind(), AstNodeKind::Embed { .. })),
            "bare filename should not parse as Embed"
        );

        // path without ./ → parse failure
        let input2 = "[@embed title docs/report.pdf]";
        let mut parsed2 = PattoLineParser::parse(Rule::statement, input2).unwrap();
        let (nodes2, _) = transform_statement(parsed2.next().unwrap(), input2, 0, 0);
        assert!(
            nodes2
                .iter()
                .all(|n| !matches!(n.kind(), AstNodeKind::Embed { .. })),
            "path without ./ should not parse as Embed"
        );
    }

    #[test]
    fn test_parse_img_unquoted_alt() -> Result<(), Box<dyn std::error::Error>> {
        for (input, exp_src, exp_alt) in [
            // unquoted alt after URL
            (
                "[@img https://example.com/photo.jpg My Caption]",
                "https://example.com/photo.jpg",
                Some("My Caption"),
            ),
            // unquoted multi-word alt after URL
            (
                "[@img https://example.com/photo.jpg A nice photo]",
                "https://example.com/photo.jpg",
                Some("A nice photo"),
            ),
            // unquoted alt before URL
            (
                "[@img My Caption https://example.com/photo.jpg]",
                "https://example.com/photo.jpg",
                Some("My Caption"),
            ),
            // unquoted multi-word alt before URL
            (
                "[@img A nice photo https://example.com/photo.jpg]",
                "https://example.com/photo.jpg",
                Some("A nice photo"),
            ),
            // unquoted alt after local ./path
            (
                "[@img ./path/to/image.png My Caption]",
                "./path/to/image.png",
                Some("My Caption"),
            ),
            // unquoted multi-word alt after local ./path
            (
                "[@img ./path/to/image.jpg Photo of cat]",
                "./path/to/image.jpg",
                Some("Photo of cat"),
            ),
            // unquoted alt BEFORE local ./path — now safe and unambiguous
            (
                "[@img My Caption ./path/to/image.png]",
                "./path/to/image.png",
                Some("My Caption"),
            ),
            // real-world: filename-as-alt before ./path
            (
                "[@img 2026-03-04-10-20-35.png ./assets/2026-03-04-10-20-35.png]",
                "./assets/2026-03-04-10-20-35.png",
                Some("2026-03-04-10-20-35.png"),
            ),
            // quoted alt variants still work (backward compat)
            (
                r#"[@img "My Caption" https://example.com/photo.jpg]"#,
                "https://example.com/photo.jpg",
                Some("My Caption"),
            ),
            (
                r#"[@img https://example.com/photo.jpg "My Caption"]"#,
                "https://example.com/photo.jpg",
                Some("My Caption"),
            ),
            (
                r#"[@img ./path/to/img.jpg "Quoted alt"]"#,
                "./path/to/img.jpg",
                Some("Quoted alt"),
            ),
        ] {
            match PattoLineParser::parse(Rule::expr_img, input) {
                Ok(mut parsed) => {
                    let node = transform_img(parsed.next().unwrap(), input, 0, 0)
                        .ok_or("transform_img failed")?;
                    if let AstNodeKind::Image { src, alt } = node.kind() {
                        assert_eq!(src, exp_src, "src mismatch for: {input}");
                        assert_eq!(
                            *alt,
                            exp_alt.map(|s| s.to_string()),
                            "alt mismatch for: {input}"
                        );
                    } else {
                        panic!("Expected Image, got {:?} for: {input}", node.kind());
                    }
                }
                Err(e) => {
                    return Err(format!("Parse failed for '{input}': {e}").into());
                }
            }
        }
        Ok(())
    }

    #[test]
    fn test_parse_img_bare_path_invalid() {
        // bare path without ./ → parse error (local_file now requires ./ or ../)
        let input = "[@img alt path/to/image.jpg]";
        assert!(
            PattoLineParser::parse(Rule::expr_img, input).is_err(),
            "bare path without ./ should fail to parse"
        );
        let input2 = "[@img path/to/image.jpg]";
        assert!(
            PattoLineParser::parse(Rule::expr_img, input2).is_err(),
            "bare path without ./ should fail to parse"
        );
    }

    #[test]
    fn test_parse_code_inline_text_anchor() -> Result<(), Box<dyn std::error::Error>> {
        let input = "[` inline ![] code 123`] raw text    #anchor";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        //assert_eq!(code.extract_str(), "inline code 123");
        let code = &nodes[0];
        match code.kind() {
            AstNodeKind::Code {
                ref lang,
                ref inline,
            } => {
                assert_eq!(lang, "");
                assert!(*inline);
            }
            _ => {
                println!("{:?}", code);
                panic! {"it is weird"};
            }
        }
        //
        let raw_text = &nodes[1];
        if let AstNodeKind::Text = raw_text.kind() {
            assert_eq!(
                &raw_text.location().input[raw_text.location().span.0..raw_text.location().span.1],
                " raw text"
            );
        } else {
            panic!("text not extracted");
        }

        assert_eq!(props.len(), 1);
        if let Property::Anchor { ref name, .. } = props[0] {
            assert_eq!(name, "anchor");
        } else {
            panic!("anchor is not extracted properly");
        }
        Ok(())
    }

    #[test]
    fn test_parse_wiki_link() {
        let input = "[test wiki_page]";
        if let Ok(mut parsed) = PattoLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match &wiki_link.kind() {
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
        if let Ok(mut parsed) = PattoLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match &wiki_link.kind() {
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
    fn test_parse_self_link_anchored() {
        let input = "[#anchored]";
        if let Ok(mut parsed) = PattoLineParser::parse(Rule::expr_wiki_link, input) {
            if let Some(wiki_link) = transform_wiki_link(parsed.next().unwrap(), input, 0, 0) {
                match &wiki_link.kind() {
                    AstNodeKind::WikiLink { link, anchor } => {
                        assert_eq!(link, "");
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
    fn test_parse_img() -> Result<(), Box<dyn std::error::Error>> {
        for (input, g_path, g_alt) in [
            (
                "[@img \"img alt title\" https://gyazo.com/path/to/icon.png]",
                "https://gyazo.com/path/to/icon.png",
                Some("img alt title".to_string()),
            ),
            (
                "[@img https://gyazo.com/path/to/icon.png \"img alt title\"]",
                "https://gyazo.com/path/to/icon.png",
                Some("img alt title".to_string()),
            ),
            (
                "[@img ./path/to/image.png.png \"alt title\"]",
                "./path/to/image.png.png",
                Some("alt title".to_string()),
            ),
            (
                "[@img https://gyazo.com/path/to/icon.png]",
                "https://gyazo.com/path/to/icon.png",
                None,
            ),
            (
                r##"[@img ./local/path/to/icon.png "img escaped \"alt title"]"##,
                "./local/path/to/icon.png",
                Some(r##"img escaped \"alt title"##.to_string()),
            ),
            (
                r##"[@img ./local/with space/path/to/icon.png "img escaped \"alt title"]"##,
                "./local/with space/path/to/icon.png",
                Some(r##"img escaped \"alt title"##.to_string()),
            ),
        ] {
            match PattoLineParser::parse(Rule::expr_img, input) {
                Ok(mut parsed) => {
                    let node = transform_img(parsed.next().unwrap(), input, 0, 0)
                        .ok_or("transform_img failed")?;
                    if let AstNodeKind::Image { src, alt } = &node.kind() {
                        assert_eq!(src, g_path);
                        assert_eq!(*alt, g_alt);
                    } else {
                        panic! {"image is not correctly transformed"};
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
        for (input, g_url, g_title) in [(
                "[https://username@example.com google]",
                "https://username@example.com",
                Some("google".to_string()),
            ),
            (
                "[google https://username@example.com]",
                "https://username@example.com",
                Some("google".to_string()),
            ),
            ("[https://google.com]", "https://google.com", None),
            (
                "[日本語の タイトル https://username@example.com]",
                "https://username@example.com",
                Some("日本語の タイトル".to_string()),
            ),
            (
                "[https://lutpub.lut.fi/bitstream/handle/10024/167844/masterthesis_khaire_shubham.pdf pdfへのリンク]",
                "https://lutpub.lut.fi/bitstream/handle/10024/167844/masterthesis_khaire_shubham.pdf",
                Some("pdfへのリンク".to_string()),
            ),
            (
                "[pdfへのリンク https://lutpub.lut.fi/bitstream/handle/10024/167844/masterthesis_khaire_shubham.pdf]",
                "https://lutpub.lut.fi/bitstream/handle/10024/167844/masterthesis_khaire_shubham.pdf",
                Some("pdfへのリンク".to_string()),
            ),
            (
                "[https://username@example.com/path/to/url?param=1&newparam=2]",
                "https://username@example.com/path/to/url?param=1&newparam=2",
                None,
            ),
            (
                "[https://google.com https://google.com]",
                "https://google.com",
                Some("https://google.com".to_string()),
            )] {
            println!("parsing {input}");
            match PattoLineParser::parse(Rule::expr_url_link, input) {
                Ok(mut parsed) => {
                    if let Some(link) = transform_url_link(parsed.next().unwrap(), input, 0, 0) {
                        match &link.kind() {
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

    #[test]
    fn test_parse_local_files() -> Result<(), Box<dyn std::error::Error>> {
        for (input, g_local_file, g_title) in [
            (
                "[./asset/to/image.png path to image]",
                "./asset/to/image.png",
                Some("path to image".to_string()),
            ),
            (
                "[./nested/path/image.png path to image]",
                "./nested/path/image.png",
                Some("path to image".to_string()),
            ),
            (
                "[title of file ./path/to/file.pdf]",
                "./path/to/file.pdf",
                Some("title of file".to_string()),
            ),
            (
                "[./path/to/only_local_file.pdf]",
                "./path/to/only_local_file.pdf",
                None,
            ),
        ] {
            println!("parsing {input}");
            match PattoLineParser::parse(Rule::expr_local_file_link, input) {
                Ok(mut parsed) => {
                    if let Some(link) =
                        transform_local_file_link(parsed.next().unwrap(), input, 0, 0)
                    {
                        match &link.kind() {
                            AstNodeKind::Link { link, title } => {
                                assert_eq!(link, g_local_file);
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

    #[test]
    fn test_parse_mails() -> Result<(), Box<dyn std::error::Error>> {
        {
            let (input, g_mail, g_title) = (
                "[mailto:hoge@example.com example email]",
                "mailto:hoge@example.com",
                Some("example email".to_string()),
            );
            println!("parsing {input}");
            match PattoLineParser::parse(Rule::expr_mail_link, input) {
                Ok(mut parsed) => {
                    if let Some(link) = transform_mail_link(parsed.next().unwrap(), input, 0, 0) {
                        match &link.kind() {
                            AstNodeKind::Link { link, title } => {
                                assert_eq!(link, g_mail);
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

    #[test]
    fn test_parse_horizontal_line() -> Result<(), Box<dyn std::error::Error>> {
        let input = "-----";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (nodes, _props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        let hr = &nodes[0];
        if !matches!(hr.kind(), AstNodeKind::HorizontalLine) {
            panic! {"HorizontalLine could not be parsed"};
        }
        Ok(())
    }

    #[test]
    fn test_parse_abbrev_task() -> Result<(), Box<dyn std::error::Error>> {
        let input = "!2024-10-10 #anchor2 -2024-10-11T20:00";
        let mut parsed = PattoLineParser::parse(Rule::statement, input)?;
        let (_nodes, props) = transform_statement(parsed.next().unwrap(), input, 0, 0);
        let task = &props[0];
        if let Property::Task { status, due, .. } = task {
            assert_eq!(*status, TaskStatus::Todo);
            assert_eq!(
                *due,
                Deadline::Date(chrono::NaiveDate::from_ymd_opt(2024, 10, 10).unwrap())
            );
        } else {
            panic!("task could not be parsed");
        };

        let task = &props[2];
        if let Property::Task { status, due, .. } = task {
            assert_eq!(*status, TaskStatus::Done);
            assert_eq!(
                *due,
                Deadline::DateTime(
                    chrono::NaiveDateTime::parse_from_str("2024-10-11T20:00", "%Y-%m-%dT%H:%M")
                        .unwrap()
                )
            );
        } else {
            panic!("task could not be parsed");
        };

        Ok(())
    }
}
