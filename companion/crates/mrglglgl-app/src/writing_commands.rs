//! Commands for writing: entries and their versions, the queue, the models and the endpoint.

use std::path::{Path, PathBuf};

use mrglglgl_core::archive;
use mrglglgl_core::db::{ProviderChoice, Settings};
use mrglglgl_core::discovery::{self, DiscoveryOverrides};
use mrglglgl_core::export;
use mrglglgl_core::narrators::{self, Invalid, Narrator, NarratorError, NarratorInfo};
use mrglglgl_core::queue::{self, Job, JobState, PreviewDay, PreviewStatus};
use mrglglgl_core::review::{self, ReviewItem};
use mrglglgl_core::stories::{self, Story, StoryKind};
use mrglglgl_core::writer::{CheckStatus, StreamEvent, WriteResult};
use mrglglgl_llm::CancellationToken;
use mrglglgl_llm::catalogue::{self, Advice, Shortfall};
use mrglglgl_llm::endpoint::{self, EndpointConfig, EndpointProvider};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

use crate::failure::Failure;
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
    stories::accepted(&db, character_id).map_err(|e| e.to_string())
}

/// The character's To review list, newest first.
#[tauri::command]
pub fn review_list(
    state: State<'_, AppState>,
    character_id: i64,
) -> Result<Vec<ReviewItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let Some(journal) =
        archive::load_for(&db, character_id, mrglglgl_core::usage::Audience::Shared)
            .map_err(|e| e.to_string())?
    else {
        return Ok(Vec::new());
    };
    review::items(&db, character_id, &journal, &writing::today()).map_err(|e| e.to_string())
}

/// One character's part of the To review list for every character.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CharacterReview {
    pub character_id: i64,
    pub items: Vec<ReviewItem>,
}

/// The To review list of every character with something on it, in the characters' order.
#[tauri::command]
pub fn review_all(state: State<'_, AppState>) -> Result<Vec<CharacterReview>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let today = writing::today();
    let mut out = Vec::new();
    for character in db.characters().map_err(|e| e.to_string())? {
        let Some(journal) =
            archive::load_for(&db, character.id, mrglglgl_core::usage::Audience::Shared)
                .map_err(|e| e.to_string())?
        else {
            continue;
        };
        let items =
            review::items(&db, character.id, &journal, &today).map_err(|e| e.to_string())?;
        if !items.is_empty() {
            out.push(CharacterReview {
                character_id: character.id,
                items,
            });
        }
    }
    Ok(out)
}

/// Puts a draft in the journal; the other drafts for that day or week are discarded.
#[tauri::command]
pub fn accept_draft(
    state: State<'_, AppState>,
    character_id: i64,
    story_id: i64,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    stories::accept(&db, character_id, story_id).map_err(|e| e.to_string())
}

/// Saves the player's own text for a day or week as a new version in the journal.
#[tauri::command]
pub fn edit_story(
    state: State<'_, AppState>,
    character_id: i64,
    story_id: i64,
    text: String,
) -> Result<Option<i64>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    stories::edit(&db, character_id, story_id, &text).map_err(|e| e.to_string())
}

/// Throws a draft away; the day stays on the To review list until written again.
#[tauri::command]
pub fn discard_draft(
    state: State<'_, AppState>,
    character_id: i64,
    story_id: i64,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    stories::discard(&db, character_id, story_id).map_err(|e| e.to_string())
}

/// A day or week to write.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Slot {
    pub kind: StoryKind,
    pub start_day: String,
}

/// Writes days and weeks ahead of the backlog: a selection from the To review list, or one
/// rewrite with the player's note. Review mode decides, as for every entry, whether they wait as
/// drafts or go straight into the journal.
#[tauri::command]
pub fn write_drafts(
    state: State<'_, AppState>,
    character_id: i64,
    slots: Vec<Slot>,
    narrator: Option<String>,
    note: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let draft = db.settings().map_err(|e| e.to_string())?.writing.review;
    let ask = queue::Ask {
        narrator: narrator.as_deref(),
        draft,
        note: note.as_deref(),
        partial: false,
    };
    for slot in &slots {
        queue::request(&db, character_id, slot.kind, &slot.start_day, &ask)
            .map_err(|e| e.to_string())?;
    }
    state.control.wake();
    Ok(())
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
    let blocked = blocked(&state, &settings, !jobs.is_empty());
    Ok(WritingStatus {
        paused: settings.writing.paused,
        current: state.control.current(),
        jobs,
        blocked: blocked.map(str::to_string),
    })
}

/// Why nothing is being written, if something stands in the way. Waiting for the game to close
/// or for the charger only counts when something waits to be written.
fn blocked(state: &AppState, settings: &Settings, waiting: bool) -> Option<&'static str> {
    let ready = match settings.writing.provider {
        ProviderChoice::Local => {
            writing::chosen_model(&writing::catalogue(&state.data_dir), settings)
                .is_some_and(|m| m.installed(&writing::models_dir(&state.data_dir)))
        }
        ProviderChoice::Endpoint => writing::endpoint_config(settings).is_some(),
    };
    if settings.writing.paused {
        Some("paused")
    } else if !ready {
        Some(match settings.writing.provider {
            ProviderChoice::Local => "noModel",
            ProviderChoice::Endpoint => "noEndpoint",
        })
    } else if !settings.writing.write_while_playing && waiting && writing::game_running() {
        Some("gameRunning")
    } else if !settings.writing.write_on_battery && waiting && crate::power::on_battery() {
        Some("onBattery")
    } else {
        None
    }
}

/// Everything the app is working on, for every character: the entry being written, what waits
/// and what failed, a narrator preview, and a model download.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Activity {
    pub paused: bool,
    /// Why nothing is being written, as in [`WritingStatus`].
    pub blocked: Option<String>,
    /// Every character's jobs: the one running, those waiting in order, then those that failed.
    pub jobs: Vec<Job>,
    pub preview: Option<PreviewStatus>,
    pub download: Option<DownloadProgress>,
}

#[tauri::command]
pub fn activity(state: State<'_, AppState>) -> Result<Activity, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let settings = db.settings().map_err(|e| e.to_string())?;
    let jobs = queue::all_jobs(&db).map_err(|e| e.to_string())?;
    drop(db);
    let waiting = jobs.iter().any(|j| j.state != JobState::Failed);
    Ok(Activity {
        paused: settings.writing.paused,
        blocked: blocked(&state, &settings, waiting).map(str::to_string),
        jobs,
        preview: state.control.preview_status(),
        download: state
            .download
            .lock()
            .map_err(|e| e.to_string())?
            .as_ref()
            .map(|(progress, _)| progress.clone()),
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
    // In review mode a rewrite from the journal waits as a draft like every other entry.
    let draft = db.settings().map_err(|e| e.to_string())?.writing.review;
    let ask = queue::Ask {
        narrator: narrator.as_deref(),
        draft,
        note: None,
        partial: false,
    };
    queue::request(&db, character_id, kind, &start_day, &ask).map_err(|e| e.to_string())?;
    state.control.wake();
    Ok(())
}

/// Writes today as it stands, while it is still going, ahead of the backlog. It shows on the
/// page at once; the full entry is still written once the day is over.
#[tauri::command]
pub fn write_so_far(
    state: State<'_, AppState>,
    character_id: i64,
    narrator: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let ask = queue::Ask {
        narrator: narrator.as_deref(),
        partial: true,
        ..queue::Ask::default()
    };
    queue::request(&db, character_id, StoryKind::Day, &writing::today(), &ask)
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
pub fn list_narrators(state: State<'_, AppState>) -> Vec<NarratorInfo> {
    state.control.narrators().list()
}

/// Event sent while a narrator preview writes; the payload is a [`PreviewEvent`].
pub const NARRATOR_PREVIEW: &str = "narrator-preview";

/// The player's own narrator with this id, to edit.
#[tauri::command]
pub fn narrator(state: State<'_, AppState>, id: String) -> Option<Narrator> {
    state.control.narrators().custom(&id).cloned()
}

/// A narrator to start a new one from: every field of `id`, in the prose language, unsaved.
/// A new narrator from scratch in the player's prose language; not saved yet.
#[tauri::command]
pub fn blank_narrator(state: State<'_, AppState>) -> Result<Narrator, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let language = writing::prose_language(&db.settings().map_err(|e| e.to_string())?);
    Ok(narrators::blank(&language))
}

#[tauri::command]
pub fn copy_narrator(state: State<'_, AppState>, id: String) -> Result<Narrator, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let language = writing::prose_language(&db.settings().map_err(|e| e.to_string())?);
    let library = state.control.narrators();
    Ok(match library.custom(&id) {
        Some(mine) => {
            let mut copy = mine.clone();
            copy.id = String::new();
            copy.name = format!("{} (copy)", copy.name);
            copy
        }
        None => narrators::copy_of(&id, &language),
    })
}

/// What saving or importing a narrator came to.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export)]
pub enum NarratorSaved {
    Saved {
        narrator: Box<Narrator>,
    },
    /// Fields to fix before it can be saved; nothing was written.
    Invalid {
        problems: Vec<Invalid>,
    },
}

fn saved(result: Result<Narrator, NarratorError>) -> Result<NarratorSaved, String> {
    match result {
        Ok(narrator) => Ok(NarratorSaved::Saved {
            narrator: Box::new(narrator),
        }),
        Err(NarratorError::Invalid(problems)) => Ok(NarratorSaved::Invalid { problems }),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn save_narrator(
    state: State<'_, AppState>,
    narrator: Narrator,
) -> Result<NarratorSaved, String> {
    saved(narrators::save(
        &writing::narrators_dir(&state.data_dir),
        narrator,
    ))
}

/// Adds a shared narrator file, read from where the player picked it, as the player's own.
#[tauri::command]
pub fn import_narrator_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<NarratorSaved, String> {
    // A narrator is a few kilobytes; anything far larger is not one.
    const MAX_BYTES: u64 = 256 * 1024;
    let path = std::path::PathBuf::from(path);
    let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if size > MAX_BYTES {
        return Err(format!("{} is too large to be a narrator", path.display()));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    saved(narrators::import(
        &writing::narrators_dir(&state.data_dir),
        &text,
    ))
}

/// Writes the player's narrator as `<id>.toml` into `folder`; returns the file's path.
#[tauri::command]
pub fn export_narrator(
    state: State<'_, AppState>,
    id: String,
    folder: String,
) -> Result<String, String> {
    let folder = std::path::PathBuf::from(folder.trim());
    if !folder.is_dir() {
        return Err(format!("{} is not a folder", folder.display()));
    }
    let library = state.control.narrators();
    let narrator = library
        .custom(&id)
        .ok_or("that narrator no longer exists")?;
    let path = folder.join(format!("{id}.toml"));
    std::fs::write(&path, narrators::to_toml(narrator)).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn delete_narrator(state: State<'_, AppState>, id: String) -> Result<(), String> {
    narrators::delete(&writing::narrators_dir(&state.data_dir), &id).map_err(|e| e.to_string())
}

/// What the preview streams while it writes.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
#[ts(export)]
pub enum PreviewEvent {
    Chunk {
        text: String,
    },
    /// The fact check sent the text back; what streamed so far is discarded.
    Restarted,
}

/// How a preview ended.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "outcome")]
#[ts(export)]
pub enum PreviewResult {
    Written {
        text: String,
        check: CheckStatus,
        reason: Option<String>,
    },
    Cancelled,
    TimedOut,
}

/// Writes one day with the narrator being edited, saved or not, and stores nothing.
#[tauri::command]
pub async fn preview_narrator(
    app: AppHandle,
    state: State<'_, AppState>,
    narrator: Narrator,
    day: PreviewDay,
) -> Result<PreviewResult, String> {
    let settings = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.settings().map_err(|e| e.to_string())?
    };
    let provider = writing::provider(&state, &settings)
        .ok_or("nothing can write yet: download a model or set up an endpoint in Settings")?;
    let emit = move |event: StreamEvent| {
        let _ = app.emit(
            NARRATOR_PREVIEW,
            match event {
                StreamEvent::Chunk(text) => PreviewEvent::Chunk { text },
                StreamEvent::Restart => PreviewEvent::Restarted,
            },
        );
    };
    let result = queue::preview(
        &state.db,
        provider.as_ref(),
        &state.control,
        &writing::prose_language(&settings),
        &narrator,
        &day,
        &emit,
    )
    .await?;
    Ok(match result {
        WriteResult::Written {
            text,
            check,
            reason,
        } => PreviewResult::Written {
            text,
            check,
            reason,
        },
        WriteResult::Cancelled => PreviewResult::Cancelled,
        WriteResult::TimedOut => PreviewResult::TimedOut,
    })
}

#[tauri::command]
pub fn cancel_preview(state: State<'_, AppState>) -> bool {
    state.control.cancel_preview()
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

/// Total memory, and free space on the disk holding `dir` when it can be told.
fn machine(dir: &Path) -> (u64, Option<u64>) {
    use sysinfo::{Disks, MemoryRefreshKind, RefreshKind, System};
    let system = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );
    let disks: Vec<(PathBuf, u64)> = Disks::new_with_refreshed_list()
        .list()
        .iter()
        .map(|d| (d.mount_point().to_path_buf(), d.available_space()))
        .collect();
    // On Windows the standard canonicalize returns a verbatim path (`\\?\C:\...`) that never
    // starts with a mount point such as `C:\`; dunce gives the plain form back.
    let dir = dunce::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    (system.total_memory(), free_space_on(&dir, &disks))
}

/// The free space of the deepest mount holding `dir`, or `None` when no mount does: that is a
/// failed measurement, and must not read as a full disk.
fn free_space_on(dir: &Path, disks: &[(PathBuf, u64)]) -> Option<u64> {
    disks
        .iter()
        .filter(|(mount, _)| dir.starts_with(mount))
        .max_by_key(|(mount, _)| mount.as_os_str().len())
        .map(|(_, free)| *free)
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
    // Space that cannot be measured is not held against a model; the download stops cleanly if
    // the disk does fill.
    let advice = match catalogue.advise(ram, free.unwrap_or(u64::MAX), &language) {
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
) -> Result<(), Failure> {
    let catalogue = writing::catalogue(&state.data_dir);
    let entry = catalogue
        .get(&model_id)
        .cloned()
        .ok_or_else(|| format!("no model called {model_id}"))?;
    let cancel = CancellationToken::new();
    let started = DownloadProgress {
        model_id: model_id.clone(),
        downloaded: entry.partial_bytes(&writing::models_dir(&state.data_dir)),
        total: entry.size_bytes,
    };
    *state.download.lock().map_err(|e| e.to_string())? = Some((started, cancel.clone()));
    let (_, free) = machine(&state.data_dir);
    let mut last_percent = u64::MAX;
    let result = catalogue::download(
        &entry,
        &writing::models_dir(&state.data_dir),
        catalogue::HUGGING_FACE,
        free,
        &cancel,
        |p| {
            // A few hundred events, not one per network read.
            let percent = p.downloaded * 1000 / p.total.max(1);
            if percent != last_percent {
                last_percent = percent;
                let progress = DownloadProgress {
                    model_id: model_id.clone(),
                    downloaded: p.downloaded,
                    total: p.total,
                };
                if let Ok(mut current) = state.download.lock()
                    && let Some((shown, _)) = current.as_mut()
                {
                    *shown = progress.clone();
                }
                let _ = app.emit(MODEL_DOWNLOAD, progress);
            }
        },
    )
    .await;
    *state.download.lock().map_err(|e| e.to_string())? = None;
    result?;
    state.control.wake();
    Ok(())
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    if let Ok(Some((_, token))) = state.download.lock().map(|d| d.clone()) {
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
    let story = stories::accepted(&db, character_id)
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

/// Every World of Warcraft folder the app knows: the ones discovered, and the game folder holding
/// each journal (`<game>/<flavor>/WTF/Account/<account>/<realm>/<character>/SavedVariables/
/// Mrglglgl.lua`), wherever it was found. Exports and the website are never written inside any.
/// Never the filesystem root, which would refuse every folder.
pub(crate) fn game_folders<'a>(
    settings: &Settings,
    journals: impl IntoIterator<Item = &'a Path>,
) -> Vec<std::path::PathBuf> {
    let mut folders: Vec<std::path::PathBuf> = discovery::discover(&DiscoveryOverrides {
        wow_root: settings.wow_root_override.clone(),
    })
    .into_iter()
    .map(|r| r.path)
    .collect();
    folders.extend(journals.into_iter().filter_map(|journal| {
        journal
            .ancestors()
            .nth(8)
            .filter(|p| p.parent().is_some())
            .map(Path::to_path_buf)
    }));
    folders
}

/// What a journal is exported as.
#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ExportFormat {
    Markdown,
    /// One web page: the window lays it out from the journal's own components and sends it here.
    Html,
}

/// The web page the window laid out: its body and its look.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ExportPage {
    pub css: String,
    pub body: String,
}

/// Writes a character's journal into `folder` as Markdown or as one web page, refusing any folder
/// inside a World of Warcraft install. Returns the file's path.
#[tauri::command]
pub fn export_character(
    state: State<'_, AppState>,
    character_id: i64,
    folder: String,
    format: ExportFormat,
    page: Option<ExportPage>,
) -> Result<String, String> {
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
    let game_folders = game_folders(&settings, [character.journal_path.as_path()]);
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
    let language = writing::prose_language(&settings);
    let (extension, content) = match format {
        ExportFormat::Markdown => {
            let journal =
                archive::load_for(&db, character_id, mrglglgl_core::usage::Audience::Shared)
                    .map_err(|e| e.to_string())?
                    .ok_or("this character has no journal yet")?;
            let stories = stories::accepted(&db, character_id).map_err(|e| e.to_string())?;
            let awards = mrglglgl_core::usage::awards(
                &db,
                character_id,
                mrglglgl_core::usage::Audience::Shared,
            )
            .map_err(|e| e.to_string())?;
            let input = export::ExportInput {
                title: &title,
                journal: &journal,
                stories: &stories,
                awards: &awards,
                language: &language,
            };
            ("md", export::markdown(&input))
        }
        ExportFormat::Html => {
            let page = page.ok_or("the page to export is missing")?;
            (
                "html",
                export::page_document(&title, &language, &page.css, &page.body),
            )
        }
    };
    let path = export::write_file(&folder, &game_folders, &title, extension, &content)
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// One of the character's screenshots, scaled to page width, as a `data:` URL for an export.
/// Only a file the character's screenshots list holds is read.
#[tauri::command]
pub async fn screenshot_data(
    app: AppHandle,
    state: State<'_, AppState>,
    character_id: i64,
    path: String,
) -> Result<String, String> {
    let known = crate::commands::character_screenshots(app, state, character_id)?
        .values()
        .flatten()
        .any(|s| s.path.to_string_lossy() == path);
    if !known {
        return Err("that picture is not one of this character's screenshots".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        mrglglgl_core::screenshots::embed(std::path::Path::new(&path)).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Opens the system print dialog for the window, where the page can be saved as a PDF.
#[tauri::command]
pub fn print_page(window: tauri::WebviewWindow) -> Result<(), String> {
    window.print().map_err(|e| e.to_string())
}

/// Whether entries sent to this address leave the machine; the settings then say what is sent.
#[tauri::command]
pub fn endpoint_sends_off_machine(url: String) -> bool {
    endpoint::sends_off_machine(&url)
}

#[cfg(test)]
mod tests {
    use super::free_space_on;
    use std::path::{Path, PathBuf};

    fn disks(list: &[(&str, u64)]) -> Vec<(PathBuf, u64)> {
        list.iter().map(|(m, f)| (PathBuf::from(m), *f)).collect()
    }

    #[test]
    fn the_deepest_mount_holding_the_folder_gives_the_free_space() {
        let disks = disks(&[("/", 10), ("/Users", 20), ("/Volumes/Games", 30)]);
        assert_eq!(
            free_space_on(Path::new("/Users/ash/data"), &disks),
            Some(20)
        );
    }

    // A folder no disk claims means the measurement failed, not a full disk: reporting 0 blocked
    // every model download.
    #[test]
    fn no_disk_holding_the_folder_means_unknown_not_full() {
        let disks = disks(&[("/Volumes/Games", 30)]);
        assert_eq!(free_space_on(Path::new("/Users/ash/data"), &disks), None);
    }
}
