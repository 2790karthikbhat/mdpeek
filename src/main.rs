use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;

/// A fast, text-first Markdown viewer for every terminal.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Markdown file to show. Reads stdin when omitted or `-`.
    file: Option<PathBuf>,

    /// Print to stdout instead of opening the pager.
    #[arg(short, long)]
    plain: bool,

    /// Wrap width in columns (default: terminal width, at most 100).
    #[arg(short, long)]
    width: Option<usize>,

    /// When to use colors in --plain output.
    #[arg(long, value_enum, default_value_t = ColorMode::Auto)]
    color: ColorMode,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

/// Default cap on the text width; long lines are hard to read.
const MAX_WIDTH: usize = 100;

fn main() {
    if let Err(e) = run() {
        eprintln!("mdpeek: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let path = cli.file.filter(|p| p.as_os_str() != "-");

    let src = match &path {
        Some(p) => {
            std::fs::read_to_string(p).with_context(|| format!("cannot read {}", p.display()))?
        }
        None => {
            if std::io::stdin().is_terminal() {
                bail!(
                    "no file given. Usage: mdpeek FILE, or pipe Markdown in: cat README.md | mdpeek"
                );
            }
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .context("cannot read stdin")?;
            s
        }
    };

    let stdout_tty = std::io::stdout().is_terminal();
    if cli.plain || !stdout_tty {
        let color = match cli.color {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                stdout_tty && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
            }
        };
        let width = cli.width.unwrap_or_else(|| {
            let term = if stdout_tty {
                crossterm::terminal::size().ok().map(|(w, _)| w as usize)
            } else {
                None
            };
            term.map_or(80, |w| w.min(MAX_WIDTH))
        });
        let text = mdpeek::render_plain(&src, width, color);
        let mut out = std::io::stdout().lock();
        // A closed pipe (e.g. `| head`) is not an error worth reporting.
        if let Err(e) = out.write_all(text.as_bytes()).and_then(|_| out.flush())
            && e.kind() != std::io::ErrorKind::BrokenPipe
        {
            return Err(e.into());
        }
        return Ok(());
    }

    mdpeek::ui::run(mdpeek::ui::Input {
        path,
        src,
        max_width: cli.width.unwrap_or(MAX_WIDTH),
    })
}
