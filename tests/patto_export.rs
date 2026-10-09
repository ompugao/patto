//! Characterisation tests for the patto renderer, which writes an AST back out
//! as patto text. It is what the markdown importer produces its output with, so
//! anything it emits has to parse back to the same kind of node.

use patto::parser::{self, AstNodeKind};
use patto::renderer::{PattoRenderer, Renderer};

fn render(patto_text: &str) -> String {
    let result = parser::parse_text(patto_text);
    assert!(
        result.parse_errors.is_empty(),
        "Parse errors: {:?}",
        result.parse_errors
    );

    let mut output = Vec::new();
    PattoRenderer::new()
        .format(&result.ast, &mut output)
        .unwrap();
    String::from_utf8(output).unwrap()
}

/// Kinds of the inline contents of every top-level line.
fn inline_kinds(patto_text: &str) -> Vec<String> {
    let result = parser::parse_text(patto_text);
    let mut kinds = Vec::new();
    for line in result.ast.children().iter() {
        for content in line.contents().iter() {
            kinds.push(format!("{:?}", content.kind()));
        }
    }
    kinds
}

fn assert_round_trips(patto_text: &str) {
    assert_eq!(render(patto_text), patto_text);
}

#[test]
fn plain_and_nested_lines_round_trip() {
    assert_round_trips("plain line\n");
    assert_round_trips("parent\n\tchild\n\t\tgrandchild\n");
}

#[test]
fn decorations_round_trip() {
    assert_round_trips("[* bold] and [/ italic]\n");
}

#[test]
fn wikilinks_round_trip() {
    assert_round_trips("[other note]\n");
    assert_round_trips("[other#anchor]\n");
}

#[test]
fn images_round_trip() {
    assert_round_trips("[@img ./a.png]\n");
}

#[test]
fn external_links_are_rewritten_title_first() {
    // The parser accepts either order; the renderer always writes title first.
    assert_eq!(
        render("[https://example.com Title]\n"),
        "[Title https://example.com]\n"
    );
    assert_round_trips("[Title https://example.com]\n");
}

#[test]
fn blocks_round_trip_apart_from_a_trailing_blank_line() {
    // Block bodies are terminated with a blank line, which the parser ignores.
    assert_eq!(
        render("[@code python]\n\tprint(1)\n"),
        "[@code python]\n\tprint(1)\n\n"
    );
    assert_eq!(render("[@quote]\n\tquoted\n"), "[@quote]\n\tquoted\n\n");
    assert_eq!(render("[@math]\n\tx = 1\n"), "[@math]\n\tx = 1\n\n");
}

#[test]
fn table_caption_is_written_in_its_long_form() {
    assert_eq!(
        render("[@table cap]\n\ta\tb\n"),
        "[@table caption=\"cap\"]\n\ta\tb\n\n"
    );
}

#[test]
fn task_property_moves_to_the_end_of_the_line() {
    assert_eq!(
        render("{@task status=todo due=2024-12-31} do it\n"),
        " do it {@task status=todo due=2024-12-31}\n"
    );
}

/// Regression: the grammar needs five or more dashes, so emitting three made
/// the rendered line parse back as plain text.
#[test]
fn horizontal_line_is_emitted_so_it_parses_back_as_one() {
    let rendered = render("------\n");
    assert_eq!(
        inline_kinds(&rendered),
        vec![format!("{:?}", AstNodeKind::HorizontalLine)],
        "rendered {rendered:?} does not parse back as a horizontal line",
    );
}

/// Every block the renderer emits has to survive a parse. This catches a
/// renderer writing a form the grammar no longer accepts.
#[test]
fn rendered_output_reparses_without_errors() {
    let cases = [
        "plain line\n",
        "parent\n\tchild\n",
        "[* bold] and [/ italic]\n",
        "[other note]\n",
        "[other#anchor]\n",
        "[https://example.com Title]\n",
        "[@code python]\n\tprint(1)\n",
        "[@quote]\n\tquoted\n",
        "[@math]\n\tx = 1\n",
        "[@table cap]\n\ta\tb\n",
        "[@img ./a.png]\n",
        "------\n",
        "{@task status=todo due=2024-12-31} do it\n",
    ];
    for case in cases {
        let rendered = render(case);
        let reparsed = parser::parse_text(&rendered);
        assert!(
            reparsed.parse_errors.is_empty(),
            "rendering {case:?} produced {rendered:?}, which fails to parse: {:?}",
            reparsed.parse_errors
        );
    }
}
