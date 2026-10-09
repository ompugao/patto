//! Turns parser errors into messages a note author can act on: what went
//! wrong, how to write it, and a few examples.

use std::borrow::Cow;
use tower_lsp::lsp_types::DiagnosticSeverity;

use crate::parser::{ParserError, PestErrorInfo, PestErrorVariantInfo, Rule};

const DEFAULT_DOCS_BASE_URL: &str = "https://github.com/ompugao/patto/wiki/Diagnostic-Errors";

#[derive(Debug, Clone)]
pub struct FriendlyDiagnostic {
    pub message: String,
    pub code: Option<String>,
    pub code_description_uri: Option<String>,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug)]
pub struct DiagnosticTranslator {
    docs_base_url: &'static str,
}

impl Default for DiagnosticTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticTranslator {
    pub fn new() -> Self {
        Self {
            docs_base_url: DEFAULT_DOCS_BASE_URL,
        }
    }

    pub fn translate(&self, error: &ParserError) -> FriendlyDiagnostic {
        match error {
            ParserError::InvalidIndentation(_) => self.guided(&INVALID_INDENTATION),
            ParserError::ParseError(_, info) => self.translate_pest_error(info),
        }
    }

    pub fn embed_error(&self) -> FriendlyDiagnostic {
        self.guided(&EMBED)
    }

    pub fn img_error(&self) -> FriendlyDiagnostic {
        self.guided(&IMG)
    }

    fn translate_pest_error(&self, info: &PestErrorInfo) -> FriendlyDiagnostic {
        match &info.variant {
            PestErrorVariantInfo::ParsingError { positives, .. } => {
                match RuleFamily::of(positives) {
                    Some(RuleFamily::Statement) => self.statement_error(positives, info),
                    Some(family) => self.guided(family.guide()),
                    None => self.generic_error(positives, info),
                }
            }
            PestErrorVariantInfo::CustomError { message } => self.diagnostic(
                compose_message("Invalid syntax", message, &[]),
                "syntax-error",
                DiagnosticSeverity::ERROR,
            ),
        }
    }

    fn guided(&self, guide: &Guide) -> FriendlyDiagnostic {
        self.diagnostic(
            compose_message(guide.primary, guide.help, guide.examples),
            guide.code,
            guide.severity,
        )
    }

    fn statement_error(&self, positives: &[Rule], info: &PestErrorInfo) -> FriendlyDiagnostic {
        let primary = describe_expectations(positives)
            .map(|desc| format!("Couldn't understand this line – expected {}.", desc))
            .unwrap_or_else(|| "Couldn't understand this line.".to_string());
        let detail = summary_from_message(&info.message).unwrap_or_else(|| {
            "Check for missing brackets, unmatched commands, or typos in this line.".to_string()
        });
        self.diagnostic(
            compose_message(&primary, &detail, &[]),
            "line-parse-error",
            DiagnosticSeverity::ERROR,
        )
    }

    fn generic_error(&self, positives: &[Rule], info: &PestErrorInfo) -> FriendlyDiagnostic {
        let primary = describe_expectations(positives)
            .map(|desc| format!("Unexpected text – expected {}.", desc))
            .unwrap_or_else(|| "Patto couldn't understand this part.".to_string());
        let detail = summary_from_message(&info.message).unwrap_or_else(|| {
            "Make sure brackets, commands, and properties are written correctly.".to_string()
        });
        self.diagnostic(
            compose_message(&primary, &detail, &[]),
            "syntax-error",
            DiagnosticSeverity::ERROR,
        )
    }

    fn diagnostic(
        &self,
        message: String,
        code: &str,
        severity: DiagnosticSeverity,
    ) -> FriendlyDiagnostic {
        let docs_base_url = self.docs_base_url.trim_end_matches('/');
        FriendlyDiagnostic {
            message,
            code: Some(code.to_string()),
            code_description_uri: Some(format!("{}/{}", docs_base_url, code)),
            severity,
        }
    }
}

/// The fixed wording for one kind of mistake.
struct Guide {
    code: &'static str,
    primary: &'static str,
    help: &'static str,
    examples: &'static [&'static str],
    severity: DiagnosticSeverity,
}

const INVALID_INDENTATION: Guide = Guide {
    code: "invalid-indentation",
    primary: "Inconsistent indentation",
    help: "Use tabs to indent nested blocks. Child lines must be indented exactly one tab deeper than their parent.",
    examples: &["Heading", "\tChild line", "\t\tNested child"],
    severity: DiagnosticSeverity::ERROR,
};

// Embed and img mistakes are warnings: the line still renders as plain text.
const EMBED: Guide = Guide {
    code: "invalid-embed",
    primary: "Invalid embed syntax",
    help: "Use [@embed ...] with a URL or local file path. \
        Local paths must start with ./ or ../\n\
        Note: bare filenames like file.pdf are not valid; use ./file.pdf",
    examples: &[
        "[@embed https://example.com/video]",
        "[@embed https://example.com/video My Title]",
        "[@embed My Title https://example.com/video]",
        "[@embed ./path/to/file.pdf]",
        "[@embed ./path/to/file.pdf My Title]",
        "[@embed My Title ./path/to/file.pdf]   ← local paths require ./",
    ],
    severity: DiagnosticSeverity::WARNING,
};

const IMG: Guide = Guide {
    code: "invalid-img",
    primary: "Invalid image syntax",
    help: "Use [@img ...] with a URL or local file path and an optional alt text. \
        Local paths must start with ./ or ../",
    examples: &[
        "[@img https://example.com/photo.jpg]",
        "[@img https://example.com/photo.jpg My Caption]",
        "[@img My Caption https://example.com/photo.jpg]",
        "[@img ./path/to/image.jpg]",
        "[@img ./path/to/image.jpg My Caption]",
        "[@img My Caption ./path/to/image.jpg]   ← local paths require ./",
    ],
    severity: DiagnosticSeverity::WARNING,
};

const LINK: Guide = Guide {
    code: "invalid-link",
    primary: "Invalid link syntax",
    help: "Wrap links in [ ] and include a note name, anchor, URL, or file path.",
    examples: &[
        "[ProjectPlan]",
        "[ProjectPlan#milestones]",
        "[https://example.com]",
    ],
    severity: DiagnosticSeverity::ERROR,
};

const COMMAND: Guide = Guide {
    code: "invalid-command",
    primary: "Unknown or malformed command",
    help: "Commands look like [@command-name optional-args]. Available commands include @code, @math, @quote, @table, and @img.",
    examples: &["[@code rust]", "[@math]", "[@quote]"],
    severity: DiagnosticSeverity::ERROR,
};

const PROPERTY: Guide = Guide {
    code: "invalid-property",
    primary: "Invalid property syntax",
    help: "Properties use {@name key=value ...}. Separate each key/value with spaces and close the property with }.",
    examples: &["{@tag project=patto}", "{@task status=todo due=2024-12-31}"],
    severity: DiagnosticSeverity::ERROR,
};

const TASK: Guide = Guide {
    code: "invalid-task",
    primary: "Invalid task syntax",
    help: "Tasks use {@task status=<todo|doing|done> due=<YYYY-MM-DD or YYYY-MM-DDThh:mm>}. Provide both status and due date.",
    examples: &[
        "{@task status=todo due=2024-12-31}",
        "{@task status=doing due=2024-12-31T14:00}",
    ],
    severity: DiagnosticSeverity::ERROR,
};

const ANCHOR: Guide = Guide {
    code: "invalid-anchor",
    primary: "Invalid anchor",
    help: "Anchors start with # and may contain letters, numbers, _ or -. Example: #ProjectAlpha.",
    examples: &["#inbox", "[#ProjectAlpha]", "[MyNote#section]"],
    severity: DiagnosticSeverity::ERROR,
};

const INLINE_CODE: Guide = Guide {
    code: "invalid-inline-code",
    primary: "Malformed inline code",
    help: "Inline code is written as [` code `]. Make sure both the opening [` and closing `] markers are present.",
    examples: &["[` println!(\"hello\"); `]"],
    severity: DiagnosticSeverity::ERROR,
};

const INLINE_MATH: Guide = Guide {
    code: "invalid-inline-math",
    primary: "Malformed inline math",
    help: "Inline math is written as [$ formula $]. Ensure you have both the opening [$ and closing $] markers.",
    examples: &["[$ a^2 + b^2 = c^2 $]"],
    severity: DiagnosticSeverity::ERROR,
};

const DECORATION: Guide = Guide {
    code: "invalid-decoration",
    primary: "Malformed text decoration",
    help: "Decorations such as bold or italics must wrap content inside [ ]. Example: [* bold *] or [/ italic /].",
    examples: &["[* bold *]", "[/ emphasis /]"],
    severity: DiagnosticSeverity::ERROR,
};

#[derive(Debug, Clone, Copy)]
enum RuleFamily {
    Embed,
    Img,
    Link,
    Command,
    Property,
    Task,
    Anchor,
    InlineCode,
    InlineMath,
    Decoration,
    Statement,
}

/// Most specific family first: a pest error lists every rule it expected,
/// and `statement` is in nearly all of them.
type RuleMatcher = fn(Rule) -> bool;

const RULE_FAMILIES: &[(RuleFamily, RuleMatcher)] = &[
    (RuleFamily::Embed, is_embed_rule),
    (RuleFamily::Img, is_img_rule),
    (RuleFamily::Link, is_link_rule),
    (RuleFamily::Command, is_command_rule),
    (RuleFamily::Property, is_property_rule),
    (RuleFamily::Task, is_task_rule),
    (RuleFamily::Anchor, is_anchor_rule),
    (RuleFamily::InlineCode, is_inline_code_rule),
    (RuleFamily::InlineMath, is_inline_math_rule),
    (RuleFamily::Decoration, is_decoration_rule),
    (RuleFamily::Statement, is_statement_rule),
];

impl RuleFamily {
    fn of(expected: &[Rule]) -> Option<Self> {
        RULE_FAMILIES
            .iter()
            .find(|(_, is_member)| expected.iter().any(|rule| is_member(*rule)))
            .map(|(family, _)| *family)
    }

    /// `Statement` has no fixed guide: its wording comes from the parser message.
    fn guide(self) -> &'static Guide {
        match self {
            RuleFamily::Embed => &EMBED,
            RuleFamily::Img => &IMG,
            RuleFamily::Link => &LINK,
            RuleFamily::Command => &COMMAND,
            RuleFamily::Property => &PROPERTY,
            RuleFamily::Task => &TASK,
            RuleFamily::Anchor => &ANCHOR,
            RuleFamily::InlineCode => &INLINE_CODE,
            RuleFamily::InlineMath => &INLINE_MATH,
            RuleFamily::Decoration => &DECORATION,
            RuleFamily::Statement => unreachable!("statement errors are worded from the message"),
        }
    }
}

fn is_embed_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::expr_embed
            | Rule::embed_url_only
            | Rule::embed_url_title
            | Rule::embed_title_url
            | Rule::embed_url_url
            | Rule::embed_local_only
            | Rule::embed_local_title
            | Rule::embed_title_local
    )
}

fn is_img_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::expr_img
            | Rule::img_alt_path_opts
            | Rule::img_path_alt_opts
            | Rule::img_path_unquoted_alt_opts
            | Rule::img_unquoted_alt_url_opts
            | Rule::img_path_opts
            | Rule::img_path
            | Rule::alt_img
    )
}

fn is_link_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::expr_wiki_link
            | Rule::wiki_link
            | Rule::wiki_link_anchored
            | Rule::self_link_anchored
            | Rule::expr_url_link
            | Rule::expr_local_file_link
            | Rule::expr_mail_link
    )
}

fn is_command_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::expr_command
            | Rule::expr_command_line
            | Rule::builtin_commands
            | Rule::command_code
            | Rule::command_math
            | Rule::command_quote
            | Rule::command_table
    )
}

fn is_property_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::expr_property
            | Rule::property_name
            | Rule::property_positional_arg
            | Rule::property_keyword_pair
            | Rule::property_keyword_arg
            | Rule::property_keyword_value
            | Rule::trailing_properties
    )
}

fn is_task_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::expr_task
            | Rule::symbol_task_done
            | Rule::symbol_task_doing
            | Rule::symbol_task_todo
            | Rule::task_due
    )
}

fn is_anchor_rule(rule: Rule) -> bool {
    matches!(rule, Rule::expr_anchor | Rule::anchor)
}

fn is_inline_code_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::expr_code_inline | Rule::code_inline | Rule::code_inline_char
    )
}

fn is_inline_math_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::expr_math_inline | Rule::math_inline | Rule::math_inline_char
    )
}

fn is_decoration_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::expr_builtin_symbols
            | Rule::builtin_symbols
            | Rule::symbol_bold
            | Rule::symbol_italic
            | Rule::symbol_underline
            | Rule::symbol_deleted
    )
}

fn is_statement_rule(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::statement | Rule::statement_nestable | Rule::raw_sentence | Rule::line
    )
}

fn describe_expectations(positives: &[Rule]) -> Option<String> {
    let mut names: Vec<String> = positives
        .iter()
        .map(|rule| rule_display_name(*rule).to_string())
        .collect();
    names.sort();
    names.dedup();

    match names.len() {
        0 => None,
        1 => Some(names.remove(0)),
        _ => Some(format!("one of {}", names.join(", "))),
    }
}

fn rule_display_name(rule: Rule) -> Cow<'static, str> {
    match rule {
        Rule::command_code => Cow::Borrowed("code block command"),
        Rule::command_math => Cow::Borrowed("math block command"),
        Rule::command_quote => Cow::Borrowed("quote block command"),
        Rule::command_table => Cow::Borrowed("table command"),
        Rule::expr_command => Cow::Borrowed("command"),
        Rule::expr_wiki_link => Cow::Borrowed("wiki link"),
        Rule::expr_url_link => Cow::Borrowed("URL link"),
        Rule::expr_local_file_link => Cow::Borrowed("local file link"),
        Rule::expr_mail_link => Cow::Borrowed("email link"),
        Rule::expr_embed => Cow::Borrowed("embed command"),
        Rule::expr_img => Cow::Borrowed("image command"),
        Rule::expr_code_inline => Cow::Borrowed("inline code"),
        Rule::expr_math_inline => Cow::Borrowed("inline math"),
        Rule::expr_property => Cow::Borrowed("property"),
        Rule::expr_anchor => Cow::Borrowed("anchor"),
        Rule::expr_task => Cow::Borrowed("task"),
        Rule::expr_builtin_symbols => Cow::Borrowed("text decoration"),
        Rule::symbol_bold => Cow::Borrowed("bold marker (*)"),
        Rule::symbol_italic => Cow::Borrowed("italic marker (/)"),
        Rule::symbol_underline => Cow::Borrowed("underline marker (_)"),
        Rule::symbol_deleted => Cow::Borrowed("strikethrough marker (-)"),
        Rule::statement => Cow::Borrowed("line content"),
        Rule::statement_nestable => Cow::Borrowed("nested line content"),
        Rule::raw_sentence => Cow::Borrowed("plain text"),
        _ => Cow::Owned(format!("{:?}", rule).replace('_', " ")),
    }
}

fn compose_message(primary: &str, help: &str, examples: &[&str]) -> String {
    let mut sections = Vec::new();
    if !primary.trim().is_empty() {
        sections.push(primary.trim().to_string());
    }
    if !help.trim().is_empty() {
        sections.push(help.trim().to_string());
    }
    if !examples.is_empty() {
        let mut block = String::from("Examples:");
        for example in examples {
            block.push('\n');
            block.push_str("  ");
            block.push_str(example);
        }
        sections.push(block);
    }
    sections.join("\n\n")
}

/// The first line of a pest message that is not the `-->` location marker.
fn summary_from_message(message: &str) -> Option<String> {
    message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("-->"))
        .map(|line| line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_text;

    fn translated(text: &str) -> Vec<FriendlyDiagnostic> {
        let translator = DiagnosticTranslator::default();
        parse_text(text)
            .parse_errors
            .iter()
            .map(|error| translator.translate(error))
            .collect()
    }

    #[test]
    fn code_links_to_its_wiki_page() {
        let diagnostic = DiagnosticTranslator::default().embed_error();
        assert_eq!(diagnostic.code.as_deref(), Some("invalid-embed"));
        assert_eq!(
            diagnostic.code_description_uri.as_deref(),
            Some("https://github.com/ompugao/patto/wiki/Diagnostic-Errors/invalid-embed")
        );
    }

    #[test]
    fn embed_and_img_mistakes_are_warnings() {
        let translator = DiagnosticTranslator::default();
        assert_eq!(
            translator.embed_error().severity,
            DiagnosticSeverity::WARNING
        );
        assert_eq!(translator.img_error().severity, DiagnosticSeverity::WARNING);
    }

    #[test]
    fn message_lists_primary_help_and_examples_as_sections() {
        let message = DiagnosticTranslator::default().img_error().message;
        let sections: Vec<&str> = message.split("\n\n").collect();
        assert_eq!(sections[0], "Invalid image syntax");
        assert!(sections[1].starts_with("Use [@img ...]"));
        assert!(sections[2].starts_with("Examples:\n  [@img https://example.com/photo.jpg]"));
    }

    #[test]
    fn inconsistent_indentation_is_an_error_with_its_own_code() {
        let diagnostics = translated("parent\n\t\ttoo deep\n");
        let indentation = diagnostics
            .iter()
            .find(|d| d.code.as_deref() == Some("invalid-indentation"))
            .expect("indentation diagnostic");
        assert_eq!(indentation.severity, DiagnosticSeverity::ERROR);
        assert!(indentation.message.starts_with("Inconsistent indentation"));
    }

    #[test]
    fn most_specific_rule_family_wins() {
        assert!(matches!(
            RuleFamily::of(&[Rule::statement, Rule::expr_task, Rule::task_due]),
            Some(RuleFamily::Task)
        ));
        assert!(matches!(
            RuleFamily::of(&[Rule::statement]),
            Some(RuleFamily::Statement)
        ));
        assert!(RuleFamily::of(&[]).is_none());
    }

    #[test]
    fn expectations_are_sorted_and_deduplicated() {
        assert_eq!(
            describe_expectations(&[Rule::expr_task, Rule::expr_anchor, Rule::expr_task])
                .as_deref(),
            Some("one of anchor, task")
        );
        assert_eq!(describe_expectations(&[]), None);
    }

    #[test]
    fn summary_skips_the_pest_location_marker() {
        assert_eq!(
            summary_from_message(" --> 1:3\n  |\nexpected task\n").as_deref(),
            Some("|")
        );
        assert_eq!(summary_from_message("\n   \n"), None);
    }
}
