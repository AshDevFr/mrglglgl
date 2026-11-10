//! Keeping the archive in step with the game: a full pass over every install at start-up and on
//! request, then a watcher that imports journals as the game writes them.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use mrglglgl_core::archive::{self, ImportOutcome};
use mrglglgl_core::db::{BackupRetention, Database};
use mrglglgl_core::discovery::{self, CharacterSource, DiscoveryOverrides, WowRoot};
use mrglglgl_core::milestones;
use mrglglgl_core::queue::Control;
use mrglglgl_core::screenshots;
use mrglglgl_core::watch::{self, Watcher};
use mrglglgl_llm::Provider;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

/// Event sent to the frontend when a character's archive changed; the payload is its id.
pub const JOURNAL_UPDATED: &str = "journal-updated";

/// Event sent to the frontend when the game wrote a screenshot.
pub const SCREENSHOTS_UPDATED: &str = "screenshots-updated";

pub struct AppState {
    pub db: Mutex<Database>,
    pub data_dir: PathBuf,
    watchers: Mutex<Vec<Watcher>>,
    /// Steers the background writer.
    pub control: Arc<Control>,
    /// The provider in use, kept with a key naming it so it is rebuilt only when it changes.
    pub provider: Mutex<Option<(String, Arc<dyn Provider>)>>,
    /// The model download in progress, how far it has come, and how to stop it.
    pub download: Mutex<
        Option<(
            crate::writing_commands::DownloadProgress,
            mrglglgl_llm::CancellationToken,
        )>,
    >,
}

impl AppState {
    pub fn new(db: Database, data_dir: PathBuf) -> Self {
        let paused = db.settings().map(|s| s.writing.paused).unwrap_or(false);
        let control = Control::new(paused).with_narrators(crate::writing::narrators_dir(&data_dir));
        Self {
            db: Mutex::new(db),
            data_dir,
            watchers: Mutex::new(Vec::new()),
            control: Arc::new(control),
            provider: Mutex::new(None),
            download: Mutex::new(None),
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
fn import_one(
    db: &Database,
    data_dir: &Path,
    source: &CharacterSource,
    retention: BackupRetention,
) -> Option<i64> {
    // A character taken out of the app is not read: the player chose not to keep its days.
    if db.is_disabled(source).unwrap_or(false) {
        tracing::debug!("{}: out of the app, not read", source.character_folder);
        return None;
    }
    let started = std::time::Instant::now();
    match archive::import(db, data_dir, source, retention) {
        Ok(ImportOutcome::Imported { character_id }) => {
            tracing::debug!(
                "imported {} (character {character_id}) from {} in {:?}",
                source.character_folder,
                source.journal_path.display(),
                started.elapsed()
            );
            if let Ok(Some(journal)) = archive::load(db, character_id)
                && let Err(e) = milestones::award(db, character_id, &journal)
            {
                tracing::warn!("milestones for {}: {e}", source.character_folder);
            }
            Some(character_id)
        }
        Ok(ImportOutcome::Unreadable { error, .. }) => {
            tracing::warn!(
                "{}: unreadable journal: {error}",
                source.journal_path.display()
            );
            None
        }
        Err(e) => {
            tracing::warn!("{}: import failed: {e}", source.journal_path.display());
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
    let started = std::time::Instant::now();
    match &settings.wow_root_override {
        Some(root) => tracing::debug!("sync: looking in the chosen game folder {}", root.display()),
        None => tracing::debug!("sync: looking for game folders"),
    }
    if roots.is_empty() {
        tracing::warn!("sync: no World of Warcraft: Forever folder found");
    }
    for root in &roots {
        tracing::debug!("sync: game folder {root:?}");
    }
    let mut report = SyncReport {
        roots: roots.clone(),
        ..SyncReport::default()
    };
    for source in sources(&roots) {
        // Out of the app: neither read nor counted, so it is not reported as unreadable.
        if db.is_disabled(&source).unwrap_or(false) {
            continue;
        }
        report.characters += 1;
        match import_one(&db, &state.data_dir, &source, settings.backup_retention) {
            Some(id) => {
                report.imported += 1;
                crate::writing::enqueue(state, &db, id);
            }
            None => report.unreadable += 1,
        }
    }
    tracing::info!(
        "sync: {} characters, {} imported, {} unreadable, in {:?}",
        report.characters,
        report.imported,
        report.unreadable,
        started.elapsed()
    );
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
    let flavors: Vec<_> = roots.iter().flat_map(discovery::flavors).collect();
    let folders: Vec<PathBuf> = flavors
        .iter()
        .map(|flavor| flavor.path.join("WTF").join("Account"))
        .filter(|path| path.is_dir())
        .collect();
    // Only folders that exist can be watched; one created later is picked up the next time the
    // app starts or the settings are saved.
    let screenshot_folders: Vec<PathBuf> = flavors
        .iter()
        .map(|flavor| flavor.path.join(screenshots::SCREENSHOTS_FOLDER))
        .filter(|path| path.is_dir())
        .collect();

    for folder in folders.iter().chain(&screenshot_folders) {
        tracing::debug!("watching {}", folder.display());
    }
    if folders.is_empty() {
        tracing::warn!("no account folder to watch: journals are read only when syncing by hand");
    }
    let mut watchers = Vec::new();
    if !screenshot_folders.is_empty() {
        let handle = app.clone();
        watchers.push(
            watch::watch(
                &screenshot_folders,
                watch::DEFAULT_DEBOUNCE,
                screenshots::is_screenshot_file,
                move |changed| {
                    tracing::debug!("new screenshots: {changed:?}");
                    let _ = handle.emit(SCREENSHOTS_UPDATED, ());
                },
            )
            .map_err(|e| e.to_string())?,
        );
    }

    let handle = app.clone();
    let watcher = watch::watch(
        &folders,
        watch::DEFAULT_DEBOUNCE,
        watch::is_journal_file,
        move |changed| {
            tracing::debug!("journals changed: {changed:?}");
            let state = handle.state::<AppState>();
            let Ok(db) = state.db.lock() else {
                return;
            };
            let retention = db
                .settings()
                .map(|s| s.backup_retention)
                .unwrap_or_default();
            for source in sources(&roots) {
                if changed
                    .iter()
                    .any(|path| same_file(path, &source.journal_path))
                    && let Some(id) = import_one(&db, &state.data_dir, &source, retention)
                {
                    tracing::info!("{} updated by the game", source.character_folder);
                    crate::writing::enqueue(&state, &db, id);
                    let _ = handle.emit(JOURNAL_UPDATED, id);
                }
            }
        },
    )
    .map_err(|e| e.to_string())?;
    watchers.push(watcher);
    // Replacing the list drops, and so stops, the previous watchers.
    *state.watchers.lock().map_err(|e| e.to_string())? = watchers;
    Ok(())
}
