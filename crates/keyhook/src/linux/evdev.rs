//! evdev `/dev/input/event*` reader, poll(2) over non-blocking fds, never grabs (SPEC §3.4).
//! Note: this module shadows the `evdev` crate inside `linux::`; use `::evdev::…` for the crate.
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

pub(crate) const BACKEND: &str = "evdev";

/// Hot-plug rescan interval / poll timeout.
pub(crate) const RESCAN_MS: i32 = 3000;
