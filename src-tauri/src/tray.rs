//! Tray icon + menu (SPEC §8.5).

use clatterbox_core::{HookStatus, PackInfo};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::commands;
use crate::error::CmdError;
use crate::state::AppState;
use crate::window;

pub const TRAY_ID: &str = "main";

const ICON_ON: &[u8] = include_bytes!("../icons/tray.png");
const ICON_OFF: &[u8] = include_bytes!("../icons/tray-off.png");

const ID_TOGGLE_ENABLED: &str = "toggle_enabled";
const ID_TOGGLE_KEYUP: &str = "toggle_keyup";
const ID_PERMISSION: &str = "permission";
const ID_SETTINGS: &str = "settings";
const ID_QUIT: &str = "quit";
const PACK_ID_PREFIX: &str = "pack:";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let enabled = app.state::<AppState>().settings.lock().enabled;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_icon(enabled)?)
        .icon_as_template(true)
        .tooltip(tray_tooltip(enabled))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(on_menu_event)
        .build(app)?;
    Ok(())
}

/// Rebuilds the menu and refreshes the tooltip/icon; called after `settings-changed` /
/// `packs-changed` (rebuilding is cheap and avoids stale checkmark/handle state).
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let enabled = app.state::<AppState>().settings.lock().enabled;
    if let Ok(menu) = build_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
    let _ = tray.set_tooltip(Some(tray_tooltip(enabled)));
    if let Ok(icon) = tray_icon(enabled) {
        let _ = tray.set_icon(Some(icon));
    }
}

fn tray_tooltip(enabled: bool) -> &'static str {
    if enabled {
        "Clatterbox — on"
    } else {
        "Clatterbox — off"
    }
}

fn tray_icon(enabled: bool) -> tauri::Result<Image<'static>> {
    Image::from_bytes(if enabled { ICON_ON } else { ICON_OFF })
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().clone();
    let hook_status = state
        .hook
        .lock()
        .as_ref()
        .map(|h| h.status())
        .unwrap_or(HookStatus::Starting);
    let packs: Vec<PackInfo> = state
        .registry
        .lock()
        .list()
        .iter()
        .filter(|p| p.valid)
        .cloned()
        .collect();

    let toggle_enabled = CheckMenuItem::with_id(
        app,
        ID_TOGGLE_ENABLED,
        "Sound enabled",
        true,
        settings.enabled,
        None::<&str>,
    )?;

    let pack_items: Vec<CheckMenuItem<tauri::Wry>> = packs
        .iter()
        .map(|p| {
            CheckMenuItem::with_id(
                app,
                format!("{PACK_ID_PREFIX}{}", p.id),
                &p.name,
                true,
                p.id == settings.pack,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<_>>()?;
    let pack_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = pack_items
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect();
    let switch = Submenu::with_items(app, "Switch", true, &pack_refs)?;

    let toggle_keyup = CheckMenuItem::with_id(
        app,
        ID_TOGGLE_KEYUP,
        "Key-up sounds",
        true,
        settings.key_up_enabled,
        None::<&str>,
    )?;

    let settings_item = MenuItem::with_id(app, ID_SETTINGS, "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, ID_QUIT, "Quit Clatterbox", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let needs_permission = matches!(
        hook_status,
        HookStatus::NeedsPermission { .. } | HookStatus::NeedsRestart
    );

    let mut items: Vec<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>> = vec![
        Box::new(toggle_enabled),
        Box::new(switch),
        Box::new(toggle_keyup),
        Box::new(separator),
    ];
    if needs_permission {
        let label = if matches!(hook_status, HookStatus::NeedsRestart) {
            "Relaunch Clatterbox…"
        } else {
            "Grant keyboard access…"
        };
        items.push(Box::new(MenuItem::with_id(
            app,
            ID_PERMISSION,
            label,
            true,
            None::<&str>,
        )?));
    }
    items.push(Box::new(settings_item));
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(quit_item));

    let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        items.iter().map(|i| i.as_ref()).collect();
    Menu::with_items(app, &refs)
}

fn on_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();

    if let Some(pack_id) = id.strip_prefix(PACK_ID_PREFIX) {
        let patch = clatterbox_core::SettingsPatch {
            pack: Some(pack_id.to_owned()),
            ..Default::default()
        };
        report(commands::apply_patch(app, patch));
        return;
    }

    match id {
        ID_TOGGLE_ENABLED => {
            let current = app.state::<AppState>().settings.lock().enabled;
            let patch = clatterbox_core::SettingsPatch {
                enabled: Some(!current),
                ..Default::default()
            };
            report(commands::apply_patch(app, patch));
        }
        ID_TOGGLE_KEYUP => {
            let current = app.state::<AppState>().settings.lock().key_up_enabled;
            let patch = clatterbox_core::SettingsPatch {
                key_up_enabled: Some(!current),
                ..Default::default()
            };
            report(commands::apply_patch(app, patch));
        }
        ID_PERMISSION => {
            let needs_restart = matches!(
                app.state::<AppState>()
                    .hook
                    .lock()
                    .as_ref()
                    .map(|h| h.status()),
                Some(HookStatus::NeedsRestart)
            );
            if needs_restart {
                app.state::<AppState>().persister.flush();
                app.restart();
            } else {
                let _ = commands::request_input_permission(app.clone());
            }
        }
        ID_SETTINGS => {
            let _ = window::open(app);
        }
        ID_QUIT => {
            let state = app.state::<AppState>();
            state.persister.flush();
            app.exit(0);
        }
        _ => {}
    }
}

fn report(result: Result<clatterbox_core::Settings, CmdError>) {
    if let Err(e) = result {
        tracing::warn!("tray settings update failed: {e}");
    }
}
