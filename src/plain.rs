//! Layout lines -> text for stdout, with or without ANSI styling.

use crate::style::{Color, Line, Style};
use std::fmt::Write;

pub struct PlainOpts {
    pub color: bool,
    /// Wrap links in OSC 8 escape sequences.
    pub hyperlinks: bool,
}

pub fn render(lines: &[Line], opts: &PlainOpts) -> String {
    let mut out = String::new();
    for line in lines {
        let mut spans = line.spans.clone();
        // Trim trailing whitespace so output diffs and pipes stay clean.
        while let Some(last) = spans.last_mut() {
            let t = last.text.trim_end().len();
            last.text.truncate(t);
            if last.text.is_empty() {
                spans.pop();
            } else {
                break;
            }
        }
        for s in &spans {
            if !opts.color {
                out.push_str(&s.text);
                continue;
            }
            let link = s.style.link.as_deref().filter(|_| opts.hyperlinks);
            if let Some(url) = link {
                let _ = write!(out, "\x1b]8;;{}\x1b\\", sanitize(url));
            }
            let sgr = sgr(&s.style);
            if sgr.is_empty() {
                out.push_str(&s.text);
            } else {
                let _ = write!(out, "\x1b[{sgr}m{}\x1b[0m", s.text);
            }
            if link.is_some() {
                out.push_str("\x1b]8;;\x1b\\");
            }
        }
        out.push('\n');
    }
    out
}

/// Strip control characters so a URL can't end the escape sequence early.
fn sanitize(url: &str) -> String {
    url.chars().filter(|c| !c.is_control()).collect()
}

fn sgr(st: &Style) -> String {
    let mut codes: Vec<String> = Vec::new();
    if st.bold {
        codes.push("1".into());
    }
    if st.dim {
        codes.push("2".into());
    }
    if st.italic {
        codes.push("3".into());
    }
    if st.underline {
        codes.push("4".into());
    }
    if st.strike {
        codes.push("9".into());
    }
    match st.fg {
        Some(Color::Ansi(n)) if n < 8 => codes.push(format!("{}", 30 + n)),
        Some(Color::Ansi(n)) => codes.push(format!("{}", 90 + n.saturating_sub(8))),
        Some(Color::Rgb(r, g, b)) => codes.push(format!("38;2;{r};{g};{b}")),
        None => {}
    }
    codes.join(";")
}
