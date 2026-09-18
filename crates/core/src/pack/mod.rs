//! Sound packs: info, decoded samples, errors (SPEC §5).

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

#[derive(Default)]
pub struct SampleSet {
    pub down: Vec<Sample>,
    pub up: Vec<Sample>,
}

impl SampleSet {
    pub fn get(&self, dir: KeyDir) -> &[Sample] {
        match dir {
            KeyDir::Down => &self.down,
            KeyDir::Up => &self.up,
        }
    }
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
    /// Builds a pack from already-resolved sets (indexed by `KeyClass as usize`).
    pub fn from_sets(info: PackInfo, gain: f32, sets: [SampleSet; KeyClass::COUNT]) -> Self {
        Self { info, gain, sets }
    }

    /// Resolved (explicit or derived) set; may be empty only for `Up`.
    pub fn samples(&self, class: KeyClass, dir: KeyDir) -> &[Sample] {
        self.sets[class as usize].get(dir)
    }

    /// Total decoded sample memory in bytes.
    pub fn f32_bytes(&self) -> usize {
        let set_bytes = |v: &[Sample]| v.iter().map(|s| s.data.len() * 4).sum::<usize>();
        self.sets
            .iter()
            .map(|s| set_bytes(&s.down) + set_bytes(&s.up))
            .sum()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackErrorKind {
    /// Manifest, file or audio validation failed.
    Invalid,
    /// Unknown pack id or missing pack directory.
    NotFound,
    /// Import target already exists.
    Exists,
    /// Filesystem error.
    Io,
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("pack {pack_id}: {message}")]
pub struct PackError {
    pub pack_id: String,
    pub message: String,
    pub kind: PackErrorKind,
}

impl PackError {
    pub fn new(
        kind: PackErrorKind,
        pack_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            pack_id: pack_id.into(),
            message: message.into(),
            kind,
        }
    }

    pub(crate) fn invalid(pack_id: &str, message: impl Into<String>) -> Self {
        Self::new(PackErrorKind::Invalid, pack_id, message)
    }
}
