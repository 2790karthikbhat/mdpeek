# AGENTS.md

Guide for AI coding agents (Claude Code, Codex, Cursor, Gemini CLI, Copilot and others).
Humans should start with [README.md](README.md).

mdpeek is a text-first Markdown viewer for the terminal, written in Rust. Body text is real
terminal text; local images are drawn with the kitty, sixel or iTerm2 protocols when the
terminal supports them. It runs on Linux, macOS and Windows, never uses the network, and
ships as one static binary with no runtime dependencies.

## Installing mdpeek for a user

Every command below is non-interactive and safe to re-run.

| Platform | Command |
| --- | --- |
| macOS, Linux | `curl -fsSL https://raw.githubusercontent.com/2790karthikbhat/mdpeek/main/scripts/install.sh \| sh` |
| Windows (PowerShell) | `irm https://raw.githubusercontent.com/2790karthikbhat/mdpeek/main/scripts/install.ps1 \| iex` |
| Any, with Rust ≥ 1.91 | `cargo install mdpeek` |
| From source | `cargo install --git https://github.com/2790karthikbhat/mdpeek` |

The scripts download the prebuilt binary for the current OS and CPU from the latest GitHub
release, check its SHA-256, and install it:

- macOS and Linux: `~/.local/bin/mdpeek`. Set `MDPEEK_INSTALL_DIR` to choose another folder.
- Windows: `%LOCALAPPDATA%\Programs\mdpeek\mdpeek.exe`, added to the user `PATH`.
- Set `MDPEEK_VERSION=v0.1.0` to pin a release. Exit code `0` means installed, anything else failed
  and the message says why.
- Prebuilt targets: Linux x86_64 and aarch64 (static musl), macOS arm64 and x86_64, Windows x86_64.
  Elsewhere, use `cargo install mdpeek`.

Verify with `mdpeek --version`. If the shell can't find it, the install folder isn't on `PATH`;
the script prints the line to add. Uninstall by deleting the binary (or `cargo uninstall mdpeek`).

## Using mdpeek from an agent

`mdpeek FILE` opens an interactive pager, which an agent can't drive. It falls back to printing
when stdout isn't a terminal, but be explicit:

```sh
mdpeek --plain --color never -w 100 README.md   # plain text, wrapped at 100 columns
mdpeek --plain --color always README.md          # with ANSI colors and OSC 8 links
```

Don't run `mdpeek` with no file and an open stdin: it waits to read Markdown from stdin.

## Working on the code

### Setup

Rust ≥ 1.91 (`rustup` recommended). Everything is pure Rust: no C libraries, no system packages.

```sh
cargo build
```

### Commands

| Task | Command |
| --- | --- |
| Run the pager | `cargo run -- fixtures/demo.md` |
| Plain output | `cargo run -- --plain --color never fixtures/demo.md` |
| **Everything CI runs** | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` |
| One test | `cargo test <name_substring>` |
| Accept changed snapshots | `cargo insta review` (or `INSTA_UPDATE=always cargo test`, then read the diff) |

Before calling a change done, the CI line above must pass. `.github/workflows/ci.yml` runs it on
Ubuntu, macOS and Windows, plus a `cargo check` on the minimum Rust version.

You can't see the pager in an agent shell. Verify rendering with `--plain` output and the snapshot
tests: layout is shared, so `--plain` shows exactly the lines the pager draws.

### Layout

```
src/
  main.rs        CLI (clap): picks the pager or --plain
  lib.rs         Module list and render_plain(), used by the tests
  doc.rs         pulldown-cmark events -> Doc { blocks } (GFM, footnotes, alerts, math, HTML <img>)
  layout.rs      Doc + width -> Vec<Line> + image slots + heading index. Pure: no terminal I/O
  style.rs       Terminal-agnostic Style/Span/Line; sanitizes control characters
  highlight.rs   syntect, loaded lazily
  plain.rs       Lines -> stdout text, optional ANSI and OSC 8
  images.rs      Local image sizing, decoding and ratatui-image protocols
  watch.rs       notify on the parent folder (live reload)
  ui/            Pager: app.rs (pure state + keys), mod.rs (event loop, drawing)
fixtures/        Sample Markdown used by tests; demo.md is the full tour
tests/           insta snapshots of --plain output for each fixture
scripts/         install.sh, install.ps1
```

### Conventions

- **Text first.** Every element needs a text rendering; graphics only replace it when detected
  support allows. Never let the pager refuse to start.
- **Keep `layout` pure.** No terminal queries or I/O there; pass what it needs in `layout::Opts`.
- **Untrusted input.** Build all document text through `Span::new` so control characters are
  replaced. Don't add network access; remote images stay placeholders unless explicitly opted in.
- **Small and boring.** Prefer well-known crates, keep dependencies few, avoid C libraries.
- New Markdown features get a fixture or a fixture section, plus a snapshot.
- Run `cargo fmt`. Clippy warnings fail CI.

### Releasing

Bump `version` in `Cargo.toml`, update `CHANGELOG.md`, commit, then push a `vX.Y.Z` tag.
`.github/workflows/release.yml` builds the binaries and publishes the GitHub release that the
install scripts download from. `cargo publish` is a separate, manual step.
