//! Markdown source -> block tree.
//!
//! The tree keeps only what the renderer needs. Parsing happens once per
//! load; layout runs again on every resize.

use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    Code {
        lang: Option<String>,
        text: String,
    },
    Quote {
        kind: Option<Alert>,
        blocks: Vec<Block>,
    },
    List {
        start: Option<u64>,
        tight: bool,
        items: Vec<Item>,
    },
    Table {
        aligns: Vec<Align>,
        head: Vec<Cell>,
        rows: Vec<Vec<Cell>>,
    },
    Rule,
    Math(String),
    Footnote {
        label: String,
        blocks: Vec<Block>,
    },
    /// Front matter (YAML or TOML metadata block).
    FrontMatter(String),
}

pub type Cell = Vec<Inline>;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alert {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Code(String),
    Math(String),
    Emph(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    Link {
        url: String,
        content: Vec<Inline>,
    },
    Image {
        url: String,
        alt: String,
    },
    FootnoteRef(String),
    SoftBreak,
    HardBreak,
    /// Task list marker; lifted into `Item::task` after parsing.
    Task(bool),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Doc {
    pub blocks: Vec<Block>,
}

pub fn parse(src: &str) -> Doc {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM
        | Options::ENABLE_MATH
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
    let mut events = Parser::new_ext(src, opts).peekable();
    let blocks = parse_blocks(&mut events);
    Doc { blocks }
}

type Events<'a> = Peekable<Parser<'a>>;

fn is_inline_event(ev: &Event) -> bool {
    match ev {
        Event::Text(_)
        | Event::Code(_)
        | Event::InlineMath(_)
        | Event::DisplayMath(_)
        | Event::InlineHtml(_)
        | Event::FootnoteReference(_)
        | Event::SoftBreak
        | Event::HardBreak
        | Event::TaskListMarker(_) => true,
        Event::Start(t) => matches!(
            t,
            Tag::Emphasis
                | Tag::Strong
                | Tag::Strikethrough
                | Tag::Superscript
                | Tag::Subscript
                | Tag::Link { .. }
                | Tag::Image { .. }
        ),
        _ => false,
    }
}

/// Parse blocks until an `End` event (left unconsumed) or the end of input.
fn parse_blocks(ev: &mut Events) -> Vec<Block> {
    let mut out = Vec::new();
    while let Some(next) = ev.peek() {
        if matches!(next, Event::End(_)) {
            break;
        }
        if is_inline_event(next) {
            // Tight list items put inline content directly in the item.
            let inl = parse_inlines(ev);
            push_paragraph(&mut out, inl);
            continue;
        }
        let event = ev.next().unwrap();
        match event {
            Event::Start(tag) => {
                parse_block(tag, ev, &mut out);
            }
            Event::Rule => out.push(Block::Rule),
            Event::Html(html) => {
                // Stray HTML outside an HtmlBlock; treat like one.
                push_paragraph(&mut out, html_to_inlines(&html));
            }
            _ => {}
        }
    }
    out
}

fn push_paragraph(out: &mut Vec<Block>, inl: Vec<Inline>) {
    // Display math on its own becomes a block.
    let mut buf = Vec::new();
    for i in inl {
        match i {
            Inline::Math(m) if m.starts_with('\n') => {
                flush_para(out, &mut buf);
                out.push(Block::Math(m.trim().to_string()));
            }
            other => buf.push(other),
        }
    }
    flush_para(out, &mut buf);
}

fn flush_para(out: &mut Vec<Block>, buf: &mut Vec<Inline>) {
    while matches!(buf.last(), Some(Inline::SoftBreak | Inline::HardBreak)) {
        buf.pop();
    }
    while matches!(buf.first(), Some(Inline::SoftBreak | Inline::HardBreak)) {
        buf.remove(0);
    }
    let blank = buf
        .iter()
        .all(|i| matches!(i, Inline::Text(t) if t.trim().is_empty()));
    if !blank {
        out.push(Block::Paragraph(std::mem::take(buf)));
    }
    buf.clear();
}

fn expect_end(ev: &mut Events) {
    if let Some(Event::End(_)) = ev.peek() {
        ev.next();
    }
}

fn parse_block(tag: Tag, ev: &mut Events, out: &mut Vec<Block>) {
    let block = match tag {
        Tag::Paragraph => {
            let inl = parse_inlines(ev);
            expect_end(ev);
            // A paragraph may split into several blocks (display math).
            push_paragraph(out, inl);
            return;
        }
        Tag::Heading { level, .. } => {
            let content = parse_inlines(ev);
            expect_end(ev);
            Block::Heading {
                level: heading_level(level),
                content,
            }
        }
        Tag::BlockQuote(kind) => {
            let blocks = parse_blocks(ev);
            expect_end(ev);
            Block::Quote {
                kind: kind.map(alert),
                blocks,
            }
        }
        Tag::CodeBlock(kind) => {
            let lang = match kind {
                CodeBlockKind::Fenced(info) => {
                    let l = info
                        .split([' ', ',', '{'])
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    (!l.is_empty()).then_some(l)
                }
                CodeBlockKind::Indented => None,
            };
            let text = collect_text(ev);
            Block::Code { lang, text }
        }
        Tag::HtmlBlock => {
            let html = collect_text(ev);
            let inl = html_to_inlines(&html);
            if inl.is_empty() {
                return;
            }
            Block::Paragraph(inl)
        }
        Tag::MetadataBlock(_) => Block::FrontMatter(collect_text(ev).trim_end().to_string()),
        Tag::List(start) => {
            let mut items = Vec::new();
            let mut tight = true;
            while let Some(Event::Start(Tag::Item)) = ev.peek() {
                ev.next();
                if let Some(Event::Start(Tag::Paragraph)) = ev.peek() {
                    tight = false;
                }
                let mut blocks = parse_blocks(ev);
                expect_end(ev);
                let task = take_task(&mut blocks);
                items.push(Item { task, blocks });
            }
            expect_end(ev);
            Block::List {
                start,
                tight,
                items,
            }
        }
        Tag::FootnoteDefinition(label) => {
            let blocks = parse_blocks(ev);
            expect_end(ev);
            Block::Footnote {
                label: label.to_string(),
                blocks,
            }
        }
        Tag::Table(aligns) => {
            let aligns = aligns
                .into_iter()
                .map(|a| match a {
                    Alignment::Center => Align::Center,
                    Alignment::Right => Align::Right,
                    _ => Align::Left,
                })
                .collect();
            let mut head = Vec::new();
            let mut rows = Vec::new();
            loop {
                match ev.next() {
                    Some(Event::Start(Tag::TableHead)) => head = parse_cells(ev),
                    Some(Event::Start(Tag::TableRow)) => rows.push(parse_cells(ev)),
                    Some(Event::End(TagEnd::Table)) | None => break,
                    _ => {}
                }
            }
            Block::Table { aligns, head, rows }
        }
        _ => {
            skip_until_end(ev);
            return;
        }
    };
    out.push(block);
}

/// Cells of one row; consumes the row's (or head's) End.
fn parse_cells(ev: &mut Events) -> Vec<Cell> {
    let mut cells = Vec::new();
    loop {
        match ev.next() {
            Some(Event::Start(Tag::TableCell)) => {
                cells.push(parse_inlines(ev));
                expect_end(ev);
            }
            Some(Event::Start(Tag::TableRow)) => {}
            Some(Event::End(_)) | None => break,
            _ => {}
        }
    }
    cells
}

fn take_task(blocks: &mut [Block]) -> Option<bool> {
    if let Some(Block::Paragraph(inl)) = blocks.first_mut()
        && let Some(Inline::Task(done)) = inl.first()
    {
        let done = *done;
        inl.remove(0);
        if let Some(Inline::Text(t)) = inl.first_mut() {
            *t = t.trim_start().to_string();
        }
        return Some(done);
    }
    None
}

fn heading_level(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn alert(k: BlockQuoteKind) -> Alert {
    match k {
        BlockQuoteKind::Note => Alert::Note,
        BlockQuoteKind::Tip => Alert::Tip,
        BlockQuoteKind::Important => Alert::Important,
        BlockQuoteKind::Warning => Alert::Warning,
        BlockQuoteKind::Caution => Alert::Caution,
    }
}

/// Collect raw text until the matching End (consumed).
fn collect_text(ev: &mut Events) -> String {
    let mut s = String::new();
    let mut depth = 0;
    for e in ev.by_ref() {
        match e {
            Event::Start(_) => depth += 1,
            Event::End(_) if depth == 0 => break,
            Event::End(_) => depth -= 1,
            Event::Text(t) | Event::Html(t) | Event::Code(t) => s.push_str(&t),
            Event::SoftBreak | Event::HardBreak => s.push('\n'),
            _ => {}
        }
    }
    s
}

fn skip_until_end(ev: &mut Events) {
    let _ = collect_text(ev);
}

/// Parse inline content until an End or a block-level event (left unconsumed).
fn parse_inlines(ev: &mut Events) -> Vec<Inline> {
    let mut out = Vec::new();
    while let Some(next) = ev.peek() {
        if !is_inline_event(next) {
            break;
        }
        match ev.next().unwrap() {
            Event::Text(t) => push_text(&mut out, &t),
            Event::Code(t) => out.push(Inline::Code(t.to_string())),
            Event::InlineMath(t) => out.push(Inline::Math(t.to_string())),
            // Marked with a leading newline so push_paragraph can lift it.
            Event::DisplayMath(t) => out.push(Inline::Math(format!("\n{t}"))),
            Event::InlineHtml(h) => out.extend(inline_html(&h)),
            Event::FootnoteReference(l) => out.push(Inline::FootnoteRef(l.to_string())),
            Event::SoftBreak => out.push(Inline::SoftBreak),
            Event::HardBreak => out.push(Inline::HardBreak),
            Event::TaskListMarker(b) => out.push(Inline::Task(b)),
            Event::Start(tag) => {
                let tag_is_image = matches!(tag, Tag::Image { .. });
                let children = if tag_is_image {
                    Vec::new()
                } else {
                    parse_inlines(ev)
                };
                match tag {
                    Tag::Emphasis => out.push(Inline::Emph(children)),
                    Tag::Strong => out.push(Inline::Strong(children)),
                    Tag::Strikethrough => out.push(Inline::Strike(children)),
                    Tag::Link { dest_url, .. } => out.push(Inline::Link {
                        url: dest_url.to_string(),
                        content: children,
                    }),
                    Tag::Image { dest_url, .. } => {
                        let alt = collect_text(ev);
                        out.push(Inline::Image {
                            url: dest_url.to_string(),
                            alt,
                        });
                        continue; // collect_text consumed the End
                    }
                    _ => out.extend(children),
                }
                expect_end(ev);
            }
            _ => {}
        }
    }
    out
}

fn push_text(out: &mut Vec<Inline>, t: &str) {
    if let Some(Inline::Text(prev)) = out.last_mut() {
        prev.push_str(t);
    } else {
        out.push(Inline::Text(t.to_string()));
    }
}

/// Inline HTML: `<br>` becomes a hard break, `<img>` an image; other tags are dropped.
fn inline_html(h: &str) -> Vec<Inline> {
    let tag = h.trim().to_ascii_lowercase();
    if tag.starts_with("<br") {
        return vec![Inline::HardBreak];
    }
    if tag.starts_with("<img") {
        return img_tag(h).into_iter().collect();
    }
    Vec::new()
}

/// Block HTML: keep images and visible text, drop tags and comments.
fn html_to_inlines(html: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix("<!--") {
            rest = stripped.find("-->").map_or("", |i| &stripped[i + 3..]);
            continue;
        }
        if rest.starts_with('<') {
            let end = rest.find('>').map_or(rest.len(), |i| i + 1);
            out.extend(inline_html(&rest[..end]));
            rest = &rest[end..];
            continue;
        }
        let end = rest.find('<').unwrap_or(rest.len());
        let text = decode_entities(&rest[..end]);
        let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !collapsed.is_empty() {
            if text.starts_with(char::is_whitespace) && matches!(out.last(), Some(Inline::Text(_)))
            {
                push_text(&mut out, " ");
            }
            push_text(&mut out, &collapsed);
        }
        rest = &rest[end..];
    }
    out
}

fn img_tag(h: &str) -> Option<Inline> {
    let url = attr(h, "src")?;
    let alt = attr(h, "alt").unwrap_or_default();
    Some(Inline::Image { url, alt })
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let start = from + i;
        from = start + name.len();
        let before_ok = start > 0 && lower.as_bytes()[start - 1].is_ascii_whitespace();
        let rest = lower[from..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        let val_start = tag.len() - rest.len() + 1;
        let val = tag[val_start..].trim_start();
        let (quote, body) = match val.chars().next()? {
            q @ ('"' | '\'') => (Some(q), &val[1..]),
            _ => (None, val),
        };
        let end = match quote {
            Some(q) => body.find(q)?,
            None => body
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(body.len()),
        };
        return Some(decode_entities(&body[..end]));
    }
    None
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", "\u{a0}")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tight_task_list() {
        let d = parse("- [x] done\n- [ ] todo\n");
        let Block::List { items, .. } = &d.blocks[0] else {
            panic!()
        };
        assert_eq!(items[0].task, Some(true));
        assert_eq!(items[1].task, Some(false));
        assert_eq!(
            items[1].blocks,
            vec![Block::Paragraph(vec![Inline::Text("todo".into())])]
        );
    }

    #[test]
    fn html_image_extracted() {
        let d = parse("<p align=\"center\"><img src=\"logo.png\" alt=\"Logo\" width=100></p>\n");
        assert_eq!(
            d.blocks,
            vec![Block::Paragraph(vec![Inline::Image {
                url: "logo.png".into(),
                alt: "Logo".into()
            }])]
        );
    }

    #[test]
    fn alert_kind() {
        let d = parse("> [!WARNING]\n> careful\n");
        assert!(matches!(
            &d.blocks[0],
            Block::Quote {
                kind: Some(Alert::Warning),
                ..
            }
        ));
    }
}
