//! Background writing: picks the provider from the settings, runs the queue while the app is
//! open, and tells the window what is being written.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use mrglglgl_core::db::{Database, ProviderChoice, Settings};
use mrglglgl_core::game;
use mrglglgl_core::queue::{self, Control, QueueEvent};
use mrglglgl_llm::Provider;
use mrglglgl_llm::catalogue::{Catalogue, ModelEntry};
use mrglglgl_llm::endpoint::{EndpointConfig, EndpointProvider};
use mrglglgl_llm::local::{LocalOptions, LocalProvider};
use tauri::{AppHandle, Emitter, Manager};

use crate::sync::AppState;

/// Event sent to the window for everything the writer does; the payload is a `QueueEvent`.
pub const WRITING: &str = "writing";
/// How often an idle writer looks again without being woken.
const IDLE_CHECK: Duration = Duration::from_secs(60);
/// How often the writer checks whether the game has closed.
const GAME_CHECK: Duration = Duration::from_secs(30);

const KEYCHAIN_SERVICE: &str = "Mrglglgl";
const KEYCHAIN_ENTRY: &str = "endpoint-api-key";

/// The folder downloaded models live in.
pub fn models_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("models")
}

/// The model list: the shipped one, or a newer one dropped into the data folder.
pub fn catalogue(data_dir: &Path) -> Catalogue {
    Catalogue::load(Some(&data_dir.join("catalogue.json")))
}

/// The language entries are written in: the chosen prose language, else the UI language.
pub fn prose_language(settings: &Settings) -> String {
    settings
        .writing
        .prose_language
        .clone()
        .or_else(|| settings.ui_language.clone())
        .unwrap_or_else(|| "en".to_string())
}

/// The catalogue model the settings point at, or the recommended one.
pub fn chosen_model(catalogue: &Catalogue, settings: &Settings) -> Option<ModelEntry> {
    settings
        .writing
        .model_id
        .as_deref()
        .and_then(|id| catalogue.get(id))
        .or_else(|| catalogue.recommended(&prose_language(settings)))
        .cloned()
}

pub fn api_key() -> Option<String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ENTRY)
        .ok()?
        .get_password()
        .ok()
}

pub fn endpoint_config(settings: &Settings) -> Option<EndpointConfig> {
    Some(EndpointConfig {
        base_url: settings
            .writing
            .endpoint_url
            .clone()
            .filter(|u| !u.is_empty())?,
        model: settings
            .writing
            .endpoint_model
            .clone()
            .filter(|m| !m.is_empty())?,
        api_key: api_key(),
    })
}

/// The provider the settings ask for, if it is ready: a downloaded model, or a configured
/// endpoint. A local provider is kept while its model stays the same, so the loaded model is not
/// thrown away between entries.
fn provider(state: &AppState, settings: &Settings) -> Option<Arc<dyn Provider>> {
    let mut cached = state.provider.lock().unwrap();
    let wanted: (String, Option<Arc<dyn Provider>>) = match settings.writing.provider {
        ProviderChoice::Endpoint => {
            let config = endpoint_config(settings)?;
            let key = format!("endpoint:{}:{}", config.base_url, config.model);
            if cached.as_ref().is_some_and(|(k, _)| *k == key) {
                return cached.as_ref().map(|(_, p)| p.clone());
            }
            (key, Some(Arc::new(EndpointProvider::new(config))))
        }
        ProviderChoice::Local => {
            let model = chosen_model(&catalogue(&state.data_dir), settings)?;
            let dir = models_dir(&state.data_dir);
            if !model.installed(&dir) {
                return None;
            }
            let key = format!("local:{}", model.id);
            if cached.as_ref().is_some_and(|(k, _)| *k == key) {
                return cached.as_ref().map(|(_, p)| p.clone());
            }
            let options = LocalOptions::new(model.path(&dir), &model.id, model.reasoning);
            (key, Some(Arc::new(LocalProvider::new(options))))
        }
    };
    let (key, provider) = wanted;
    *cached = provider.clone().map(|p| (key, p));
    provider
}

/// Whether the game is running, from the process list.
pub fn game_running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
    let system = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
    );
    let mut system = system;
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
        .processes()
        .values()
        .any(|p| game::is_game_process(&p.name().to_string_lossy()))
}

/// The local date, `YYYY-MM-DD`: days before it are finished.
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Queues every finished, unwritten day and week of a character, and wakes the writer.
pub fn enqueue(state: &AppState, db: &Database, character_id: i64) {
    match mrglglgl_core::archive::load(db, character_id) {
        Ok(Some(journal)) => {
            if let Err(e) = queue::enqueue_finished(db, character_id, &journal, &today()) {
                tracing::warn!("queueing character {character_id}: {e}");
            }
            state.control.wake();
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("queueing character {character_id}: {e}"),
    }
}

/// Runs the queue for as long as the app is open.
pub async fn run(app: AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(db) = state.db.lock()
        && let Err(e) = queue::recover(&db)
    {
        tracing::warn!("recovering the writing queue: {e}");
    }
    let control: Arc<Control> = state.control.clone();
    let emit = {
        let app = app.clone();
        move |event: QueueEvent| {
            let _ = app.emit(WRITING, event);
        }
    };
    loop {
        // Read in its own statement so the lock is released before any await.
        let settings = state.db.lock().ok().and_then(|db| db.settings().ok());
        let settings = match settings {
            Some(s) => s,
            None => {
                queue::idle(&control, IDLE_CHECK).await;
                continue;
            }
        };
        control.set_paused_quietly(settings.writing.paused);
        if control.paused() {
            queue::idle(&control, IDLE_CHECK).await;
            continue;
        }
        if !settings.writing.write_while_playing && game_running() {
            queue::idle(&control, GAME_CHECK).await;
            continue;
        }
        let Some(provider) = provider(&state, &settings) else {
            queue::idle(&control, IDLE_CHECK).await;
            continue;
        };
        let language = prose_language(&settings);
        match queue::run_next(&state.db, provider.as_ref(), &control, &language, &emit).await {
            // Look at the settings and the game again before the next entry.
            Ok(Some(_)) => continue,
            Ok(None) => {
                let due = state
                    .db
                    .lock()
                    .ok()
                    .and_then(|db| queue::next_due_in(&db, chrono::Utc::now().timestamp()).ok())
                    .flatten();
                let wait = due.map_or(IDLE_CHECK, |s| Duration::from_secs(s.max(1) as u64));
                queue::idle(&control, wait.min(IDLE_CHECK)).await;
            }
            Err(e) => {
                tracing::warn!("writing: {e}");
                queue::idle(&control, IDLE_CHECK).await;
            }
        }
    }
}
