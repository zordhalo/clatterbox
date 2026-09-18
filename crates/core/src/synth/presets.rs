//! Preset parameter tables and class modifiers (SPEC §6.3).
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

use super::SynthPreset;

pub(crate) struct PresetParams {
    pub(crate) click_hz: f32,
    pub(crate) modes_hz: [f32; 3],
    pub(crate) modes_q: [f32; 3],
    pub(crate) thump_hz: f32,
    pub(crate) g_click: f32,
    pub(crate) g_thump: f32,
}

pub(crate) fn params(preset: SynthPreset) -> PresetParams {
    todo!("WP2")
}
