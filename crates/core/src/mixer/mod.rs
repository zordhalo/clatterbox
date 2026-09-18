//! Real-time voice mixer (SPEC §4.4). Pure function of its inputs; no I/O, never allocates in
//! `render`.
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

pub mod math;
mod voice;

use std::sync::Arc;

use crate::pack::LoadedPack;
use crate::params::EngineParams;
use crate::types::{PackSlot, Trigger};
use voice::Voice;

pub const MAX_VOICES: usize = 32;

pub enum MixerCmd {
    SetPack {
        slot: PackSlot,
        pack: Arc<LoadedPack>,
    },
}

pub struct Mixer {
    voices: [Voice; MAX_VOICES],
    /// Indexed by `PackSlot as usize`.
    packs: [Option<Arc<LoadedPack>>; 2],
    rng: fastrand::Rng,
    out_rate: u32,
    params: Arc<EngineParams>,
}

impl Mixer {
    pub fn new(params: Arc<EngineParams>, out_rate: u32, seed: u64) -> Self {
        todo!("WP2")
    }

    pub fn set_output_rate(&mut self, out_rate: u32) {
        todo!("WP2")
    }

    /// Returns the retired pack (caller routes it to the garbage queue).
    pub fn apply(&mut self, cmd: MixerCmd) -> Option<Arc<LoadedPack>> {
        todo!("WP2")
    }

    pub fn trigger(&mut self, t: Trigger) {
        todo!("WP2")
    }

    /// Interleaved output, overwrites `out`. Never allocates.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        todo!("WP2")
    }

    pub fn active_voices(&self) -> usize {
        todo!("WP2")
    }
}
