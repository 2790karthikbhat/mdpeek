//! Live reload: watch the file's parent folder so editors that save by
//! writing a temp file and renaming it over the original still count.

use anyhow::{Context, Result};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::Sender;

/// Sends `()` on `tx` whenever `path` may have changed. Keep the returned
/// watcher alive for as long as events are wanted.
pub fn watch(path: &Path, tx: Sender<()>) -> Result<RecommendedWatcher> {
    let path = std::path::absolute(path).context("cannot resolve path")?;
    let dir = path
        .parent()
        .context("file has no parent folder")?
        .to_path_buf();
    let name = path.file_name().context("not a file")?.to_os_string();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(ev) = res else { return };
        if matches!(ev.kind, EventKind::Access(_)) {
            return;
        }
        if ev
            .paths
            .iter()
            .any(|p| p.file_name() == Some(name.as_os_str()))
        {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}
