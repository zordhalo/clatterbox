//! Load-time resolution of missing sample sets into real buffers (SPEC §5.2.1).
//! Explicit sets always win and are never mixed with derived ones.

use super::decode::fade_out_linear;
use super::{Sample, SampleSet};
use crate::mixer::math::{cents_to_ratio, db_to_gain, hermite};
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
    let ratio = f64::from(cents_to_ratio(cents));
    let full_len = (src.data.len() as f64 / ratio).floor() as usize;
    let len = match keep_fraction {
        Some(k) => ((full_len as f32 * k.clamp(0.0, 1.0)).round() as usize).max(1),
        None => full_len.max(1),
    };
    let gain = db_to_gain(gain_db);
    let mut data: Vec<f32> = (0..len)
        .map(|i| hermite(&src.data, i as f64 * ratio) * gain)
        .collect();
    if keep_fraction.is_some() {
        let n = (fade_ms * 0.001 * src.rate as f32).round() as usize;
        fade_out_linear(&mut data, n);
    }
    Sample {
        data: data.into_boxed_slice(),
        rate: src.rate,
    }
}

fn derive_set(src: &[Sample], cents: f32, gain_db: f32, up: bool) -> Vec<Sample> {
    let keep = up.then_some(UP_KEEP_FRACTION);
    src.iter()
        .take(MAX_SOURCE_VARIATIONS)
        .map(|s| derive_sample(s, cents, gain_db, keep, UP_FADE_MS))
        .collect()
}

fn apply_gain(set: &mut [Sample], gain_db: f32) {
    if gain_db != 0.0 {
        let g = db_to_gain(gain_db);
        for s in set {
            s.data.iter_mut().for_each(|x| *x *= g);
        }
    }
}

/// Fills every empty set in `sets` (indexed by `KeyClass as usize`) per the resolution table.
/// `class_gain_db` is each class's manifest `gain_db` (0.0 if absent); it is baked into the
/// explicit buffers here, so a derived set's effective gain = source class gain + offset gain.
/// Returns the derived set names for `PackInfo::derived`, e.g. `["space.down", "default.up"]`.
pub fn resolve(
    sets: &mut [SampleSet; KeyClass::COUNT],
    class_gain_db: &[f32; KeyClass::COUNT],
) -> Vec<String> {
    for c in KeyClass::ALL {
        let set = &mut sets[c as usize];
        apply_gain(&mut set.down, class_gain_db[c as usize]);
        apply_gain(&mut set.up, class_gain_db[c as usize]);
    }
    let mut derived = Vec::new();
    let (default, rest) = sets.split_first_mut().expect("5 classes");
    for c in &KeyClass::ALL[1..] {
        let set = &mut rest[*c as usize - 1];
        if set.down.is_empty() {
            let (cents, gain) = class_offset(*c);
            set.down = derive_set(&default.down, cents, gain, false);
            derived.push(format!("{}.down", c.as_str()));
        }
    }
    for c in KeyClass::ALL {
        let set = &mut sets[c as usize];
        if set.up.is_empty() && !set.down.is_empty() {
            set.up = derive_set(&set.down, UP_CENTS, UP_GAIN_DB, true);
            derived.push(format!("{}.up", c.as_str()));
        }
    }
    derived
}
