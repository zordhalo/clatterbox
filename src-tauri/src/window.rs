//! Settings window: created on demand, destroyed on close (SPEC §8.6).

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "settings";

/// Creates the window, or `show()+set_focus()` if it already exists.
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    // Not the main/last window and no `prevent_close` handler is attached, so Tauri's default
    // close behavior already destroys the webview (SPEC §8.6, §2.3 RSS budget) — nothing else
    // to wire up here.
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Clatterbox Settings")
        .inner_size(440.0, 620.0)
        .resizable(false)
        .maximizable(false)
        .center()
        .build()?;

    Ok(())
}
