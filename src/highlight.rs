//! Code highlighting with syntect. Grammars and themes load lazily, only
//! for documents that contain fenced code with a known language.

use crate::style::{Color, Span, Style};
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

fn syntaxes() -> &'static SyntaxSet {
    static S: OnceLock<SyntaxSet> = OnceLock::new();
    S.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme() -> &'static Theme {
    static T: OnceLock<Theme> = OnceLock::new();
    T.get_or_init(|| {
        let mut set = ThemeSet::load_defaults();
        set.themes
            .remove("base16-ocean.dark")
            .expect("bundled theme")
    })
}

/// Highlight `code` as `lang`. Returns one span list per source line, or
/// `None` when the language is unknown (callers then show plain text).
pub fn highlight(code: &str, lang: &str) -> Option<Vec<Vec<Span>>> {
    let ss = syntaxes();
    let syntax = ss
        .find_syntax_by_token(lang)
        .or_else(|| ss.find_syntax_by_token(&lang.to_ascii_lowercase()))?;
    if syntax.name == "Plain Text" {
        return None;
    }
    let mut h = HighlightLines::new(syntax, theme());
    let mut out = Vec::new();
    for line in LinesWithEndings::from(code) {
        let ranges = h.highlight_line(line, ss).ok()?;
        let spans = ranges
            .into_iter()
            .filter_map(|(st, text)| {
                let text = text.trim_end_matches(['\n', '\r']);
                if text.is_empty() {
                    return None;
                }
                let fg = st.foreground;
                Some(Span::new(
                    text,
                    Style {
                        fg: Some(Color::Rgb(fg.r, fg.g, fg.b)),
                        bold: st.font_style.contains(FontStyle::BOLD),
                        italic: st.font_style.contains(FontStyle::ITALIC),
                        underline: st.font_style.contains(FontStyle::UNDERLINE),
                        ..Style::default()
                    },
                ))
            })
            .collect();
        out.push(spans);
    }
    Some(out)
}
