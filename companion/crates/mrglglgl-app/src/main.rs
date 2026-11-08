// Keeps a console window from opening next to the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod addon_commands;
mod commands;
mod failure;
mod logging;
mod power;
mod site_commands;
mod sync;
#[cfg(test)]
mod test_data;
mod writing;
mod writing_commands;

use mrglglgl_core::db::Database;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // The window opens at the size and place it was closed at.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            // Kept for the app's lifetime: dropping it would stop the log file being written.
            app.manage(logging::init(&data_dir));
            tracing::info!("mrglglgl {} starting", env!("CARGO_PKG_VERSION"));
            let db = Database::open(&data_dir)?;
            // The archive is the only copy of days the game has compacted: keep a weekly copy.
            match db.snapshot_if_due(&data_dir, chrono::Utc::now().timestamp()) {
                Ok(Some(path)) => tracing::info!("database snapshot: {}", path.display()),
                Ok(None) => {}
                Err(e) => tracing::warn!("database snapshot: {e}"),
            }
            app.manage(sync::AppState::new(db, data_dir));
            // Importing can take a moment on a big archive; the window opens meanwhile and
            // refreshes when the journals arrive.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let state = handle.state::<sync::AppState>();
                if let Err(e) = sync::sync_all(&state) {
                    tracing::warn!("first sync: {e}");
                }
                if let Err(e) = sync::start_watching(&handle) {
                    tracing::warn!("watching journals: {e}");
                }
                let _ = tauri::Emitter::emit(&handle, sync::JOURNAL_UPDATED, ());
            });
            tauri::async_runtime::spawn(writing::run(app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::list_characters,
            commands::character_journal,
            commands::character_milestones,
            commands::character_journal_shared,
            commands::character_milestones_shared,
            commands::character_data_use,
            commands::set_character_data_use,
            commands::character_screenshots,
            commands::get_settings,
            logging::show_log,
            logging::recent_log,
            logging::open_link,
            addon_commands::addon_views,
            addon_commands::addon_release,
            addon_commands::set_addon_prereleases,
            addon_commands::install_addon,
            addon_commands::remove_addon,
            addon_commands::restore_addon,
            commands::save_settings,
            commands::sync_now,
            commands::delete_character,
            writing_commands::character_stories,
            writing_commands::review_list,
            writing_commands::review_all,
            writing_commands::accept_draft,
            writing_commands::edit_story,
            writing_commands::discard_draft,
            writing_commands::write_drafts,
            writing_commands::writing_status,
            writing_commands::activity,
            writing_commands::write_now,
            writing_commands::write_so_far,
            writing_commands::cancel_writing,
            writing_commands::retry_writing,
            writing_commands::set_writing_paused,
            writing_commands::list_narrators,
            writing_commands::narrator,
            writing_commands::copy_narrator,
            writing_commands::blank_narrator,
            writing_commands::save_narrator,
            writing_commands::import_narrator_file,
            writing_commands::export_narrator,
            writing_commands::delete_narrator,
            writing_commands::preview_narrator,
            writing_commands::cancel_preview,
            writing_commands::models,
            writing_commands::download_model,
            writing_commands::cancel_download,
            writing_commands::delete_model,
            writing_commands::save_api_key,
            writing_commands::has_api_key,
            writing_commands::test_endpoint,
            writing_commands::endpoint_sends_off_machine,
            writing_commands::copy_entry_text,
            writing_commands::default_export_folder,
            writing_commands::export_character,
            writing_commands::export_debug,
            writing_commands::screenshot_data,
            writing_commands::print_page,
            site_commands::save_site_settings,
            site_commands::site_stand_in_names,
            site_commands::build_site,
            site_commands::copy_site_theme,
        ])
        .run(tauri::generate_context!())
        .expect("the Mrglglgl window failed to start");
}
