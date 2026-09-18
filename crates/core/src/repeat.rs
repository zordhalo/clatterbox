//! Uniform auto-repeat suppression (SPEC §3.6).
//! PRIVACY: no logging, no Debug derives in this file (enforced by scripts/check-privacy.sh).

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
        let k = key as usize;
        let (word, bit) = (k / 64, 1u64 << (k % 64));
        let held = self.held[word] & bit != 0;
        match dir {
            KeyDir::Down => {
                let fresh = !held || now_ms.saturating_sub(self.last_ms[k]) > REPEAT_WINDOW_MS;
                self.held[word] |= bit;
                self.last_ms[k] = now_ms;
                fresh
            }
            KeyDir::Up => {
                self.held[word] &= !bit;
                held
            }
        }
    }

    /// Clears all state (called when a backend restarts).
    pub fn reset(&mut self) {
        self.held = [0; 2];
        self.last_ms = [0; 128];
    }
}

impl Default for RepeatFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use KeyDir::{Down, Up};

    #[test]
    fn repeats_are_rejected_until_up() {
        let mut f = RepeatFilter::new();
        assert!(f.accept(PhysKey::A, Down, 1000));
        assert!(!f.accept(PhysKey::A, Down, 1100));
        assert!(!f.accept(PhysKey::A, Down, 1200));
        assert!(f.accept(PhysKey::A, Up, 1250));
        assert!(f.accept(PhysKey::A, Down, 1300));
    }

    #[test]
    fn long_repeat_chain_stays_rejected() {
        let mut f = RepeatFilter::new();
        assert!(f.accept(PhysKey::Space, Down, 0));
        // Initial delay up to 1000 ms, then ~30 ms repeats for 5 s.
        let mut t = 1000;
        while t < 6000 {
            assert!(!f.accept(PhysKey::Space, Down, t));
            t += 30;
        }
    }

    #[test]
    fn stale_hold_is_accepted_again() {
        let mut f = RepeatFilter::new();
        assert!(f.accept(PhysKey::A, Down, 0));
        assert!(!f.accept(PhysKey::A, Down, REPEAT_WINDOW_MS));
        // Missed key-up: next down after > window counts as fresh.
        assert!(f.accept(PhysKey::A, Down, 2 * REPEAT_WINDOW_MS + 1));
    }

    #[test]
    fn orphan_up_is_rejected() {
        let mut f = RepeatFilter::new();
        assert!(!f.accept(PhysKey::Enter, Up, 10));
        assert!(f.accept(PhysKey::Enter, Down, 20));
        assert!(f.accept(PhysKey::Enter, Up, 30));
        assert!(!f.accept(PhysKey::Enter, Up, 40));
    }

    #[test]
    fn keys_are_independent() {
        let mut f = RepeatFilter::new();
        assert!(f.accept(PhysKey::A, Down, 0));
        assert!(f.accept(PhysKey::S, Down, 10));
        assert!(!f.accept(PhysKey::A, Down, 20));
        assert!(f.accept(PhysKey::S, Up, 30));
        assert!(!f.accept(PhysKey::A, Down, 40));
        assert!(f.accept(PhysKey::A, Up, 50));
        // High discriminants use the second bitset word.
        assert!(f.accept(PhysKey::Unknown, Down, 60));
        assert!(!f.accept(PhysKey::Unknown, Down, 70));
        assert!(f.accept(PhysKey::Escape, Down, 80));
    }

    #[test]
    fn reset_clears_held_keys() {
        let mut f = RepeatFilter::new();
        assert!(f.accept(PhysKey::A, Down, 0));
        f.reset();
        assert!(!f.accept(PhysKey::A, Up, 10));
        assert!(f.accept(PhysKey::A, Down, 20));
    }
}
