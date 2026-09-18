//! A single playing sample (SPEC §4.4). References samples by index into the pack slot; no Arc
//! clones in the callback.

use super::math::hermite;
use crate::types::{KeyClass, KeyDir, PackSlot};

/// Length of the linear fade-out used on pack swap.
pub(crate) const FADE_LEN: u16 = 64;

#[derive(Clone, Copy)]
pub(crate) struct Voice {
    pub(crate) active: bool,
    pub(crate) slot: PackSlot,
    /// `true` while playing from the slot's retiring pack (after a swap, during the fade).
    pub(crate) retiring: bool,
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

impl Voice {
    pub(crate) const IDLE: Voice = Voice {
        active: false,
        slot: PackSlot::Main,
        retiring: false,
        class: KeyClass::Default,
        dir: KeyDir::Down,
        var: 0,
        pos: 0.0,
        step: 1.0,
        gain_l: 0.0,
        gain_r: 0.0,
        fade: 0,
    };

    /// Playback progress in `0.0..` (≥ 1.0 = finished); used for voice stealing.
    pub(crate) fn progress(&self, len: usize) -> f64 {
        if len == 0 {
            f64::INFINITY
        } else {
            self.pos / len as f64
        }
    }

    /// Adds this voice into interleaved `out` (`channels` ≥ 1) scaled by `master`. Deactivates the
    /// voice when the sample or its fade ends. Never allocates.
    #[inline]
    pub(crate) fn render(&mut self, data: &[f32], out: &mut [f32], channels: usize, master: f32) {
        let len = data.len() as f64;
        let gl = self.gain_l * master;
        let gr = self.gain_r * master;
        for frame in out.chunks_exact_mut(channels) {
            if self.pos >= len {
                self.active = false;
                return;
            }
            let fading = self.fade > 0;
            let mut s = hermite(data, self.pos);
            if fading {
                s *= f32::from(self.fade) / f32::from(FADE_LEN + 1);
            }
            if channels == 1 {
                frame[0] += s * (gl + gr) * 0.5;
            } else {
                frame[0] += s * gl;
                frame[1] += s * gr;
            }
            self.pos += self.step;
            if fading {
                self.fade -= 1;
                if self.fade == 0 {
                    self.active = false;
                    return;
                }
            }
        }
    }
}
