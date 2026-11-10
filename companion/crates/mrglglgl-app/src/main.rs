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

/// The macOS app menu's Check for Updates item, and the event it sends the window.
#[cfg(target_os = "macos")]
const CHECK_FOR_UPDATES: &str = "check-for-updates";

/// macOS's standard menus, with Check for Updates under About in the app menu, where Mac apps
/// keep it. The label follows the system language, as the rest of that menu does. Windows has
/// no menu bar, and gets none.
#[cfg(target_os = "macos")]
fn app_menu(handle: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem, MenuItemKind};
    let menu = Menu::default(handle)?;
    let french = sys_locale::get_locale().is_some_and(|l| l.to_lowercase().starts_with("fr"));
    let label = if french {
        "Rechercher des mises à jour…"
    } else {
        "Check for Updates…"
    };
    if let Some(MenuItemKind::Submenu(app)) = menu.items()?.into_iter().next() {
        let item = MenuItem::with_id(handle, CHECK_FOR_UPDATES, label, true, None::<&str>)?;
        // Right after "About Mrglglgl".
        app.insert(&item, 1)?;
    }
    Ok(menu)
}

fn main() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.menu(app_menu).on_menu_event(|app, event| {
        if event.id() == CHECK_FOR_UPDATES {
            tracing::info!("update check asked for from the app menu");
            let _ = tauri::Emitter::emit(app, CHECK_FOR_UPDATES, ());
        }
    });
    builder
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
            tracing::info!(
                "mrglglgl {} starting on {} {} ({}), locale {}",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                sysinfo::System::os_version().unwrap_or_default(),
                std::env::consts::ARCH,
                sys_locale::get_locale().unwrap_or_else(|| "unknown".into()),
            );
            tracing::debug!("data folder: {}", data_dir.display());
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
                match sync::sync_all(&state) {
                    Ok(_) => {}
                    Err(e) => tracing::warn!("first sync: {e}"),
                }
                match sync::start_watching(&handle) {
                    Ok(()) => tracing::debug!("watching the journals for changes"),
                    Err(e) => tracing::warn!("watching journals: {e}"),
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
            logging::log_from_window,
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
            commands::set_character_enabled,
            writing_commands::character_stories,
            writing_commands::review_list,
            writing_commands::review_all,
            writing_commands::character_writing,
            writing_commands::set_character_writing,
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
            writing_commands::narrator_profile,
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
            site_commands::show_site_folder,
            site_commands::open_site,
            site_commands::show_in_folder,
            site_commands::site_stand_in_names,
            site_commands::build_site,
            site_commands::copy_site_theme,
        ])
        .run(tauri::generate_context!())
        .expect("the Mrglglgl window failed to start");
}
