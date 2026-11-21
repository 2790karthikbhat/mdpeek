//! Terminal-agnostic styled text, shared by layout, the TUI and `--plain`.

use std::rc::Rc;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// One of the 16 terminal palette colors, so the user's theme applies.
    Ansi(u8),
    Rgb(u8, u8, u8),
}

pub const RED: Color = Color::Ansi(1);
pub const GREEN: Color = Color::Ansi(2);
pub const YELLOW: Color = Color::Ansi(3);
pub const BLUE: Color = Color::Ansi(4);
pub const MAGENTA: Color = Color::Ansi(5);
pub const CYAN: Color = Color::Ansi(6);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    /// Target of a hyperlink, for OSC 8.
    pub link: Option<Rc<str>>,
}

impl Style {
    pub fn fg(c: Color) -> Self {
        Style {
            fg: Some(c),
            ..Style::default()
        }
    }
    pub fn dim() -> Self {
        Style {
            dim: true,
            ..Style::default()
        }
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

impl Span {
    /// Control characters (other than newline and tab, which layout
    /// consumes) are replaced, so a document can't send escape sequences
    /// to the terminal.
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Span {
            text: sanitize(text.into()),
            style,
        }
    }
    pub fn plain(text: impl Into<String>) -> Self {
        Span::new(text, Style::default())
    }
    pub fn width(&self) -> usize {
        self.text.width()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub spans: Vec<Span>,
    /// Index into `Layout::images` of an image whose top row is this line.
    pub image: Option<usize>,
    /// Index into `Layout::headings` of a heading that starts on this line.
    pub heading: Option<usize>,
}

impl Line {
    pub fn new(spans: Vec<Span>) -> Self {
        Line {
            spans,
            ..Line::default()
        }
    }
    pub fn width(&self) -> usize {
        self.spans.iter().map(Span::width).sum()
    }
    /// Append a span, merging it into the previous one when styles match.
    pub fn push(&mut self, span: Span) {
        if span.text.is_empty() {
            return;
        }
        match self.spans.last_mut() {
            Some(last) if last.style == span.style => last.text.push_str(&span.text),
            _ => self.spans.push(span),
        }
    }
    pub fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}

pub fn sanitize(s: String) -> String {
    if s.chars().any(is_unsafe) {
        s.chars()
            .map(|c| if is_unsafe(c) { '\u{fffd}' } else { c })
            .collect()
    } else {
        s
    }
}

fn is_unsafe(c: char) -> bool {
    c.is_control() && c != '\n' && c != '\t'
}

#[cfg(test)]
mod tests {
    #[test]
    fn strips_escapes() {
        assert_eq!(
            super::Span::plain("a\x1b[2Jb\x07").text,
            "a\u{fffd}[2Jb\u{fffd}"
        );
    }
}
