//! Local images: resolve, size, decode and encode for the terminal.
//!
//! Only local files are loaded; remote URLs always fall back to the
//! `[image: alt]` placeholder (remote images are opt-in, planned for v0.3).

use crate::doc::{Block, Doc, Inline};
use crate::layout::ImageSlot;
use image::{DynamicImage, ImageReader};
use ratatui::layout::Size;
use ratatui_image::Resize;
use ratatui_image::picker::Picker;
use ratatui_image::sliced::SlicedProtocol;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Images {
    base: PathBuf,
    picker: Option<Picker>,
    /// Pixel size of each image the document references, when readable.
    dims: HashMap<String, (u32, u32)>,
    decoded: HashMap<String, Option<DynamicImage>>,
    protos: HashMap<(String, u16, u16), Option<SlicedProtocol>>,
}

impl Images {
    /// `picker` is `None` when the terminal has no usable graphics protocol.
    pub fn new(base: PathBuf, picker: Option<Picker>) -> Self {
        Images {
            base,
            picker,
            dims: HashMap::new(),
            decoded: HashMap::new(),
            protos: HashMap::new(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.picker.is_some()
    }

    /// Read the header of every local image in `doc`. Clears old caches, so
    /// call it again after a reload.
    pub fn scan(&mut self, doc: &Doc) {
        self.dims.clear();
        self.decoded.clear();
        self.protos.clear();
        if self.picker.is_none() {
            return;
        }
        let mut srcs = Vec::new();
        collect_blocks(&doc.blocks, &mut srcs);
        for src in srcs {
            if self.dims.contains_key(&src) {
                continue;
            }
            let Some(path) = resolve(&self.base, &src) else {
                continue;
            };
            let dims = ImageReader::open(&path)
                .and_then(|r| r.with_guessed_format())
                .ok()
                .and_then(|r| r.into_dimensions().ok());
            if let Some(d) = dims {
                self.dims.insert(src, d);
            }
        }
    }

    /// Size in cells for `src`, fitted within `max_cols` x `max_rows`.
    pub fn size(&self, src: &str, max_cols: u16, max_rows: u16) -> Option<(u16, u16)> {
        let picker = self.picker.as_ref()?;
        let &(w, h) = self.dims.get(src)?;
        let fs = picker.font_size();
        let (fw, fh) = (fs.width.max(1) as f64, fs.height.max(1) as f64);
        let mut cols = (w as f64 / fw).ceil();
        let mut rows = (h as f64 / fh).ceil();
        let scale = (max_cols as f64 / cols)
            .min(max_rows.max(1) as f64 / rows)
            .min(1.0);
        cols = (cols * scale).floor().max(1.0);
        rows = (rows * scale).ceil().max(1.0);
        Some((cols as u16, rows as u16))
    }

    /// The encoded image for a slot, decoding on first use.
    pub fn protocol(&mut self, slot: &ImageSlot) -> Option<&SlicedProtocol> {
        let picker = self.picker.as_ref()?;
        let key = (slot.src.clone(), slot.width, slot.height);
        if !self.protos.contains_key(&key) {
            let img = self
                .decoded
                .entry(slot.src.clone())
                .or_insert_with(|| {
                    let path = resolve(&self.base, &slot.src)?;
                    ImageReader::open(path)
                        .ok()?
                        .with_guessed_format()
                        .ok()?
                        .decode()
                        .ok()
                })
                .clone();
            let proto = img.and_then(|img| {
                let size = Size::new(slot.width, slot.height);
                SlicedProtocol::new_with_resize(picker, img, size, Resize::Fit(None)).ok()
            });
            self.protos.insert(key.clone(), proto);
        }
        self.protos.get(&key)?.as_ref()
    }
}

/// Map an image reference to a local path. Remote and `data:` URLs are refused.
fn resolve(base: &Path, src: &str) -> Option<PathBuf> {
    let src = src.trim();
    if src.is_empty() || src.contains("://") || src.starts_with("data:") || src.starts_with("//") {
        return None;
    }
    let src = src.split(['?', '#']).next()?.replace("%20", " ");
    let p = Path::new(&src);
    Some(if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    })
}

fn collect_blocks(blocks: &[Block], out: &mut Vec<String>) {
    for b in blocks {
        match b {
            Block::Paragraph(inl) | Block::Heading { content: inl, .. } => {
                collect_inlines(inl, out)
            }
            Block::Quote { blocks, .. } | Block::Footnote { blocks, .. } => {
                collect_blocks(blocks, out)
            }
            Block::List { items, .. } => items.iter().for_each(|i| collect_blocks(&i.blocks, out)),
            _ => {}
        }
    }
}

fn collect_inlines(inl: &[Inline], out: &mut Vec<String>) {
    for i in inl {
        match i {
            Inline::Image { url, .. } => out.push(url.clone()),
            Inline::Emph(c)
            | Inline::Strong(c)
            | Inline::Strike(c)
            | Inline::Link { content: c, .. } => collect_inlines(c, out),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_refuses_remote() {
        let base = Path::new("/docs");
        assert_eq!(resolve(base, "https://x.io/a.png"), None);
        assert_eq!(resolve(base, "data:image/png;base64,AAAA"), None);
        assert_eq!(
            resolve(base, "img/a%20b.png?raw=1"),
            Some(PathBuf::from("/docs/img/a b.png"))
        );
    }
}
