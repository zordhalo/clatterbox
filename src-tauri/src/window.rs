//! Settings window: created on demand, destroyed on close (SPEC §8.6).
// WP0 stub: remove this allow once implemented (WP1).
#![allow(unused_variables, dead_code)]

use tauri::AppHandle;

pub const LABEL: &str = "settings";

/// Creates the window, or `show()+set_focus()` if it already exists.
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    todo!("WP1")
}
