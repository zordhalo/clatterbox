//! Tauri IPC commands (SPEC §8.3).

use std::path::PathBuf;
use std::sync::Arc;

use clatterbox_core::{
    AudioStatus, HookStatus, LoadedPack, PackError, PackInfo, PackSlot, Settings, SettingsPatch,
};
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

/// Loads `id` on a blocking thread (decode is not cheap) unless it is already in the LRU cache,
/// in which case this returns immediately with no thread hop at all.
async fn load_pack_cached(app: &AppHandle, id: &str) -> Result<Arc<LoadedPack>, CmdError> {
    if let Some(cached) = app.state::<AppState>().loaded.lock().get(id) {
        return Ok(cached);
    }

    let app2 = app.clone();
    let pack_id = id.to_owned();
    let loaded: Result<LoadedPack, PackError> = tauri::async_runtime::spawn_blocking(move || {
        let state = app2.state::<AppState>();
        let registry = state.registry.lock();
        registry.load(&pack_id, crate::SYNTH_RATE)
    })
    .await
    .map_err(CmdError::from)?;
    let loaded = Arc::new(loaded.map_err(CmdError::from)?);

    app.state::<AppState>().loaded.lock().insert(loaded.clone());
    Ok(loaded)
}

/// Single code path for settings changes from IPC and the tray (SPEC §8.5). Called from both the
/// synchronous tray menu handler and the async `update_settings` command, so it stays
/// synchronous itself — `update_settings` pre-warms [`crate::packs::PackCache`] off-thread via
/// [`load_pack_cached`] before calling this, and this checks the cache first too (so a
/// just-previewed or just-warmed pack is never decoded twice), falling back to a direct
/// (blocking) registry load only on a genuine cache miss, e.g. picking a pack from the tray.
pub(crate) fn apply_patch(app: &AppHandle, patch: SettingsPatch) -> Result<Settings, CmdError> {
    let state = app.state::<AppState>();

    let current = state.settings.lock().clone();
    let next = current.merged(&patch);

    if next.pack != current.pack {
        let cached = state.loaded.lock().get(&next.pack);
        let loaded = match cached {
            Some(pack) => pack,
            None => {
                let loaded = state
                    .registry
                    .lock()
                    .load(&next.pack, crate::SYNTH_RATE)
                    .map_err(CmdError::from)?;
                let loaded = Arc::new(loaded);
                state.loaded.lock().insert(loaded.clone());
                loaded
            }
        };
        state.engine.set_pack(PackSlot::Main, loaded);
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
pub async fn update_settings(app: AppHandle, patch: SettingsPatch) -> Result<Settings, CmdError> {
    // Warm the cache off the calling thread before touching any state: `apply_patch` itself
    // never decodes, so a bad/never-loaded `pack` id must be resolved (or fail) here first.
    if let Some(pack_id) = &patch.pack {
        let current_pack = app.state::<AppState>().settings.lock().pack.clone();
        if *pack_id != current_pack {
            load_pack_cached(&app, pack_id).await?;
        }
    }
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
    tray::refresh(&app);
    Ok(infos)
}

#[tauri::command]
pub async fn preview_pack(app: AppHandle, id: String) -> Result<(), CmdError> {
    let loaded = load_pack_cached(&app, &id).await?;
    app.state::<AppState>()
        .engine
        .set_pack(PackSlot::Preview, loaded);
    packs::schedule_preview(app);
    Ok(())
}

/// JS key: `srcDir`.
#[tauri::command]
pub async fn import_pack(app: AppHandle, src_dir: String) -> Result<PackInfo, CmdError> {
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

    let app2 = app.clone();
    // `import_dir` now reports a duplicate id as `PackErrorKind::Exists`, which
    // `CmdError::from` maps to `ErrorCode::Exists`.
    let info: Result<PackInfo, PackError> = tauri::async_runtime::spawn_blocking(move || {
        let state = app2.state::<AppState>();
        let mut registry = state.registry.lock();
        registry.import_dir(&src)
    })
    .await
    .map_err(CmdError::from)?;
    let info = info.map_err(CmdError::from)?;

    let infos = app.state::<AppState>().registry.lock().list().to_vec();
    let _ = app.emit(events::PACKS_CHANGED, &infos);
    tray::refresh(&app);
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
    app.state::<AppState>().persister.flush();
    app.restart()
}
