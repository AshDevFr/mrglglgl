//! Commands the frontend calls with `invoke`. Every type crossing to the frontend derives `TS`,
//! and the `export_bindings_*` tests ts-rs generates write those types to `ui/src/bindings/`
//! (set in `.cargo/config.toml`), so the frontend's types cannot drift from these definitions
//! without a test run noticing.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    pub version: String,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_package_version() {
        assert_eq!(app_info().version, env!("CARGO_PKG_VERSION"));
    }
}
