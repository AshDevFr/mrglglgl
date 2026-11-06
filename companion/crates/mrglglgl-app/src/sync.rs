//! Keeping the archive in step with the game: a full pass over every install at start-up and on
//! request, then a watcher that imports journals as the game writes them.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mrglglgl_core::archive::{self, ImportOutcome};
use mrglglgl_core::db::Database;
use mrglglgl_core::discovery::{self, CharacterSource, DiscoveryOverrides, WowRoot};
use mrglglgl_core::milestones;
use mrglglgl_core::watch::{self, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

/// Event sent to the frontend when a character's archive changed; the payload is its id.
pub const JOURNAL_UPDATED: &str = "journal-updated";

pub struct AppState {
    pub db: Mutex<Database>,
    pub data_dir: PathBuf,
    watcher: Mutex<Option<Watcher>>,
}

impl AppState {
    pub fn new(db: Database, data_dir: PathBuf) -> Self {
        Self {
            db: Mutex::new(db),
            data_dir,
            watcher: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncReport {
    /// The WoW installs found; empty means none, and the settings screen offers to pick one.
    pub roots: Vec<WowRoot>,
    pub characters: u32,
    pub imported: u32,
    pub unreadable: u32,
}

fn sources(roots: &[WowRoot]) -> Vec<CharacterSource> {
    roots
        .iter()
        .flat_map(discovery::flavors)
        .flat_map(|flavor| discovery::characters(&flavor))
        .collect()
}

/// Imports one character's journal and refreshes its milestones. Returns its id when imported.
fn import_one(db: &Database, data_dir: &Path, source: &CharacterSource, keep: u32) -> Option<i64> {
    match archive::import(db, data_dir, source, keep) {
        Ok(ImportOutcome::Imported { character_id }) => {
            if let Ok(Some(journal)) = archive::load(db, character_id)
                && let Err(e) = milestones::award(db, character_id, &journal)
            {
                tracing::warn!("milestones for {}: {e}", source.character_folder);
            }
            Some(character_id)
        }
        Ok(ImportOutcome::Unreadable { error, .. }) => {
            tracing::warn!("{}: {error}", source.journal_path.display());
            None
        }
        Err(e) => {
            tracing::warn!("{}: {e}", source.journal_path.display());
            None
        }
    }
}

/// Finds every install and imports every character's journal.
pub fn sync_all(state: &AppState) -> Result<SyncReport, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let settings = db.settings().map_err(|e| e.to_string())?;
    let roots = discovery::discover(&DiscoveryOverrides {
        wow_root: settings.wow_root_override.clone(),
    });
    let mut report = SyncReport {
        roots: roots.clone(),
        ..SyncReport::default()
    };
    for source in sources(&roots) {
        report.characters += 1;
        match import_one(&db, &state.data_dir, &source, settings.backup_count) {
            Some(_) => report.imported += 1,
            None => report.unreadable += 1,
        }
    }
    Ok(report)
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Watches every install's account folders and imports journals as the game writes them,
/// telling the frontend which character changed.
pub fn start_watching(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let roots = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let settings = db.settings().map_err(|e| e.to_string())?;
        discovery::discover(&DiscoveryOverrides {
            wow_root: settings.wow_root_override,
        })
    };
    let folders: Vec<PathBuf> = roots
        .iter()
        .flat_map(discovery::flavors)
        .map(|flavor| flavor.path.join("WTF").join("Account"))
        .filter(|path| path.is_dir())
        .collect();

    let handle = app.clone();
    let watcher = watch::watch(
        &folders,
        watch::DEFAULT_DEBOUNCE,
        watch::is_journal_file,
        move |changed| {
            let state = handle.state::<AppState>();
            let Ok(db) = state.db.lock() else {
                return;
            };
            let keep = db.settings().map(|s| s.backup_count).unwrap_or(30);
            for source in sources(&roots) {
                if changed
                    .iter()
                    .any(|path| same_file(path, &source.journal_path))
                    && let Some(id) = import_one(&db, &state.data_dir, &source, keep)
                {
                    let _ = handle.emit(JOURNAL_UPDATED, id);
                }
            }
        },
    )
    .map_err(|e| e.to_string())?;
    *state.watcher.lock().map_err(|e| e.to_string())? = Some(watcher);
    Ok(())
}
