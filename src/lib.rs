//! mdpeek: a fast, text-first Markdown viewer for every terminal.
//!
//! Pipeline: source -> [`doc::parse`] -> [`layout::layout`] -> TUI or [`plain::render`].

pub mod doc;
pub mod highlight;
pub mod images;
pub mod layout;
pub mod plain;
pub mod style;
pub mod ui;
pub mod watch;

/// Render Markdown the way `mdpeek --plain` does.
pub fn render_plain(src: &str, width: usize, color: bool) -> String {
    let doc = doc::parse(src);
    let opts = layout::Opts {
        width,
        show_urls: !color,
        images: None,
    };
    let lay = layout::layout(&doc, &opts);
    plain::render(
        &lay.lines,
        &plain::PlainOpts {
            color,
            hyperlinks: color,
        },
    )
}
