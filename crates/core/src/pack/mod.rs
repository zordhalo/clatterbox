//! Sound packs: info, decoded samples, errors (SPEC §5).
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

pub mod decode;
pub mod derive;
pub mod manifest;
pub mod registry;

use serde::Serialize;

use crate::types::{KeyClass, KeyDir};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackKind {
    Synth,
    Builtin,
    User,
}

#[derive(Clone, Debug, Serialize)]
pub struct PackInfo {
    /// `synth/<preset>` | `builtin/<dir>` | `user/<dir>`
    pub id: String,
    pub name: String,
    pub author: String,
    pub license: String,
    pub source_url: Option<String>,
    pub description: Option<String>,
    pub kind: PackKind,
    pub valid: bool,
    pub error: Option<String>,
    /// Derived sets (SPEC §5.2.1), e.g. `["space.down", "default.up"]`; empty for synth.
    pub derived: Vec<String>,
}

/// Mono `f32` sample at its native rate.
pub struct Sample {
    pub data: Box<[f32]>,
    pub rate: u32,
}

pub struct SampleSet {
    pub down: Vec<Sample>,
    pub up: Vec<Sample>,
}

pub struct LoadedPack {
    pub info: PackInfo,
    /// Linear gain from `gain_db`.
    pub gain: f32,
    /// Indexed by `KeyClass as usize`. Fully resolved at load: missing sets are derived into
    /// real buffers (SPEC §5.2.1), so no fallback lookup happens in the callback.
    sets: [SampleSet; KeyClass::COUNT],
}

impl LoadedPack {
    /// Resolved (explicit or derived) set; may be empty only for `Up`.
    pub fn samples(&self, class: KeyClass, dir: KeyDir) -> &[Sample] {
        todo!("WP2")
    }
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("pack {pack_id}: {message}")]
pub struct PackError {
    pub pack_id: String,
    pub message: String,
}
