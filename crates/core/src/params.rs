//! Lock-free engine parameters shared by the main thread, keyhook and audio callback (SPEC §4.5).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::settings::Settings;

pub struct EngineParams {
    pub enabled: AtomicBool,
    pub key_up: AtomicBool,
    pub spatial: AtomicBool,
    // f32 bits
    volume: AtomicU32,
    pitch_variation: AtomicU32,
    spatial_width: AtomicU32,
}

/// Clamps to `0.0..=1.0`; NaN → `fallback`.
fn unit(v: f32, fallback: f32) -> f32 {
    if v.is_nan() {
        fallback
    } else {
        v.clamp(0.0, 1.0)
    }
}

impl EngineParams {
    pub fn from_settings(s: &Settings) -> Self {
        let p = Self {
            enabled: AtomicBool::new(true),
            key_up: AtomicBool::new(true),
            spatial: AtomicBool::new(true),
            volume: AtomicU32::new(0),
            pitch_variation: AtomicU32::new(0),
            spatial_width: AtomicU32::new(0),
        };
        p.apply(s);
        p
    }

    /// Relaxed stores. Values are clamped defensively (settings are sanitized upstream).
    pub fn apply(&self, s: &Settings) {
        let d = Settings::default();
        self.enabled.store(s.enabled, Ordering::Relaxed);
        self.key_up.store(s.key_up_enabled, Ordering::Relaxed);
        self.spatial.store(s.spatial_enabled, Ordering::Relaxed);
        let store =
            |a: &AtomicU32, v: f32, f: f32| a.store(unit(v, f).to_bits(), Ordering::Relaxed);
        store(&self.volume, s.volume, d.volume);
        store(&self.pitch_variation, s.pitch_variation, d.pitch_variation);
        store(&self.spatial_width, s.spatial_width, d.spatial_width);
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }

    pub fn pitch_variation(&self) -> f32 {
        f32::from_bits(self.pitch_variation.load(Ordering::Relaxed))
    }

    pub fn spatial_width(&self) -> f32 {
        f32::from_bits(self.spatial_width.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_clamps() {
        let mut s = Settings {
            volume: 0.25,
            pitch_variation: 2.0,
            spatial_width: f32::NAN,
            key_up_enabled: false,
            ..Settings::default()
        };
        let p = EngineParams::from_settings(&s);
        assert_eq!(p.volume(), 0.25);
        assert_eq!(p.pitch_variation(), 1.0);
        assert_eq!(p.spatial_width(), Settings::default().spatial_width);
        assert!(!p.key_up.load(Ordering::Relaxed));
        s.enabled = false;
        p.apply(&s);
        assert!(!p.enabled.load(Ordering::Relaxed));
    }
}
