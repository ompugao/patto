//! Characterisation tests for `MarkdownRenderer::format_range`, which the
//! LSP's export command uses for a selection of lines (0-indexed, inclusive).

use patto::markdown::{MarkdownFlavor, MarkdownRendererOptions};
use patto::parser;
use patto::renderer::MarkdownRenderer;

const NOTE: &str =
    "line one\nline two\n\tchild of two\n\t\tgrand\nline three\n\t[@quote]\n\t\tq\nline four\n";

fn render_range(start_line: usize, end_line: usize) -> String {
    let result = parser::parse_text(NOTE);
    assert!(result.parse_errors.is_empty());
    let renderer = MarkdownRenderer::new(MarkdownRendererOptions::new(MarkdownFlavor::Standard));
    let mut output = Vec::new();
    renderer
        .format_range(&result.ast, &mut output, start_line, end_line)
        .unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn the_whole_note_renders_as_format_does() {
    assert_eq!(
        render_range(0, 7),
        "line one\n- line two\n  - child of two\n    - grand\n- line three\n  > q\nline four\n"
    );
}

#[test]
fn a_single_top_level_line_renders_alone() {
    assert_eq!(render_range(0, 0), "line one\n");
    assert_eq!(render_range(7, 7), "line four\n");
}

#[test]
fn a_selected_line_brings_its_children_even_past_the_range() {
    assert_eq!(
        render_range(1, 1),
        "- line two\n  - child of two\n    - grand\n"
    );
    assert_eq!(render_range(4, 4), "- line three\n  > q\n");
}

#[test]
fn a_selected_child_is_rendered_from_the_top_level() {
    assert_eq!(render_range(2, 2), "- child of two\n  - grand\n");
    assert_eq!(render_range(3, 3), "grand\n");
}

#[test]
fn a_row_inside_a_quote_block_is_not_reachable_on_its_own() {
    assert_eq!(render_range(6, 6), "");
}
