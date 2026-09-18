//! X11 XI2 RawKeyPress/RawKeyRelease on the root window (SPEC §3.4).
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

pub(crate) const BACKEND: &str = "x11_xi2";

/// X keycode → evdev code offset.
pub(crate) const X_KEYCODE_OFFSET: u32 = 8;
