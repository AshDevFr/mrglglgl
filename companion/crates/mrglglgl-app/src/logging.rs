//! The app's log: to the terminal, and to a file in the data folder so a player on a release
//! build (no console on Windows) has something to send when asking for help. The file rotates
//! daily and a week of files is kept.
//!
//! Levels: `error` for what the player sees fail, `warn` for what went wrong and was worked
//! around, `info` for what the app did (a sync, an entry written, an install), `debug` for the
//! detail behind it (timings, sizes, decisions), `trace` for what repeats every few seconds. The
//! app's own code logs from `debug` up by default and everything else from `warn`; `RUST_LOG`
//! replaces that, as in `RUST_LOG=mrglglgl=trace`.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

/// Days of log files kept.
const KEEP_FILES: usize = 7;
const PREFIX: &str = "mrglglgl";
const SUFFIX: &str = "log";
/// What is logged when `RUST_LOG` is not set: the app's own crates (whose targets all start with
/// `mrglglgl`) in detail, libraries only when something goes wrong.
const DEFAULT_FILTER: &str = "warn,mrglglgl=debug";

pub fn log_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("logs")
}

/// Keeps the file writer flushing until the app exits.
pub struct LogGuard(#[allow(dead_code)] Option<WorkerGuard>);

/// Starts logging to the terminal and, when the folder can be made, to the log file.
pub fn init(data_dir: &Path) -> LogGuard {
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix(PREFIX)
        .filename_suffix(SUFFIX)
        .max_log_files(KEEP_FILES)
        .build(log_dir(data_dir));
    let (file, guard) = match appender {
        Ok(appender) => {
            let (writer, guard) = tracing_appender::non_blocking(appender);
            (
                Some(fmt::layer().with_ansi(false).with_writer(writer)),
                Some(guard),
            )
        }
        Err(e) => {
            eprintln!("log file unavailable: {e}");
            (None, None)
        }
    };
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER)))
        .with(fmt::layer())
        .with(file)
        .try_init();
    LogGuard(guard)
}

/// A line from the window: what failed in the interface, or what it did that the Rust side
/// cannot see (an update check, a page crash). Logged under `mrglglgl::window`.
#[tauri::command]
pub fn log_from_window(level: String, message: String) {
    // A runaway message (a huge stack, a whole response) would drown the file.
    const MOST_CHARS: usize = 4000;
    let message: String = message.chars().take(MOST_CHARS).collect();
    match level.as_str() {
        "error" => tracing::error!(target: "mrglglgl::window", "{message}"),
        "warn" => tracing::warn!(target: "mrglglgl::window", "{message}"),
        "info" => tracing::info!(target: "mrglglgl::window", "{message}"),
        "debug" => tracing::debug!(target: "mrglglgl::window", "{message}"),
        _ => tracing::trace!(target: "mrglglgl::window", "{message}"),
    }
}

/// The newest log file, or the log folder when there is none yet.
fn newest(dir: &Path) -> PathBuf {
    std::fs::read_dir(dir)
        .ok()
        .and_then(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with(PREFIX) && n.ends_with(SUFFIX))
                })
                .max()
        })
        .unwrap_or_else(|| dir.to_path_buf())
}

/// The most lines the window is sent at once: a few days of an ordinary log.
pub const MOST_LINES: usize = 2000;

/// The last `limit` lines of the log, oldest first, reading back through the daily files from the
/// newest until there are enough.
pub fn recent_lines(dir: &Path, limit: usize) -> Vec<String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with(PREFIX) && n.ends_with(SUFFIX))
                })
                .collect()
        })
        .unwrap_or_default();
    // Daily files are named by date, so the name sorts them in time.
    files.sort();
    let mut lines: Vec<String> = Vec::new();
    for file in files.iter().rev() {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let mut earlier: Vec<String> = text.lines().map(str::to_string).collect();
        earlier.append(&mut lines);
        lines = earlier;
        if lines.len() >= limit {
            break;
        }
    }
    let skip = lines.len().saturating_sub(limit);
    lines.split_off(skip)
}

/// The log's last lines, for the window's Logs tab.
#[tauri::command]
pub fn recent_log(app: AppHandle, limit: usize) -> Result<Vec<String>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(recent_lines(&log_dir(&data_dir), limit.min(MOST_LINES)))
}

/// The only addresses the app opens in the browser: Wowhead pages, from names in a day's list.
const LINK_PREFIX: &str = "https://www.wowhead.com/";

/// Opens a Wowhead page in the system's browser. Any other address is refused, so the window
/// cannot be used to open arbitrary links.
#[tauri::command]
pub fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with(LINK_PREFIX) || url.chars().any(char::is_whitespace) {
        return Err(format!("not a Wowhead address: {url}"));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// Shows the newest log file in the system's file manager.
#[tauri::command]
pub fn show_log(app: AppHandle) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = log_dir(&data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .reveal_item_in_dir(newest(&dir))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_filter_logs_the_app_in_detail_and_libraries_when_they_fail() {
        let filter = EnvFilter::try_new(DEFAULT_FILTER).unwrap();
        assert_eq!(
            filter.max_level_hint(),
            Some(tracing::level_filters::LevelFilter::DEBUG)
        );
    }

    #[test]
    fn recent_lines_read_back_across_daily_files_oldest_first() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mrglglgl.2026-11-05.log"), "a\nb\nc\n").unwrap();
        std::fs::write(dir.path().join("mrglglgl.2026-11-06.log"), "d\ne\n").unwrap();
        std::fs::write(dir.path().join("other.txt"), "ignored\n").unwrap();
        assert_eq!(recent_lines(dir.path(), 3), ["c", "d", "e"]);
        assert_eq!(recent_lines(dir.path(), 10), ["a", "b", "c", "d", "e"]);
        assert!(recent_lines(&dir.path().join("missing"), 5).is_empty());
    }
}
