//! Builds made to try changes out (`make dev`, `make build`) keep their own data: their own data
//! folder (database, logs, snapshots, models), endpoint key and window place, so they never touch
//! the installed app's. They read the same journals from the game folder, which the app only
//! reads. Releases, built by the release workflow, use the real ones.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// Whether this build keeps its data apart from the installed app's: every debug build, and a
/// release build made with `MRGLGLGL_SEPARATE_DATA` set, as `make build` does.
pub const SEPARATE: bool =
    cfg!(debug_assertions) || option_env!("MRGLGLGL_SEPARATE_DATA").is_some();

/// Added to the data folder's name and the keychain entry of a build that keeps its data apart.
const SUFFIX: &str = "dev";

/// The app's data folder: the system's folder for its identifier, or one beside it for a build
/// that keeps its data apart.
pub fn data_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    Ok(if SEPARATE { apart(dir) } else { dir })
}

fn apart(dir: PathBuf) -> PathBuf {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    dir.with_file_name(format!("{name}.{SUFFIX}"))
}

/// The keychain service the endpoint key is kept under.
pub fn keychain_service() -> &'static str {
    if SEPARATE {
        "Mrglglgl (dev)"
    } else {
        "Mrglglgl"
    }
}

/// The file the window's size and place are kept in.
pub fn window_state_file() -> &'static str {
    if SEPARATE {
        ".window-state.dev.json"
    } else {
        ".window-state.json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_separate_build_keeps_its_data_beside_the_real_folder() {
        assert_eq!(
            apart(PathBuf::from(
                "/Users/w/Library/Application Support/io.github.ashdevfr.mrglglgl"
            )),
            PathBuf::from("/Users/w/Library/Application Support/io.github.ashdevfr.mrglglgl.dev")
        );
    }

    #[test]
    fn tests_run_as_a_separate_build() {
        // Tests are debug builds: they must never name the installed app's keychain entry.
        const { assert!(SEPARATE) };
        assert_eq!(keychain_service(), "Mrglglgl (dev)");
    }
}
