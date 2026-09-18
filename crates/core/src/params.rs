//! Lock-free engine parameters shared by the main thread, keyhook and audio callback (SPEC §4.5).
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

use std::sync::atomic::{AtomicBool, AtomicU32};

use crate::settings::Settings;

pub struct EngineParams {
    pub enabled: AtomicBool,
    pub key_up: AtomicBool,
    pub spatial: AtomicBool,
    // f32 bits
    volume: AtomicU32,
    pitch_variation: AtomicU32,
    spatial_width: AtomicU32,
}

impl EngineParams {
    pub fn from_settings(s: &Settings) -> Self {
        todo!("WP2")
    }

    /// Relaxed stores.
    pub fn apply(&self, s: &Settings) {
        todo!("WP2")
    }

    pub fn volume(&self) -> f32 {
        todo!("WP2")
    }

    pub fn pitch_variation(&self) -> f32 {
        todo!("WP2")
    }

    pub fn spatial_width(&self) -> f32 {
        todo!("WP2")
    }
}
