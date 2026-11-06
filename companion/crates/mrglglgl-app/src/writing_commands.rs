//! Commands for writing: entries and their versions, the queue, the models and the endpoint.

use std::path::Path;

use mrglglgl_core::archive;
use mrglglgl_core::db::ProviderChoice;
use mrglglgl_core::discovery::{self, DiscoveryOverrides};
use mrglglgl_core::export;
use mrglglgl_core::milestones;
use mrglglgl_core::narrators::{self, NarratorInfo};
use mrglglgl_core::queue::{self, Job};
use mrglglgl_core::stories::{self, Story, StoryKind};
use mrglglgl_llm::CancellationToken;
use mrglglgl_llm::catalogue::{self, Advice, Shortfall};
use mrglglgl_llm::endpoint::{self, EndpointConfig, EndpointProvider};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

use crate::sync::AppState;
use crate::writing;

/// Event sent while a model downloads; the payload is a [`DownloadProgress`].
pub const MODEL_DOWNLOAD: &str = "model-download";

#[tauri::command]
pub fn character_stories(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<Vec<Story>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    stories::all(&db, character_id).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WritingStatus {
    pub paused: bool,
    /// The entry being written now, for any character.
    pub current: Option<Job>,
    /// This character's waiting and failed jobs.
    pub jobs: Vec<Job>,
    /// Why nothing is being written, if something stands in the way: `"paused"`, `"noModel"`,
    /// `"noEndpoint"`, `"gameRunning"` or `"onBattery"`.
    pub blocked: Option<String>,
}

#[tauri::command]
pub fn writing_status(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<WritingStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let settings = db.settings().map_err(|e| e.to_string())?;
    let jobs = queue::jobs(&db, character_id).map_err(|e| e.to_string())?;
    drop(db);
    let ready = match settings.writing.provider {
        ProviderChoice::Local => {
            writing::chosen_model(&writing::catalogue(&state.data_dir), &settings)
                .is_some_and(|m| m.installed(&writing::models_dir(&state.data_dir)))
        }
        ProviderChoice::Endpoint => writing::endpoint_config(&settings).is_some(),
    };
    let blocked = if settings.writing.paused {
        Some("paused")
    } else if !ready {
        Some(match settings.writing.provider {
            ProviderChoice::Local => "noModel",
            ProviderChoice::Endpoint => "noEndpoint",
        })
    } else if !settings.writing.write_while_playing && !jobs.is_empty() && writing::game_running() {
        Some("gameRunning")
    } else if !settings.writing.write_on_battery && !jobs.is_empty() && crate::power::on_battery() {
        Some("onBattery")
    } else {
        None
    };
    Ok(WritingStatus {
        paused: settings.writing.paused,
        current: state.control.current(),
        jobs,
        blocked: blocked.map(str::to_string),
    })
}

/// Writes a day or week (again), ahead of the backlog, optionally with another narrator.
#[tauri::command]
pub fn write_now(
    state: State<'_, AppState>,
    character_id: i64,
    kind: StoryKind,
    start_day: String,
    narrator: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    queue::request(&db, character_id, kind, &start_day, narrator.as_deref())
        .map_err(|e| e.to_string())?;
    state.control.wake();
    Ok(())
}

/// Stops the entry being written; returns whether it was this one.
#[tauri::command]
pub fn cancel_writing(
    state: State<'_, AppState>,
    character_id: i64,
    kind: StoryKind,
    start_day: String,
) -> bool {
    state
        .control
        .cancel(Some((character_id, kind, start_day.as_str())))
}

#[tauri::command]
pub fn retry_writing(
    state: State<'_, AppState>,
    character_id: i64,
    kind: StoryKind,
    start_day: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    queue::retry(&db, character_id, kind, &start_day).map_err(|e| e.to_string())?;
    state.control.wake();
    Ok(())
}

#[tauri::command]
pub fn set_writing_paused(state: State<'_, AppState>, paused: bool) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut settings = db.settings().map_err(|e| e.to_string())?;
    settings.writing.paused = paused;
    db.save_settings(&settings).map_err(|e| e.to_string())?;
    state.control.set_paused(paused);
    Ok(())
}

#[tauri::command]
pub fn list_narrators() -> Vec<NarratorInfo> {
    narrators::list()
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelView {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub licence: String,
    pub languages: Vec<String>,
    pub min_ram_gb: u32,
    /// The recommendation for the current prose language.
    pub recommended: bool,
    /// The note on its prose in the current prose language.
    pub prose_note: Option<String>,
    pub installed: bool,
    /// Bytes of an interrupted download that the next one resumes from.
    pub partial_bytes: u64,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export)]
pub enum FirstRunAdvice {
    Recommend {
        id: String,
    },
    #[serde(rename_all = "camelCase")]
    NotEnoughMemory {
        needed_gb: u32,
        has_gb: f64,
    },
    #[serde(rename_all = "camelCase")]
    NotEnoughDisk {
        needed_gb: f64,
        free_gb: f64,
    },
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelsView {
    pub models: Vec<ModelView>,
    /// The model writing uses: the chosen one, or the recommendation.
    pub chosen_id: Option<String>,
    pub advice: FirstRunAdvice,
}

/// Total memory, and free space on the disk holding `dir`.
fn machine(dir: &Path) -> (u64, u64) {
    use sysinfo::{Disks, MemoryRefreshKind, RefreshKind, System};
    let system = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );
    let disks = Disks::new_with_refreshed_list();
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let free = disks
        .list()
        .iter()
        .filter(|d| dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map_or(0, |d| d.available_space());
    (system.total_memory(), free)
}

#[tauri::command]
pub fn models(state: State<'_, AppState>) -> Result<ModelsView, String> {
    let settings = state
        .db
        .lock()
        .map_err(|e| e.to_string())?
        .settings()
        .map_err(|e| e.to_string())?;
    let language = writing::prose_language(&settings);
    let catalogue = writing::catalogue(&state.data_dir);
    let dir = writing::models_dir(&state.data_dir);
    let recommended = catalogue.recommended(&language).map(|m| m.id.clone());
    let (ram, free) = machine(&state.data_dir);
    let advice = match catalogue.advise(ram, free, &language) {
        Advice::Recommend { id } => FirstRunAdvice::Recommend { id },
        Advice::BelowFloor {
            reason: Shortfall::Memory { needed_gb, has_gb },
        } => FirstRunAdvice::NotEnoughMemory { needed_gb, has_gb },
        Advice::BelowFloor {
            reason: Shortfall::Disk { needed_gb, free_gb },
        } => FirstRunAdvice::NotEnoughDisk { needed_gb, free_gb },
    };
    Ok(ModelsView {
        chosen_id: writing::chosen_model(&catalogue, &settings).map(|m| m.id),
        models: catalogue
            .models
            .iter()
            .map(|m| ModelView {
                id: m.id.clone(),
                name: m.name.clone(),
                size_bytes: m.size_bytes,
                licence: m.licence.clone(),
                languages: m.languages.clone(),
                min_ram_gb: m.min_ram_gb,
                recommended: recommended.as_deref() == Some(m.id.as_str()),
                prose_note: m
                    .prose_note
                    .get(&language)
                    .or_else(|| m.prose_note.get("en"))
                    .cloned(),
                installed: m.installed(&dir),
                partial_bytes: m.partial_bytes(&dir),
            })
            .collect(),
        advice,
    })
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: u64,
}

/// Downloads a model into the app's cache, reporting progress as events. Only after the player
/// confirmed its size in the window.
#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let catalogue = writing::catalogue(&state.data_dir);
    let entry = catalogue
        .get(&model_id)
        .cloned()
        .ok_or_else(|| format!("no model called {model_id}"))?;
    let cancel = CancellationToken::new();
    *state.download.lock().map_err(|e| e.to_string())? = Some(cancel.clone());
    let (_, free) = machine(&state.data_dir);
    let mut last_percent = u64::MAX;
    let result = catalogue::download(
        &entry,
        &writing::models_dir(&state.data_dir),
        catalogue::HUGGING_FACE,
        Some(free),
        &cancel,
        |p| {
            // A few hundred events, not one per network read.
            let percent = p.downloaded * 1000 / p.total.max(1);
            if percent != last_percent {
                last_percent = percent;
                let _ = app.emit(
                    MODEL_DOWNLOAD,
                    DownloadProgress {
                        model_id: model_id.clone(),
                        downloaded: p.downloaded,
                        total: p.total,
                    },
                );
            }
        },
    )
    .await;
    *state.download.lock().map_err(|e| e.to_string())? = None;
    result.map_err(|e| e.to_string())?;
    state.control.wake();
    Ok(())
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    if let Ok(Some(token)) = state.download.lock().map(|d| d.clone()) {
        token.cancel();
    }
}

#[tauri::command]
pub fn delete_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let catalogue = writing::catalogue(&state.data_dir);
    let entry = catalogue
        .get(&model_id)
        .ok_or_else(|| format!("no model called {model_id}"))?;
    // A loaded model is dropped first so its file is not held open.
    *state.provider.lock().map_err(|e| e.to_string())? = None;
    catalogue::delete(entry, &writing::models_dir(&state.data_dir)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_api_key(key: Option<String>) -> Result<(), String> {
    writing::set_api_key(key.as_deref())
}

#[tauri::command]
pub fn has_api_key() -> bool {
    writing::api_key().is_some()
}

/// Checks an endpoint answers, accepts the stored key and has the model. The error is a plain
/// sentence.
#[tauri::command]
pub async fn test_endpoint(url: String, model: String) -> Result<(), String> {
    EndpointProvider::new(EndpointConfig {
        base_url: url,
        model,
        api_key: writing::api_key(),
    })
    .test_connection()
    .await
    .map_err(|e| e.to_string())
}

/// One entry as plain text for the clipboard: the day, the entry, and the footer unless the
/// player turned it off.
#[tauri::command]
pub fn copy_entry_text(
    state: State<'_, AppState>,
    character_id: i64,
    story_id: i64,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let footer = db
        .settings()
        .map_err(|e| e.to_string())?
        .writing
        .copy_footer;
    let story = stories::all(&db, character_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|s| s.id == story_id)
        .ok_or("that entry no longer exists")?;
    Ok(export::copy_entry(
        &story.start_day,
        &story.language,
        &story.text,
        footer,
    ))
}

/// Where exports go unless the player picks another folder.
#[tauri::command]
pub fn default_export_folder() -> Option<String> {
    dirs::document_dir()
        .or_else(dirs::home_dir)
        .map(|p| p.to_string_lossy().into_owned())
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportedFiles {
    pub markdown: String,
    pub html: String,
}

/// Writes a character's journal as Markdown and as one HTML page into `folder`, refusing any
/// folder inside a World of Warcraft install.
#[tauri::command]
pub fn export_character(
    state: State<'_, AppState>,
    character_id: i64,
    folder: String,
) -> Result<ExportedFiles, String> {
    let folder = std::path::PathBuf::from(folder.trim());
    if !folder.is_dir() {
        return Err(format!("{} is not a folder", folder.display()));
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let settings = db.settings().map_err(|e| e.to_string())?;
    let character = db
        .character(character_id)
        .map_err(|e| e.to_string())?
        .ok_or("that character is not in the app any more")?;
    let journal = archive::load(&db, character_id)
        .map_err(|e| e.to_string())?
        .ok_or("this character has no journal yet")?;
    let stories = stories::all(&db, character_id).map_err(|e| e.to_string())?;
    let awards = milestones::awards(&db, character_id).map_err(|e| e.to_string())?;
    let mut game_folders: Vec<std::path::PathBuf> = discovery::discover(&DiscoveryOverrides {
        wow_root: settings.wow_root_override.clone(),
    })
    .into_iter()
    .map(|r| r.path)
    .collect();
    // The game folder holding this character's journal (<game>/<flavor>/WTF/Account/<account>/
    // <realm>/<character>/SavedVariables/Mrglglgl.lua), wherever it was found. Never the
    // filesystem root, which would refuse every folder.
    game_folders.extend(
        character
            .journal_path
            .ancestors()
            .nth(8)
            .filter(|p| p.parent().is_some())
            .map(Path::to_path_buf),
    );
    let title = [character.name.as_deref(), character.surname.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    let title = if title.is_empty() {
        character.character_folder.clone()
    } else {
        title
    };
    let files = export::write(
        &folder,
        &game_folders,
        &export::ExportInput {
            title: &title,
            journal: &journal,
            stories: &stories,
            awards: &awards,
            language: &writing::prose_language(&settings),
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(ExportedFiles {
        markdown: files.markdown.to_string_lossy().into_owned(),
        html: files.html.to_string_lossy().into_owned(),
    })
}

/// Whether entries sent to this address leave the machine; the settings then say what is sent.
#[tauri::command]
pub fn endpoint_sends_off_machine(url: String) -> bool {
    endpoint::sends_off_machine(&url)
}
