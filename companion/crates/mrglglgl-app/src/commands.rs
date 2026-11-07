//! Commands the frontend calls with `invoke`. Every type crossing to the frontend derives `TS`,
//! and the `export_bindings_*` tests ts-rs generates write those types to `ui/src/bindings/`
//! (set in the repository's `.cargo/config.toml`), so the frontend's types cannot drift from
//! these definitions without a test run noticing.

use std::collections::{BTreeMap, BTreeSet};

use mrglglgl_core::addon_status::{self, CharacterRun, FlavorAddon};
use mrglglgl_core::archive;
use mrglglgl_core::db::{Settings, StoredCharacter};
use mrglglgl_core::discovery::{self, DiscoveryOverrides, Flavor, WowRoot};
use mrglglgl_core::milestones::Award;
use mrglglgl_core::schema::Journal;
use mrglglgl_core::screenshots::{self, Screenshot};
use mrglglgl_core::usage::{Audience, DataUse};
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
    /// Days and weeks with a draft waiting for the player.
    pub to_review: u32,
}

#[tauri::command]
pub fn list_characters(state: State<'_, AppState>) -> Result<Vec<CharacterSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.characters()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|character| {
            let milestones = mrglglgl_core::usage::awards(&db, character.id, Audience::View)
                .map_err(|e| e.to_string())?
                .len() as u32;
            let to_review = mrglglgl_core::review::waiting(&db, character.id)
                .map_err(|e| e.to_string())? as u32;
            Ok(CharacterSummary {
                archived: !character.journal_path.is_file(),
                milestones,
                to_review,
                character,
            })
        })
        .collect()
}

/// The journal as the window shows it: without what the player hid from "What happened".
#[tauri::command]
pub fn character_journal(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<Option<Journal>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    archive::load_for(&db, character_id, Audience::View).map_err(|e| e.to_string())
}

/// The journal as the website and exports may carry it: without what the player kept from
/// narrators and sharing.
#[tauri::command]
pub fn character_journal_shared(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<Option<Journal>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    archive::load_for(&db, character_id, Audience::Shared).map_err(|e| e.to_string())
}

/// What the player set aside for a character: from narrators and sharing, and from the window.
#[tauri::command]
pub fn character_data_use(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<DataUse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.data_use(character_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_character_data_use(
    state: State<'_, AppState>,
    character_id: i64,
    choice: DataUse,
) -> Result<(), String> {
    let known = |set: BTreeSet<String>| -> BTreeSet<String> {
        set.into_iter()
            .filter(|c| mrglglgl_core::usage::CATEGORIES.contains(&c.as_str()))
            .collect()
    };
    let choice = DataUse {
        shared: known(choice.shared),
        view: known(choice.view),
    };
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_data_use(character_id, &choice)
        .map_err(|e| e.to_string())
}

/// Milestones as the window shows them.
#[tauri::command]
pub fn character_milestones(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<Vec<Award>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    mrglglgl_core::usage::awards(&db, character_id, Audience::View).map_err(|e| e.to_string())
}

/// Milestones as the website and exports may carry them.
#[tauri::command]
pub fn character_milestones_shared(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<Vec<Award>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    mrglglgl_core::usage::awards(&db, character_id, Audience::Shared).map_err(|e| e.to_string())
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
    /// Settings groups that could not be read and are shown at their defaults.
    pub unreadable: Vec<String>,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsView, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let (settings, unreadable) = db.settings_checked().map_err(|e| e.to_string())?;
    let roots = discovery::discover(&DiscoveryOverrides {
        wow_root: settings.wow_root_override.clone(),
    });
    Ok(SettingsView {
        settings,
        roots,
        unreadable,
    })
}

/// The addon in each WoW: Forever flavor found: installed or not, which version, and whether it
/// has been recording, from the installed TOC and what the addon noted in the archived journals.
pub fn addon_statuses(state: &AppState) -> Result<Vec<(Flavor, FlavorAddon)>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let settings = db.settings().map_err(|e| e.to_string())?;
    let characters = db.characters().map_err(|e| e.to_string())?;
    let mut notes = Vec::new();
    for character in &characters {
        let (heartbeat, errors) =
            archive::addon_notes(&db, character.id).map_err(|e| e.to_string())?;
        notes.push((character, heartbeat, errors));
    }
    let now = mrglglgl_core::watch::now_unix();
    let mut found = Vec::new();
    for root in discovery::discover(&DiscoveryOverrides {
        wow_root: settings.wow_root_override.clone(),
    }) {
        for flavor in discovery::flavors(&root) {
            if !addon_status::is_forever(&flavor) {
                continue;
            }
            let client = addon_status::product(&flavor)
                .and_then(|product| addon_status::client_version(&root.path, &product));
            let runs: Vec<CharacterRun<'_>> = notes
                .iter()
                .filter(|(character, _, _)| character.journal_path.starts_with(&flavor.path))
                .map(|(character, heartbeat, errors)| CharacterRun {
                    character: character
                        .name
                        .as_deref()
                        .unwrap_or(&character.character_folder),
                    heartbeat: heartbeat.as_ref(),
                    errors: errors.as_ref(),
                })
                .collect();
            let status = addon_status::assess(
                &flavor,
                client,
                addon_status::installed(&flavor),
                &runs,
                now,
            );
            found.push((flavor, status));
        }
    }
    Ok(found)
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
    // The writer reads the settings each time round; a pause or a new provider applies now.
    state.control.set_paused(settings.writing.paused);
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
