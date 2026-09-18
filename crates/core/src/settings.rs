//! Persistent settings: JSON on disk and over IPC (SPEC §8.1). Schema is fixed in Phase 0;
//! logic (sanitize / merge / load / save) is WP1.
// WP0 stub: remove this allow once implemented (WP1).
#![allow(unused_variables, dead_code)]

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Missing fields → defaults; unknown fields ignored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Always `1` (future migrations switch on it).
    pub version: u32,
    pub enabled: bool,
    /// `0.0..=1.0`
    pub volume: f32,
    /// Pack id, e.g. `"builtin/classic"`, `"synth/thock"`.
    pub pack: String,
    pub key_up_enabled: bool,
    /// `0.0..=1.0`
    pub pitch_variation: f32,
    pub spatial_enabled: bool,
    /// `0.0..=1.0`
    pub spatial_width: f32,
    pub launch_at_login: bool,
}

impl Settings {
    pub const VERSION: u32 = 1;
    /// Default pack per SPEC §16.3. Falls back to [`Settings::FALLBACK_PACK`] if it fails to load.
    pub const DEFAULT_PACK: &'static str = "builtin/classic";
    pub const FALLBACK_PACK: &'static str = "synth/thock";

    /// Clamp ranges, NaN → default, version → 1.
    pub fn sanitize(&mut self) {
        todo!("WP1")
    }

    /// Applies only the `Some` fields of `patch`; returns a sanitized copy.
    pub fn merged(&self, patch: &SettingsPatch) -> Settings {
        todo!("WP1")
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            enabled: true,
            volume: 0.6,
            pack: Self::DEFAULT_PACK.to_owned(),
            key_up_enabled: true,
            pitch_variation: 0.35,
            spatial_enabled: true,
            spatial_width: 0.6,
            launch_at_login: false,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsPatch {
    pub enabled: Option<bool>,
    pub volume: Option<f32>,
    pub pack: Option<String>,
    pub key_up_enabled: Option<bool>,
    pub pitch_variation: Option<f32>,
    pub spatial_enabled: Option<bool>,
    pub spatial_width: Option<f32>,
    pub launch_at_login: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadOutcome {
    Loaded,
    FirstRun,
    Recovered,
}

/// Missing → default + `FirstRun`; parse error → rename to `settings.json.corrupt-<unix_ts>`,
/// default + `Recovered`.
pub fn load(path: &Path) -> (Settings, LoadOutcome) {
    todo!("WP1")
}

/// Write `settings.json.tmp`, fsync, rename.
pub fn save_atomic(path: &Path, s: &Settings) -> io::Result<()> {
    todo!("WP1")
}
