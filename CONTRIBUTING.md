# Contributing to mdpeek

Thanks for helping. Bug reports, small fixes and new fixtures are all welcome.

## Reporting a bug

Open an issue with:

- the smallest Markdown sample that shows the problem,
- what you expected and what you saw (`mdpeek --plain --color never FILE` output is ideal),
- your OS, terminal (and tmux/zellij if used) and `mdpeek --version`.

## Making a change

1. Install Rust 1.91 or newer.
2. `cargo run -- fixtures/demo.md` to try the pager.
3. Before opening a pull request, run what CI runs:

   ```sh
   cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
   ```

Rendering changes show up as snapshot diffs. Review them with `cargo insta review`
(`cargo install cargo-insta`) and commit the updated `.snap` files with your change.

## Design rules

- **Text first.** Every element needs a text rendering. Graphics only replace it when the
  terminal supports them, and mdpeek never refuses to start.
- **Layout stays pure.** `src/layout.rs` does no terminal I/O, so `--plain` output and the
  snapshot tests show exactly what the pager draws.
- **Safe with untrusted files.** No network access by default, and all document text goes
  through `Span::new`, which replaces control characters.
- **Small and boring.** Well-known crates, few dependencies, no C libraries.

New Markdown features need a fixture in `fixtures/` and a snapshot in `tests/snapshots.rs`.
See [AGENTS.md](AGENTS.md) for the code layout.

By contributing you agree that your work is released under the [MIT license](LICENSE).
