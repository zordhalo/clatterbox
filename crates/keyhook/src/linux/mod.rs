//! Linux backends: X11 XInput2 raw events, evdev fallback for Wayland (SPEC §3.4).
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

pub(crate) mod evdev;
pub(crate) mod keycode;
pub(crate) mod x11;
