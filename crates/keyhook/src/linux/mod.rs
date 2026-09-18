//! Linux backends: X11 XInput2 raw events, evdev fallback for Wayland (SPEC §3.4).

pub(crate) mod evdev;
pub(crate) mod keycode;
pub(crate) mod x11;

use crate::dispatch::Dispatcher;
use crate::shared::Shared;

pub(crate) fn run(mut d: Dispatcher, shared: &Shared) {
    // X11 setup failure (no server, no XInput 2) falls through to evdev.
    if prefer_x11() && x11::run(&mut d, shared).is_ok() {
        return;
    }
    evdev::run(&mut d, shared);
}

fn prefer_x11() -> bool {
    let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|t| t == "x11")
        || (set("DISPLAY") && !set("WAYLAND_DISPLAY"))
}
