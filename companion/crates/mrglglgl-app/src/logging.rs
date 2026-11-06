//! The app's log: to the terminal, and to a file in the data folder so a player on a release
//! build (no console on Windows) has something to send when asking for help. The file rotates
//! daily and a week of files is kept.

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
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("mrglglgl=info")))
        .with(fmt::layer())
        .with(file)
        .try_init();
    LogGuard(guard)
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
