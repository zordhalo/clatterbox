//! Load-time resolution of missing sample sets into real buffers (SPEC §5.2.1).
//! Explicit sets always win and are never mixed with derived ones.
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

use super::{Sample, SampleSet};
use crate::types::KeyClass;

/// Derive from at most this many source variations.
pub const MAX_SOURCE_VARIATIONS: usize = 16;

/// Derived `up` from a down set: pitch, gain, kept fraction of the resampled length, fade.
pub const UP_CENTS: f32 = -300.0;
pub const UP_GAIN_DB: f32 = -9.0;
pub const UP_KEEP_FRACTION: f32 = 0.6;
pub const UP_FADE_MS: f32 = 5.0;

/// Offset `(cents, gain_db)` for a derived `down` of `class` from `default.down`.
/// `Default` → `(0.0, 0.0)` (never derived).
pub fn class_offset(class: KeyClass) -> (f32, f32) {
    match class {
        KeyClass::Default => (0.0, 0.0),
        KeyClass::Space => (-200.0, 1.0),
        KeyClass::Enter => (-150.0, 1.0),
        KeyClass::Backspace => (-50.0, 0.0),
        KeyClass::Modifier => (100.0, -2.0),
    }
}

/// Offline resample of `src` by `cents` (4-point Hermite, output length = `len / ratio`), then
/// `gain_db`, then optional truncation to `keep_fraction` with a `fade_ms` linear fade-out.
pub fn derive_sample(
    src: &Sample,
    cents: f32,
    gain_db: f32,
    keep_fraction: Option<f32>,
    fade_ms: f32,
) -> Sample {
    todo!("WP2")
}

/// Fills every empty set in `sets` (indexed by `KeyClass as usize`) per the resolution table.
/// `class_gain_db` is each class's manifest `gain_db` (0.0 if absent); a derived set's effective
/// gain = source class gain + offset gain. Returns the derived set names for
/// `PackInfo::derived`, e.g. `["space.down", "default.up"]`.
pub fn resolve(
    sets: &mut [SampleSet; KeyClass::COUNT],
    class_gain_db: &[f32; KeyClass::COUNT],
) -> Vec<String> {
    todo!("WP2")
}
