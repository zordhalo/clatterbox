//! `pack.toml` schema + path validation (SPEC §5.2, §5.3).
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub name: String,
    pub author: String,
    pub license: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub gain_db: Option<f32>,
    pub sounds: Sounds,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sounds {
    pub default: SoundFiles,
    #[serde(default)]
    pub space: Option<SoundFiles>,
    #[serde(default)]
    pub enter: Option<SoundFiles>,
    #[serde(default)]
    pub backspace: Option<SoundFiles>,
    #[serde(default)]
    pub modifier: Option<SoundFiles>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundFiles {
    #[serde(default)]
    pub down: Vec<String>,
    #[serde(default)]
    pub up: Vec<String>,
    /// Optional per-class trim, clamped to `-24.0..=12.0` (loudness-match mixed sources).
    #[serde(default)]
    pub gain_db: Option<f32>,
}

/// Parse + validate field rules (lengths, schema, source_url scheme, gain clamp).
pub fn parse(text: &str) -> Result<Manifest, String> {
    todo!("WP2")
}

/// Resolve a manifest-relative file path; must stay under the canonicalized `pack_dir`.
pub fn resolve_file(pack_dir: &Path, rel: &str) -> Result<PathBuf, String> {
    todo!("WP2")
}
