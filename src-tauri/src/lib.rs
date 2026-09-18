//! Clatterbox app shell (SPEC §8). Wires the real audio engine + keyhook per the startup order
//! in SPEC §8.2.

mod commands;
mod error;
mod events;
mod packs;
mod persist;
mod state;
mod tray;
mod window;

use std::sync::Arc;

use clatterbox_core::settings::{self, LoadOutcome};
use clatterbox_core::{EngineParams, HookStatus, PackRegistry, Settings};
use tauri::{Manager, path::BaseDirectory};
use tauri_plugin_autostart::MacosLauncher;

use packs::PackCache;
use persist::Persister;
use state::AppState;
use tracing::Level;
use tracing_subscriber::{filter::Targets, prelude::*};

/// Sample rate procedural (`synth/*`) packs are generated at; the audio engine resamples per
/// SPEC §4.4/§6.
pub(crate) const SYNTH_RATE: u32 = 48_000;

pub fn run() {
    // symphonia logs every unknown WAV chunk at INFO; keep our own logs at INFO.
    let filter = Targets::new()
        .with_default(Level::INFO)
        .with_target("symphonia", Level::WARN);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    tauri::Builder::default()
        // Single-instance must be registered first.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let _ = window::open(app);
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
            let handle = app.handle().clone();

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_config_dir = handle.path().app_config_dir()?;
            std::fs::create_dir_all(&app_config_dir)?;
            let settings_path = app_config_dir.join("settings.json");

            let (mut loaded_settings, load_outcome) = settings::load(&settings_path);

            let params = Arc::new(EngineParams::from_settings(&loaded_settings));

            let audio_status_handle = handle.clone();
            let (engine, hook_tx) = clatterbox_audio::AudioEngine::start(
                params.clone(),
                Box::new(move |_status| emit_status(&audio_status_handle)),
            );

            let builtin_dir = handle.path().resolve("packs", BaseDirectory::Resource).ok();
            let user_dir = app_config_dir.join("packs");
            std::fs::create_dir_all(&user_dir)?;

            let mut registry = PackRegistry::new(builtin_dir, user_dir);
            registry.rescan();

            let mut cache = PackCache::new();
            let (loaded_pack, pack_err) =
                packs::load_with_fallback(&registry, &mut cache, &loaded_settings.pack, SYNTH_RATE);
            if let Some(err) = pack_err {
                tracing::warn!(
                    "pack '{}' failed to load, falling back: {err}",
                    loaded_settings.pack
                );
                loaded_settings.pack = Settings::FALLBACK_PACK.to_owned();
                let _ = settings::save_atomic(&settings_path, &loaded_settings);
            }
            engine.set_pack(clatterbox_core::PackSlot::Main, loaded_pack);

            let hook_status_handle = handle.clone();
            let hook = clatterbox_keyhook::start(
                hook_tx,
                params.clone(),
                Box::new(move |status| {
                    emit_status(&hook_status_handle);
                    // `hook.status()` right after `start()` races the hook thread — it can
                    // still read `Starting` even when the backend is about to report
                    // `NeedsPermission`/`NeedsRestart`. Open the window from here instead, once
                    // the real status lands; dispatched to the main thread since this callback
                    // can fire from the hook's own thread.
                    if matches!(
                        status,
                        HookStatus::NeedsPermission { .. } | HookStatus::NeedsRestart
                    ) {
                        let window_handle = hook_status_handle.clone();
                        let _ = hook_status_handle.run_on_main_thread(move || {
                            let _ = window::open(&window_handle);
                        });
                    }
                }),
            );

            let persister = Persister::new(settings_path);

            app.manage(AppState {
                settings: parking_lot::Mutex::new(loaded_settings),
                params,
                engine,
                hook: parking_lot::Mutex::new(Some(hook)),
                registry: parking_lot::Mutex::new(registry),
                loaded: parking_lot::Mutex::new(cache),
                persister,
            });

            tray::build(&handle)?;

            if matches!(load_outcome, LoadOutcome::FirstRun) {
                let _ = window::open(&handle);
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Clatterbox")
        .run(|app_handle, event| {
            // Flush any pending debounced settings write before the process actually goes away
            // (quit via tray already flushes; this also covers OS-initiated shutdown/exit).
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) && let Some(state) = app_handle.try_state::<AppState>()
            {
                state.persister.flush();
            }
        });
}

/// Reads the current hook + audio status from managed state and emits the combined `Status`
/// used by `get_status` (SPEC §8.3/§8.4).
fn emit_status(app: &tauri::AppHandle) {
    use tauri::Emitter;

    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let hook = state
        .hook
        .lock()
        .as_ref()
        .map(|h| h.status())
        .unwrap_or(HookStatus::Starting);
    let status = commands::Status {
        hook,
        audio: state.engine.status(),
        platform: commands::PLATFORM,
        version: env!("CARGO_PKG_VERSION"),
    };
    let _ = app.emit(events::STATUS_CHANGED, &status);
    // Rebuilds the permission item / relaunch label; `tray::refresh` hops to the main thread
    // itself, so this is safe to call from the hook/audio status callback threads.
    tray::refresh(app);
}
