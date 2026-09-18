//! Preset parameter tables and class modifiers (SPEC §6.3).

use super::SynthPreset;
use crate::types::KeyClass;

pub(crate) struct PresetParams {
    pub(crate) click_hz: f32,
    pub(crate) click_q: f32,
    /// Second click impulse `(delay_ms, level_db)` (tactile click leaf).
    pub(crate) second_click: Option<(f32, f32)>,
    pub(crate) modes_hz: [f32; 3],
    pub(crate) modes_q: [f32; 3],
    /// Relative weight of each mode in the body.
    pub(crate) modes_g: [f32; 3],
    pub(crate) thump_hz: f32,
    pub(crate) g_click: f32,
    pub(crate) g_thump: f32,
    /// Excitation envelope time constant (SPEC: 0.6..1.2 ms).
    pub(crate) excite_tau_ms: f32,
    /// Thump envelope time constant (SPEC: 8..14 ms).
    pub(crate) thump_tau_ms: f32,
    /// Base down length (SPEC: 90..160 ms).
    pub(crate) length_ms: f32,
}

pub(crate) fn params(preset: SynthPreset) -> PresetParams {
    match preset {
        SynthPreset::Thock => PresetParams {
            click_hz: 2400.0,
            click_q: 0.8,
            second_click: None,
            modes_hz: [210.0, 620.0, 1350.0],
            modes_q: [14.0, 18.0, 22.0],
            modes_g: [1.0, 0.6, 0.35],
            thump_hz: 95.0,
            g_click: 0.5,
            g_thump: 1.0,
            excite_tau_ms: 1.1,
            thump_tau_ms: 14.0,
            length_ms: 130.0,
        },
        SynthPreset::Click => PresetParams {
            click_hz: 4800.0,
            click_q: 3.0,
            second_click: Some((4.0, -4.0)),
            modes_hz: [380.0, 1100.0, 2900.0],
            modes_q: [18.0, 22.0, 25.0],
            modes_g: [1.0, 0.6, 0.35],
            thump_hz: 140.0,
            g_click: 1.0,
            g_thump: 0.5,
            excite_tau_ms: 0.6,
            thump_tau_ms: 8.0,
            length_ms: 110.0,
        },
    }
}

/// Per-class modifiers: mode/thump frequency factor, decay factor, level dB.
pub(crate) struct ClassMod {
    pub(crate) freq: f32,
    pub(crate) decay: f32,
    pub(crate) level_db: f32,
    /// Stabilizer rattle (space bar only).
    pub(crate) rattle: bool,
}

pub(crate) fn class_mod(class: KeyClass) -> ClassMod {
    let (freq, decay, level_db) = match class {
        KeyClass::Default => (1.0, 1.0, 0.0),
        KeyClass::Modifier => (0.95, 1.05, -1.0),
        KeyClass::Backspace => (0.88, 1.15, 0.5),
        KeyClass::Enter => (0.85, 1.2, 1.0),
        KeyClass::Space => (0.7, 1.4, 1.5),
    };
    ClassMod {
        freq,
        decay,
        level_db,
        rattle: class == KeyClass::Space,
    }
}

/// Highest class level; normalization leaves this much headroom so every class peaks at or
/// below -3 dBFS.
pub(crate) const MAX_CLASS_LEVEL_DB: f32 = 1.5;
/// Peak level of the loudest class.
pub(crate) const PEAK_DBFS: f32 = -3.0;

/// Up-stroke modifiers.
pub(crate) const UP_CLICK_GAIN: f32 = 0.6;
pub(crate) const UP_MODE_FREQ: f32 = 1.15;
pub(crate) const UP_LENGTH_MS: f32 = 60.0;
pub(crate) const UP_LEVEL_DB: f32 = -8.0;

/// Space-bar rattle: bandpassed noise burst.
pub(crate) const RATTLE_HZ: f32 = 3000.0;
pub(crate) const RATTLE_Q: f32 = 2.0;
pub(crate) const RATTLE_DELAY_MS: f32 = 8.0;
pub(crate) const RATTLE_LEVEL_DB: f32 = -14.0;

pub(crate) const VARIATIONS: usize = 5;
pub(crate) const SEED_BASE: u64 = 0xA11CE;
