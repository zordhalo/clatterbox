//! Tauri IPC commands (SPEC §8.3).
// WP0 stub: remove this allow once implemented (WP1).
#![allow(unused_variables, dead_code)]

use clatterbox_core::{AudioStatus, HookStatus, PackInfo, Settings, SettingsPatch};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::error::CmdError;
use crate::state::AppState;

#[derive(Serialize, Clone)]
pub struct Status {
    pub hook: HookStatus,
    pub audio: AudioStatus,
    /// "windows" | "macos" | "linux"
    pub platform: &'static str,
    pub version: &'static str,
}

/// Single code path for settings changes from IPC and the tray (SPEC §8.5).
pub(crate) fn apply_patch(app: &AppHandle, patch: SettingsPatch) -> Result<Settings, CmdError> {
    todo!("WP1")
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, CmdError> {
    todo!("WP1")
}

#[tauri::command]
pub fn update_settings(app: AppHandle, patch: SettingsPatch) -> Result<Settings, CmdError> {
    apply_patch(&app, patch)
}

#[tauri::command]
pub fn list_packs(state: State<'_, AppState>) -> Result<Vec<PackInfo>, CmdError> {
    todo!("WP1")
}

#[tauri::command]
pub fn reload_packs(app: AppHandle) -> Result<Vec<PackInfo>, CmdError> {
    todo!("WP1")
}

#[tauri::command]
pub fn preview_pack(app: AppHandle, id: String) -> Result<(), CmdError> {
    todo!("WP1")
}

/// JS key: `srcDir`.
#[tauri::command]
pub fn import_pack(app: AppHandle, src_dir: String) -> Result<PackInfo, CmdError> {
    todo!("WP1")
}

#[tauri::command]
pub fn open_packs_dir(app: AppHandle) -> Result<(), CmdError> {
    todo!("WP1")
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> Result<Status, CmdError> {
    todo!("WP1")
}

#[tauri::command]
pub fn request_input_permission(app: AppHandle) -> Result<HookStatus, CmdError> {
    todo!("WP1")
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart()
}
