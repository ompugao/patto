//! Syntax highlighting for code blocks in the TUI preview using `syntect`.
//!
//! Lazily initialises a [`SyntaxSet`] and [`ThemeSet`] on first use (both are
//! embedded at compile time via syntect's `default-syntaxes` / `default-themes`
//! features).

use ratatui::style::{Color, Style};
use ratatui::text::Span;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntectColor, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static ENGINE: OnceLock<(SyntaxSet, ThemeSet)> = OnceLock::new();

fn engine() -> &'static (SyntaxSet, ThemeSet) {
    ENGINE.get_or_init(|| {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        (ss, ts)
    })
}

const DEFAULT_THEME: &str = "base16-ocean.dark";

/// Convert a syntect foreground color to a ratatui [`Color`].
fn to_ratatui_color(c: SyntectColor) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

/// Highlight `lines` of code for `lang` using the given `theme` name (falls
/// back to [`DEFAULT_THEME`] when `None`).
///
/// Returns one `Vec<Span<'static>>` per input line.  On unknown / empty `lang`,
/// or any highlighting error, each line is returned as a single unstyled span.
pub fn highlight_code(lang: &str, lines: &[&str], theme: Option<&str>) -> Vec<Vec<Span<'static>>> {
    let (ss, ts) = engine();

    let theme_name = theme.unwrap_or(DEFAULT_THEME);
    let theme = ts
        .themes
        .get(theme_name)
        .or_else(|| ts.themes.get(DEFAULT_THEME))
        .or_else(|| ts.themes.values().next());

    let Some(theme) = theme else {
        return plain_fallback(lines);
    };

    // Find syntax by token (e.g. "rust", "python", "js").  Fall back to plain text.
    let syntax = if lang.is_empty() {
        ss.find_syntax_plain_text()
    } else {
        ss.find_syntax_by_token(lang)
            .unwrap_or_else(|| ss.find_syntax_plain_text())
    };

    let mut highlighter = HighlightLines::new(syntax, theme);

    // syntect's HighlightLines expects newlines; join the lines back with '\n'
    // and then use LinesWithEndings to iterate them.
    let joined = lines.join("\n");

    let mut result: Vec<Vec<Span<'static>>> = Vec::with_capacity(lines.len());

    for raw_line in LinesWithEndings::from(&joined) {
        let tokens = match highlighter.highlight_line(raw_line, ss) {
            Ok(t) => t,
            Err(_) => {
                result.push(vec![Span::raw(raw_line.trim_end_matches('\n').to_owned())]);
                continue;
            }
        };

        let spans: Vec<Span<'static>> = tokens
            .into_iter()
            .map(|(style, text)| {
                let text = text.trim_end_matches('\n').to_owned();
                Span::styled(
                    text,
                    Style::default().fg(to_ratatui_color(style.foreground)),
                )
            })
            .filter(|s| !s.content.is_empty())
            .collect();

        result.push(spans);
    }

    // Ensure we always return exactly `lines.len()` rows (guards against
    // off-by-one from the joined iteration).
    result.truncate(lines.len());
    while result.len() < lines.len() {
        result.push(vec![]);
    }

    result
}

/// Plain (unstyled) fallback — one span per line.
fn plain_fallback(lines: &[&str]) -> Vec<Vec<Span<'static>>> {
    lines
        .iter()
        .map(|l| vec![Span::raw((*l).to_owned())])
        .collect()
}
