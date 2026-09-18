//! Tauri IPC commands (SPEC §8.3).

use std::path::PathBuf;

use clatterbox_core::{AudioStatus, HookStatus, PackInfo, PackSlot, Settings, SettingsPatch};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{CmdError, ErrorCode};
use crate::events;
use crate::packs;
use crate::state::AppState;
use crate::tray;

#[derive(Serialize, Clone)]
pub struct Status {
    pub hook: HookStatus,
    pub audio: AudioStatus,
    /// "windows" | "macos" | "linux"
    pub platform: &'static str,
    pub version: &'static str,
}

#[cfg(target_os = "windows")]
pub(crate) const PLATFORM: &str = "windows";
#[cfg(target_os = "macos")]
pub(crate) const PLATFORM: &str = "macos";
#[cfg(target_os = "linux")]
pub(crate) const PLATFORM: &str = "linux";

/// Single code path for settings changes from IPC and the tray (SPEC §8.5).
pub(crate) fn apply_patch(app: &AppHandle, patch: SettingsPatch) -> Result<Settings, CmdError> {
    let state = app.state::<AppState>();

    let current = state.settings.lock().clone();
    let next = current.merged(&patch);

    // Pack change: load (with fallback baked into `load_with_fallback`), but only accept the
    // new setting if the requested pack itself loaded (a bad `pack` id must not silently swap
    // in the fallback and pretend it succeeded).
    if next.pack != current.pack {
        let registry = state.registry.lock();
        let mut cache = state.loaded.lock();
        let out_rate = crate::SYNTH_RATE;
        match registry.load(&next.pack, out_rate) {
            Ok(loaded) => {
                let loaded = std::sync::Arc::new(loaded);
                cache.insert(loaded.clone());
                drop(registry);
                drop(cache);
                state.engine.set_pack(PackSlot::Main, loaded);
            }
            Err(err) => {
                return Err(CmdError::from(err));
            }
        }
    }

    state.params.apply(&next);

    if next.launch_at_login != current.launch_at_login {
        // WP1 owns autostart wiring; the plugin is registered in `lib.rs::run`.
        use tauri_plugin_autostart::ManagerExt;
        let autostart = app.autolaunch();
        let result = if next.launch_at_login {
            autostart.enable()
        } else {
            autostart.disable()
        };
        if let Err(e) = result {
            tracing::warn!("autostart toggle failed: {e}");
        }
    }

    *state.settings.lock() = next.clone();
    state.persister.schedule(next.clone());

    tray::refresh(app);
    let _ = app.emit(events::SETTINGS_CHANGED, &next);

    Ok(next)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, CmdError> {
    Ok(state.settings.lock().clone())
}

#[tauri::command]
pub fn update_settings(app: AppHandle, patch: SettingsPatch) -> Result<Settings, CmdError> {
    apply_patch(&app, patch)
}

#[tauri::command]
pub fn list_packs(state: State<'_, AppState>) -> Result<Vec<PackInfo>, CmdError> {
    Ok(state.registry.lock().list().to_vec())
}

#[tauri::command]
pub fn reload_packs(app: AppHandle) -> Result<Vec<PackInfo>, CmdError> {
    let state = app.state::<AppState>();
    let infos = state.registry.lock().rescan().to_vec();
    let _ = app.emit(events::PACKS_CHANGED, &infos);
    Ok(infos)
}

#[tauri::command]
pub fn preview_pack(app: AppHandle, id: String) -> Result<(), CmdError> {
    let state = app.state::<AppState>();
    let out_rate = crate::SYNTH_RATE;
    let loaded = {
        let registry = state.registry.lock();
        let mut cache = state.loaded.lock();
        if let Some(cached) = cache.get(&id) {
            cached
        } else {
            let loaded = registry.load(&id, out_rate)?;
            let loaded = std::sync::Arc::new(loaded);
            cache.insert(loaded.clone());
            loaded
        }
    };
    state.engine.set_pack(PackSlot::Preview, loaded);
    packs::schedule_preview(app);
    Ok(())
}

/// JS key: `srcDir`.
#[tauri::command]
pub fn import_pack(app: AppHandle, src_dir: String) -> Result<PackInfo, CmdError> {
    if src_dir.trim().is_empty() {
        return Err(CmdError::new(ErrorCode::InvalidInput, "srcDir is empty"));
    }
    let src = PathBuf::from(&src_dir);
    if !src.is_dir() {
        return Err(CmdError::new(
            ErrorCode::NotFound,
            format!("{src_dir} is not a directory"),
        ));
    }
    let state = app.state::<AppState>();
    let registry = state.registry.lock();
    if let Some(dir_name) = src.file_name().and_then(|n| n.to_str()) {
        let candidate_id = format!("user/{dir_name}");
        if registry.list().iter().any(|p| p.id == candidate_id) {
            return Err(CmdError::new(
                ErrorCode::Exists,
                format!("a pack named '{dir_name}' is already imported"),
            ));
        }
    }
    drop(registry);

    let info = state.registry.lock().import_dir(&src)?;
    let infos = state.registry.lock().list().to_vec();
    let _ = app.emit(events::PACKS_CHANGED, &infos);
    Ok(info)
}

#[tauri::command]
pub fn open_packs_dir(app: AppHandle) -> Result<(), CmdError> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(CmdError::from)?
        .join("packs");
    std::fs::create_dir_all(&dir)?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| CmdError::new(ErrorCode::Platform, e.to_string()))
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> Result<Status, CmdError> {
    let hook = state
        .hook
        .lock()
        .as_ref()
        .map(|h| h.status())
        .unwrap_or(HookStatus::Starting);
    Ok(Status {
        hook,
        audio: state.engine.status(),
        platform: PLATFORM,
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tauri::command]
pub fn request_input_permission(app: AppHandle) -> Result<HookStatus, CmdError> {
    #[cfg(target_os = "macos")]
    {
        clatterbox_keyhook::request_permission();
        let _ = app
            .opener()
            .open_url(clatterbox_keyhook::MACOS_PRIVACY_URL, None::<&str>);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = &app;
    }
    let state = app.state::<AppState>();
    let hook = state
        .hook
        .lock()
        .as_ref()
        .map(|h| h.status())
        .unwrap_or(HookStatus::Starting);
    Ok(hook)
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart()
}
