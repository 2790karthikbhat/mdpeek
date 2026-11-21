# mdpeek

A fast, text-first Markdown viewer for every terminal.

[![CI](https://github.com/2790karthikbhat/mdpeek/actions/workflows/ci.yml/badge.svg)](https://github.com/2790karthikbhat/mdpeek/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/mdpeek.svg)](https://crates.io/crates/mdpeek)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Body text is real terminal text: fast, selectable and copyable. Local images are drawn
as graphics where the terminal supports it (kitty, Ghostty, WezTerm, iTerm2, sixel
terminals) and shown as `[image: alt]` everywhere else, including tmux. Works on
Linux, macOS and Windows as a single binary with no dependencies.

- GitHub-flavored Markdown: tables, task lists, footnotes, strikethrough, alerts (`> [!NOTE]`)
- Syntax-highlighted code blocks with a language label
- Live reload: save the file and the view updates in place
- Rewraps on resize; tables fit the width, CJK and emoji included
- `--plain` mode prints to stdout for pipes, scripts, CI and AI agents, with clickable
  OSC 8 links when colors are on

## Install

**macOS / Linux**

```sh
curl -fsSL https://raw.githubusercontent.com/2790karthikbhat/mdpeek/main/scripts/install.sh | sh
```

**Windows (PowerShell)**

```powershell
irm https://raw.githubusercontent.com/2790karthikbhat/mdpeek/main/scripts/install.ps1 | iex
```

**With Rust** (1.91 or newer)

```sh
cargo install mdpeek
```

The scripts download the binary for your platform from the
[latest release](https://github.com/2790karthikbhat/mdpeek/releases/latest), verify its
SHA-256 checksum and put it in `~/.local/bin` (Windows: `%LOCALAPPDATA%\Programs\mdpeek`).
They are short; [read them](scripts/) before piping to a shell if you like.

**With an AI coding agent.** Ask Claude Code, Codex, Cursor or similar:

> Install mdpeek from https://github.com/2790karthikbhat/mdpeek

Agents find non-interactive install steps in [AGENTS.md](AGENTS.md) and
[llms.txt](llms.txt).

## Usage

```sh
mdpeek README.md            # open the pager
cat notes.md | mdpeek       # read from stdin
mdpeek --plain README.md    # print to stdout (also the default when piped)
mdpeek --plain --color never -w 72 doc.md
```

| Keys | Action |
| --- | --- |
| `j` `k` `↓` `↑` `Enter` | Scroll one line |
| `Space` `f` `PgDn` / `b` `PgUp` | Page down / up |
| `d` `u` (or `Ctrl-d` `Ctrl-u`) | Half page down / up |
| `g` `Home` / `G` `End` | Top / bottom |
| Mouse wheel | Scroll (text selection still works) |
| `q` `Esc` `Ctrl-c` | Quit |

Try the tour: `mdpeek fixtures/demo.md` in a clone of this repo.

## Safety

mdpeek never touches the network: remote images are shown as placeholders.
Control characters in documents are replaced, so a file can't send escape
sequences to your terminal.

## Roadmap

- **0.2:** search (`/`, `n`, `N`), contents sidebar, following links to local `.md` files,
  fuzzy file picker
- **0.3:** Mermaid diagrams, rendered math, light theme and config file, opt-in remote images

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bug reports with a small Markdown sample are
the most helpful.

## License

[MIT](LICENSE)
