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

/// The folder the player's own narrators live in, one TOML file each.
pub fn narrators_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("narrators")
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

/// Stores the endpoint key in the system keychain, or removes it when empty.
pub fn set_api_key(key: Option<&str>) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ENTRY).map_err(|e| e.to_string())?;
    match key.filter(|k| !k.is_empty()) {
        Some(k) => entry.set_password(k).map_err(|e| e.to_string()),
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        },
    }
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
pub fn provider(state: &AppState, settings: &Settings) -> Option<Arc<dyn Provider>> {
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
    match mrglglgl_core::archive::load_for(db, character_id, mrglglgl_core::usage::Audience::Shared)
    {
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
        if !settings.writing.write_on_battery && crate::power::on_battery() {
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use mrglglgl_core::beats::Thresholds;
    use mrglglgl_core::narrators::{self, DEFAULT_NARRATOR};
    use mrglglgl_core::prompt::{DayRequest, Meddling, day_prompt};
    use mrglglgl_core::writer::{WriteOptions, WriteResult, write};
    use mrglglgl_core::{milestones, schema};
    use mrglglgl_llm::CancellationToken;
    use mrglglgl_llm::local::{LocalOptions, LocalProvider};

    /// One fixture day through the whole pipeline on a real model: beats, prompt, the bundled
    /// llama.cpp provider and the fact check. Models are gigabytes and never in the repository,
    /// so this runs only when `MRGLGLGL_TEST_MODEL` points at a GGUF (with
    /// `MRGLGLGL_TEST_MODEL_REASONING=1` for Qwen3.5 and SmolLM3) and is skipped otherwise.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_real_model_writes_a_fixture_day() {
        let Some(path) = std::env::var_os("MRGLGLGL_TEST_MODEL") else {
            eprintln!("skipped: set MRGLGLGL_TEST_MODEL to a GGUF file to run this test");
            return;
        };
        let reasoning = std::env::var_os("MRGLGLGL_TEST_MODEL_REASONING").is_some();
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../fixtures/savedvariables/synthetic-character.lua"),
        )
        .unwrap();
        let journal = schema::journal_from_text(&text).unwrap();
        let awards = milestones::compute(&journal);
        let language = std::env::var("MRGLGLGL_TEST_LANGUAGE").unwrap_or_else(|_| "en".into());
        let prompt = day_prompt(&DayRequest {
            character_id: 1,
            journal: &journal,
            day: "2026-11-08",
            awards: &awards,
            running_jokes: &[],
            narrator: narrators::get(DEFAULT_NARRATOR, &language),
            language: &language,
            meddling: Meddling::Sometimes,
            pseudonyms: false,
            thresholds: &Thresholds::default(),
            note: None,
            preferences: None,
        });
        let provider = LocalProvider::new(LocalOptions::new(path, "test", reasoning));
        let started = std::time::Instant::now();
        let result = write(
            &provider,
            &prompt,
            &WriteOptions {
                language: language.clone(),
                max_tokens: 400,
                temperature: mrglglgl_llm::DEFAULT_TEMPERATURE,
                timeout: Duration::from_secs(600),
                allowed: vec!["Horde".into()],
            },
            &CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
        eprintln!("took {:?}\n{result:#?}", started.elapsed());
        assert!(matches!(result, WriteResult::Written { .. }));
    }

    /// Samples for the owner's prose review: every day of the shared fixture, written by each
    /// shipped narrator in English and French through the real pipeline, one plain text file per
    /// entry named `<narrator>-<day>-<lang>.txt`, with the fact check's outcome on the first line.
    ///
    /// Run by hand, since it takes a model and some minutes:
    /// `MRGLGLGL_TEST_MODEL=<gguf> MRGLGLGL_TEST_MODEL_REASONING=1 MRGLGLGL_SAMPLES_DIR=<folder>
    /// cargo test -p mrglglgl-app narrator_samples -- --ignored --nocapture`
    ///
    /// Every narrator, language and day takes the better part of an hour on a laptop.
    /// `MRGLGLGL_SAMPLES_NARRATORS` and `MRGLGLGL_SAMPLES_DAYS`, comma-separated, keep only those.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "writes samples with a real model; needs MRGLGLGL_TEST_MODEL and MRGLGLGL_SAMPLES_DIR"]
    async fn narrator_samples() {
        let (Some(path), Some(out)) = (
            std::env::var_os("MRGLGLGL_TEST_MODEL"),
            std::env::var_os("MRGLGLGL_SAMPLES_DIR"),
        ) else {
            eprintln!("skipped: set MRGLGLGL_TEST_MODEL and MRGLGLGL_SAMPLES_DIR");
            return;
        };
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        let reasoning = std::env::var_os("MRGLGLGL_TEST_MODEL_REASONING").is_some();
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../fixtures/savedvariables/synthetic-character.lua"),
        )
        .unwrap();
        let journal = schema::journal_from_text(&text).unwrap();
        let awards = milestones::compute(&journal);
        let provider = LocalProvider::new(LocalOptions::new(path, "samples", reasoning));
        let only = |var: &str| -> Option<Vec<String>> {
            std::env::var(var)
                .ok()
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
        };
        let (only_narrators, only_days) = (
            only("MRGLGLGL_SAMPLES_NARRATORS"),
            only("MRGLGLGL_SAMPLES_DAYS"),
        );
        let mut summary = Vec::new();
        for narrator in narrators::list() {
            if only_narrators
                .as_ref()
                .is_some_and(|keep| !keep.contains(&narrator.id))
            {
                continue;
            }
            for language in ["en", "fr"] {
                for day in journal.days.keys() {
                    if only_days.as_ref().is_some_and(|keep| !keep.contains(day)) {
                        continue;
                    }
                    let prompt = day_prompt(&DayRequest {
                        character_id: 1,
                        journal: &journal,
                        day,
                        awards: &awards,
                        running_jokes: &[],
                        narrator: narrators::get(&narrator.id, language),
                        language,
                        meddling: Meddling::Sometimes,
                        pseudonyms: false,
                        thresholds: &Thresholds::default(),
                        note: None,
                        preferences: None,
                    });
                    let started = std::time::Instant::now();
                    let result = write(
                        &provider,
                        &prompt,
                        &WriteOptions {
                            language: language.to_string(),
                            max_tokens: mrglglgl_core::prompt::tokens_for(prompt.words),
                            temperature: mrglglgl_llm::DEFAULT_TEMPERATURE,
                            timeout: Duration::from_secs(600),
                            allowed: vec!["Horde".into()],
                        },
                        &CancellationToken::new(),
                        None,
                    )
                    .await;
                    let seconds = started.elapsed().as_secs_f32();
                    let (first_line, body) = match result {
                        Ok(WriteResult::Written {
                            text,
                            check,
                            reason,
                        }) => (
                            match reason {
                                Some(r) => format!("fact check: {} ({r})", check.key()),
                                None => format!("fact check: {}", check.key()),
                            },
                            text,
                        ),
                        Ok(other) => (format!("not written: {other:?}"), String::new()),
                        Err(e) => (format!("not written: {e}"), String::new()),
                    };
                    let name = format!("{}-{day}-{language}.txt", narrator.id);
                    let margin = if prompt.meddling {
                        "margin note asked for"
                    } else {
                        "no margin note asked for"
                    };
                    std::fs::write(
                        out.join(&name),
                        format!("{first_line}\n{margin}; {seconds:.1} s\n\n{body}\n"),
                    )
                    .unwrap();
                    eprintln!("{name}: {first_line} in {seconds:.1} s");
                    summary.push(format!("{name}\t{first_line}\t{seconds:.1} s"));
                }
            }
        }
        std::fs::write(out.join("summary.tsv"), summary.join("\n") + "\n").unwrap();
    }
}
