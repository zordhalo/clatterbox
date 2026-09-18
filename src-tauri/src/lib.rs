//! Clatterbox app shell (SPEC §8). Phase 0 skeleton: plugins + placeholder tray only.
//! WP1 wires setup per the startup order in SPEC §8.2.

mod commands;
mod error;
mod events;
mod packs;
mod persist;
mod state;
mod tray;
mod window;

use tauri_plugin_autostart::MacosLauncher;

pub fn run() {
    tauri::Builder::default()
        // Single-instance must be registered first.
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
            // WP1: window::open(app)
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_settings,
            commands::list_packs,
            commands::reload_packs,
            commands::preview_pack,
            commands::import_pack,
            commands::open_packs_dir,
            commands::get_status,
            commands::request_input_permission,
            commands::restart_app,
        ])
        .setup(|app| {
            tray::build(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Clatterbox");
}
