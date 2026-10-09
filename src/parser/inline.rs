use pest::iterators::Pair;

use super::property::transform_property;
use super::{AstNode, Property, Rule, Span};

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
            Rule::expr_img => nodes.push(transform_img(inner, line, row, indent)),
            Rule::expr_embed => nodes.push(transform_embed(inner, line, row, indent)),
            Rule::expr_builtin_symbols => {
                nodes.push(transform_decoration(inner, line, row, indent))
            }
            Rule::expr_wiki_link => nodes.push(transform_wiki_link(inner, line, row, indent)),
            Rule::expr_url_link | Rule::expr_local_file_link | Rule::expr_mail_link => {
                nodes.push(transform_link(inner, line, row, indent))
            }
            Rule::expr_code_inline => nodes.push(transform_inline_code(inner, line, row, indent)),
            Rule::expr_math_inline => nodes.push(transform_inline_math(inner, line, row, indent)),
            Rule::expr_property | Rule::expr_anchor | Rule::expr_task => {
                props.extend(transform_property(inner, line, row, indent))
            }
            Rule::trailing_properties => props.extend(
                inner
                    .into_inner()
                    .filter_map(|prop| transform_property(prop, line, row, indent)),
            ),
            Rule::raw_sentence => {
                nodes.push(AstNode::text(line, row, Some(span_of(&inner, indent))))
            }
            Rule::expr_hr => nodes.push(AstNode::horizontal_line(
                line,
                row,
                Some(span_of(&inner, indent)),
            )),
            Rule::EOI => {}
            other => unreachable!("{:?} is not part of a statement", other),
        }
    }
    (nodes, props)
}

fn span_of(pair: &Pair<Rule>, indent: usize) -> Span {
    Span::from(pair.as_span()) + indent
}

fn transform_decoration<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> AstNode {
    let span = span_of(&pair, indent);
    let mut inner = pair.into_inner();
    let symbols = inner.next().unwrap();
    let mut fontsize = 0;
    let mut italic = false;
    let mut underline = false;
    let mut deleted = false;
    for symbol in symbols.into_inner() {
        match symbol.as_rule() {
            Rule::symbol_bold => fontsize += 1,
            Rule::symbol_italic => italic = true,
            Rule::symbol_underline => underline = true,
            Rule::symbol_deleted => deleted = true,
            other => unreachable!("{:?} is not a decoration symbol", other),
        }
    }
    let node = AstNode::decoration(line, row, Some(span), fontsize, italic, underline, deleted);
    // The body is a `statement_nestable`, which the grammar keeps a subset of
    // `statement`, so the same transform serves both.
    let (body, _) = transform_statement(inner.next().unwrap(), line, row, indent);
    node.add_contents(body);
    node
}

fn transform_inline_code<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> AstNode {
    let code = AstNode::code(line, row, Some(span_of(&pair, indent)), "", true);
    code.add_content(inner_text(pair, line, row, indent));
    code
}

fn transform_inline_math<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> AstNode {
    let math = AstNode::math(line, row, Some(span_of(&pair, indent)), true);
    math.add_content(inner_text(pair, line, row, indent));
    math
}

fn inner_text<'a>(pair: Pair<'a, Rule>, line: &'a str, row: usize, indent: usize) -> AstNode {
    let inner = pair.into_inner().next().unwrap();
    AstNode::text(line, row, Some(span_of(&inner, indent)))
}

fn transform_img<'a>(pair: Pair<'a, Rule>, line: &'a str, row: usize, indent: usize) -> AstNode {
    let span = span_of(&pair, indent);
    let inner = pair.into_inner().next().unwrap();
    let rule = inner.as_rule();
    let mut parts = inner.into_inner();
    let (path, alt) = match rule {
        Rule::img_alt_path_opts => {
            let alt = quoted_alt(parts.next().unwrap());
            (img_path(parts.next().unwrap()), Some(alt))
        }
        Rule::img_path_alt_opts => {
            let path = img_path(parts.next().unwrap());
            (path, Some(quoted_alt(parts.next().unwrap())))
        }
        Rule::img_unquoted_alt_path_opts | Rule::img_unquoted_alt_url_opts => {
            let alt = parts.next().unwrap().as_str();
            (parts.next().unwrap().as_str(), Some(alt))
        }
        Rule::img_path_unquoted_alt_opts => {
            let path = img_path(parts.next().unwrap());
            (path, Some(parts.next().unwrap().as_str()))
        }
        Rule::img_path_opts => (img_path(parts.next().unwrap()), None),
        other => unreachable!("{:?} is not an image form", other),
    };
    AstNode::image(line, row, Some(span), path, alt)
}

/// `alt_img` wraps `escaped_string`, which wraps the quote-free `inner_string`.
fn quoted_alt<'a>(alt_img: Pair<'a, Rule>) -> &'a str {
    alt_img
        .into_inner()
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap()
        .as_str()
}

/// `img_path` wraps either a `URL` or a `local_file`.
fn img_path<'a>(img_path: Pair<'a, Rule>) -> &'a str {
    img_path.into_inner().next().unwrap().as_str()
}

fn transform_wiki_link<'a>(
    pair: Pair<'a, Rule>,
    line: &'a str,
    row: usize,
    indent: usize,
) -> AstNode {
    let span = span_of(&pair, indent);
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::wiki_link_anchored => {
            let mut parts = inner.into_inner();
            let link = parts.next().unwrap().as_str();
            let anchor = anchor_name(parts.next().unwrap());
            AstNode::wikilink(line, row, Some(span), link, Some(anchor))
        }
        Rule::wiki_link => AstNode::wikilink(line, row, Some(span), inner.as_str(), None),
        Rule::self_link_anchored => {
            let anchor = anchor_name(inner.into_inner().next().unwrap());
            AstNode::wikilink(line, row, Some(span), "", Some(anchor))
        }
        other => unreachable!("{:?} is not a wiki link form", other),
    }
}

/// `expr_anchor` is `#` followed by the bare `anchor` token.
fn anchor_name<'a>(expr_anchor: Pair<'a, Rule>) -> &'a str {
    expr_anchor.into_inner().next().unwrap().as_str()
}

/// Which of the one or two bracket parts is the link target.
enum PartOrder {
    TargetOnly,
    TargetThenTitle,
    TitleThenTarget,
}

fn link_parts<'a>(inner: Pair<'a, Rule>, order: PartOrder) -> (&'a str, Option<&'a str>) {
    let mut parts = inner.into_inner();
    let first = parts.next().unwrap().as_str();
    match order {
        PartOrder::TargetOnly => (first, None),
        PartOrder::TargetThenTitle => (first, Some(parts.next().unwrap().as_str())),
        PartOrder::TitleThenTarget => (parts.next().unwrap().as_str(), Some(first)),
    }
}

/// URL, local file and mail links share one node kind; only the grammar rules
/// that name their part order differ.
fn transform_link<'a>(pair: Pair<'a, Rule>, line: &'a str, row: usize, indent: usize) -> AstNode {
    let inner = pair.into_inner().next().unwrap();
    let span = span_of(&inner, indent);
    let order = match inner.as_rule() {
        Rule::expr_url_title
        | Rule::expr_url_url
        | Rule::expr_local_file_title
        | Rule::expr_mail_title
        | Rule::expr_mail_mail => PartOrder::TargetThenTitle,
        Rule::expr_title_url | Rule::expr_title_local_file | Rule::expr_title_mail => {
            PartOrder::TitleThenTarget
        }
        Rule::expr_url_only | Rule::expr_local_file_only | Rule::expr_mail_only => {
            PartOrder::TargetOnly
        }
        other => unreachable!("{:?} is not a link form", other),
    };
    let (target, title) = link_parts(inner, order);
    AstNode::link(line, row, Some(span), target, title)
}

fn transform_embed<'a>(pair: Pair<'a, Rule>, line: &'a str, row: usize, indent: usize) -> AstNode {
    let inner = pair.into_inner().next().unwrap();
    let span = span_of(&inner, indent);
    let order = match inner.as_rule() {
        Rule::embed_url_title | Rule::embed_url_url | Rule::embed_local_title => {
            PartOrder::TargetThenTitle
        }
        Rule::embed_title_url | Rule::embed_title_local => PartOrder::TitleThenTarget,
        Rule::embed_url_only | Rule::embed_local_only => PartOrder::TargetOnly,
        other => unreachable!("{:?} is not an embed form", other),
    };
    let (target, title) = link_parts(inner, order);
    AstNode::embed(line, row, Some(span), target, title)
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
                    let node = transform_img(parsed.next().unwrap(), input, 0, 0);
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
            {
                let wiki_link = transform_wiki_link(parsed.next().unwrap(), input, 0, 0);
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
            {
                let wiki_link = transform_wiki_link(parsed.next().unwrap(), input, 0, 0);
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
            {
                let wiki_link = transform_wiki_link(parsed.next().unwrap(), input, 0, 0);
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
                    let node = transform_img(parsed.next().unwrap(), input, 0, 0);
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
                    { let link = transform_link(parsed.next().unwrap(), input, 0, 0);
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
                    {
                        let link = transform_link(parsed.next().unwrap(), input, 0, 0);
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
                    {
                        let link = transform_link(parsed.next().unwrap(), input, 0, 0);
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
