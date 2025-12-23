//! Markdown to Patto converter
//!
//! Converts markdown content to patto format using pulldown-cmark for parsing.

use super::options::{ImportMode, ImportOptions, MarkdownInputFlavor};
use super::report::{ConversionReport, ImportWarning, WarningKind};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::time::Instant;

/// Error type for import operations
#[derive(Debug, Clone)]
pub struct ImportError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ImportError {}

/// Result of markdown import
#[derive(Debug)]
pub struct ImportResult {
    /// Converted patto content
    pub patto_content: String,
    /// Conversion report
    pub report: ConversionReport,
}

/// Markdown to Patto importer
pub struct MarkdownImporter {
    options: ImportOptions,
}

impl MarkdownImporter {
    /// Create a new importer with the given options
    pub fn new(options: ImportOptions) -> Self {
        Self { options }
    }

    /// Detect the markdown flavor from content
    pub fn detect_flavor(content: &str) -> MarkdownInputFlavor {
        // Check for Obsidian-specific syntax
        let obsidian_wikilink = Regex::new(r"\[\[[^\]]+\]\]").unwrap();
        let obsidian_block_ref = Regex::new(r"\s\^[a-zA-Z0-9-]+$").unwrap();
        let obsidian_dataview = Regex::new(r"\[due::\s*\d{4}-\d{2}-\d{2}\]").unwrap();
        let obsidian_task_emoji = Regex::new(r"📅\s*\d{4}-\d{2}-\d{2}").unwrap();

        if obsidian_wikilink.is_match(content)
            || obsidian_block_ref.is_match(content)
            || obsidian_dataview.is_match(content)
            || obsidian_task_emoji.is_match(content)
        {
            return MarkdownInputFlavor::Obsidian;
        }

        // Check for GitHub-specific syntax (task lists with mentions)
        let github_mention = Regex::new(r"@[a-zA-Z0-9_-]+").unwrap();
        let github_issue_ref = Regex::new(r"#\d+").unwrap();
        if github_mention.is_match(content) || github_issue_ref.is_match(content) {
            return MarkdownInputFlavor::GitHub;
        }

        MarkdownInputFlavor::Standard
    }

    /// Import markdown content to patto format
    pub fn import(
        &self,
        markdown: &str,
        input_path: &str,
        output_path: &str,
    ) -> Result<ImportResult, ImportError> {
        let start_time = Instant::now();

        // Detect or use specified flavor
        let flavor = self
            .options
            .flavor
            .unwrap_or_else(|| Self::detect_flavor(markdown));

        let mut report = ConversionReport::new(input_path, output_path, self.options.mode, flavor);
        report.statistics.total_lines = markdown.lines().count();

        // Convert
        let patto_content = self.convert_markdown(markdown, &mut report)?;

        report.statistics.converted_lines =
            report.statistics.total_lines - report.statistics.failed_lines;
        report.duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(ImportResult {
            patto_content,
            report,
        })
    }

    /// Convert markdown content to patto format
    fn convert_markdown(
        &self,
        markdown: &str,
        report: &mut ConversionReport,
    ) -> Result<String, ImportError> {
        let mut output = String::new();
        let mut current_line = 1;

        // Enable all markdown extensions
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_FOOTNOTES);

        let parser = Parser::new_ext(markdown, options);

        // State tracking
        let mut indent_level: usize = 0;
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_content: Vec<String> = Vec::new();
        let mut in_table = false;
        let mut table_rows: Vec<Vec<String>> = Vec::new();
        let mut current_row: Vec<String> = Vec::new();
        let mut current_cell = String::new();
        let mut in_blockquote = false;
        let mut blockquote_content: Vec<String> = Vec::new();
        let mut list_stack: Vec<bool> = Vec::new(); // true = ordered, false = unordered
        let mut current_task_status: Option<bool> = None; // Some(true) = checked, Some(false) = unchecked
        let mut in_heading = false;
        let mut heading_level: u8 = 0;
        let mut heading_text = String::new();
        let mut pending_text = String::new();
        let mut in_emphasis = false;
        let mut in_strong = false;
        let mut in_link = false;
        let mut link_url = String::new();
        let mut link_text = String::new();

        for event in parser {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Heading { level, .. } => {
                            in_heading = true;
                            heading_level = match level {
                                HeadingLevel::H1 => 1,
                                HeadingLevel::H2 => 2,
                                HeadingLevel::H3 => 3,
                                HeadingLevel::H4 => 4,
                                HeadingLevel::H5 => 5,
                                HeadingLevel::H6 => 6,
                            };
                            heading_text.clear();
                            report.statistics.increment_feature("headings");
                        }
                        Tag::List(ordered) => {
                            // Flush any pending text before starting a nested list
                            if !pending_text.is_empty() && !list_stack.is_empty() {
                                self.write_indent(&mut output, indent_level);
                                if let Some(checked) = current_task_status {
                                    let status = if checked { "done" } else { "todo" };
                                    let due = self.extract_due_date(&pending_text);
                                    let text = self.strip_due_date(&pending_text);

                                    if let Some(due) = due {
                                        output.push_str(&format!(
                                            "{} {{@task status={} due={}}}\n",
                                            text.trim(),
                                            status,
                                            due
                                        ));
                                    } else {
                                        output.push_str(&format!(
                                            "{} {{@task status={}}}\n",
                                            text.trim(),
                                            status
                                        ));
                                    }
                                    report.statistics.increment_feature("tasks");
                                } else {
                                    output.push_str(&format!("{}\n", pending_text.trim()));
                                }
                                pending_text.clear();
                                current_task_status = None;
                            }
                            list_stack.push(ordered.is_some());
                            report.statistics.increment_feature("lists");
                        }
                        Tag::Item => {
                            indent_level = list_stack.len();
                            current_task_status = None;
                        }
                        Tag::CodeBlock(kind) => {
                            in_code_block = true;
                            code_lang = match kind {
                                pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                                pulldown_cmark::CodeBlockKind::Indented => String::new(),
                            };
                            code_content.clear();
                            report.statistics.increment_feature("code_blocks");
                        }
                        Tag::BlockQuote(_) => {
                            in_blockquote = true;
                            blockquote_content.clear();
                            report.statistics.increment_feature("blockquotes");
                        }
                        Tag::Table(_) => {
                            in_table = true;
                            table_rows.clear();
                            report.statistics.increment_feature("tables");
                        }
                        Tag::TableHead => {
                            current_row.clear();
                        }
                        Tag::TableRow => {
                            current_row.clear();
                        }
                        Tag::TableCell => {
                            current_cell.clear();
                        }
                        Tag::Emphasis => {
                            in_emphasis = true;
                        }
                        Tag::Strong => {
                            in_strong = true;
                        }
                        Tag::Link { dest_url, .. } => {
                            in_link = true;
                            link_url = dest_url.to_string();
                            link_text.clear();
                            report.statistics.increment_feature("links");
                        }
                        Tag::Image {
                            dest_url, title, ..
                        } => {
                            // Output image immediately
                            let alt = if title.is_empty() {
                                None
                            } else {
                                Some(title.to_string())
                            };
                            self.write_indent(&mut output, indent_level);
                            if let Some(alt) = alt {
                                output.push_str(&format!("[@img {} \"{}\"]\n", dest_url, alt));
                            } else {
                                output.push_str(&format!("[@img {}]\n", dest_url));
                            }
                            report.statistics.increment_feature("images");
                        }
                        Tag::Paragraph => {
                            // Paragraphs are handled implicitly
                        }
                        Tag::FootnoteDefinition(_) => {
                            // Handle based on mode
                            match self.options.mode {
                                ImportMode::Strict => {
                                    return Err(ImportError {
                                        line: current_line,
                                        message: "Footnotes are not supported by patto".to_string(),
                                    });
                                }
                                ImportMode::Lossy => {
                                    report.add_warning(ImportWarning {
                                        line: current_line,
                                        column: None,
                                        kind: WarningKind::UnsupportedFeature,
                                        feature: "footnote".to_string(),
                                        message: "Dropped footnote definition".to_string(),
                                        suggestion: Some(
                                            "Move footnote content inline".to_string(),
                                        ),
                                    });
                                    report.statistics.increment_unsupported("footnotes");
                                }
                                ImportMode::Preserve => {
                                    // Will be handled in text event
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::End(tag_end) => {
                    match tag_end {
                        TagEnd::Heading(_) => {
                            in_heading = false;
                            // Convert heading to patto format
                            // H1 gets text + horizontal line, others get decoration
                            if heading_level == 1 {
                                output.push_str(&heading_text);
                                output.push('\n');
                                output.push_str("---\n");
                            } else {
                                // Use bold decoration for h2-h6
                                self.write_indent(&mut output, indent_level);
                                output.push_str(&format!("[* {}]\n", heading_text.trim()));
                            }
                            // Add warning about heading conversion
                            report.add_warning(ImportWarning {
                                line: current_line,
                                column: None,
                                kind: WarningKind::LossyConversion,
                                feature: "heading".to_string(),
                                message: format!(
                                    "Converted h{} heading to {}",
                                    heading_level,
                                    if heading_level == 1 {
                                        "text with horizontal line"
                                    } else {
                                        "emphasized text"
                                    }
                                ),
                                suggestion: None,
                            });
                        }
                        TagEnd::List(_) => {
                            list_stack.pop();
                            indent_level = list_stack.len();
                        }
                        TagEnd::Item => {
                            // Flush pending text
                            if !pending_text.is_empty() {
                                self.write_indent(&mut output, indent_level);

                                // Add task status if present
                                if let Some(checked) = current_task_status {
                                    let status = if checked { "done" } else { "todo" };
                                    let due = self.extract_due_date(&pending_text);
                                    let text = self.strip_due_date(&pending_text);

                                    if let Some(due) = due {
                                        output.push_str(&format!(
                                            "{} {{@task status={} due={}}}\n",
                                            text.trim(),
                                            status,
                                            due
                                        ));
                                    } else {
                                        output.push_str(&format!(
                                            "{} {{@task status={}}}\n",
                                            text.trim(),
                                            status
                                        ));
                                    }
                                    report.statistics.increment_feature("tasks");
                                } else {
                                    output.push_str(&format!("{}\n", pending_text.trim()));
                                }
                                pending_text.clear();
                            }
                            current_task_status = None;
                        }
                        TagEnd::CodeBlock => {
                            in_code_block = false;
                            // Output code block
                            self.write_indent(&mut output, indent_level);
                            if code_lang.is_empty() {
                                output.push_str("[@code]\n");
                            } else {
                                output.push_str(&format!("[@code {}]\n", code_lang));
                            }
                            for line in &code_content {
                                self.write_indent(&mut output, indent_level + 1);
                                output.push_str(line);
                                output.push('\n');
                            }
                        }
                        TagEnd::BlockQuote(_) => {
                            in_blockquote = false;
                            // Output blockquote
                            self.write_indent(&mut output, indent_level);
                            output.push_str("[@quote]\n");
                            for line in &blockquote_content {
                                self.write_indent(&mut output, indent_level + 1);
                                output.push_str(line);
                                output.push('\n');
                            }
                        }
                        TagEnd::Table => {
                            in_table = false;
                            // Output table
                            self.write_indent(&mut output, indent_level);
                            output.push_str("[@table]\n");
                            for row in &table_rows {
                                self.write_indent(&mut output, indent_level + 1);
                                output.push_str(&row.join("\t"));
                                output.push('\n');
                            }
                        }
                        TagEnd::TableHead | TagEnd::TableRow => {
                            table_rows.push(current_row.clone());
                        }
                        TagEnd::TableCell => {
                            current_row.push(current_cell.clone());
                        }
                        TagEnd::Emphasis => {
                            in_emphasis = false;
                        }
                        TagEnd::Strong => {
                            in_strong = false;
                        }
                        TagEnd::Link => {
                            in_link = false;
                            // Convert link to patto format
                            let patto_link = self.convert_link(&link_url, &link_text);
                            if in_heading {
                                heading_text.push_str(&patto_link);
                            } else {
                                pending_text.push_str(&patto_link);
                            }
                        }
                        TagEnd::Paragraph => {
                            // Flush pending text
                            if !pending_text.is_empty() {
                                self.write_indent(&mut output, indent_level);
                                output.push_str(&format!("{}\n", pending_text.trim()));
                                pending_text.clear();
                            }
                        }
                        _ => {}
                    }
                }
                Event::Text(text) => {
                    let text_str = text.to_string();
                    current_line += text_str.matches('\n').count();

                    if in_code_block {
                        // Add to code block content
                        for line in text_str.lines() {
                            code_content.push(line.to_string());
                        }
                    } else if in_table {
                        current_cell.push_str(&text_str);
                    } else if in_blockquote {
                        for line in text_str.lines() {
                            blockquote_content.push(line.to_string());
                        }
                    } else if in_heading {
                        heading_text.push_str(&text_str);
                    } else if in_link {
                        link_text.push_str(&text_str);
                    } else {
                        // Apply decorations
                        let decorated = if in_strong && in_emphasis {
                            format!("[*/ {}]", text_str)
                        } else if in_strong {
                            format!("[* {}]", text_str)
                        } else if in_emphasis {
                            format!("[/ {}]", text_str)
                        } else {
                            text_str
                        };
                        pending_text.push_str(&decorated);
                    }
                }
                Event::Code(code) => {
                    // Inline code
                    let patto_code = format!("[` {} `]", code);
                    if in_heading {
                        heading_text.push_str(&patto_code);
                    } else {
                        pending_text.push_str(&patto_code);
                    }
                    report.statistics.increment_feature("inline_code");
                }
                Event::Html(html) => {
                    let html_str = html.to_string();
                    current_line += html_str.matches('\n').count();

                    match self.options.mode {
                        ImportMode::Strict => {
                            return Err(ImportError {
                                line: current_line,
                                message: format!(
                                    "HTML is not supported by patto: {}",
                                    html_str.trim()
                                ),
                            });
                        }
                        ImportMode::Lossy => {
                            report.add_warning(ImportWarning {
                                line: current_line,
                                column: None,
                                kind: WarningKind::UnsupportedFeature,
                                feature: "html".to_string(),
                                message: format!("Dropped HTML: {}", html_str.trim()),
                                suggestion: Some(
                                    "Use plain text or patto markup instead".to_string(),
                                ),
                            });
                            report.statistics.increment_unsupported("html");
                        }
                        ImportMode::Preserve => {
                            // Wrap in code block
                            self.write_indent(&mut output, indent_level);
                            output.push_str("[@code html]\n");
                            for line in html_str.lines() {
                                self.write_indent(&mut output, indent_level + 1);
                                output.push_str(line);
                                output.push('\n');
                            }
                            report.add_warning(ImportWarning {
                                line: current_line,
                                column: None,
                                kind: WarningKind::PreservedContent,
                                feature: "html".to_string(),
                                message: "Preserved HTML in code block for manual editing"
                                    .to_string(),
                                suggestion: None,
                            });
                        }
                    }
                }
                Event::SoftBreak => {
                    if in_heading {
                        heading_text.push(' ');
                    } else if !in_code_block {
                        pending_text.push(' ');
                    }
                }
                Event::HardBreak => {
                    current_line += 1;
                    if !in_code_block && !pending_text.is_empty() {
                        self.write_indent(&mut output, indent_level);
                        output.push_str(&format!("{}\n", pending_text.trim()));
                        pending_text.clear();
                    }
                }
                Event::Rule => {
                    self.write_indent(&mut output, indent_level);
                    output.push_str("---\n");
                    report.statistics.increment_feature("horizontal_rules");
                }
                Event::TaskListMarker(checked) => {
                    current_task_status = Some(checked);
                }
                Event::FootnoteReference(name) => match self.options.mode {
                    ImportMode::Strict => {
                        return Err(ImportError {
                            line: current_line,
                            message: format!(
                                "Footnote reference [^{}] is not supported by patto",
                                name
                            ),
                        });
                    }
                    ImportMode::Lossy => {
                        report.add_warning(ImportWarning {
                            line: current_line,
                            column: None,
                            kind: WarningKind::UnsupportedFeature,
                            feature: "footnote_ref".to_string(),
                            message: format!("Dropped footnote reference [^{}]", name),
                            suggestion: Some("Move footnote content inline".to_string()),
                        });
                        report.statistics.increment_unsupported("footnotes");
                    }
                    ImportMode::Preserve => {
                        pending_text.push_str(&format!("[^{}]", name));
                    }
                },
                _ => {}
            }
        }

        // Flush any remaining pending text
        if !pending_text.is_empty() {
            self.write_indent(&mut output, indent_level);
            output.push_str(&format!("{}\n", pending_text.trim()));
        }

        Ok(output)
    }

    /// Write indentation (tabs)
    fn write_indent(&self, output: &mut String, level: usize) {
        for _ in 0..level {
            output.push('\t');
        }
    }

    /// Convert a markdown link to patto format
    fn convert_link(&self, url: &str, text: &str) -> String {
        // Check if it's a wikilink (Obsidian format)
        if let Some(captures) = Regex::new(r"^\[\[([^\]]+)\]\]$").unwrap().captures(url) {
            let link = captures.get(1).map_or("", |m| m.as_str());
            return format!("[{}]", link);
        }

        // Check if it's a self-anchor link
        if url.starts_with('#') {
            return format!("[{}]", url);
        }

        // Check if it's an internal link (ends with .md or .pn)
        if url.ends_with(".md")
            || url.ends_with(".pn")
            || url.contains(".md#")
            || url.contains(".pn#")
        {
            // Handle anchor in URL
            if let Some(hash_pos) = url.find('#') {
                let (file_part, anchor) = url.split_at(hash_pos);
                let note_name = file_part.trim_end_matches(".md").trim_end_matches(".pn");
                if note_name.is_empty() {
                    return format!("[{}]", anchor);
                } else {
                    return format!("[{}{}]", note_name, anchor);
                }
            }
            let note_name = url.trim_end_matches(".md").trim_end_matches(".pn");
            return format!("[{}]", note_name);
        }

        // External URL
        if text.is_empty() || text == url {
            format!("[{}]", url)
        } else {
            format!("[{} {}]", text, url)
        }
    }

    /// Extract due date from task text
    fn extract_due_date(&self, text: &str) -> Option<String> {
        // Pattern: 📅 2024-12-31
        if let Some(captures) = Regex::new(r"📅\s*(\d{4}-\d{2}-\d{2})")
            .unwrap()
            .captures(text)
        {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        // Pattern: (due: 2024-12-31)
        if let Some(captures) = Regex::new(r"\(due:\s*(\d{4}-\d{2}-\d{2})\)")
            .unwrap()
            .captures(text)
        {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        // Pattern: [due:: 2024-12-31]
        if let Some(captures) = Regex::new(r"\[due::\s*(\d{4}-\d{2}-\d{2})\]")
            .unwrap()
            .captures(text)
        {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        // Pattern: @2024-12-31
        if let Some(captures) = Regex::new(r"@(\d{4}-\d{2}-\d{2})").unwrap().captures(text) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        None
    }

    /// Strip due date patterns from text
    fn strip_due_date(&self, text: &str) -> String {
        let patterns = [
            r"📅\s*\d{4}-\d{2}-\d{2}",
            r"\(due:\s*\d{4}-\d{2}-\d{2}\)",
            r"\[due::\s*\d{4}-\d{2}-\d{2}\]",
            r"@\d{4}-\d{2}-\d{2}",
        ];

        let mut result = text.to_string();
        for pattern in patterns {
            let re = Regex::new(pattern).unwrap();
            result = re.replace_all(&result, "").to_string();
        }
        result
    }
}

#[cfg(test)]
mod tests {
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
        // Check that nested item has indentation
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
    fn test_horizontal_rule() {
        let result = import_lossy("---");
        assert!(result.patto_content.contains("---"));
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
        assert!(result.report.warnings.len() >= 1);
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
        assert_eq!(stats.feature_counts.get("lists"), Some(&1)); // one list with 2 items
        assert_eq!(stats.feature_counts.get("code_blocks"), Some(&1));
    }

    #[test]
    fn test_lossy_mode_continues_on_error() {
        // pulldown-cmark treats inline HTML differently - text after/between HTML
        // may be parsed as more HTML events. Test with more realistic cases.
        let md = "Normal text\n\n<div>html content</div>\n\nAnother paragraph";
        let result = import_lossy(md);

        // Should have warnings for HTML
        assert!(
            !result.report.warnings.is_empty(),
            "Expected warnings for HTML"
        );
        assert!(
            result.report.warnings.iter().any(|w| w.feature == "html"),
            "Expected HTML warning"
        );
        // Should still produce output for valid text
        assert!(
            result.patto_content.contains("Normal text"),
            "Missing 'Normal text'"
        );
        // The paragraph after HTML should be captured
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
        let importer = MarkdownImporter::new(ImportOptions::default());
        // Note: actual wikilinks in pulldown-cmark need to be detected differently
        // This tests the helper function directly
        assert_eq!(importer.convert_link("note.md", "note"), "[note]");
        assert_eq!(
            importer.convert_link("note.md#anchor", "note"),
            "[note#anchor]"
        );
        assert_eq!(importer.convert_link("#anchor", "section"), "[#anchor]");
        assert_eq!(
            importer.convert_link("https://example.com", "Example"),
            "[Example https://example.com]"
        );
    }

    #[test]
    fn test_extract_due_date() {
        let importer = MarkdownImporter::new(ImportOptions::default());

        assert_eq!(
            importer.extract_due_date("task 📅 2024-12-31"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            importer.extract_due_date("task (due: 2024-12-31)"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            importer.extract_due_date("task [due:: 2024-12-31]"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(
            importer.extract_due_date("task @2024-12-31"),
            Some("2024-12-31".to_string())
        );
        assert_eq!(importer.extract_due_date("task without date"), None);
    }

    #[test]
    fn test_strip_due_date() {
        let importer = MarkdownImporter::new(ImportOptions::default());

        assert_eq!(importer.strip_due_date("task 📅 2024-12-31").trim(), "task");
        assert_eq!(
            importer.strip_due_date("task (due: 2024-12-31)").trim(),
            "task"
        );
        assert_eq!(
            importer.strip_due_date("task [due:: 2024-12-31]").trim(),
            "task"
        );
    }
}
