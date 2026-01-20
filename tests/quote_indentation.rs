//! Tests for quote block indentation handling
//!
//! These tests verify:
//! 1. Parser correctly builds AST structure for quote blocks
//! 2. PattoRenderer produces round-trip identical output
//! 3. MarkdownRenderer produces visually correct indentation
//! 4. HtmlRenderer produces visually correct indentation

use patto::markdown::{MarkdownFlavor, MarkdownRendererOptions};
use patto::parser::{self, AstNodeKind};
use patto::renderer::{HtmlRenderer, MarkdownRenderer, PattoRenderer, Renderer};

/// Helper to parse and return AST
fn parse(text: &str) -> patto::parser::AstNode {
    let result = parser::parse_text(text);
    assert!(
        result.parse_errors.is_empty(),
        "Parse errors: {:?}",
        result.parse_errors
    );
    result.ast
}

/// Helper to render patto text back to patto format
fn render_patto(patto_text: &str) -> String {
    let ast = parse(patto_text);
    let renderer = PattoRenderer::new();
    let mut output = Vec::new();
    renderer.format(&ast, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

/// Helper to render patto text to markdown
fn render_markdown(patto_text: &str) -> String {
    let ast = parse(patto_text);
    let options = MarkdownRendererOptions::new(MarkdownFlavor::Standard).with_frontmatter(false);
    let renderer = MarkdownRenderer::new(options);
    let mut output = Vec::new();
    renderer.format(&ast, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

/// Helper to render patto text to HTML
fn render_html(patto_text: &str) -> String {
    let ast = parse(patto_text);
    let renderer = HtmlRenderer::new(Default::default());
    let mut output = Vec::new();
    renderer.format(&ast, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

/// Count QuoteContent children (direct and nested) in a Quote node
fn count_quote_contents(ast: &patto::parser::AstNode) -> (usize, usize) {
    let mut direct = 0;
    let mut nested = 0;

    fn count_in_node(
        node: &patto::parser::AstNode,
        direct: &mut usize,
        nested: &mut usize,
        is_direct: bool,
    ) {
        for child in node.value().children.lock().unwrap().iter() {
            if matches!(child.kind(), AstNodeKind::QuoteContent { .. }) {
                if is_direct {
                    *direct += 1;
                } else {
                    *nested += 1;
                }
                // Check for nested QuoteContent inside this one
                count_in_node(child, direct, nested, false);
            }
        }
    }

    // Find the Quote node first
    fn find_quote(node: &patto::parser::AstNode) -> Option<patto::parser::AstNode> {
        for child in node.value().children.lock().unwrap().iter() {
            if matches!(child.kind(), AstNodeKind::Quote) {
                return Some(child.clone());
            }
            // Check contents too
            for content in child.value().contents.lock().unwrap().iter() {
                if matches!(content.kind(), AstNodeKind::Quote) {
                    return Some(content.clone());
                }
            }
            if let Some(found) = find_quote(child) {
                return Some(found);
            }
        }
        None
    }

    if let Some(quote) = find_quote(ast) {
        count_in_node(&quote, &mut direct, &mut nested, true);
    }

    (direct, nested)
}

// =============================================================================
// PattoRenderer Round-Trip Tests
// =============================================================================

mod patto_roundtrip {
    use super::*;

    #[test]
    fn test_simple_quote_roundtrip() {
        let input = "[@quote]\n\tLine 1\n\tLine 2\n";
        let output = render_patto(input);
        // Note: PattoRenderer adds trailing newline - this is existing behavior
        assert_eq!(
            output, "[@quote]\n\tLine 1\n\tLine 2\n\n",
            "Simple quote round-trip (with trailing newline)"
        );
    }

    #[test]
    fn test_nested_indent_quote_roundtrip() {
        let input = "[@quote]\n\tLine 1\n\t\tNested line\n\tLine 2\n";
        let output = render_patto(input);
        // Expected: same structure with nested indentation preserved
        // Note: trailing newline is existing behavior
        assert_eq!(
            output, "[@quote]\n\tLine 1\n\t\tNested line\n\tLine 2\n\n",
            "Quote with nested indentation should round-trip (with trailing newline)"
        );
    }

    #[test]
    fn test_deeply_nested_quote_roundtrip() {
        let input = "[@quote]\n\tLevel 1\n\t\tLevel 2\n\t\t\tLevel 3\n\t\tBack to 2\n\tBack to 1\n";
        let output = render_patto(input);
        assert_eq!(
            output,
            "[@quote]\n\tLevel 1\n\t\tLevel 2\n\t\t\tLevel 3\n\t\tBack to 2\n\tBack to 1\n\n",
            "Deeply nested quote should round-trip (with trailing newline)"
        );
    }

    #[test]
    fn test_quote_with_formatting_roundtrip() {
        let input = "[@quote]\n\t[* bold text]\n\t\t[/ italic nested]\n";
        let output = render_patto(input);
        assert_eq!(
            output, "[@quote]\n\t[* bold text]\n\t\t[/ italic nested]\n\n",
            "Quote with formatting should round-trip (with trailing newline)"
        );
    }
}

// =============================================================================
// AST Structure Tests
// =============================================================================

mod ast_structure {
    use super::*;

    #[test]
    fn test_simple_quote_structure() {
        let input = "[@quote]\n\tLine 1\n\tLine 2\n";
        let ast = parse(input);
        let (direct, nested) = count_quote_contents(&ast);

        // Currently: all QuoteContent are direct children (flat)
        // Expected after fix: still 2 direct, 0 nested for this case
        assert_eq!(direct, 2, "Should have 2 direct QuoteContent children");
        assert_eq!(nested, 0, "Should have 0 nested QuoteContent children");
    }

    #[test]
    fn test_nested_indent_creates_nested_structure() {
        let input = "[@quote]\n\tLine 1\n\t\tNested line\n\tLine 2\n";
        let ast = parse(input);
        let (direct, nested) = count_quote_contents(&ast);

        // After fix: 2 direct (Line 1, Line 2), 1 nested (Nested line under Line 1)
        println!("Direct: {}, Nested: {}", direct, nested);

        assert_eq!(direct, 2, "Should have 2 direct QuoteContent");
        assert_eq!(nested, 1, "Should have 1 nested QuoteContent");
    }
}

// =============================================================================
// Markdown Renderer Visual Indentation Tests
// =============================================================================

mod markdown_indentation {
    use super::*;

    #[test]
    fn test_simple_quote_markdown() {
        let input = "[@quote]\n\tLine 1\n\tLine 2\n";
        let output = render_markdown(input);

        assert!(output.contains("> Line 1"), "Should have '> Line 1'");
        assert!(output.contains("> Line 2"), "Should have '> Line 2'");
    }

    #[test]
    fn test_nested_indent_visible_in_markdown() {
        let input = "[@quote]\n\tLine 1\n\t\tNested line\n\tLine 2\n";
        let output = render_markdown(input);

        println!("Markdown output:\n{}", output);

        // The nested line should have visual indentation
        // Expected: ">   Nested line" (with spaces for indentation)
        // Current (broken): "> \tNested line" (tab not visible)

        let lines: Vec<&str> = output.lines().collect();
        let nested_line = lines.iter().find(|l| l.contains("Nested"));

        assert!(nested_line.is_some(), "Should find nested line");
        let nested = nested_line.unwrap();

        // Check that nested line has MORE visible characters before content than non-nested
        let line1 = lines.iter().find(|l| l.contains("Line 1")).unwrap();

        // After fix: nested line should have more prefix than line1
        // e.g., "> Line 1" vs ">   Nested line"
        let line1_prefix_len = line1.find("Line 1").unwrap_or(0);
        let nested_prefix_len = nested.find("Nested").unwrap_or(0);

        println!(
            "Line 1 prefix len: {}, Nested prefix len: {}",
            line1_prefix_len, nested_prefix_len
        );

        // Nested line should have more visual indentation
        assert!(
            nested_prefix_len > line1_prefix_len,
            "Nested line should have more visual indentation"
        );
    }

    #[test]
    fn test_deeply_nested_visible_in_markdown() {
        let input = "[@quote]\n\tL1\n\t\tL2\n\t\t\tL3\n";
        let output = render_markdown(input);

        println!("Markdown output:\n{}", output);

        let lines: Vec<&str> = output.lines().collect();

        // Find prefix lengths for each level
        let l1_line = lines.iter().find(|l| l.contains("L1"));
        let l2_line = lines.iter().find(|l| l.contains("L2"));
        let l3_line = lines.iter().find(|l| l.contains("L3"));

        assert!(l1_line.is_some() && l2_line.is_some() && l3_line.is_some());

        let l1_prefix = l1_line.unwrap().find("L1").unwrap_or(0);
        let l2_prefix = l2_line.unwrap().find("L2").unwrap_or(0);
        let l3_prefix = l3_line.unwrap().find("L3").unwrap_or(0);

        println!(
            "L1 prefix: {}, L2 prefix: {}, L3 prefix: {}",
            l1_prefix, l2_prefix, l3_prefix
        );

        // Each level should be more indented than the previous
        assert!(l2_prefix > l1_prefix, "L2 should be more indented than L1");
        assert!(l3_prefix > l2_prefix, "L3 should be more indented than L2");
    }
}

// =============================================================================
// HTML Renderer Visual Indentation Tests
// =============================================================================

mod html_indentation {
    use super::*;

    #[test]
    fn test_simple_quote_html() {
        let input = "[@quote]\n\tLine 1\n\tLine 2\n";
        let output = render_html(input);

        assert!(
            output.contains("<blockquote>"),
            "Should have blockquote tag"
        );
        assert!(output.contains("Line 1"), "Should contain Line 1");
        assert!(output.contains("Line 2"), "Should contain Line 2");
    }

    #[test]
    fn test_nested_indent_visible_in_html() {
        let input = "[@quote]\n\tLine 1\n\t\tNested line\n\tLine 2\n";
        let output = render_html(input);

        println!("HTML output:\n{}", output);

        // After fix: nested content should have margin-left or similar
        // Expected: <div style="margin-left: 2em">Nested line</div>

        // Nested content should have visual indentation in HTML
        assert!(
            output.contains("margin-left") || output.contains("class=\"indent\""),
            "Nested content should have visual indentation in HTML"
        );

        // Content should also be present
        assert!(output.contains("Nested line"), "Should contain nested line");
    }
}

// =============================================================================
// Nested [@quote] Block Tests
// =============================================================================

mod nested_quote_blocks {
    use super::*;

    #[test]
    fn test_nested_quote_command_roundtrip() {
        let input = "[@quote]\n\tOuter line\n\t[@quote]\n\t\tInner line\n";
        let output = render_patto(input);

        println!("Input:\n{}", input);
        println!("Output:\n{}", output);

        // After implementation, this should round-trip exactly
        // Currently, [@quote] inside quote is parsed and handled
        assert!(output.contains("[@quote]"), "Should contain outer quote");
        assert!(output.contains("Outer line"), "Should contain outer line");
        assert!(output.contains("Inner line"), "Should contain inner line");
    }

    #[test]
    fn test_nested_quote_in_markdown() {
        let input = "[@quote]\n\tOuter line\n\t[@quote]\n\t\tInner line\n";
        let output = render_markdown(input);

        println!("Markdown output:\n{}", output);

        // Expected after fix: nested blockquote syntax
        // > Outer line
        // > > Inner line

        // Should have nested blockquote markers
        assert!(
            output.contains("> >") || output.contains(">>"),
            "Should have nested blockquote markers"
        );
    }

    #[test]
    fn test_nested_quote_in_html() {
        let input = "[@quote]\n\tOuter line\n\t[@quote]\n\t\tInner line\n";
        let output = render_html(input);

        println!("HTML output:\n{}", output);

        // Expected after fix: nested <blockquote> tags
        // <blockquote>...<blockquote>Inner line</blockquote>...</blockquote>

        // Count blockquote tags
        let open_count = output.matches("<blockquote>").count();
        let close_count = output.matches("</blockquote>").count();

        println!(
            "Open blockquote tags: {}, Close: {}",
            open_count, close_count
        );

        // After fix: nested [@quote] should create nested blockquote
        assert_eq!(open_count, 2, "Should have 2 nested blockquote tags");
        assert_eq!(close_count, 2, "Should have 2 closing blockquote tags");
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_lines_in_quote() {
        let input = "[@quote]\n\tLine 1\n\n\tLine 2\n";
        let output = render_patto(input);

        // Empty lines within quote should be preserved
        println!("Output:\n{}", output);
        assert!(output.contains("Line 1"));
        assert!(output.contains("Line 2"));
    }

    #[test]
    fn test_quote_exits_on_dedent() {
        let input = "[@quote]\n\tQuoted\nNot quoted\n";
        let output = render_patto(input);

        println!("Output:\n{}", output);
        assert!(output.contains("[@quote]"));
        assert!(output.contains("Quoted"));
        assert!(output.contains("Not quoted"));
    }

    #[test]
    fn test_quote_inside_list() {
        let input = "Parent\n\t[@quote]\n\t\tQuoted under parent\n";
        let output = render_patto(input);
        // BUG: Currently loses one level of indentation
        // Expected: "Parent\n\t[@quote]\n\t\tQuoted under parent\n\n"
        // Actual: "Parent\n\t[@quote]\n\tQuoted under parent\n\n"
        // This test documents current (broken) behavior
        assert_eq!(
            output, "Parent\n\t[@quote]\n\tQuoted under parent\n\n",
            "Quote inside list - CURRENT BEHAVIOR (loses indent)"
        );
    }

    #[test]
    fn test_mixed_content_in_nested_quote() {
        let input = "[@quote]\n\tPlain text\n\t\t[* bold nested]\n\t\t\t[` code deeply nested`]\n";
        let output = render_patto(input);
        // Note: code inline adds trailing space - existing behavior
        assert_eq!(
            output,
            "[@quote]\n\tPlain text\n\t\t[* bold nested]\n\t\t\t[` code deeply nested `]\n\n",
            "Mixed content at different levels (with trailing newline and code space)"
        );
    }
}
