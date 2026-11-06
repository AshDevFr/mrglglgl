//! Noticing when the game writes a journal (or a screenshot) so the app refreshes on its own.
//!
//! The game writes SavedVariables only on logout or `/reload`, and writes the file in several
//! steps; changes are collected until the folder has been quiet for a moment, then reported once
//! per file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use notify::{RecursiveMode, Watcher as _};

use crate::discovery::JOURNAL_FILE;

/// How long a folder must stay quiet before its changes are reported.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(750);

#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("could not watch {path}: {source}")]
    Watch {
        path: PathBuf,
        source: notify::Error,
    },
}

/// Watches folders until dropped.
pub struct Watcher {
    _inner: notify::RecommendedWatcher,
    _thread: JoinHandle<()>,
}

/// True for a character journal file.
pub fn is_journal_file(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == JOURNAL_FILE)
}

/// A file's modification time as Unix seconds: the "last synced" time shown for a character.
pub fn modified_unix(path: &Path) -> Option<i64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let seconds = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    i64::try_from(seconds).ok()
}

/// Watches `folders` recursively. Files for which `wanted` is true are collected, and
/// `on_change` receives each batch once `debounce` passes with no further change.
pub fn watch(
    folders: &[PathBuf],
    debounce: Duration,
    wanted: impl Fn(&Path) -> bool + Send + 'static,
    on_change: impl Fn(BTreeSet<PathBuf>) + Send + 'static,
) -> Result<Watcher, WatchError> {
    let (sender, receiver) = mpsc::channel::<PathBuf>();
    let mut inner = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        if let Ok(event) = result {
            for path in event.paths {
                if wanted(&path) {
                    // The receiving thread is gone only when the watcher is being dropped.
                    let _ = sender.send(path);
                }
            }
        }
    })
    .map_err(|source| WatchError::Watch {
        path: PathBuf::new(),
        source,
    })?;
    for folder in folders {
        inner
            .watch(folder, RecursiveMode::Recursive)
            .map_err(|source| WatchError::Watch {
                path: folder.clone(),
                source,
            })?;
    }
    let thread = std::thread::spawn(move || {
        // Ends when the watcher, and with it the sender, is dropped.
        while let Ok(first) = receiver.recv() {
            let mut batch = BTreeSet::from([first]);
            loop {
                match receiver.recv_timeout(debounce) {
                    Ok(path) => {
                        batch.insert(path);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            on_change(batch);
        }
    });
    Ok(Watcher {
        _inner: inner,
        _thread: thread,
    })
}

/// Seconds since the Unix epoch now.
pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
