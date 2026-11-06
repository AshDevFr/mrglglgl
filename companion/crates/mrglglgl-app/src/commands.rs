//! Commands the frontend calls with `invoke`. Every type crossing to the frontend derives `TS`,
//! and the `export_bindings_*` tests ts-rs generates write those types to `ui/src/bindings/`
//! (set in the repository's `.cargo/config.toml`), so the frontend's types cannot drift from
//! these definitions without a test run noticing.

use std::collections::BTreeMap;

use mrglglgl_core::archive;
use mrglglgl_core::db::{Settings, StoredCharacter};
use mrglglgl_core::discovery::{self, DiscoveryOverrides, WowRoot};
use mrglglgl_core::milestones::{self, Award};
use mrglglgl_core::schema::Journal;
use mrglglgl_core::screenshots::{self, Screenshot};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;

use crate::sync::{self, AppState, SyncReport};

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

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CharacterSummary {
    pub character: StoredCharacter,
    /// The game's journal file is gone; the character lives on in the archive.
    pub archived: bool,
    pub milestones: u32,
}

#[tauri::command]
pub fn list_characters(state: State<'_, AppState>) -> Result<Vec<CharacterSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.characters()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|character| {
            let milestones = milestones::awards(&db, character.id)
                .map_err(|e| e.to_string())?
                .len() as u32;
            Ok(CharacterSummary {
                archived: !character.journal_path.is_file(),
                milestones,
                character,
            })
        })
        .collect()
}

#[tauri::command]
pub fn character_journal(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<Option<Journal>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    archive::load(&db, character_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn character_milestones(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<Vec<Award>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    milestones::awards(&db, character_id).map_err(|e| e.to_string())
}

/// A character's screenshots, matched to the moments the addon took them, by day. Lets the
/// window show images from that character's Screenshots folder, and nothing else, read-only.
#[tauri::command]
pub fn character_screenshots(
    app: AppHandle,
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<BTreeMap<String, Vec<Screenshot>>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let (Some(character), Some(journal)) = (
        db.character(character_id).map_err(|e| e.to_string())?,
        archive::load(&db, character_id).map_err(|e| e.to_string())?,
    ) else {
        return Ok(BTreeMap::new());
    };
    let Some(folder) = screenshots::folder_for_journal(&character.journal_path) else {
        return Ok(BTreeMap::new());
    };
    if !folder.is_dir() {
        return Ok(BTreeMap::new());
    }
    app.asset_protocol_scope()
        .allow_directory(&folder, false)
        .map_err(|e| e.to_string())?;
    Ok(screenshots::match_to_journal(
        &journal,
        &screenshots::list(&folder),
    ))
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsView {
    pub settings: Settings,
    /// The installs found with the current settings.
    pub roots: Vec<WowRoot>,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsView, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let settings = db.settings().map_err(|e| e.to_string())?;
    let roots = discovery::discover(&DiscoveryOverrides {
        wow_root: settings.wow_root_override.clone(),
    });
    Ok(SettingsView { settings, roots })
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<SyncReport, String> {
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.save_settings(&settings).map_err(|e| e.to_string())?;
    }
    let report = sync::sync_all(&state)?;
    sync::start_watching(&app)?;
    Ok(report)
}

#[tauri::command]
pub fn sync_now(state: State<'_, AppState>) -> Result<SyncReport, String> {
    sync::sync_all(&state)
}

#[tauri::command]
pub fn delete_character(state: State<'_, AppState>, character_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    archive::delete_character(&db, &state.data_dir, character_id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_package_version() {
        assert_eq!(app_info().version, env!("CARGO_PKG_VERSION"));
    }
}
