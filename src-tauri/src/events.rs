//! Rust → UI event names (SPEC §8.4). No other events; never one per keystroke.
// WP0 stub: remove this allow once wired (WP1).
#![allow(dead_code)]

/// Payload: `Settings`.
pub const SETTINGS_CHANGED: &str = "settings-changed";
/// Payload: `PackInfo[]`.
pub const PACKS_CHANGED: &str = "packs-changed";
/// Payload: `Status`.
pub const STATUS_CHANGED: &str = "status-changed";
