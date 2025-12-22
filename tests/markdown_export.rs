//! Comprehensive tests for the markdown exporter
//!
//! Tests cover all three flavors (Standard, Obsidian, GitHub) and all AST node types.

use patto::markdown::{MarkdownFlavor, MarkdownRendererOptions};
use patto::parser;
use patto::renderer::{MarkdownRenderer, Renderer};

/// Helper to render patto text to markdown
fn render_markdown(patto_text: &str, flavor: MarkdownFlavor) -> String {
    let result = parser::parse_text(patto_text);
    assert!(
        result.parse_errors.is_empty(),
        "Parse errors: {:?}",
        result.parse_errors
    );

    let options = MarkdownRendererOptions::new(flavor);
    let renderer = MarkdownRenderer::new(options);

    let mut output = Vec::new();
    renderer.format(&result.ast, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

/// Helper to render with frontmatter disabled
fn render_markdown_no_frontmatter(patto_text: &str, flavor: MarkdownFlavor) -> String {
    let result = parser::parse_text(patto_text);
    assert!(
        result.parse_errors.is_empty(),
        "Parse errors: {:?}",
        result.parse_errors
    );

    let options = MarkdownRendererOptions::new(flavor).with_frontmatter(false);
    let renderer = MarkdownRenderer::new(options);

    let mut output = Vec::new();
    renderer.format(&result.ast, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

// =============================================================================
// Basic Structure Tests
// =============================================================================

mod structure {
    use super::*;

    #[test]
    fn test_simple_line() {
        let output = render_markdown("Hello world.", MarkdownFlavor::Standard);
        assert!(output.contains("Hello world."));
    }

    #[test]
    fn test_nested_structure() {
        let input = "Root\n\tChild1\n\t\tGrandchild\n\tChild2";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("- Root"));
        assert!(output.contains("  - Child1"));
        assert!(output.contains("    - Grandchild"));
        assert!(output.contains("  - Child2"));
    }

    #[test]
    fn test_multiple_root_lines() {
        let input = "Line1\n\nLine2\n\nLine3";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("Line1"));
        assert!(output.contains("Line2"));
        assert!(output.contains("Line3"));
    }

    #[test]
    fn test_empty_lines_preserved() {
        let input = "Before\n\nAfter";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        // Should have blank line between
        assert!(output.contains("Before"));
        assert!(output.contains("After"));
        // Count newlines - should have separation
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 3);
    }

    #[test]
    fn test_deep_nesting() {
        let input = "L0\n\tL1\n\t\tL2\n\t\t\tL3\n\t\t\t\tL4";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("- L0"));
        assert!(output.contains("  - L1"));
        assert!(output.contains("    - L2"));
        assert!(output.contains("      - L3"));
        assert!(output.contains("        - L4"));
    }
}

// =============================================================================
// Code Block Tests
// =============================================================================

mod code_blocks {
    use super::*;

    #[test]
    fn test_code_block_basic() {
        let input = "[@code python]\n\tprint('hello')";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("```python"));
        assert!(output.contains("print('hello')"));
        assert!(output.contains("```\n"));
    }

    #[test]
    fn test_code_block_multiline() {
        let input = "[@code rust]\n\tfn main() {\n\t    println!(\"Hello\");\n\t}";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("```rust"));
        assert!(output.contains("fn main()"));
        assert!(output.contains("println!"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_code_block_not_in_list() {
        let input = "[@code python]\n\tcode here";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        // Code block should NOT start with "- ```"
        assert!(!output.contains("- ```"));
        // Should be proper fenced block
        assert!(output.contains("```python\n"));
    }

    #[test]
    fn test_inline_code() {
        let input = "Use [` some_function() `] here";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("`some_function() `"));
    }

    #[test]
    fn test_code_block_empty_lang() {
        let input = "[@code]\n\tplain code";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("```\n") || output.contains("```"));
        assert!(output.contains("plain code"));
    }
}

// =============================================================================
// Table Tests
// =============================================================================

mod tables {
    use super::*;

    #[test]
    fn test_table_basic() {
        let input = "[@table]\n\theader1\theader2\n\trow1col1\trow1col2";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("| header1 | header2 |"));
        assert!(output.contains("| --- | --- |"));
        assert!(output.contains("| row1col1 | row1col2 |"));
    }

    #[test]
    fn test_table_with_caption() {
        let input = "[@table caption=\"My Table\"]\n\ta\tb\n\t1\t2";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("*My Table*"));
        assert!(output.contains("| a | b |"));
    }

    #[test]
    fn test_table_three_columns() {
        let input = "[@table]\n\tcol1\tcol2\tcol3\n\ta\tb\tc";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("| col1 | col2 | col3 |"));
        assert!(output.contains("| --- | --- | --- |"));
        assert!(output.contains("| a | b | c |"));
    }

    #[test]
    fn test_table_multiple_rows() {
        let input = "[@table]\n\th1\th2\n\tr1c1\tr1c2\n\tr2c1\tr2c2\n\tr3c1\tr3c2";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("| h1 | h2 |"));
        assert!(output.contains("| r1c1 | r1c2 |"));
        assert!(output.contains("| r2c1 | r2c2 |"));
        assert!(output.contains("| r3c1 | r3c2 |"));
    }
}

// =============================================================================
// Quote Tests
// =============================================================================

mod quotes {
    use super::*;

    #[test]
    fn test_quote_basic() {
        let input = "[@quote]\n\tThis is quoted";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("> This is quoted"));
    }

    #[test]
    fn test_quote_multiline() {
        let input = "[@quote]\n\tLine 1\n\tLine 2\n\tLine 3";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("> Line 1"));
        assert!(output.contains("> Line 2"));
        assert!(output.contains("> Line 3"));
    }

    #[test]
    fn test_quote_nested_in_structure() {
        let input = "Parent\n\t[@quote]\n\t\tQuoted text";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("- Parent"));
        assert!(output.contains("> Quoted text"));
        // Quote should NOT have list marker
        assert!(!output.contains("- > Quoted"));
    }

    #[test]
    fn test_quote_not_as_list_item() {
        let input = "[@quote]\n\tquote content";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        // Should start with > not with -
        let first_content_line = output.lines().find(|l| l.contains("quote content"));
        assert!(first_content_line.is_some());
        assert!(first_content_line.unwrap().trim().starts_with(">"));
    }
}

// =============================================================================
// Task Tests
// =============================================================================

mod tasks {
    use super::*;

    #[test]
    fn test_task_todo() {
        let input = "Task item {@task status=todo}";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[ ] Task item"));
    }

    #[test]
    fn test_task_done() {
        let input = "Completed {@task status=done}";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[x] Completed"));
    }

    #[test]
    fn test_task_with_due_date() {
        let input = "Task {@task status=todo due=2024-12-31}";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[ ] Task"));
        assert!(output.contains("(due: 2024-12-31)"));
    }

    #[test]
    fn test_task_done_no_due_shown() {
        let input = "Done task {@task status=done due=2024-12-31}";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[x] Done task"));
        // Due date should NOT be shown for done tasks
        assert!(!output.contains("(due:"));
    }

    #[test]
    fn test_task_no_due_no_empty_text() {
        let input = "Task without due {@task status=todo}";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[ ] Task without due"));
        // Should NOT have empty "due: "
        assert!(!output.contains("due:"));
    }

    #[test]
    fn test_task_abbreviated_todo() {
        let input = "Abbreviated task !2024-12-31";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[ ] Abbreviated task"));
        assert!(output.contains("(due: 2024-12-31)"));
    }

    #[test]
    fn test_task_abbreviated_done() {
        let input = "Done abbreviated -2024-12-31";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[x] Done abbreviated"));
    }

    #[test]
    fn test_task_obsidian_emoji_format() {
        let input = "Task {@task status=todo due=2024-12-31}";
        let output = render_markdown(input, MarkdownFlavor::Obsidian);

        assert!(output.contains("[ ] Task"));
        assert!(output.contains("📅 2024-12-31"));
    }
}

// =============================================================================
// WikiLink Tests
// =============================================================================

mod wikilinks {
    use super::*;

    #[test]
    fn test_wikilink_standard_format() {
        let input = "Link to [other note]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[other note](other note.md)"));
    }

    #[test]
    fn test_wikilink_with_anchor_standard() {
        let input = "Link to [note#section]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[note#section](note.md#section)"));
    }

    #[test]
    fn test_wikilink_self_anchor_standard() {
        let input = "Self link [#myanchor]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[#myanchor](#myanchor)"));
    }

    #[test]
    fn test_wikilink_obsidian_format() {
        let input = "Link to [other note]";
        let output = render_markdown_no_frontmatter(input, MarkdownFlavor::Obsidian);

        assert!(output.contains("[[other note]]"));
    }

    #[test]
    fn test_wikilink_with_anchor_obsidian() {
        let input = "Link to [note#section]";
        let output = render_markdown_no_frontmatter(input, MarkdownFlavor::Obsidian);

        assert!(output.contains("[[note#section]]"));
    }

    #[test]
    fn test_wikilink_self_anchor_obsidian() {
        let input = "Self link [#myanchor]";
        let output = render_markdown_no_frontmatter(input, MarkdownFlavor::Obsidian);

        assert!(output.contains("[[#myanchor]]"));
    }

    #[test]
    fn test_wikilink_github_format() {
        let input = "Link to [other note]";
        let output = render_markdown(input, MarkdownFlavor::GitHub);

        assert!(output.contains("[other note](other note.md)"));
    }
}

// =============================================================================
// URL Link Tests
// =============================================================================

mod url_links {
    use super::*;

    #[test]
    fn test_url_with_title() {
        let input = "[https://example.com Example Site]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[Example Site](https://example.com)"));
    }

    #[test]
    fn test_url_title_first() {
        let input = "[Example Site https://example.com]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[Example Site](https://example.com)"));
    }

    #[test]
    fn test_url_only() {
        let input = "[https://example.com]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("[https://example.com](https://example.com)"));
    }
}

// =============================================================================
// Anchor Tests
// =============================================================================

mod anchors {
    use super::*;

    #[test]
    fn test_anchor_standard_html() {
        let input = "Line with anchor #myanchor";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("<a id=\"myanchor\"></a>"));
    }

    #[test]
    fn test_anchor_obsidian_block() {
        let input = "Line with anchor #myanchor";
        let output = render_markdown_no_frontmatter(input, MarkdownFlavor::Obsidian);

        assert!(output.contains("^myanchor"));
    }

    #[test]
    fn test_anchor_github_comment() {
        let input = "Line with anchor #myanchor";
        let output = render_markdown(input, MarkdownFlavor::GitHub);

        assert!(output.contains("<!-- anchor: myanchor -->"));
    }

    #[test]
    fn test_anchor_inline_syntax() {
        let input = "Line with anchor #myanchor";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("<a id=\"myanchor\"></a>"));
    }
}

// =============================================================================
// Decoration Tests
// =============================================================================

mod decorations {
    use super::*;

    #[test]
    fn test_bold() {
        let input = "[* bold text]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("**bold text**"));
    }

    #[test]
    fn test_italic() {
        let input = "[/ italic text]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("*italic text*"));
    }

    #[test]
    fn test_bold_italic() {
        let input = "[*/ bold italic]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("***bold italic***"));
    }

    #[test]
    fn test_deleted() {
        let input = "[- deleted text]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("~~deleted text~~"));
    }

    #[test]
    fn test_underline() {
        let input = "[_ underlined]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("<ins>underlined</ins>"));
    }
}

// =============================================================================
// Math Tests
// =============================================================================

mod math {
    use super::*;

    #[test]
    fn test_inline_math() {
        let input = "Formula [$x^2$]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("$x^2$"));
    }

    #[test]
    fn test_block_math() {
        let input = "[@math]\n\tx^2 + y^2 = z^2";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("$$"));
        assert!(output.contains("x^2 + y^2 = z^2"));
    }
}

// =============================================================================
// Image Tests
// =============================================================================

mod images {
    use super::*;

    #[test]
    fn test_image_with_alt() {
        let input = "[@img https://example.com/img.png \"Alt text\"]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("![Alt text](https://example.com/img.png)"));
    }

    #[test]
    fn test_image_without_alt() {
        let input = "[@img https://example.com/img.png]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("![](https://example.com/img.png)"));
    }
}

// =============================================================================
// Horizontal Rule Tests
// =============================================================================

mod horizontal_rule {
    use super::*;

    #[test]
    fn test_horizontal_rule() {
        let input = "-----";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("---"));
    }
}

// =============================================================================
// Frontmatter Tests
// =============================================================================

mod frontmatter {
    use super::*;

    #[test]
    fn test_obsidian_has_frontmatter() {
        let input = "Content";
        let output = render_markdown(input, MarkdownFlavor::Obsidian);

        assert!(output.starts_with("---\n"));
        assert!(output.contains("patto_source: true"));
        assert!(output.contains("flavor: obsidian"));
        assert!(output.contains("---\n\n"));
    }

    #[test]
    fn test_standard_no_frontmatter() {
        let input = "Content";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(!output.contains("---"));
        assert!(!output.contains("patto_source"));
    }

    #[test]
    fn test_github_no_frontmatter() {
        let input = "Content";
        let output = render_markdown(input, MarkdownFlavor::GitHub);

        assert!(!output.starts_with("---"));
    }

    #[test]
    fn test_obsidian_frontmatter_disabled() {
        let input = "Content";
        let output = render_markdown_no_frontmatter(input, MarkdownFlavor::Obsidian);

        assert!(!output.starts_with("---"));
        assert!(!output.contains("patto_source"));
    }
}

// =============================================================================
// Flavor Comparison Tests
// =============================================================================

mod flavor_comparison {
    use super::*;

    #[test]
    fn test_all_flavors_produce_output() {
        let input = "Test line";

        let standard = render_markdown(input, MarkdownFlavor::Standard);
        let obsidian = render_markdown(input, MarkdownFlavor::Obsidian);
        let github = render_markdown(input, MarkdownFlavor::GitHub);

        assert!(!standard.is_empty());
        assert!(!obsidian.is_empty());
        assert!(!github.is_empty());
    }

    #[test]
    fn test_wikilink_differs_by_flavor() {
        let input = "[some note]";

        let standard = render_markdown(input, MarkdownFlavor::Standard);
        let obsidian = render_markdown_no_frontmatter(input, MarkdownFlavor::Obsidian);
        let github = render_markdown(input, MarkdownFlavor::GitHub);

        // Standard and GitHub use markdown links
        assert!(standard.contains("[some note](some note.md)"));
        assert!(github.contains("[some note](some note.md)"));

        // Obsidian uses wiki links
        assert!(obsidian.contains("[[some note]]"));
    }

    #[test]
    fn test_task_format_differs_by_flavor() {
        let input = "Task {@task status=todo due=2024-12-31}";

        let standard = render_markdown(input, MarkdownFlavor::Standard);
        let obsidian = render_markdown_no_frontmatter(input, MarkdownFlavor::Obsidian);
        let github = render_markdown(input, MarkdownFlavor::GitHub);

        // Standard and GitHub use (due: date)
        assert!(standard.contains("(due: 2024-12-31)"));
        assert!(github.contains("(due: 2024-12-31)"));

        // Obsidian uses emoji
        assert!(obsidian.contains("📅 2024-12-31"));
    }

    #[test]
    fn test_anchor_format_differs_by_flavor() {
        let input = "Line #anchor";

        let standard = render_markdown(input, MarkdownFlavor::Standard);
        let obsidian = render_markdown_no_frontmatter(input, MarkdownFlavor::Obsidian);
        let github = render_markdown(input, MarkdownFlavor::GitHub);

        // Standard uses HTML anchor
        assert!(standard.contains("<a id=\"anchor\"></a>"));

        // Obsidian uses block reference
        assert!(obsidian.contains("^anchor"));

        // GitHub uses HTML comment
        assert!(github.contains("<!-- anchor: anchor -->"));
    }
}

// =============================================================================
// Complex Document Tests
// =============================================================================

mod complex_documents {
    use super::*;

    #[test]
    fn test_full_document() {
        let input = r#"Title
	Subtitle
		Detail
	Another point #anchor

Task List
	Todo item {@task status=todo due=2024-12-31}
	Done item {@task status=done}

[@quote]
	Important quote

[@code rust]
	fn main() {
	    println!("Hello");
	}

[@table caption="Data"]
	Col1	Col2
	A	B
"#;

        let output = render_markdown(input, MarkdownFlavor::Standard);

        // Structure
        assert!(output.contains("- Title"));
        assert!(output.contains("  - Subtitle"));
        assert!(output.contains("    - Detail"));
        assert!(output.contains("  - Another point"));
        assert!(output.contains("<a id=\"anchor\"></a>"));

        // Tasks
        assert!(output.contains("[ ] Todo item"));
        assert!(output.contains("(due: 2024-12-31)"));
        assert!(output.contains("[x] Done item"));

        // Quote
        assert!(output.contains("> Important quote"));

        // Code
        assert!(output.contains("```rust"));
        assert!(output.contains("fn main()"));
        assert!(output.contains("```\n"));

        // Table
        assert!(output.contains("*Data*"));
        assert!(output.contains("| Col1 | Col2 |"));
        assert!(output.contains("| A | B |"));
    }

    #[test]
    fn test_nested_elements() {
        let input = "Root\n\tChild with [link to note]\n\tChild with [* bold] and [/ italic]\n\tChild with [`code`]";

        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("- Root"));
        assert!(output.contains("[link to note](link to note.md)"));
        assert!(output.contains("**bold**"));
        assert!(output.contains("*italic*"));
        assert!(output.contains("`code`"));
    }
}

// =============================================================================
// Edge Case Tests
// =============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_input() {
        let output = render_markdown("", MarkdownFlavor::Standard);
        assert!(output.is_empty() || output.trim().is_empty());
    }

    #[test]
    fn test_only_whitespace() {
        let output = render_markdown("   ", MarkdownFlavor::Standard);
        // Should handle gracefully
        assert!(!output.contains("- "));
    }

    #[test]
    fn test_unicode_content() {
        let input = "日本語テスト 🎉 émojis";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("日本語テスト"));
        assert!(output.contains("🎉"));
        assert!(output.contains("émojis"));
    }

    #[test]
    fn test_special_markdown_characters() {
        let input = "Text with * and _ and # and [brackets]";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        // Should render without breaking
        assert!(output.contains("Text with"));
    }

    #[test]
    fn test_very_long_line() {
        let long_text = "A".repeat(1000);
        let input = format!("{}", long_text);
        let output = render_markdown(&input, MarkdownFlavor::Standard);

        assert!(output.contains(&long_text));
    }

    #[test]
    fn test_mixed_content_line() {
        let input = "Text [* bold] more [/ italic] and [`code`] end";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("**bold**"));
        assert!(output.contains("*italic*"));
        assert!(output.contains("`code`"));
    }
}

// =============================================================================
// Regression Tests
// =============================================================================

mod regressions {
    use super::*;

    #[test]
    fn test_code_block_not_nested_in_list() {
        // Regression: code blocks were being rendered as list items
        let input = "[@code python]\n\tprint('hello')";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(!output.contains("- ```"));
        assert!(!output.contains("* ```"));
    }

    #[test]
    fn test_table_columns_separated() {
        // Regression: table columns were not separated
        let input = "[@table]\n\ta\tb\tc";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("| a | b | c |"));
        assert!(!output.contains("abc"));
    }

    #[test]
    fn test_quote_not_as_list_item() {
        // Regression: quotes were rendered with list markers
        let input = "[@quote]\n\tquoted";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(!output.contains("- > quoted"));
        assert!(output.contains("> quoted"));
    }

    #[test]
    fn test_task_no_empty_due() {
        // Regression: tasks showed "due: " even when empty
        let input = "Task {@task status=todo}";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(!output.contains("due:"));
        assert!(!output.contains("(due: )"));
    }

    #[test]
    fn test_empty_lines_not_list_items() {
        // Regression: empty lines were rendered as "- "
        let input = "Line1\n\nLine2";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        // Should not have standalone "- " for empty lines
        let lines: Vec<&str> = output.lines().collect();
        for line in lines {
            if line.trim() == "-" {
                panic!("Found standalone list marker: '{}'", line);
            }
        }
    }

    #[test]
    fn test_nested_structure_preserved() {
        // Regression: nested structure was flattened
        let input = "Root\n\tChild";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("- Root"));
        assert!(output.contains("  - Child"));
    }

    #[test]
    fn test_no_extra_blank_lines_between_consecutive_lines() {
        // Regression: consecutive lines had extra blank lines inserted
        let input = "Line1\nLine2\nLine3";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        // Should be exactly 3 lines with no blank lines between
        assert_eq!(output, "Line1\nLine2\nLine3\n");
    }

    #[test]
    fn test_no_extra_blank_line_after_horizontal_rule() {
        // Regression: horizontal rule added extra blank line after
        let input = "Before\n-----\nAfter";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert_eq!(output, "Before\n---\nAfter\n");
    }

    #[test]
    fn test_no_extra_blank_lines_around_code_block() {
        // Regression: code blocks added extra blank lines before/after
        let input = "Before\n[@code python]\n\tprint('hello')\nAfter";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        // Should have no extra blank lines
        assert!(output.contains("Before\n```python"));
        assert!(output.contains("```\nAfter"));
    }

    #[test]
    fn test_no_extra_blank_line_after_quote() {
        // Regression: quotes added extra blank line after
        let input = "Before\n[@quote]\n\tquoted text\nAfter";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("> quoted text\nAfter"));
    }

    #[test]
    fn test_no_extra_blank_line_after_table() {
        // Regression: tables added extra blank line after
        let input = "Before\n[@table]\n\ta\tb\n\t1\t2\nAfter";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        assert!(output.contains("| 1 | 2 |\nAfter"));
    }

    #[test]
    fn test_explicit_blank_lines_preserved() {
        // Explicit blank lines in original should be preserved (not doubled)
        let input = "Line1\n\nLine2";
        let output = render_markdown(input, MarkdownFlavor::Standard);

        // Should have exactly one blank line between
        assert_eq!(output, "Line1\n\nLine2\n");
    }
}
