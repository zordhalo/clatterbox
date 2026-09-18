//! X11 XI2 RawKeyPress/RawKeyRelease on the root window (SPEC §3.4).

use std::sync::Arc;

use clatterbox_core::{HookStatus, KeyDir};
use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xinput::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{
    AtomEnum, ClientMessageEvent, ConnectionExt as _, CreateWindowAux, EventMask, WindowClass,
};
use x11rb::rust_connection::RustConnection;

use super::keycode;
use crate::dispatch::Dispatcher;
use crate::shared::Shared;

pub(crate) const BACKEND: &str = "x11_xi2";

/// X keycode → evdev code offset.
pub(crate) const X_KEYCODE_OFFSET: u32 = 8;

/// Sets up XI2 raw key events and runs until stopped. `Err` = setup failed before the backend
/// reported `Running` (caller falls back to evdev).
pub(crate) fn run(d: &mut Dispatcher, shared: &Shared) -> Result<(), String> {
    let (conn, screen) = x11rb::connect(None).map_err(|e| format!("X11 connect: {e}"))?;
    let conn = Arc::new(conn);
    let root = conn.setup().roots[screen].root;
    let wake_win = setup(&conn, root).map_err(|e| format!("XInput2 setup: {e}"))?;

    let waker_conn = conn.clone();
    let armed = shared.arm(Box::new(move || {
        // An event sent with an empty mask goes to the window's creator: us.
        let ev = ClientMessageEvent::new(32, wake_win, AtomEnum::NONE, [0u32; 5]);
        let _ = waker_conn.send_event(false, wake_win, EventMask::NO_EVENT, ev);
        let _ = waker_conn.flush();
    }));
    if !armed {
        return Ok(());
    }
    shared.set_status(HookStatus::Running {
        backend: BACKEND.into(),
    });
    loop {
        match conn.wait_for_event() {
            Ok(Event::XinputRawKeyPress(e)) => on_key(d, e.detail, KeyDir::Down),
            Ok(Event::XinputRawKeyRelease(e)) => on_key(d, e.detail, KeyDir::Up),
            Ok(Event::ClientMessage(_)) if shared.stop_requested() => break,
            Ok(_) => {}
            Err(e) => {
                shared.failed(format!("X11 connection lost: {e}"));
                break;
            }
        }
    }
    shared.disarm();
    Ok(())
}

fn setup(conn: &RustConnection, root: u32) -> Result<u32, Box<dyn std::error::Error>> {
    let ver = conn.xinput_xi_query_version(2, 0)?.reply()?;
    if ver.major_version < 2 {
        return Err("XInput 2.0 not supported".into());
    }
    conn.xinput_xi_select_events(
        root,
        &[xinput::EventMask {
            deviceid: xinput::Device::ALL_MASTER.into(),
            mask: vec![xinput::XIEventMask::RAW_KEY_PRESS | xinput::XIEventMask::RAW_KEY_RELEASE],
        }],
    )?
    .check()?;
    // Unmapped input-only window used only as the target of the stop wake-up.
    let win = conn.generate_id()?;
    conn.create_window(
        0,
        win,
        root,
        0,
        0,
        1,
        1,
        0,
        WindowClass::INPUT_ONLY,
        0,
        &CreateWindowAux::new(),
    )?
    .check()?;
    Ok(win)
}

#[inline]
fn on_key(d: &mut Dispatcher, detail: u32, dir: KeyDir) {
    let Some(code) = detail.checked_sub(X_KEYCODE_OFFSET) else {
        return;
    };
    let key = keycode::to_phys(u16::try_from(code).unwrap_or(0));
    let now = d.now_ms();
    d.dispatch(key, dir, now);
}
