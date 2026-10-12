use patto::line_tracker::LineTracker;
use patto::parser::{self, AstNode, AstNodeKind, ParserError};

fn parse(text: &str) -> AstNode {
    let result = parser::parse_text(text);
    assert!(
        result.parse_errors.is_empty(),
        "unexpected parse errors: {:?}",
        result.parse_errors
    );
    result.ast
}

fn child(node: &AstNode, index: usize) -> AstNode {
    node.children()[index].clone()
}

fn content(node: &AstNode, index: usize) -> AstNode {
    node.contents()[index].clone()
}

fn child_texts(node: &AstNode) -> Vec<String> {
    node.children()
        .iter()
        .map(|child| child.extract_str().to_string())
        .collect()
}

fn content_texts(node: &AstNode) -> Vec<String> {
    node.contents()
        .iter()
        .map(|content| content.extract_str().to_string())
        .collect()
}

#[test]
fn empty_lines_keep_the_depth_of_the_previous_line() {
    let root = parse(
        "Some line\n\tIndented 1\n\tIndented 2\n\n\tIndented after empty\n\t\tNested\n\n\t\tNested after empty\n",
    );
    assert_eq!(child_texts(&root), ["Some line"]);
    let top = child(&root, 0);
    assert_eq!(
        child_texts(&top),
        ["\tIndented 1", "\tIndented 2", "", "\tIndented after empty"]
    );
    let nested = child(&top, 3);
    assert_eq!(
        child_texts(&nested),
        ["\t\tNested", "", "\t\tNested after empty"]
    );
}

#[test]
fn code_block_lines_become_code_content_until_the_dedent() {
    let root = parse("[@code rust]\n\tfn main() {}\n\t\tnested\nafter\n");
    assert_eq!(child_texts(&root), ["[@code rust]", "after"]);
    let code = content(&child(&root, 0), 0);
    assert!(matches!(code.kind(), AstNodeKind::Code { lang, inline: false } if lang == "rust"));
    assert_eq!(child_texts(&code), ["fn main() {}", "\tnested"]);
    assert!(code
        .children()
        .iter()
        .all(|line| matches!(line.kind(), AstNodeKind::CodeContent)));
}

#[test]
fn code_block_keeps_inner_empty_lines_but_not_trailing_ones() {
    let root = parse("[@code]\n\tone\n\n\ttwo\n\nafter\n");
    let code = content(&child(&root, 0), 0);
    assert_eq!(child_texts(&code), ["one", "", "two"]);
    assert_eq!(child_texts(&root), ["[@code]", "", "after"]);
}

#[test]
fn block_command_inside_a_list_item_ends_at_the_next_sibling() {
    let root = parse("item\n\t[@code]\n\t\tbody\n\tsibling\n");
    let item = child(&root, 0);
    assert_eq!(child_texts(&item), ["\t[@code]", "\tsibling"]);
    let code = content(&child(&item, 0), 0);
    assert_eq!(child_texts(&code), ["body"]);
}

#[test]
fn math_block_lines_become_math_content() {
    let root = parse("[@math]\n\tx^2\n");
    let math = content(&child(&root, 0), 0);
    assert!(matches!(math.kind(), AstNodeKind::Math { inline: false }));
    assert_eq!(child_texts(&math), ["x^2"]);
    assert!(matches!(child(&math, 0).kind(), AstNodeKind::MathContent));
}

#[test]
fn table_rows_split_columns_on_tabs() {
    let root = parse("[@table caption=\"Scores\"]\n\tname\tscore\n\talice\t10\n");
    let table = content(&child(&root, 0), 0);
    assert!(
        matches!(table.kind(), AstNodeKind::Table { caption: Some(caption) } if caption == "Scores")
    );
    assert_eq!(table.children().len(), 2);
    let row = child(&table, 1);
    assert!(matches!(row.kind(), AstNodeKind::TableRow));
    assert_eq!(content_texts(&row), ["alice", "10"]);
    assert!(row
        .contents()
        .iter()
        .all(|column| matches!(column.kind(), AstNodeKind::TableColumn)));
}

#[test]
fn over_indented_line_is_reported_and_attached_to_the_previous_line() {
    let result = parser::parse_text("top\n\t\t\ttoo deep\n");
    let [ParserError::InvalidIndentation(location)] = result.parse_errors.as_slice() else {
        panic!("expected one indentation error: {:?}", result.parse_errors);
    };
    assert_eq!(location.row, 1);
    let top = child(&result.ast, 0);
    assert_eq!(child_texts(&top), ["\t\t\ttoo deep"]);
}

#[test]
fn unparsable_line_is_kept_as_plain_text_with_an_error() {
    let result = parser::parse_text("[\n");
    assert!(matches!(
        result.parse_errors.as_slice(),
        [ParserError::ParseError(location, _)] if location.row == 0
    ));
    let line = child(&result.ast, 0);
    assert_eq!(content_texts(&line), ["["]);
    assert!(matches!(content(&line, 0).kind(), AstNodeKind::Text));
}

#[test]
fn line_tracking_assigns_stable_ids_that_survive_reordering() {
    let mut tracker = LineTracker::new().unwrap();
    let first = parser::parse_text_with_persistent_line_tracking("alpha\nbeta\n", &mut tracker);
    let alpha_id = child(&first.ast, 0).stable_id().unwrap();
    let beta_id = child(&first.ast, 1).stable_id().unwrap();
    assert_ne!(alpha_id, beta_id);

    let second = parser::parse_text_with_persistent_line_tracking("beta\nalpha\n", &mut tracker);
    assert_eq!(child(&second.ast, 0).stable_id(), Some(beta_id));
    assert_eq!(child(&second.ast, 1).stable_id(), Some(alpha_id));
}

#[test]
fn parsing_without_line_tracking_leaves_stable_ids_unset() {
    let root = parse("alpha\n");
    assert_eq!(child(&root, 0).stable_id(), None);
}
