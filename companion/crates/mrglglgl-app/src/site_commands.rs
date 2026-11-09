//! Commands for the journal website: its settings, its build, and copying the default theme.

use std::collections::BTreeSet;
use std::path::PathBuf;

use mrglglgl_core::archive;
use mrglglgl_core::beats::DayRecord;
use mrglglgl_core::db::SiteSettings;
use mrglglgl_core::prompt::{self, Alias};
use mrglglgl_core::site::{self, SiteData, WriteReport};
use tauri::{AppHandle, State};

use crate::sync::AppState;

/// Saves the website's settings alone: unlike the other settings, they need no rescan.
#[tauri::command]
pub fn save_site_settings(state: State<'_, AppState>, site: SiteSettings) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut settings = db.settings().map_err(|e| e.to_string())?;
    tracing::info!("website settings saved");
    tracing::debug!("website settings: {site:?}");
    settings.site = site;
    db.save_settings(&settings).map_err(|e| e.to_string())
}

/// Stand-in names for every other player the given characters grouped with, by first name.
#[tauri::command]
pub fn site_stand_in_names(
    state: State<'_, AppState>,
    character_ids: Vec<i64>,
) -> Result<Vec<Alias>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut names = BTreeSet::new();
    for id in character_ids {
        if let Some(journal) = archive::load_for(&db, id, mrglglgl_core::usage::Audience::Shared)
            .map_err(|e| e.to_string())?
        {
            for day in journal.days.values() {
                names.extend(DayRecord::of(day).companions);
            }
        }
    }
    // Sorted, so the same companions get the same stand-ins on every build.
    let names: Vec<String> = names.into_iter().collect();
    Ok(prompt::stand_in_names(&names))
}

/// Renders the site the window prepared through the chosen theme and writes the site folder.
/// Screenshots are read only from the site's own characters' screenshots.
#[tauri::command]
pub async fn build_site(
    app: AppHandle,
    state: State<'_, AppState>,
    data: SiteData,
) -> Result<WriteReport, String> {
    let (settings, journals) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let settings = db.settings().map_err(|e| e.to_string())?;
        let mut journals = Vec::new();
        for id in &settings.site.characters {
            if let Some(c) = db.character(*id).map_err(|e| e.to_string())? {
                journals.push(c.journal_path);
            }
        }
        (settings, journals)
    };
    let folder = settings
        .site
        .folder
        .clone()
        .ok_or("choose a folder for the website first")?;

    let mut allowed = BTreeSet::new();
    for id in settings.site.characters.clone() {
        let shots = crate::commands::character_screenshots(app.clone(), state.clone(), id)?;
        allowed.extend(shots.into_values().flatten().map(|s| s.path));
    }
    for character in &data.characters {
        for post in &character.posts {
            for day in &post.days {
                for photo in &day.photos {
                    if !allowed.contains(&PathBuf::from(&photo.source)) {
                        return Err(format!(
                            "{} is not one of the site's characters' screenshots",
                            photo.source
                        ));
                    }
                }
            }
        }
    }

    let game_folders =
        crate::writing_commands::game_folders(&settings, journals.iter().map(PathBuf::as_path));
    let theme = settings.site.theme_folder.clone();
    tracing::info!(
        "building the website into {} ({} characters, theme {})",
        folder.display(),
        data.characters.len(),
        theme
            .as_ref()
            .map_or("built in".to_string(), |t| t.display().to_string())
    );
    let started = std::time::Instant::now();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let files = site::render(&data, theme.as_deref()).map_err(|e| e.to_string())?;
        site::write(&folder, &game_folders, &files).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?;
    match &result {
        Ok(report) => tracing::info!("website built in {:?}: {report:?}", started.elapsed()),
        Err(e) => tracing::warn!("website build failed: {e}"),
    }
    result
}

/// Writes the default theme into `folder` as a starting point; files already there are kept.
#[tauri::command]
pub fn copy_site_theme(folder: String) -> Result<Vec<String>, String> {
    let folder = PathBuf::from(folder.trim());
    if !folder.is_dir() {
        return Err(format!("{} is not a folder", folder.display()));
    }
    tracing::info!("copying the default theme into {}", folder.display());
    site::copy_default_theme(&folder).map_err(|e| e.to_string())
}

/// The built website's front page, from the folder in Settings, once a build wrote it.
fn site_index(state: &AppState) -> Result<PathBuf, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let folder = db
        .settings()
        .map_err(|e| e.to_string())?
        .site
        .folder
        .ok_or("no folder is chosen for the website")?;
    let index = folder.join("index.html");
    if index.is_file() {
        Ok(index)
    } else {
        Err("the website has not been built yet".to_string())
    }
}

/// Shows the built website in the system's file manager.
#[tauri::command]
pub fn show_site_folder(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(site_index(&state)?)
        .map_err(|e| e.to_string())
}

/// Opens the built website's front page in the browser.
#[tauri::command]
pub fn open_site(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let index = site_index(&state)?;
    app.opener()
        .open_path(index.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Shows a file the app wrote, an export or a debug file, in the system's file manager. It is
/// only shown, never opened.
#[tauri::command]
pub fn show_in_folder(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err(format!("{} is not there any more", path.display()));
    }
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
}
