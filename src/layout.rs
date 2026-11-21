//! Doc + width -> styled lines, image slots and a heading index.
//!
//! Pure: no terminal I/O. The TUI and `--plain` both draw from the result,
//! so `--plain` output doubles as the renderer's snapshot test.

use crate::doc::{Alert, Align, Block, Doc, Inline, Item};
use crate::highlight;
use crate::style::*;
use std::rc::Rc;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Asks the caller how big an image would be, in terminal cells, when it
/// may use at most `max_cols` columns. `None` means "show a placeholder".
pub type ImageSizer<'a> = &'a dyn Fn(&str, u16) -> Option<(u16, u16)>;

pub struct Opts<'a> {
    pub width: usize,
    /// Append ` (url)` after link text (for output without hyperlinks).
    pub show_urls: bool,
    pub images: Option<ImageSizer<'a>>,
}

impl Default for Opts<'_> {
    fn default() -> Self {
        Opts {
            width: 80,
            show_urls: false,
            images: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSlot {
    pub src: String,
    pub alt: String,
    /// Position of the top-left cell, filled in once nesting is resolved.
    pub line: usize,
    pub col: usize,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub title: String,
    pub line: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub lines: Vec<Line>,
    pub images: Vec<ImageSlot>,
    pub headings: Vec<Heading>,
}

pub fn layout(doc: &Doc, opts: &Opts) -> Layout {
    let mut cx = Cx {
        opts,
        images: Vec::new(),
        headings: Vec::new(),
        depth: 0,
    };
    let width = opts.width.max(10);
    let lines = cx.blocks(&doc.blocks, width);
    let mut out = Layout {
        lines,
        images: cx.images,
        headings: cx.headings,
    };
    for (i, line) in out.lines.iter().enumerate() {
        if let Some(id) = line.image {
            out.images[id].line = i;
            out.images[id].col = line.width();
        }
        if let Some(id) = line.heading {
            out.headings[id].line = i;
        }
    }
    out
}

struct Cx<'a> {
    opts: &'a Opts<'a>,
    images: Vec<ImageSlot>,
    headings: Vec<Heading>,
    /// Current list nesting, for bullet glyphs.
    depth: usize,
}

const BULLETS: [&str; 3] = ["•", "◦", "▪"];

impl Cx<'_> {
    /// Lay out sibling blocks separated by blank lines.
    fn blocks(&mut self, blocks: &[Block], width: usize) -> Vec<Line> {
        self.blocks_sep(blocks, width, true)
    }

    fn blocks_sep(&mut self, blocks: &[Block], width: usize, blank: bool) -> Vec<Line> {
        let mut out: Vec<Line> = Vec::new();
        for b in blocks {
            let lines = self.block(b, width);
            if lines.is_empty() {
                continue;
            }
            if blank && !out.is_empty() {
                out.push(Line::default());
            }
            out.extend(lines);
        }
        out
    }

    fn block(&mut self, b: &Block, width: usize) -> Vec<Line> {
        match b {
            Block::Heading { level, content } => self.heading(*level, content, width),
            Block::Paragraph(inl) => self.paragraph(inl, width),
            Block::Code { lang, text } => self.code(lang.as_deref(), text, width, None),
            Block::Math(tex) => {
                self.code(Some("math"), tex, width, Some(Style::fg(MAGENTA).italic()))
            }
            Block::FrontMatter(text) => {
                self.code(Some("front matter"), text, width, Some(Style::dim()))
            }
            Block::Quote { kind, blocks } => self.quote(*kind, blocks, width),
            Block::List {
                start,
                tight,
                items,
            } => self.list(*start, *tight, items, width),
            Block::Table { aligns, head, rows } => self.table(aligns, head, rows, width),
            Block::Rule => vec![Line::new(vec![Span::new("─".repeat(width), Style::dim())])],
            Block::Footnote { label, blocks } => {
                let marker = format!("[{label}] ");
                let w = marker.width();
                let inner = self.blocks(blocks, width.saturating_sub(w).max(1));
                prefix(
                    inner,
                    vec![Span::new(marker, Style::fg(CYAN))],
                    vec![Span::plain(" ".repeat(w))],
                )
            }
        }
    }

    fn heading(&mut self, level: u8, content: &[Inline], width: usize) -> Vec<Line> {
        let style = match level {
            1 => Style::fg(MAGENTA).bold(),
            2 => Style::fg(CYAN).bold(),
            3 => Style::fg(YELLOW).bold(),
            4 => Style::fg(GREEN).bold(),
            _ => Style::default().bold(),
        };
        let spans = self.inlines(content, &style);
        let title: String = spans.iter().map(|s| s.text.as_str()).collect();
        let mut lines = wrap(&spans, width);
        let rule = match level {
            1 => Some("━"),
            2 => Some("─"),
            _ => None,
        };
        if let Some(r) = rule {
            let w = lines.iter().map(Line::width).max().unwrap_or(0).max(1);
            let rule_w = if level == 1 { width } else { w };
            lines.push(Line::new(vec![Span::new(
                r.repeat(rule_w),
                Style {
                    dim: level == 2,
                    ..style.clone()
                },
            )]));
        }
        let id = self.headings.len();
        self.headings.push(Heading {
            level,
            title: title.replace('\n', " "),
            line: 0,
        });
        if let Some(first) = lines.first_mut() {
            first.heading = Some(id);
        }
        lines
    }

    fn paragraph(&mut self, inl: &[Inline], width: usize) -> Vec<Line> {
        // An image alone in its paragraph is a block image.
        let solo: Vec<&Inline> = inl
            .iter()
            .filter(|i| !matches!(i, Inline::Text(t) if t.trim().is_empty()))
            .collect();
        if let [Inline::Image { url, alt }] = solo.as_slice()
            && let Some(sizer) = self.opts.images
            && let Some((w, h)) = sizer(url, width.min(u16::MAX as usize) as u16)
        {
            let h = h.max(1);
            let id = self.images.len();
            self.images.push(ImageSlot {
                src: url.clone(),
                alt: alt.clone(),
                line: 0,
                col: 0,
                width: w,
                height: h,
            });
            let mut lines = vec![Line::default(); h as usize];
            lines[0].image = Some(id);
            return lines;
        }
        let spans = self.inlines(inl, &Style::default());
        wrap(&spans, width)
    }

    fn code(
        &mut self,
        lang: Option<&str>,
        text: &str,
        width: usize,
        style: Option<Style>,
    ) -> Vec<Line> {
        let text = text.trim_end_matches('\n').replace('\t', "    ");
        let label = lang.unwrap_or("");
        let mut lines = Vec::new();
        let head = if label.is_empty() {
            "─".repeat(width)
        } else {
            let fill = width.saturating_sub(label.width() + 4);
            format!("── {label} {}", "─".repeat(fill))
        };
        lines.push(Line::new(vec![Span::new(head, Style::dim())]));
        let highlighted = match (&style, lang) {
            (None, Some(l)) => highlight::highlight(&text, l),
            _ => None,
        };
        let src_lines: Vec<Vec<Span>> = match highlighted {
            Some(h) => h,
            None => text
                .lines()
                .map(|l| vec![Span::new(l, style.clone().unwrap_or_default())])
                .collect(),
        };
        let indent = 2;
        let avail = width.saturating_sub(indent).max(1);
        for spans in src_lines {
            for chunk in hard_wrap(&spans, avail) {
                let mut line = Line::new(vec![Span::plain(" ".repeat(indent))]);
                for s in chunk {
                    line.push(s);
                }
                lines.push(line);
            }
        }
        lines.push(Line::new(vec![Span::new("─".repeat(width), Style::dim())]));
        lines
    }

    fn quote(&mut self, kind: Option<Alert>, blocks: &[Block], width: usize) -> Vec<Line> {
        let (color, label) = match kind {
            Some(Alert::Note) => (Some(BLUE), "Note"),
            Some(Alert::Tip) => (Some(GREEN), "Tip"),
            Some(Alert::Important) => (Some(MAGENTA), "Important"),
            Some(Alert::Warning) => (Some(YELLOW), "Warning"),
            Some(Alert::Caution) => (Some(RED), "Caution"),
            None => (None, ""),
        };
        let bar_style = match color {
            Some(c) => Style::fg(c),
            None => Style::dim(),
        };
        let mut inner = self.blocks(blocks, width.saturating_sub(2).max(1));
        if let Some(c) = color {
            inner.insert(0, Line::new(vec![Span::new(label, Style::fg(c).bold())]));
        }
        let bar = vec![Span::new("│ ", bar_style)];
        prefix(inner, bar.clone(), bar)
    }

    fn list(&mut self, start: Option<u64>, tight: bool, items: &[Item], width: usize) -> Vec<Line> {
        let markers: Vec<String> = match start {
            Some(n) => (0..items.len() as u64)
                .map(|i| format!("{}.", n + i))
                .collect(),
            None => vec![BULLETS[self.depth % BULLETS.len()].to_string(); items.len()],
        };
        let mw = markers.iter().map(|m| m.width()).max().unwrap_or(1) + 1;
        let marker_style = if start.is_some() {
            Style::dim()
        } else {
            Style::fg(CYAN)
        };
        self.depth += 1;
        let mut out = Vec::new();
        for (item, marker) in items.iter().zip(markers) {
            if !tight && !out.is_empty() {
                out.push(Line::default());
            }
            let mut first = vec![Span::new(
                format!("{marker:>w$} ", w = mw - 1),
                marker_style.clone(),
            )];
            let mut rest = vec![Span::plain(" ".repeat(mw))];
            let mut inner_w = width.saturating_sub(mw).max(1);
            if let Some(done) = item.task {
                let (txt, st) = if done {
                    ("[x] ", Style::fg(GREEN))
                } else {
                    ("[ ] ", Style::dim())
                };
                first.push(Span::new(txt, st));
                rest.push(Span::plain("    "));
                inner_w = inner_w.saturating_sub(4).max(1);
            }
            let mut inner = self.blocks_sep(&item.blocks, inner_w, !tight);
            if inner.is_empty() {
                inner.push(Line::default());
            }
            out.extend(prefix(inner, first, rest));
        }
        self.depth -= 1;
        out
    }

    fn table(
        &mut self,
        aligns: &[Align],
        head: &[Vec<Inline>],
        rows: &[Vec<Vec<Inline>>],
        width: usize,
    ) -> Vec<Line> {
        let ncols = rows
            .iter()
            .map(Vec::len)
            .chain([head.len()])
            .max()
            .unwrap_or(0);
        if ncols == 0 {
            return Vec::new();
        }
        let head_style = Style::default().bold();
        let cell_spans = |cx: &mut Self, row: &[Vec<Inline>], st: &Style| -> Vec<Vec<Span>> {
            (0..ncols)
                .map(|c| {
                    row.get(c)
                        .map(|cell| cx.inlines(cell, st))
                        .unwrap_or_default()
                })
                .collect()
        };
        let head_cells = cell_spans(self, head, &head_style);
        let body: Vec<Vec<Vec<Span>>> = rows
            .iter()
            .map(|r| cell_spans(self, r, &Style::default()))
            .collect();

        // Natural width of each column, then shrink the widest until it fits.
        let mut widths = vec![1usize; ncols];
        for row in std::iter::once(&head_cells).chain(&body) {
            for (c, spans) in row.iter().enumerate() {
                let w = wrap(spans, usize::MAX / 2)
                    .iter()
                    .map(Line::width)
                    .max()
                    .unwrap_or(0);
                widths[c] = widths[c].max(w);
            }
        }
        let overhead = 3 * ncols + 1;
        let avail = width.saturating_sub(overhead);
        while widths.iter().sum::<usize>() > avail {
            let (i, &w) = widths.iter().enumerate().max_by_key(|(_, w)| **w).unwrap();
            if w <= 3 {
                break;
            }
            widths[i] -= 1;
        }

        let border = |l: &str, m: &str, r: &str| {
            let mut s = String::from(l);
            for (i, w) in widths.iter().enumerate() {
                s.push_str(&"─".repeat(w + 2));
                s.push_str(if i + 1 == ncols { r } else { m });
            }
            Line::new(vec![Span::new(s, Style::dim())])
        };
        let render_row = |cells: &[Vec<Span>]| -> Vec<Line> {
            let wrapped: Vec<Vec<Line>> = cells
                .iter()
                .zip(&widths)
                .map(|(spans, &w)| wrap(spans, w))
                .collect();
            let h = wrapped.iter().map(Vec::len).max().unwrap_or(0).max(1);
            (0..h)
                .map(|r| {
                    let mut line = Line::new(vec![Span::new("│", Style::dim())]);
                    for (c, cell) in wrapped.iter().enumerate() {
                        let content = cell.get(r).cloned().unwrap_or_default();
                        let pad = widths[c].saturating_sub(content.width());
                        let (lp, rp) = match aligns.get(c).copied().unwrap_or(Align::Left) {
                            Align::Left => (0, pad),
                            Align::Right => (pad, 0),
                            Align::Center => (pad / 2, pad - pad / 2),
                        };
                        line.push(Span::plain(" ".repeat(lp + 1)));
                        for s in content.spans {
                            line.push(s);
                        }
                        line.push(Span::plain(" ".repeat(rp + 1)));
                        line.push(Span::new("│", Style::dim()));
                    }
                    line
                })
                .collect()
        };

        let mut out = vec![border("┌", "┬", "┐")];
        out.extend(render_row(&head_cells));
        out.push(border("├", "┼", "┤"));
        for row in &body {
            out.extend(render_row(row));
        }
        out.push(border("└", "┴", "┘"));
        out
    }

    /// Flatten inline content into styled spans. Hard breaks become "\n".
    fn inlines(&mut self, inl: &[Inline], base: &Style) -> Vec<Span> {
        let mut out = Vec::new();
        self.inlines_into(inl, base, &mut out);
        out
    }

    fn inlines_into(&mut self, inl: &[Inline], st: &Style, out: &mut Vec<Span>) {
        for i in inl {
            match i {
                Inline::Text(t) => out.push(Span::new(t.clone(), st.clone())),
                Inline::Code(t) => out.push(Span::new(
                    t.clone(),
                    Style {
                        fg: Some(GREEN),
                        ..st.clone()
                    },
                )),
                Inline::Math(t) => out.push(Span::new(
                    t.clone(),
                    Style {
                        fg: Some(MAGENTA),
                        italic: true,
                        ..st.clone()
                    },
                )),
                Inline::Emph(c) => self.inlines_into(
                    c,
                    &Style {
                        italic: true,
                        ..st.clone()
                    },
                    out,
                ),
                Inline::Strong(c) => self.inlines_into(
                    c,
                    &Style {
                        bold: true,
                        ..st.clone()
                    },
                    out,
                ),
                Inline::Strike(c) => self.inlines_into(
                    c,
                    &Style {
                        strike: true,
                        ..st.clone()
                    },
                    out,
                ),
                Inline::Link { url, content } => {
                    let ls = Style {
                        fg: Some(BLUE),
                        underline: true,
                        link: Some(Rc::from(url.as_str())),
                        ..st.clone()
                    };
                    let start = out.len();
                    self.inlines_into(content, &ls, out);
                    let text: String = out[start..].iter().map(|s| s.text.as_str()).collect();
                    if self.opts.show_urls && !url.starts_with('#') && text != *url {
                        out.push(Span::new(format!(" ({url})"), Style::dim()));
                    }
                }
                Inline::Image { url, alt } => {
                    let name = if alt.is_empty() {
                        url.rsplit('/').next().unwrap_or(url)
                    } else {
                        alt
                    };
                    out.push(Span::new(
                        format!("[image: {name}]"),
                        Style {
                            dim: true,
                            italic: true,
                            ..st.clone()
                        },
                    ));
                }
                Inline::FootnoteRef(l) => out.push(Span::new(format!("[{l}]"), Style::fg(CYAN))),
                Inline::SoftBreak => out.push(Span::new(" ", st.clone())),
                Inline::HardBreak => out.push(Span::new("\n", st.clone())),
                Inline::Task(_) => {}
            }
        }
    }
}

/// Prepend `first` to the first line and `rest` to the others.
fn prefix(lines: Vec<Line>, first: Vec<Span>, rest: Vec<Span>) -> Vec<Line> {
    lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let mut spans = if i == 0 { first.clone() } else { rest.clone() };
            spans.extend(line.spans);
            Line { spans, ..line }
        })
        .collect()
}

/// A word: styled fragments with no whitespace between them.
#[derive(Default)]
struct Word {
    parts: Vec<Span>,
    width: usize,
    /// Style of the whitespace before this word, if any.
    space_before: Option<Style>,
    /// This word starts after a hard line break.
    break_before: bool,
}

fn words(spans: &[Span]) -> Vec<Word> {
    let mut out = Vec::new();
    let mut cur = Word::default();
    let mut pending_space: Option<Style> = None;
    let mut pending_break = false;
    let flush = |cur: &mut Word, out: &mut Vec<Word>| {
        if !cur.parts.is_empty() {
            out.push(std::mem::take(cur));
        }
    };
    for span in spans {
        let mut buf = String::new();
        for ch in span.text.chars() {
            if ch == '\n' || (ch.is_whitespace() && ch != '\u{a0}') {
                if !buf.is_empty() {
                    cur.width += buf.width();
                    cur.parts
                        .push(Span::new(std::mem::take(&mut buf), span.style.clone()));
                }
                flush(&mut cur, &mut out);
                if ch == '\n' {
                    pending_break = true;
                    pending_space = None;
                } else if !pending_break {
                    pending_space = Some(span.style.clone());
                }
                continue;
            }
            if buf.is_empty() && cur.parts.is_empty() {
                cur.space_before = pending_space.take();
                cur.break_before = std::mem::take(&mut pending_break);
            }
            buf.push(ch);
        }
        if !buf.is_empty() {
            cur.width += buf.width();
            cur.parts.push(Span::new(buf, span.style.clone()));
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// Greedy word wrap. Words wider than `width` are split by character.
pub fn wrap(spans: &[Span], width: usize) -> Vec<Line> {
    let width = width.max(1);
    let mut lines = vec![Line::default()];
    let mut cur_w = 0;
    for word in words(spans) {
        if word.break_before && (cur_w > 0 || lines.len() > 1 || !lines[0].spans.is_empty()) {
            lines.push(Line::default());
            cur_w = 0;
        }
        let space = if cur_w > 0 && word.space_before.is_some() {
            1
        } else {
            0
        };
        if cur_w > 0 && cur_w + space + word.width > width {
            lines.push(Line::default());
            cur_w = 0;
        } else if space == 1 {
            let st = word.space_before.clone().unwrap();
            lines.last_mut().unwrap().push(Span::new(" ", st));
            cur_w += 1;
        }
        if word.width <= width - cur_w {
            for p in word.parts {
                lines.last_mut().unwrap().push(p);
            }
            cur_w += word.width;
            continue;
        }
        // Overlong word: split by character.
        for p in word.parts {
            for ch in p.text.chars() {
                let cw = ch.width().unwrap_or(0);
                if cur_w + cw > width && cur_w > 0 {
                    lines.push(Line::default());
                    cur_w = 0;
                }
                lines
                    .last_mut()
                    .unwrap()
                    .push(Span::new(ch.to_string(), p.style.clone()));
                cur_w += cw;
            }
        }
    }
    if lines.len() == 1 && lines[0].spans.is_empty() {
        return Vec::new();
    }
    lines
}

/// Break a line of spans into chunks of at most `width` columns, by character.
fn hard_wrap(spans: &[Span], width: usize) -> Vec<Vec<Span>> {
    let mut out = vec![Line::default()];
    let mut w = 0;
    for s in spans {
        for ch in s.text.chars() {
            let cw = ch.width().unwrap_or(0);
            if w + cw > width && w > 0 {
                out.push(Line::default());
                w = 0;
            }
            out.last_mut()
                .unwrap()
                .push(Span::new(ch.to_string(), s.style.clone()));
            w += cw;
        }
    }
    out.into_iter().map(|l| l.spans).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(lines: &[Line]) -> Vec<String> {
        lines.iter().map(Line::text).collect()
    }

    #[test]
    fn wraps_words() {
        let l = wrap(&[Span::plain("the quick brown fox jumps")], 10);
        assert_eq!(texts(&l), ["the quick", "brown fox", "jumps"]);
    }

    #[test]
    fn keeps_punctuation_with_styled_word() {
        let spans = [
            Span::plain("aaaa "),
            Span::new("bold", Style::default().bold()),
            Span::plain(", x"),
        ];
        let l = wrap(&spans, 9);
        assert_eq!(texts(&l), ["aaaa", "bold, x"]);
    }

    #[test]
    fn splits_long_words_and_cjk() {
        let l = wrap(&[Span::plain("漢字漢字漢字")], 5);
        assert_eq!(texts(&l), ["漢字", "漢字", "漢字"]);
    }

    #[test]
    fn hard_break() {
        let l = wrap(&[Span::plain("a\nb c")], 20);
        assert_eq!(texts(&l), ["a", "b c"]);
    }
}
