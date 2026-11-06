// Keeps a console window from opening next to the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod sync;
#[cfg(test)]
mod test_data;
mod writing;

use mrglglgl_core::db::Database;
use tauri::Manager;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mrglglgl=info".into()),
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let db = Database::open(&data_dir)?;
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
            commands::character_screenshots,
            commands::get_settings,
            commands::save_settings,
            commands::sync_now,
            commands::delete_character,
        ])
        .run(tauri::generate_context!())
        .expect("the Mrglglgl window failed to start");
}
