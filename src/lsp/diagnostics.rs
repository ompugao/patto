//! `textDocument/publishDiagnostics` payload for one document: parse errors,
//! `[@embed]`/`[@img]` expressions the grammar rejected, and task fields left
//! behind by a clock-out.

use pest::Parser as _;
use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, Url,
};

use crate::lsp::diagnostic_translator::{DiagnosticTranslator, FriendlyDiagnostic};
use crate::lsp::locate::utf16_range;
use crate::parser::{
    self, AstNode, AstNodeKind, ParserResult, PattoLineParser, Property, Rule, TaskStatus,
};

pub(super) fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
    let ParserResult { ast, parse_errors } = parser::parse_text(text);
    let translator = DiagnosticTranslator::default();

    let mut diagnostics: Vec<Diagnostic> = parse_errors
        .iter()
        .map(|error| {
            let location = error.location();
            let range = Range::new(
                Position::new(location.row as u32, location.span.0 as u32),
                Position::new(location.row as u32, location.span.1 as u32),
            );
            lsp_diagnostic(range, translator.translate(error))
        })
        .collect();

    diagnostics.extend(malformed_command_diagnostics(text, &translator));
    diagnostics.extend(stale_started_at_diagnostics(&ast));
    diagnostics
}

fn lsp_diagnostic(range: Range, friendly: FriendlyDiagnostic) -> Diagnostic {
    let FriendlyDiagnostic {
        message,
        code,
        code_description_uri,
        severity,
    } = friendly;
    Diagnostic {
        range,
        severity: Some(severity),
        code: code.map(NumberOrString::String),
        code_description: code_description_uri
            .and_then(|href| Url::parse(&href).ok())
            .map(|href| CodeDescription { href }),
        source: Some("patto".into()),
        message,
        ..Diagnostic::default()
    }
}

type CommandErrorFn = fn(&DiagnosticTranslator) -> FriendlyDiagnostic;

/// Commands whose failed parse falls through to plain text, so their mistakes
/// never reach the parser's own error list.
const MALFORMED_COMMAND_CHECKS: &[(&str, Rule, CommandErrorFn)] = &[
    (
        "[@embed ",
        Rule::expr_embed,
        DiagnosticTranslator::embed_error,
    ),
    ("[@img ", Rule::expr_img, DiagnosticTranslator::img_error),
];

fn malformed_command_diagnostics(text: &str, translator: &DiagnosticTranslator) -> Vec<Diagnostic> {
    text.lines()
        .enumerate()
        .flat_map(|(row, line)| {
            MALFORMED_COMMAND_CHECKS
                .iter()
                .flat_map(move |(prefix, rule, error)| {
                    bracket_expressions(line, prefix)
                        .filter(|(start, end)| {
                            PattoLineParser::parse(*rule, &line[*start..*end]).is_err()
                        })
                        .map(move |span| {
                            lsp_diagnostic(utf16_range(line, row as u32, span), error(translator))
                        })
                })
        })
        .collect()
}

/// Byte spans of each `prefix ... ]` in `line`, in order.
fn bracket_expressions<'a>(
    line: &'a str,
    prefix: &'a str,
) -> impl Iterator<Item = (usize, usize)> + 'a {
    let mut search_start = 0;
    std::iter::from_fn(move || {
        let start = search_start + line[search_start..].find(prefix)?;
        let end = start + line[start..].find(']')? + 1;
        search_start = end;
        Some((start, end))
    })
}

/// A done task should no longer carry `started_at`: the clock-out transition
/// folds it into `time_spent`, and a leftover value lets tooling count the
/// elapsed time twice.
fn stale_started_at_diagnostics(root: &AstNode) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    collect_stale_started_at(root, &mut diagnostics);
    diagnostics
}

fn collect_stale_started_at(node: &AstNode, diagnostics: &mut Vec<Diagnostic>) {
    if let AstNodeKind::Line { properties } = node.kind() {
        if let Some(location) = properties.iter().find_map(stale_started_at_location) {
            let line = node.extract_str();
            let span = (location.span.0, location.span.1.min(line.len()));
            diagnostics.push(lsp_diagnostic(
                utf16_range(line, node.location().row as u32, span),
                FriendlyDiagnostic {
                    message: "Done task has a stale started_at field. \
                        The time_spent field already accounts for all accumulated time. \
                        Remove started_at= to silence this warning."
                        .into(),
                    code: Some("stale-started-at".into()),
                    code_description_uri: None,
                    severity: DiagnosticSeverity::WARNING,
                },
            ));
        }
    }
    for child in node.children().iter() {
        collect_stale_started_at(child, diagnostics);
    }
}

fn stale_started_at_location(property: &Property) -> Option<&parser::Location> {
    match property {
        Property::Task {
            status: TaskStatus::Done,
            started_at: Some(_),
            location,
            ..
        } => Some(location),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(text: &str) -> Vec<String> {
        diagnostics_for(text)
            .into_iter()
            .filter_map(|d| match d.code {
                Some(NumberOrString::String(code)) => Some(code),
                _ => None,
            })
            .collect()
    }

    fn has_warning(text: &str) -> bool {
        diagnostics_for(text)
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::WARNING))
    }

    #[test]
    fn embed_with_url_or_dot_slash_path_is_not_warned() {
        assert!(!has_warning("[@embed https://example.com/video]"));
        assert!(!has_warning("[@embed ./docs/report.pdf]"));
        assert!(!has_warning("[@embed My Title ./docs/report.pdf]"));
    }

    #[test]
    fn embed_with_bare_path_is_invalid_embed() {
        assert_eq!(codes("[@embed docs/report.pdf]"), ["invalid-embed"]);
    }

    #[test]
    fn img_with_filename_alt_before_bare_path_is_invalid_img() {
        assert_eq!(
            codes("[@img 2026-03-04-10-20-35.png assets/2026-03-04-10-20-35.png]"),
            ["invalid-img"]
        );
    }

    #[test]
    fn img_with_unquoted_alt_before_dot_slash_path_is_not_warned() {
        assert!(!has_warning(
            "[@img 2026-03-04-10-20-35.png ./assets/2026-03-04-10-20-35.png]"
        ));
    }

    #[test]
    fn every_malformed_command_on_a_line_is_reported() {
        assert_eq!(
            codes("see [@embed docs/a.pdf] and [@img assets/b.png] for details"),
            ["invalid-embed", "invalid-img"]
        );
        assert_eq!(
            codes("[@embed docs/a.pdf] and [@embed docs/b.pdf]"),
            ["invalid-embed", "invalid-embed"]
        );
    }

    #[test]
    fn malformed_command_range_is_in_utf16_units() {
        let diagnostics = diagnostics_for("牛乳 [@embed docs/a.pdf]");
        let range = diagnostics[0].range;
        assert_eq!((range.start.character, range.end.character), (3, 22));
    }

    #[test]
    fn unclosed_command_is_left_to_the_parser() {
        assert!(!codes("[@embed docs/a.pdf").contains(&"invalid-embed".to_string()));
    }

    #[test]
    fn done_task_with_started_at_is_warned() {
        let diagnostics = diagnostics_for(
            "buy milk {@task status=done due=2026-06-01 completed_at=2026-06-01T11:00 started_at=2026-06-01T09:00 time_spent=2h}\n",
        );
        let stale: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == Some(NumberOrString::String("stale-started-at".into())))
            .collect();
        assert_eq!(stale.len(), 1, "{:?}", diagnostics);
        assert_eq!(stale[0].severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(stale[0].code_description, None);
    }

    #[test]
    fn doing_task_with_started_at_is_not_warned() {
        assert!(!codes(
            "buy milk {@task status=doing due=2026-06-01 started_at=2026-06-01T09:00}\n"
        )
        .contains(&"stale-started-at".to_string()));
    }

    #[test]
    fn done_task_without_started_at_is_not_warned() {
        assert!(!codes(
            "buy milk {@task status=done due=2026-06-01 completed_at=2026-06-01T11:00 time_spent=2h}\n"
        )
        .contains(&"stale-started-at".to_string()));
    }

    #[test]
    fn stale_started_at_in_nested_line_is_warned() {
        assert!(codes(
            "parent\n\tchild {@task status=done due=2026-06-01 started_at=2026-06-01T09:00}\n"
        )
        .contains(&"stale-started-at".to_string()));
    }
}
