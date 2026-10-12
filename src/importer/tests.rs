use super::converter::{ImportError, ImportResult};
use super::*;

fn import_lossy(md: &str) -> ImportResult {
    let importer = MarkdownImporter::new(ImportOptions::new(ImportMode::Lossy));
    importer.import(md, "test.md", "test.pn").unwrap()
}

fn import_strict(md: &str) -> Result<ImportResult, ImportError> {
    let importer = MarkdownImporter::new(ImportOptions::new(ImportMode::Strict));
    importer.import(md, "test.md", "test.pn")
}

fn import_preserve(md: &str) -> ImportResult {
    let importer = MarkdownImporter::new(ImportOptions::new(ImportMode::Preserve));
    importer.import(md, "test.md", "test.pn").unwrap()
}

#[test]
fn test_plain_text() {
    let result = import_lossy("Hello world");
    assert_eq!(result.patto_content.trim(), "Hello world");
}

#[test]
fn test_list_conversion() {
    let result = import_lossy("- item 1\n- item 2");
    assert!(result.patto_content.contains("item 1"));
    assert!(result.patto_content.contains("item 2"));
}

#[test]
fn test_nested_list_conversion() {
    let result = import_lossy("- item 1\n  - nested");
    let lines: Vec<&str> = result.patto_content.lines().collect();
    assert!(lines
        .iter()
        .any(|l| l.starts_with('\t') && l.contains("nested")));
}

#[test]
fn test_code_block_conversion() {
    let result = import_lossy("```python\nprint('hello')\n```");
    assert!(result.patto_content.contains("[@code python]"));
    assert!(result.patto_content.contains("print('hello')"));
}

#[test]
fn test_inline_code_conversion() {
    let result = import_lossy("Use `code` here");
    assert!(result.patto_content.contains("[` code `]"));
}

#[test]
fn test_heading_conversion_h1() {
    let result = import_lossy("# Title");
    assert!(result.patto_content.contains("Title"));
    assert!(result.patto_content.contains("---"));
    assert_eq!(result.report.warnings.len(), 1);
    assert!(result.report.warnings[0].message.contains("h1"));
}

#[test]
fn test_heading_conversion_h2() {
    let result = import_lossy("## Subtitle");
    assert!(result.patto_content.contains("[* Subtitle]"));
}

#[test]
fn test_bold_conversion() {
    let result = import_lossy("This is **bold** text");
    assert!(result.patto_content.contains("[* bold]"));
}

#[test]
fn test_italic_conversion() {
    let result = import_lossy("This is *italic* text");
    assert!(result.patto_content.contains("[/ italic]"));
}

#[test]
fn test_bold_italic_conversion() {
    let result = import_lossy("This is ***bold italic*** text");
    assert!(result.patto_content.contains("[*/ bold italic]"));
}

#[test]
fn test_link_internal() {
    let result = import_lossy("[link](note.md)");
    assert!(result.patto_content.contains("[note]"));
}

#[test]
fn test_link_external() {
    let result = import_lossy("[Google](https://google.com)");
    assert!(result.patto_content.contains("[Google https://google.com]"));
}

#[test]
fn test_link_anchor() {
    let result = import_lossy("[section](#anchor)");
    assert!(result.patto_content.contains("[#anchor]"));
}

#[test]
fn test_blockquote_conversion() {
    let result = import_lossy("> This is a quote");
    assert!(result.patto_content.contains("[@quote]"));
    assert!(result.patto_content.contains("This is a quote"));
}

#[test]
fn test_table_conversion() {
    let result = import_lossy("| h1 | h2 |\n|---|---|\n| a | b |");
    assert!(result.patto_content.contains("[@table]"));
    assert!(result.patto_content.contains("\th1\th2"));
    assert!(result.patto_content.contains("\ta\tb"));
}

#[test]
fn test_table_with_inline_content() {
    let result = import_lossy(
        "| h1 | h2 | h3 |\n|---|---|---|\n| [Google](https://google.com) | **bold** | `code` |\n\nnext paragraph",
    );
    assert!(result
        .patto_content
        .contains("\t[Google https://google.com]\t[* bold]\t[` code `]"));
    assert!(result
        .patto_content
        .lines()
        .any(|l| l.trim() == "next paragraph"));
}

#[test]
fn test_table_with_image() {
    let result = import_lossy("| h1 |\n|---|\n| ![alt](img.png) |");
    assert!(result.patto_content.contains("\t[@img img.png]"));
}

#[test]
fn test_horizontal_rule() {
    let result = import_lossy("---");
    assert!(result.patto_content.contains("-----"));
}

#[test]
fn test_task_list_unchecked() {
    let result = import_lossy("- [ ] todo task");
    assert!(result.patto_content.contains("{@task status=todo}"));
}

#[test]
fn test_task_list_checked() {
    let result = import_lossy("- [x] done task");
    assert!(result.patto_content.contains("{@task status=done}"));
}

#[test]
fn test_task_with_due_date_emoji() {
    let result = import_lossy("- [ ] task 📅 2024-12-31");
    assert!(result
        .patto_content
        .contains("{@task status=todo due=2024-12-31}"));
}

#[test]
fn test_task_with_due_date_parentheses() {
    let result = import_lossy("- [ ] task (due: 2024-12-31)");
    assert!(result
        .patto_content
        .contains("{@task status=todo due=2024-12-31}"));
}

#[test]
fn test_task_with_due_date_dataview() {
    let result = import_lossy("- [ ] task [due:: 2024-12-31]");
    assert!(result
        .patto_content
        .contains("{@task status=todo due=2024-12-31}"));
}

#[test]
fn test_strict_mode_fails_on_html() {
    let result = import_strict("<div>html</div>");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("HTML is not supported"));
}

#[test]
fn test_lossy_mode_drops_html() {
    let result = import_lossy("<div>html</div>");
    assert!(!result.report.warnings.is_empty());
    assert!(result.report.warnings.iter().any(|w| w.feature == "html"));
}

#[test]
fn test_preserve_mode_wraps_html() {
    let result = import_preserve("<div>html</div>");
    assert!(result.patto_content.contains("[@code html]"));
    assert!(result.patto_content.contains("<div>html</div>"));
}

#[test]
fn test_detect_flavor_obsidian() {
    assert_eq!(
        MarkdownImporter::detect_flavor("[[wikilink]]"),
        MarkdownInputFlavor::Obsidian
    );
    assert_eq!(
        MarkdownImporter::detect_flavor("task 📅 2024-12-31"),
        MarkdownInputFlavor::Obsidian
    );
    assert_eq!(
        MarkdownImporter::detect_flavor("[due:: 2024-12-31]"),
        MarkdownInputFlavor::Obsidian
    );
}

#[test]
fn test_detect_flavor_github() {
    assert_eq!(
        MarkdownImporter::detect_flavor("cc @username"),
        MarkdownInputFlavor::GitHub
    );
}

#[test]
fn test_detect_flavor_standard() {
    assert_eq!(
        MarkdownImporter::detect_flavor("Just normal text"),
        MarkdownInputFlavor::Standard
    );
}

#[test]
fn test_report_generation() {
    let result = import_lossy("# Title\n- item\n- [ ] task 📅 2024-12-31");
    let report = &result.report;

    assert_eq!(report.mode, ImportMode::Lossy);
    assert!(report.statistics.feature_counts.contains_key("headings"));
    assert!(report.statistics.feature_counts.contains_key("lists"));
    assert!(report.statistics.feature_counts.contains_key("tasks"));
}

#[test]
fn test_statistics_tracking() {
    let result = import_lossy("# Title\n## Subtitle\n- item 1\n- item 2\n```code\ntest\n```");
    let stats = &result.report.statistics;

    assert_eq!(stats.feature_counts.get("headings"), Some(&2));
    assert_eq!(stats.feature_counts.get("lists"), Some(&1));
    assert_eq!(stats.feature_counts.get("code_blocks"), Some(&1));
}

#[test]
fn test_lossy_mode_continues_on_error() {
    let md = "Normal text\n\n<div>html content</div>\n\nAnother paragraph";
    let result = import_lossy(md);

    assert!(
        !result.report.warnings.is_empty(),
        "Expected warnings for HTML"
    );
    assert!(
        result.report.warnings.iter().any(|w| w.feature == "html"),
        "Expected HTML warning"
    );
    assert!(
        result.patto_content.contains("Normal text"),
        "Missing 'Normal text'"
    );
    assert!(
        result.patto_content.contains("Another paragraph"),
        "Missing 'Another paragraph', content: {}",
        result.patto_content
    );
}

#[test]
fn test_image_conversion() {
    let result = import_lossy("![alt text](image.png)");
    assert!(result.patto_content.contains("[@img"));
    assert!(result.patto_content.contains("image.png"));
}

#[test]
fn test_convert_link_wikilink() {
    let result = import_lossy("[note](note.md)");
    assert!(
        result.patto_content.contains("[note]"),
        "Internal .md link should become wikilink"
    );

    let result = import_lossy("[text](note.md#anchor)");
    assert!(
        result.patto_content.contains("[note#anchor]"),
        "Link with anchor should preserve anchor"
    );

    let result = import_lossy("[section](#anchor)");
    assert!(
        result.patto_content.contains("[#anchor]"),
        "Self-anchor link"
    );

    let result = import_lossy("[Example](https://example.com)");
    assert!(
        result
            .patto_content
            .contains("[Example https://example.com]"),
        "External URL"
    );
}
