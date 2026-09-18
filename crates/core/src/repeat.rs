//! Uniform auto-repeat suppression (SPEC §3.6).
//! PRIVACY: no logging, no Debug derives in this file (enforced by scripts/check-privacy.sh).
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

use crate::keymap::PhysKey;
use crate::types::KeyDir;

/// A held key re-pressed within this window is an OS auto-repeat.
pub const REPEAT_WINDOW_MS: u64 = 1200;

pub struct RepeatFilter {
    /// Bitset over `PhysKey as u8`.
    held: [u64; 2],
    last_ms: [u64; 128],
}

impl RepeatFilter {
    pub fn new() -> Self {
        Self {
            held: [0; 2],
            last_ms: [0; 128],
        }
    }

    /// `true` = fresh event, play it; `false` = repeat / orphan up, drop it.
    pub fn accept(&mut self, key: PhysKey, dir: KeyDir, now_ms: u64) -> bool {
        todo!("WP3")
    }

    /// Clears all state (called when a backend restarts).
    pub fn reset(&mut self) {
        todo!("WP3")
    }
}

impl Default for RepeatFilter {
    fn default() -> Self {
        Self::new()
    }
}
