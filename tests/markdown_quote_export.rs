//! Characterisation tests for how the markdown exporter lays out quotes:
//! one `> ` per quote level, four spaces per nesting level inside a quote,
//! and the list indent in front of both.

use patto::markdown::{MarkdownFlavor, MarkdownRendererOptions};
use patto::parser;
use patto::renderer::{MarkdownRenderer, Renderer};

fn render(patto_text: &str) -> String {
    let result = parser::parse_text(patto_text);
    assert!(
        result.parse_errors.is_empty(),
        "Parse errors: {:?}",
        result.parse_errors
    );
    let renderer = MarkdownRenderer::new(MarkdownRendererOptions::new(MarkdownFlavor::Standard));
    let mut output = Vec::new();
    renderer.format(&result.ast, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn nesting_inside_a_quote_is_shown_with_four_spaces_per_level() {
    assert_eq!(
        render("[@quote]\n\ta\n\t\tb\n\t\t\tc\n"),
        "> a\n>     b\n>         c\n"
    );
}

#[test]
fn a_quote_inside_a_quote_gets_a_second_marker() {
    assert_eq!(
        render("[@quote]\n\ta\n\t[@quote]\n\t\tb\n\t\t\tc\n"),
        "> a\n> > b\n>     c\n"
    );
}

#[test]
fn a_quote_under_a_list_item_keeps_the_list_indent() {
    assert_eq!(
        render("parent\n\t[@quote]\n\t\tq\n\t\t\tdeeper\n"),
        "- parent\n  > q\n  >     deeper\n"
    );
}

#[test]
fn a_nested_quote_at_an_inner_level_uses_markers_not_spaces() {
    assert_eq!(
        render("[@quote]\n\ta\n\t\t[@quote]\n\t\t\tb\n\t\t\t\tc\n\t\td\n"),
        "> a\n> > b\n>     c\n>     d\n"
    );
}

#[test]
fn a_third_quote_level_is_flattened_into_the_second() {
    assert_eq!(
        render("[@quote]\n\ta\n\t\t[@quote]\n\t\t\tb\n\t\t\t\t[@quote]\n\t\t\t\t\tc\n"),
        "> a\n> > b\n> > c\n"
    );
}

#[test]
fn line_properties_are_not_written_inside_a_quote() {
    assert_eq!(
        render("[@quote]\n\ta {@task status=todo due=2025-01-01}\n\tb #anc\n"),
        "> a \n> b\n"
    );
}
