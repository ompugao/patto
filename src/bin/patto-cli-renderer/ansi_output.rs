use patto::tui_renderer::{DocElement, RenderedDoc};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

fn ansi_fg(color: Color) -> Option<String> {
    match color {
        Color::Reset => Some("\x1b[39m".to_string()),
        Color::Black => Some("\x1b[30m".to_string()),
        Color::Red => Some("\x1b[31m".to_string()),
        Color::Green => Some("\x1b[32m".to_string()),
        Color::Yellow => Some("\x1b[33m".to_string()),
        Color::Blue => Some("\x1b[34m".to_string()),
        Color::Magenta => Some("\x1b[35m".to_string()),
        Color::Cyan => Some("\x1b[36m".to_string()),
        Color::Gray => Some("\x1b[37m".to_string()),
        Color::DarkGray => Some("\x1b[90m".to_string()),
        Color::LightRed => Some("\x1b[91m".to_string()),
        Color::LightGreen => Some("\x1b[92m".to_string()),
        Color::LightYellow => Some("\x1b[93m".to_string()),
        Color::LightBlue => Some("\x1b[94m".to_string()),
        Color::LightMagenta => Some("\x1b[95m".to_string()),
        Color::LightCyan => Some("\x1b[96m".to_string()),
        Color::White => Some("\x1b[97m".to_string()),
        Color::Rgb(r, g, b) => Some(format!("\x1b[38;2;{r};{g};{b}m")),
        Color::Indexed(n) => Some(format!("\x1b[38;5;{n}m")),
    }
}

fn ansi_bg(color: Color) -> Option<String> {
    match color {
        Color::Reset => Some("\x1b[49m".to_string()),
        Color::Black => Some("\x1b[40m".to_string()),
        Color::Red => Some("\x1b[41m".to_string()),
        Color::Green => Some("\x1b[42m".to_string()),
        Color::Yellow => Some("\x1b[43m".to_string()),
        Color::Blue => Some("\x1b[44m".to_string()),
        Color::Magenta => Some("\x1b[45m".to_string()),
        Color::Cyan => Some("\x1b[46m".to_string()),
        Color::Gray => Some("\x1b[47m".to_string()),
        Color::DarkGray => Some("\x1b[100m".to_string()),
        Color::LightRed => Some("\x1b[101m".to_string()),
        Color::LightGreen => Some("\x1b[102m".to_string()),
        Color::LightYellow => Some("\x1b[103m".to_string()),
        Color::LightBlue => Some("\x1b[104m".to_string()),
        Color::LightMagenta => Some("\x1b[105m".to_string()),
        Color::LightCyan => Some("\x1b[106m".to_string()),
        Color::White => Some("\x1b[107m".to_string()),
        Color::Rgb(r, g, b) => Some(format!("\x1b[48;2;{r};{g};{b}m")),
        Color::Indexed(n) => Some(format!("\x1b[48;5;{n}m")),
    }
}

fn span_to_ansi(span: &Span, no_color: bool) -> String {
    if no_color {
        return span.content.to_string();
    }
    let style: Style = span.style;
    let mut open = String::new();

    if let Some(fg) = style.fg {
        if let Some(code) = ansi_fg(fg) {
            open.push_str(&code);
        }
    }
    if let Some(bg) = style.bg {
        if let Some(code) = ansi_bg(bg) {
            open.push_str(&code);
        }
    }

    let m = style.add_modifier;
    if m.contains(Modifier::BOLD) {
        open.push_str("\x1b[1m");
    }
    if m.contains(Modifier::DIM) {
        open.push_str("\x1b[2m");
    }
    if m.contains(Modifier::ITALIC) {
        open.push_str("\x1b[3m");
    }
    if m.contains(Modifier::UNDERLINED) {
        open.push_str("\x1b[4m");
    }
    if m.contains(Modifier::SLOW_BLINK) || m.contains(Modifier::RAPID_BLINK) {
        open.push_str("\x1b[5m");
    }
    if m.contains(Modifier::REVERSED) {
        open.push_str("\x1b[7m");
    }
    if m.contains(Modifier::CROSSED_OUT) {
        open.push_str("\x1b[9m");
    }

    if open.is_empty() {
        span.content.to_string()
    } else {
        format!("{}{}\x1b[0m", open, span.content)
    }
}

/// Render a `RenderedDoc` as ANSI-colored text, one line per element.
/// `width` is used to truncate long lines (0 = no truncation).
pub fn render_to_stdout(doc: &RenderedDoc, width: usize, no_color: bool) {
    for elem in &doc.elements {
        match elem {
            DocElement::TextLine(line) => {
                let mut out = String::new();
                for span in &line.spans {
                    out.push_str(&span_to_ansi(span, no_color));
                }
                // Truncate visible chars to width if requested
                if width > 0 {
                    let visible: String = strip_ansi_chars(&out, width);
                    println!("{}", visible);
                } else {
                    println!("{}", out);
                }
            }
            DocElement::Image { alt, .. } => {
                let label = alt
                    .as_deref()
                    .map(|a| format!("[image: {}]", a))
                    .unwrap_or_else(|| "[image]".to_string());
                if no_color {
                    println!("{}", label);
                } else {
                    println!("\x1b[2m\x1b[3m{}\x1b[0m", label);
                }
            }
            DocElement::ImageRow(imgs) => {
                let alts: Vec<_> = imgs
                    .iter()
                    .map(|(_, a)| a.as_deref().unwrap_or("image"))
                    .collect();
                let label = format!("[images: {}]", alts.join(", "));
                if no_color {
                    println!("{}", label);
                } else {
                    println!("\x1b[2m\x1b[3m{}\x1b[0m", label);
                }
            }
            DocElement::Spacer => {
                println!();
            }
        }
    }
}

/// Truncate `s` so the visible (non-ANSI) content is at most `max_chars` wide,
/// preserving the ANSI escape codes inline.
fn strip_ansi_chars(s: &str, max_chars: usize) -> String {
    let mut result = String::new();
    let mut visible = 0usize;
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if visible >= max_chars {
            break;
        }
        if ch == '\x1b' {
            // consume the escape sequence without counting it
            result.push(ch);
            if chars.peek() == Some(&'[') {
                result.push(chars.next().unwrap());
                for c in chars.by_ref() {
                    result.push(c);
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
            visible += 1;
        }
    }
    result
}
