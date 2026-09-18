//! A single playing sample (SPEC §4.4). References samples by index into the pack slot; no Arc
//! clones in the callback.
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

use crate::types::{KeyClass, KeyDir, PackSlot};

pub(crate) struct Voice {
    pub(crate) active: bool,
    pub(crate) slot: PackSlot,
    pub(crate) class: KeyClass,
    pub(crate) dir: KeyDir,
    /// Variation index into the pack's sample set.
    pub(crate) var: u8,
    /// Fractional read position in source samples.
    pub(crate) pos: f64,
    /// Source samples advanced per output frame (pitch × rate ratio).
    pub(crate) step: f64,
    pub(crate) gain_l: f32,
    pub(crate) gain_r: f32,
    /// Remaining fade-out samples (0 = not fading).
    pub(crate) fade: u16,
}
