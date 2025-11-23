use std::borrow::Cow;

use crate::parser::{ParserError, PestErrorInfo, PestErrorVariantInfo, Rule};

const DEFAULT_DOCS_BASE_URL: &str = "https://patto.dev/docs/errors";

#[derive(Debug, Clone)]
pub struct FriendlyDiagnostic {
    pub message: String,
    pub code: Option<String>,
    pub code_description_uri: Option<String>,
}

#[derive(Debug)]
pub struct DiagnosticTranslator {
    docs_base_url: &'static str,
}

impl DiagnosticTranslator {
    pub fn new() -> Self {
        Self {
            docs_base_url: DEFAULT_DOCS_BASE_URL,
        }
    }

    pub fn translate(&self, error: &ParserError) -> FriendlyDiagnostic {
        match error {
            ParserError::InvalidIndentation(_) => self.invalid_indentation_message(),
            ParserError::ParseError(_, info) => self.translate_pest_error(info),
        }
    }

    fn invalid_indentation_message(&self) -> FriendlyDiagnostic {
        let primary = "Inconsistent indentation";
        let help = "Use tabs to indent nested blocks. Child lines must be indented exactly one tab deeper than their parent.";
        let examples = ["Heading", "\tChild line", "\t\tNested child"];
        FriendlyDiagnostic::new(
            compose_message(primary, help, &examples),
            Some("invalid-indentation"),
            self.docs_base_url,
        )
    }

    fn translate_pest_error(&self, info: &PestErrorInfo) -> FriendlyDiagnostic {
        match &info.variant {
            PestErrorVariantInfo::ParsingError { positives, .. } => {
                match ErrorCategory::from_rules(positives) {
                    Some(ErrorCategory::Link) => self.link_error(),
                    Some(ErrorCategory::Command) => self.command_error(),
                    Some(ErrorCategory::Property) => self.property_error(),
                    Some(ErrorCategory::Task) => self.task_error(),
                    Some(ErrorCategory::Anchor) => self.anchor_error(),
                    Some(ErrorCategory::InlineCode) => self.inline_code_error(),
                    Some(ErrorCategory::InlineMath) => self.inline_math_error(),
                    Some(ErrorCategory::Decoration) => self.decoration_error(),
                    Some(ErrorCategory::Statement) => self.statement_error(positives, info),
                    None => self.generic_error(positives, info),
                }
            }
            PestErrorVariantInfo::CustomError { message } => {
                let composed = compose_message("Invalid syntax", message, &[]);
                FriendlyDiagnostic::new(composed, Some("syntax-error"), self.docs_base_url)
            }
        }
    }

    fn link_error(&self) -> FriendlyDiagnostic {
        let primary = "Invalid link syntax";
        let help = "Wrap links in [ ] and include a note name, anchor, URL, or file path.";
        let examples = [
            "[ProjectPlan]",
            "[ProjectPlan#milestones]",
            "[https://example.com]",
        ];
        FriendlyDiagnostic::new(
            compose_message(primary, help, &examples),
            Some("invalid-link"),
            self.docs_base_url,
        )
    }

    fn command_error(&self) -> FriendlyDiagnostic {
        let primary = "Unknown or malformed command";
        let help = "Commands look like [@command-name optional-args]. Available commands include @code, @math, @quote, @table, and @img.";
        let examples = ["[@code rust]", "[@math]", "[@quote]"];
        FriendlyDiagnostic::new(
            compose_message(primary, help, &examples),
            Some("invalid-command"),
            self.docs_base_url,
        )
    }

    fn property_error(&self) -> FriendlyDiagnostic {
        let primary = "Invalid property syntax";
        let help = "Properties use {@name key=value ...}. Separate each key/value with spaces and close the property with }.";
        let examples = ["{@tag project=patto}", "{@task status=todo due=2024-12-31}"];
        FriendlyDiagnostic::new(
            compose_message(primary, help, &examples),
            Some("invalid-property"),
            self.docs_base_url,
        )
    }

    fn task_error(&self) -> FriendlyDiagnostic {
        let primary = "Invalid task syntax";
        let help = "Tasks use {@task status=<todo|doing|done> due=<YYYY-MM-DD or YYYY-MM-DDThh:mm>}. Provide both status and due date.";
        let examples = [
            "{@task status=todo due=2024-12-31}",
            "{@task status=doing due=2024-12-31T14:00}",
        ];
        FriendlyDiagnostic::new(
            compose_message(primary, help, &examples),
            Some("invalid-task"),
            self.docs_base_url,
        )
    }

    fn anchor_error(&self) -> FriendlyDiagnostic {
        let primary = "Invalid anchor";
        let help = "Anchors start with # and may contain letters, numbers, _ or -. Example: #ProjectAlpha.";
        let examples = ["#inbox", "[#ProjectAlpha]", "[MyNote#section]"];
        FriendlyDiagnostic::new(
            compose_message(primary, help, &examples),
            Some("invalid-anchor"),
            self.docs_base_url,
        )
    }

    fn inline_code_error(&self) -> FriendlyDiagnostic {
        let primary = "Malformed inline code";
        let help = "Inline code is written as [` code `]. Make sure both the opening [` and closing `] markers are present.";
        let examples = ["[` println!(\"hello\"); `]"];
        FriendlyDiagnostic::new(
            compose_message(primary, help, &examples),
            Some("invalid-inline-code"),
            self.docs_base_url,
        )
    }

    fn inline_math_error(&self) -> FriendlyDiagnostic {
        let primary = "Malformed inline math";
        let help = "Inline math is written as [$ formula $]. Ensure you have both the opening [$ and closing $] markers.";
        let examples = ["[$ a^2 + b^2 = c^2 $]"];
        FriendlyDiagnostic::new(
            compose_message(primary, help, &examples),
            Some("invalid-inline-math"),
            self.docs_base_url,
        )
    }

    fn decoration_error(&self) -> FriendlyDiagnostic {
        let primary = "Malformed text decoration";
        let help = "Decorations such as bold or italics must wrap content inside [ ]. Example: [* bold *] or [/ italic /].";
        let examples = ["[* bold *]", "[/ emphasis /]"];
        FriendlyDiagnostic::new(
            compose_message(primary, help, &examples),
            Some("invalid-decoration"),
            self.docs_base_url,
        )
    }

    fn statement_error(&self, positives: &[Rule], info: &PestErrorInfo) -> FriendlyDiagnostic {
        let primary = self
            .describe_expectations(positives)
            .map(|desc| format!("Couldn't understand this line – expected {}.", desc))
            .unwrap_or_else(|| "Couldn't understand this line.".to_string());
        let detail = summary_from_message(&info.message).unwrap_or_else(|| {
            "Check for missing brackets, unmatched commands, or typos in this line.".to_string()
        });
        FriendlyDiagnostic::new(
            compose_message(&primary, &detail, &[]),
            Some("line-parse-error"),
            self.docs_base_url,
        )
    }

    fn generic_error(&self, positives: &[Rule], info: &PestErrorInfo) -> FriendlyDiagnostic {
        let expectation = self.describe_expectations(positives);
        let primary = expectation
            .map(|desc| format!("Unexpected text – expected {}.", desc))
            .unwrap_or_else(|| "Patto couldn't understand this part.".to_string());
        let detail = summary_from_message(&info.message).unwrap_or_else(|| {
            "Make sure brackets, commands, and properties are written correctly.".to_string()
        });
        FriendlyDiagnostic::new(
            compose_message(&primary, &detail, &[]),
            Some("syntax-error"),
            self.docs_base_url,
        )
    }

    fn describe_expectations(&self, positives: &[Rule]) -> Option<String> {
        if positives.is_empty() {
            return None;
        }

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
}

impl Default for DiagnosticTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl FriendlyDiagnostic {
    fn new(message: String, code: Option<&str>, docs_base_url: &str) -> Self {
        let normalized_base = docs_base_url.trim_end_matches('/');
        let code_owned = code.map(|value| value.to_string());
        let code_description_uri = code_owned
            .as_ref()
            .map(|value| format!("{}/{}", normalized_base, value));
        Self {
            message,
            code: code_owned,
            code_description_uri,
        }
    }
}

enum ErrorCategory {
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

impl ErrorCategory {
    fn from_rules(rules: &[Rule]) -> Option<Self> {
        if rules.is_empty() {
            return None;
        }
        if rules.iter().any(|rule| is_link_rule(*rule)) {
            return Some(ErrorCategory::Link);
        }
        if rules.iter().any(|rule| is_command_rule(*rule)) {
            return Some(ErrorCategory::Command);
        }
        if rules.iter().any(|rule| is_property_rule(*rule)) {
            return Some(ErrorCategory::Property);
        }
        if rules.iter().any(|rule| is_task_rule(*rule)) {
            return Some(ErrorCategory::Task);
        }
        if rules
            .iter()
            .any(|rule| matches!(rule, Rule::expr_anchor | Rule::anchor))
        {
            return Some(ErrorCategory::Anchor);
        }
        if rules.iter().any(|rule| {
            matches!(
                rule,
                Rule::expr_code_inline | Rule::code_inline | Rule::code_inline_char
            )
        }) {
            return Some(ErrorCategory::InlineCode);
        }
        if rules.iter().any(|rule| {
            matches!(
                rule,
                Rule::expr_math_inline | Rule::math_inline | Rule::math_inline_char
            )
        }) {
            return Some(ErrorCategory::InlineMath);
        }
        if rules.iter().any(|rule| is_decoration_rule(*rule)) {
            return Some(ErrorCategory::Decoration);
        }
        if rules.iter().any(|rule| {
            matches!(
                rule,
                Rule::statement | Rule::statement_nestable | Rule::raw_sentence | Rule::line
            )
        }) {
            return Some(ErrorCategory::Statement);
        }
        None
    }
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
            | Rule::expr_img
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
            | Rule::property_arg
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
        _ => Cow::Owned(format!("{}", format!("{:?}", rule).replace('_', " "))),
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

fn summary_from_message(message: &str) -> Option<String> {
    message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("-->"))
        .map(|line| line.to_string())
}
