//! Integration tests for markdown import

use patto::importer::{ImportMode, ImportOptions, MarkdownImporter, MarkdownInputFlavor};

fn import(md: &str, mode: ImportMode) -> String {
    let options = ImportOptions::new(mode);
    let importer = MarkdownImporter::new(options);
    let result = importer.import(md, "test.md", "test.pn").unwrap();
    result.patto_content
}

fn import_lossy(md: &str) -> String {
    import(md, ImportMode::Lossy)
}

#[test]
fn test_full_document_conversion() {
    let md = r#"# My Document

This is the introduction paragraph.

## Section 1

Some content with **bold** and *italic* text.

- List item 1
- List item 2
  - Nested item

## Tasks

- [ ] Todo task 📅 2024-12-31
- [x] Completed task

## Code Example

```python
def hello():
    print("Hello, World!")
```

## Table

| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |

## Quote

> This is a quote
> spanning multiple lines

[Link to other note](other.md)
"#;

    let patto = import_lossy(md);

    // Check headings
    assert!(
        patto.contains("My Document\n---"),
        "H1 should become text + horizontal line"
    );
    assert!(
        patto.contains("[* Section 1]"),
        "H2 should become emphasized text"
    );

    // Check content
    assert!(patto.contains("introduction paragraph"));

    // Check decorations
    assert!(patto.contains("[* bold]"), "Bold should be converted");
    assert!(patto.contains("[/ italic]"), "Italic should be converted");

    // Check lists
    assert!(
        patto.contains("\tList item 1"),
        "List items should be indented"
    );
    assert!(
        patto.contains("\t\tNested item"),
        "Nested items should have double indent"
    );

    // Check tasks
    assert!(
        patto.contains("{@task status=todo due=2024-12-31}"),
        "Task with date should be converted"
    );
    assert!(
        patto.contains("{@task status=done}"),
        "Completed task should be converted"
    );

    // Check code
    assert!(
        patto.contains("[@code python]"),
        "Code block should be converted"
    );
    assert!(patto.contains("def hello():"), "Code content preserved");

    // Check table
    assert!(patto.contains("[@table]"), "Table should be converted");
    assert!(
        patto.contains("Header 1\tHeader 2"),
        "Table header should be tab-separated"
    );

    // Check quote
    assert!(patto.contains("[@quote]"), "Quote should be converted");

    // Check links
    assert!(
        patto.contains("[other]"),
        "Internal link should become wikilink"
    );
}

#[test]
fn test_obsidian_wikilinks() {
    // Note: pulldown-cmark doesn't parse [[wikilinks]] natively
    // They would appear as text, which we can detect and convert
    let md = "See [[other note]] for details";
    let patto = import_lossy(md);
    // For now, wikilinks in input text are preserved as-is
    // A future enhancement could detect and convert them
    assert!(patto.contains("other note"));
}

#[test]
fn test_task_status_detection() {
    let md = r#"
- [ ] unchecked - todo
- [x] checked - done
- [X] uppercase checked - done
"#;
    let patto = import_lossy(md);

    // All should have task properties
    assert!(
        patto.contains("{@task status=todo}"),
        "Unchecked becomes todo"
    );
    // Both lowercase and uppercase x should be done
    let done_count = patto.matches("{@task status=done}").count();
    assert_eq!(done_count, 2, "Both [x] and [X] should become done");
}

#[test]
fn test_due_date_extraction() {
    let md = r#"
- [ ] task with emoji 📅 2024-01-15
- [ ] task with parens (due: 2024-02-20)
- [ ] task with dataview [due:: 2024-03-25]
- [ ] task without date
"#;
    let patto = import_lossy(md);

    assert!(patto.contains("due=2024-01-15"), "Emoji date extracted");
    assert!(patto.contains("due=2024-02-20"), "Parens date extracted");
    assert!(patto.contains("due=2024-03-25"), "Dataview date extracted");

    // Task without date should just have status
    let lines: Vec<&str> = patto.lines().collect();
    let no_date_line = lines.iter().find(|l| l.contains("without date")).unwrap();
    assert!(
        !no_date_line.contains("due="),
        "Task without date should not have due="
    );
}

#[test]
fn test_preserve_mode_wraps_html() {
    let options = ImportOptions::new(ImportMode::Preserve);
    let importer = MarkdownImporter::new(options);

    let md = "Normal text\n\n<div>some html</div>\n\nMore text";
    let result = importer.import(md, "test.md", "test.pn").unwrap();

    // HTML should be wrapped in code block
    assert!(
        result.patto_content.contains("[@code html]"),
        "HTML should be in code block"
    );
    assert!(
        result.patto_content.contains("<div>some html</div>"),
        "HTML content preserved"
    );
}

#[test]
fn test_strict_mode_fails_on_unsupported() {
    let options = ImportOptions::new(ImportMode::Strict);
    let importer = MarkdownImporter::new(options);

    // Use block-level HTML which pulldown-cmark treats as HTML event
    let md = "<div>block html</div>";
    let result = importer.import(md, "test.md", "test.pn");

    assert!(result.is_err(), "Strict mode should fail on block HTML");
}

#[test]
fn test_conversion_report() {
    let options = ImportOptions::new(ImportMode::Lossy);
    let importer = MarkdownImporter::new(options);

    let md = "# Heading\n\n- [ ] Task\n\n```code\ntest\n```";
    let result = importer.import(md, "test.md", "test.pn").unwrap();

    let report = &result.report;

    // Check statistics
    assert!(report.statistics.feature_counts.contains_key("headings"));
    assert!(report.statistics.feature_counts.contains_key("tasks"));
    assert!(report.statistics.feature_counts.contains_key("code_blocks"));

    // Check warnings for heading conversion
    assert!(
        !report.warnings.is_empty(),
        "Should have warning for heading conversion"
    );
}

#[test]
fn test_flavor_detection() {
    // Obsidian flavor (wikilinks, emoji tasks)
    assert_eq!(
        MarkdownImporter::detect_flavor("[[wikilink]]"),
        MarkdownInputFlavor::Obsidian
    );
    assert_eq!(
        MarkdownImporter::detect_flavor("- [ ] task 📅 2024-12-31"),
        MarkdownInputFlavor::Obsidian
    );

    // GitHub flavor (mentions)
    assert_eq!(
        MarkdownImporter::detect_flavor("cc @username"),
        MarkdownInputFlavor::GitHub
    );

    // Standard
    assert_eq!(
        MarkdownImporter::detect_flavor("Just plain text"),
        MarkdownInputFlavor::Standard
    );
}

#[test]
fn test_inline_code() {
    let patto = import_lossy("Use `some_function()` to call it");
    assert!(
        patto.contains("[` some_function() `]"),
        "Inline code should be converted"
    );
}

#[test]
fn test_image_conversion() {
    let patto = import_lossy("![alt text](image.png)");
    assert!(patto.contains("[@img"), "Image should be converted");
    assert!(patto.contains("image.png"), "Image URL preserved");
}

#[test]
fn test_horizontal_rule() {
    let patto = import_lossy("Above\n\n---\n\nBelow");
    let dash_lines = patto.lines().filter(|l| l.trim() == "---").count();
    assert!(dash_lines >= 1, "Horizontal rule should be preserved");
}

#[test]
fn test_complex_nesting() {
    let md = r#"
- Level 1
  - Level 2
    - Level 3
      - Level 4
"#;
    let patto = import_lossy(md);

    // Check increasing indentation
    assert!(patto.contains("\tLevel 1"), "Level 1 should have 1 tab");
    assert!(patto.contains("\t\tLevel 2"), "Level 2 should have 2 tabs");
    assert!(
        patto.contains("\t\t\tLevel 3"),
        "Level 3 should have 3 tabs"
    );
    assert!(
        patto.contains("\t\t\t\tLevel 4"),
        "Level 4 should have 4 tabs"
    );
}

#[test]
fn test_report_json_serialization() {
    let options = ImportOptions::new(ImportMode::Lossy);
    let importer = MarkdownImporter::new(options);

    let md = "# Test\n\n- item";
    let result = importer.import(md, "test.md", "test.pn").unwrap();

    let json = result.report.to_json().unwrap();

    // Verify JSON structure
    assert!(json.contains("\"input_file\": \"test.md\""));
    assert!(json.contains("\"output_file\": \"test.pn\""));
    assert!(json.contains("\"mode\": \"Lossy\""));
    assert!(json.contains("\"statistics\""));
    assert!(json.contains("\"warnings\""));
}

#[test]
fn test_report_text_format() {
    let options = ImportOptions::new(ImportMode::Lossy);
    let importer = MarkdownImporter::new(options);

    let md = "# Test";
    let result = importer.import(md, "test.md", "test.pn").unwrap();

    let text = result.report.to_text();

    // Verify text structure
    assert!(text.contains("Markdown Import Report"));
    assert!(text.contains("Input:  test.md"));
    assert!(text.contains("Output: test.pn"));
    assert!(text.contains("Mode:   lossy"));
}

#[test]
fn test_mixed_paragraphs_and_lists() {
    // Test case inspired by README.md structure: paragraphs should NOT be indented
    // but list items SHOULD be indented
    let md = r#"# Patto Note

A simple plain-text format for note-taking.

## Description

Patto Note is a text format designed for outlining.

## Features

* Primary Zettelkasten support
* Real-time Preview support
* Task management
* Language Server Protocol support
    * asynchronous scanning
    * diagnostics
    * jumping between notes

## Another Section

Some more paragraph text here.
"#;

    let patto = import_lossy(md);

    // Paragraphs should NOT be indented
    assert!(
        patto.contains("\nA simple plain-text format"),
        "Paragraph after h1 should not be indented"
    );
    assert!(
        patto.contains("\nPatto Note is a text format"),
        "Paragraph after h2 should not be indented"
    );
    assert!(
        patto.contains("\nSome more paragraph text"),
        "Paragraph after list should not be indented"
    );

    // List items SHOULD be indented
    assert!(
        patto.contains("\n\tPrimary Zettelkasten"),
        "First-level list item should have 1 tab"
    );
    assert!(
        patto.contains("\n\tReal-time Preview"),
        "First-level list item should have 1 tab"
    );
    assert!(
        patto.contains("\n\tLanguage Server Protocol"),
        "First-level list item should have 1 tab"
    );

    // Nested list items should have 2 tabs
    assert!(
        patto.contains("\n\t\tasynchronous scanning"),
        "Nested list item should have 2 tabs"
    );
    assert!(
        patto.contains("\n\t\tdiagnostics"),
        "Nested list item should have 2 tabs"
    );
}

#[test]
fn test_multiple_separate_lists() {
    // Multiple separate lists should each have their own indentation context
    let md = r#"First paragraph.

- List 1 item 1
- List 1 item 2

Middle paragraph.

- List 2 item 1
- List 2 item 2
  - List 2 nested

Final paragraph.
"#;

    let patto = import_lossy(md);

    // Paragraphs should NOT be indented (check they don't have tabs before them)
    assert!(
        patto.starts_with("First paragraph") || patto.contains("\nFirst paragraph"),
        "First paragraph should not be indented"
    );
    assert!(
        patto.contains("\nMiddle paragraph"),
        "Middle paragraph should not be indented"
    );
    assert!(
        patto.contains("\nFinal paragraph"),
        "Final paragraph should not be indented"
    );

    // All list items should be indented
    assert!(
        patto.contains("\n\tList 1 item 1"),
        "First list items should have 1 tab"
    );
    assert!(
        patto.contains("\n\tList 2 item 1"),
        "Second list items should have 1 tab"
    );
    assert!(
        patto.contains("\n\t\tList 2 nested"),
        "Nested item should have 2 tabs"
    );
}

#[test]
fn test_code_blocks_not_indented_by_lists() {
    // Code blocks should not be affected by list indentation context
    let md = r#"Some text.

- List item

```python
print("hello")
```

More text.
"#;

    let patto = import_lossy(md);

    // Code block should start at root level (no extra indent from list context)
    assert!(
        patto.contains("\n[@code python]"),
        "Code block should not be indented due to list"
    );
}

#[test]
fn test_headings_not_indented_by_lists() {
    // Headings should not be affected by surrounding list content
    let md = r#"- List before

## Heading

- List after
"#;

    let patto = import_lossy(md);

    // Heading should not be indented
    assert!(
        patto.contains("\n[* Heading]"),
        "Heading should not be indented"
    );
}

#[test]
fn test_deep_nesting_with_paragraphs() {
    let md = r#"Intro paragraph.

- Level 1 item 1
  - Level 2 item 1
    - Level 3 item 1
  - Level 2 item 2
- Level 1 item 2

Outro paragraph.
"#;

    let patto = import_lossy(md);

    // Check paragraphs are not indented
    assert!(
        patto.starts_with("Intro paragraph") || patto.contains("\nIntro paragraph"),
        "Intro should not be indented"
    );
    assert!(
        patto.contains("\nOutro paragraph"),
        "Outro should not be indented"
    );

    // Check list structure
    assert!(patto.contains("\n\tLevel 1 item 1"));
    assert!(patto.contains("\n\t\tLevel 2 item 1"));
    assert!(patto.contains("\n\t\t\tLevel 3 item 1"));
    assert!(patto.contains("\n\t\tLevel 2 item 2"));
    assert!(patto.contains("\n\tLevel 1 item 2"));
}

#[test]
fn test_links_in_lists() {
    let md = r#"- Item with [internal link](note.md)
- Item with [external link](https://example.com)
  - Nested with [[wikilink]]
"#;

    let options = ImportOptions::new(ImportMode::Lossy).with_flavor(MarkdownInputFlavor::Obsidian);
    let importer = MarkdownImporter::new(options);
    let result = importer.import(md, "test.md", "test.pn").unwrap();
    let patto = result.patto_content;

    // All items should be properly indented
    assert!(patto.contains("\n\tItem with [note]") || patto.starts_with("\tItem with [note]"));
    assert!(patto.contains("\n\tItem with [external link https://example.com]"));
    // Wikilinks in text are preserved as-is (pulldown-cmark doesn't parse them)
    assert!(patto.contains("\n\t\tNested with [[wikilink]]"));
}

#[test]
fn test_blockquote_not_affected_by_list_context() {
    let md = r#"- List item

> Quote text

- Another item
"#;

    let patto = import_lossy(md);

    // Quote should be at root level
    assert!(
        patto.contains("\n[@quote]"),
        "Quote block should not be indented"
    );
}

#[test]
fn test_strikethrough_conversion() {
    let md = r#"This is ~~strikethrough~~ text.

Combined **~~bold strikethrough~~** and *~~italic strikethrough~~*.
"#;

    let patto = import_lossy(md);

    // Simple strikethrough
    assert!(
        patto.contains("[- strikethrough]"),
        "Strikethrough should be converted to [- ...]"
    );

    // Combined bold + strikethrough
    assert!(
        patto.contains("[*- bold strikethrough]"),
        "Bold strikethrough should be converted to [*- ...]"
    );

    // Combined italic + strikethrough
    assert!(
        patto.contains("[/- italic strikethrough]"),
        "Italic strikethrough should be converted to [/- ...]"
    );
}
