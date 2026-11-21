# mdpeek demo

A tour of everything **mdpeek** renders. Scroll with `j`/`k`, the mouse wheel,
`Space`, or jump with `g`/`G`. Try selecting and copying this text with the
mouse: it's real terminal text.

## Text

Plain, *italic*, **bold**, ***both***, ~~struck~~ and `inline code`. Links look
like [the Rust site](https://www.rust-lang.org), and autolinks like
<https://github.com>. A footnote reference sits here[^why].

This paragraph is long on purpose so you can watch it rewrap when you resize the
window: make the terminal narrow, then wide again, and the text should reflow
while staying at roughly the same place in the document.

Unicode width check: 日本語のテキスト, 한국어, emoji 🦀🚀, and an em dash — all fine.

## Lists

- Text first
- Graphics as an upgrade
  - kitty, Ghostty, WezTerm
  - iTerm2
  - sixel terminals
    - and placeholders everywhere else
- Small and boring

1. Parse once
2. Lay out per width
3. Draw

- [x] Pager and `--plain` mode
- [x] Live reload
- [ ] Search (v0.2)
- [ ] Contents sidebar (v0.2), with a long description that wraps under its checkbox

## Quotes and alerts

> Text is the interface. Graphics are a bonus.
>
> > Nested quotes get a second bar.

> [!NOTE]
> Alerts use GitHub's `> [!NOTE]` syntax.

> [!TIP]
> Edit this file in another window and save: the view reloads in place.

> [!WARNING]
> Remote images are never fetched.

> [!CAUTION]
> Control characters in documents are neutralized.

## Code

```rust
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut src = String::new();
    io::stdin().read_to_string(&mut src)?;
    let words = src.split_whitespace().count();
    println!("{words} words"); // count them
    Ok(())
}
```

```python
def fib(n: int) -> int:
    """Return the n-th Fibonacci number."""
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a
```

```sh
cat README.md | mdpeek            # pipe in
mdpeek --plain -w 72 README.md    # print to stdout
```

## Tables

| Feature | mdpeek | Notes |
| :--- | :---: | ---: |
| Body text | Terminal text | selectable |
| Images | kitty / sixel / iTerm2 | placeholders elsewhere |
| Platforms | Linux, macOS, Windows | 3 |
| Startup | fast | ~70 ms for 25k lines |

## Images

A local image, drawn as graphics when your terminal supports it:

![A red square](img/square.png)

An inline image ![icon](img/square.png) stays a placeholder, and so does a remote one:

![Remote badge](https://example.com/badge.svg)

## Math

Inline math like $e^{i\pi} + 1 = 0$ shows its TeX source, and so does display math:

$$
\sum_{k=1}^{n} k = \frac{n(n+1)}{2}
$$

---

That's the tour. Press `q` to quit.

[^why]: Footnotes render where they are defined, usually at the end.
