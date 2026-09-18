//! Procedural synth packs (SPEC §6). Always available: zero-asset operation.
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

pub mod dsp;
mod presets;

use crate::pack::{LoadedPack, PackInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynthPreset {
    Thock,
    Click,
}

impl SynthPreset {
    pub const ALL: [SynthPreset; 2] = [Self::Thock, Self::Click];

    /// `"synth/thock"` | `"synth/click"`
    pub fn id(self) -> &'static str {
        match self {
            Self::Thock => "synth/thock",
            Self::Click => "synth/click",
        }
    }

    /// license "CC0-1.0", author "Clatterbox contributors".
    pub fn info(self) -> PackInfo {
        todo!("WP2")
    }
}

/// Deterministic generation at `rate` Hz (budget < 10 ms total in release).
pub fn generate(preset: SynthPreset, rate: u32) -> LoadedPack {
    todo!("WP2")
}
