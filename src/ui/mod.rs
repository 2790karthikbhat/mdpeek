//! Interactive pager: event loop and drawing.

pub mod app;

use crate::doc::{self, Doc};
use crate::images::Images;
use crate::layout::{self, Layout};
use crate::style::{self, Color};
use crate::watch;
use anyhow::Result;
use app::{Pager, key_action};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color as RColor, Modifier, Style as RStyle};
use ratatui::text::{Line as RLine, Span as RSpan};
use ratatui::widgets::Paragraph;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::sliced::{SignedPosition, SlicedImage};
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct Input {
    pub path: Option<PathBuf>,
    pub src: String,
    pub max_width: usize,
}

/// Wait this long after the last file event before reloading, so a burst
/// of writes from one save becomes a single reload.
const RELOAD_DEBOUNCE: Duration = Duration::from_millis(80);

pub fn run(input: Input) -> Result<()> {
    let mut terminal = ratatui::init();
    // Graphics detection talks to the terminal over stdin; skip it when
    // stdin is a pipe (`cat x.md | mdpeek`).
    // If the terminal never answers, the query gives up after 2s, and its
    // reader thread swallows the first key pressed (ratatui-image 11.1).
    let picker = if std::io::stdin().is_terminal() {
        Picker::from_query_stdio()
            .ok()
            .filter(|p| p.protocol_type() != ProtocolType::Halfblocks)
    } else {
        None
    };
    // Alternate scroll mode: the wheel sends arrow keys, and the terminal's
    // own text selection keeps working because we don't capture the mouse.
    let _ = write!(std::io::stdout(), "\x1b[?1007h");
    let _ = std::io::stdout().flush();

    let result = View::new(input, picker).and_then(|mut v| v.run(&mut terminal));

    let _ = write!(std::io::stdout(), "\x1b[?1007l");
    ratatui::restore();
    result
}

struct View {
    path: Option<PathBuf>,
    name: String,
    max_width: usize,
    doc: Doc,
    layout: Layout,
    images: Images,
    pager: Pager,
    /// Size the current layout was made for.
    size: (u16, u16),
    message: Option<String>,
}

impl View {
    fn new(input: Input, picker: Option<Picker>) -> Result<Self> {
        let base = match &input.path {
            Some(p) => std::path::absolute(p)?
                .parent()
                .map(PathBuf::from)
                .unwrap_or_default(),
            None => std::env::current_dir()?,
        };
        let name = style::sanitize(match &input.path {
            Some(p) => p
                .file_name()
                .map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into()),
            None => "stdin".to_string(),
        });
        let doc = doc::parse(&input.src);
        let mut images = Images::new(base, picker);
        images.scan(&doc);
        Ok(View {
            path: input.path,
            name,
            max_width: input.max_width,
            doc,
            layout: Layout::default(),
            images,
            pager: Pager::default(),
            size: (0, 0),
            message: None,
        })
    }

    /// Content rectangle: centered, at most `max_width` wide, above the status bar.
    fn content_rect(&self, area: Rect) -> Rect {
        let h = area.height.saturating_sub(1);
        let w = (area.width.saturating_sub(2) as usize)
            .min(self.max_width)
            .max(1) as u16;
        let x = area.x + area.width.saturating_sub(w) / 2;
        Rect {
            x,
            y: area.y,
            width: w.min(area.width),
            height: h,
        }
    }

    fn relayout(&mut self, area: Rect) {
        let content = self.content_rect(area);
        let old_len = self.pager.len.max(1);
        let old_scroll = self.pager.scroll;
        let max_rows = content.height.saturating_sub(1).max(1);
        let images = &self.images;
        let sizer = |src: &str, cols: u16| images.size(src, cols, max_rows);
        let opts = layout::Opts {
            width: content.width as usize,
            show_urls: false,
            images: images.enabled().then_some(&sizer as layout::ImageSizer),
        };
        self.layout = layout::layout(&self.doc, &opts);
        self.pager.len = self.layout.lines.len();
        self.pager.height = content.height as usize;
        // Keep roughly the same place in the document when the width changes.
        if self.size.0 != area.width && self.size != (0, 0) {
            self.pager.scroll = old_scroll * self.pager.len / old_len;
        }
        self.pager.clamp();
        self.size = (area.width, area.height);
    }

    fn reload(&mut self) {
        let Some(path) = &self.path else { return };
        match std::fs::read_to_string(path) {
            Ok(src) => {
                let at_bottom =
                    self.pager.max_scroll() > 0 && self.pager.scroll >= self.pager.max_scroll();
                self.doc = doc::parse(&src);
                self.images.scan(&self.doc);
                // Same width: the scroll position carries over line for line.
                let (w, h) = self.size;
                self.size = (0, 0);
                self.relayout(Rect::new(0, 0, w, h));
                // Follow the end of a growing file, like `tail -f`.
                if at_bottom {
                    self.pager.scroll = self.pager.max_scroll();
                }
                self.message = Some("reloaded".into());
            }
            Err(e) => self.message = Some(format!("reload failed: {e}")),
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let _watcher = match &self.path {
            Some(p) => match watch::watch(p, tx) {
                Ok(w) => Some(w),
                Err(e) => {
                    self.message = Some(format!("live reload off: {e}"));
                    None
                }
            },
            None => None,
        };
        let mut reload_at: Option<Instant> = None;
        let mut message_until: Option<Instant> = None;
        let mut dirty = true;

        loop {
            let area: Rect = terminal.size()?.into();
            if (area.width, area.height) != self.size {
                self.relayout(area);
                dirty = true;
            }
            if dirty {
                terminal.draw(|f| self.draw(f))?;
                dirty = false;
                if self.message.is_some() && message_until.is_none() {
                    message_until = Some(Instant::now() + Duration::from_secs(2));
                }
            }

            let now = Instant::now();
            let next_deadline = [reload_at, message_until].into_iter().flatten().min();
            let timeout = next_deadline.map_or(Duration::from_millis(250), |t| {
                t.saturating_duration_since(now)
                    .min(Duration::from_millis(250))
            });
            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(k) if k.kind != KeyEventKind::Release => {
                        if let Some(a) = key_action(k) {
                            let before = self.pager.scroll;
                            self.pager.apply(a);
                            dirty |= self.pager.scroll != before;
                        }
                    }
                    Event::Resize(..) => dirty = true,
                    _ => {}
                }
            }
            if self.pager.quit {
                return Ok(());
            }
            while rx.try_recv().is_ok() {
                reload_at = Some(Instant::now() + RELOAD_DEBOUNCE);
            }
            let now = Instant::now();
            if reload_at.is_some_and(|t| now >= t) {
                reload_at = None;
                self.reload();
                message_until = None;
                dirty = true;
            }
            if message_until.is_some_and(|t| now >= t) {
                message_until = None;
                self.message = None;
                dirty = true;
            }
        }
    }

    fn draw(&mut self, f: &mut ratatui::Frame) {
        let area = f.area();
        let content = self.content_rect(area);
        let top = self.pager.scroll;
        let bottom = (top + content.height as usize).min(self.layout.lines.len());
        let lines: Vec<RLine> = self.layout.lines[top..bottom]
            .iter()
            .map(to_rline)
            .collect();
        f.render_widget(Paragraph::new(lines), content);

        for slot in &self.layout.images {
            let (start, end) = (slot.line, slot.line + slot.height as usize);
            if end <= top || start >= bottom {
                continue;
            }
            let y = start as i64 - top as i64;
            match self.images.protocol(slot) {
                Some(proto) => {
                    let pos = SignedPosition {
                        x: slot.col as i16,
                        y: y as i16,
                    };
                    f.render_widget(SlicedImage::new(proto, pos), content);
                }
                None if y >= 0 => {
                    let r = Rect {
                        y: content.y + y as u16,
                        height: 1,
                        ..content
                    };
                    let alt = if slot.alt.is_empty() {
                        &slot.src
                    } else {
                        &slot.alt
                    };
                    let st = RStyle::default().add_modifier(Modifier::DIM | Modifier::ITALIC);
                    let text = style::sanitize(format!("[image: {alt}]"));
                    f.render_widget(Paragraph::new(text).style(st), r);
                }
                None => {}
            }
        }

        let [_, status] =
            ratatui::layout::Layout::vertical([Constraint::Min(0), Constraint::Length(1)])
                .areas(area);
        let right = format!("{}%  q quit ", self.pager.percent());
        let left = match &self.message {
            Some(m) => format!(" {}  ·  {m}", self.name),
            None => format!(" {}", self.name),
        };
        let pad = (status.width as usize).saturating_sub(left.chars().count() + right.len());
        let bar = format!("{left}{}{right}", " ".repeat(pad));
        f.render_widget(
            Paragraph::new(bar).style(RStyle::default().add_modifier(Modifier::REVERSED)),
            status,
        );
    }
}

fn to_rline(line: &style::Line) -> RLine<'static> {
    RLine::from(
        line.spans
            .iter()
            .map(|s| RSpan::styled(s.text.clone(), to_rstyle(&s.style)))
            .collect::<Vec<_>>(),
    )
}

fn to_rstyle(st: &style::Style) -> RStyle {
    let mut r = RStyle::default();
    if let Some(c) = st.fg {
        r = r.fg(to_rcolor(c));
    }
    let mods = [
        (st.bold, Modifier::BOLD),
        (st.dim, Modifier::DIM),
        (st.italic, Modifier::ITALIC),
        (st.underline, Modifier::UNDERLINED),
        (st.strike, Modifier::CROSSED_OUT),
    ];
    for (on, m) in mods {
        if on {
            r = r.add_modifier(m);
        }
    }
    r
}

fn to_rcolor(c: Color) -> RColor {
    match c {
        Color::Rgb(r, g, b) => RColor::Rgb(r, g, b),
        Color::Ansi(n) => match n {
            0 => RColor::Black,
            1 => RColor::Red,
            2 => RColor::Green,
            3 => RColor::Yellow,
            4 => RColor::Blue,
            5 => RColor::Magenta,
            6 => RColor::Cyan,
            7 => RColor::Gray,
            8 => RColor::DarkGray,
            n => RColor::Indexed(n),
        },
    }
}
