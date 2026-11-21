# Changelog

## 0.1.0 (2026-11-21)

- First release. Pager (ratatui + crossterm) and `--plain` stdout mode, sharing one pure layout.
- GFM: headings, emphasis, strikethrough, lists, task lists, tables, footnotes, alerts, front matter.
- Fenced code highlighted with syntect; math shown as TeX source.
- Local images via kitty, sixel or iTerm2 (ratatui-image); `[image: alt]` elsewhere.
- OSC 8 hyperlinks in `--plain` color output; URLs shown inline without color.
- Live reload that keeps the scroll position (and follows the end of a growing file).
- Control characters in documents are replaced so files can't send escape sequences.
