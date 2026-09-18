//! macOS backend: listen-only CGEventTap on a CFRunLoop thread (SPEC §3.3).
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

pub(crate) mod keycode;
pub(crate) mod permission;

pub(crate) const BACKEND: &str = "event_tap";
