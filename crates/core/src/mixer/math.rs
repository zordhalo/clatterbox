//! Mixer DSP helpers: pan, pitch, interpolation, soft clip (SPEC §4.4).
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

/// Equal-power pan. Returns `(gain_l, gain_r)` with `l² + r² = 1`.
pub fn pan_gains(x: f32, spatial: bool, width: f32) -> (f32, f32) {
    todo!("WP2")
}

/// `2^(cents / 1200)`.
pub fn cents_to_ratio(cents: f32) -> f32 {
    todo!("WP2")
}

pub fn db_to_gain(db: f32) -> f32 {
    todo!("WP2")
}

/// 4-point cubic Hermite read of mono `data` at fractional `pos`; reads past the end are 0.
pub fn hermite(data: &[f32], pos: f64) -> f32 {
    todo!("WP2")
}

/// Smooth-knee soft clip above |x| > 0.8; guarantees `|y| <= 1`.
pub fn soft_clip(x: f32) -> f32 {
    todo!("WP2")
}
