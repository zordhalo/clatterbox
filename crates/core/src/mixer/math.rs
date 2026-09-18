//! Mixer DSP helpers: pan, pitch, interpolation, soft clip (SPEC §4.4).

use std::f32::consts::FRAC_PI_4;

/// Equal-power pan. Returns `(gain_l, gain_r)` with `l² + r² = 1`.
pub fn pan_gains(x: f32, spatial: bool, width: f32) -> (f32, f32) {
    let p = if spatial && x.is_finite() {
        ((x * 2.0 - 1.0) * width).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let theta = (p + 1.0) * FRAC_PI_4;
    (theta.cos(), theta.sin())
}

/// `2^(cents / 1200)`.
pub fn cents_to_ratio(cents: f32) -> f32 {
    (cents / 1200.0).exp2()
}

pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// 4-point cubic Hermite (Catmull-Rom) read of mono `data` at fractional `pos`; reads outside
/// the buffer are 0.
#[inline]
pub fn hermite(data: &[f32], pos: f64) -> f32 {
    if pos.is_nan() || pos < 0.0 {
        return 0.0;
    }
    let i = pos as usize;
    let t = (pos - i as f64) as f32;
    let at = |k: usize| data.get(k).copied().unwrap_or(0.0);
    let xm1 = if i == 0 { 0.0 } else { at(i - 1) };
    let x0 = at(i);
    let x1 = at(i + 1);
    let x2 = at(i + 2);
    let c1 = 0.5 * (x1 - xm1);
    let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
    let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
    ((c3 * t + c2) * t + c1) * t + x0
}

const KNEE: f32 = 0.8;
const HEADROOM: f32 = 1.0 - KNEE;

/// Smooth-knee soft clip: identity for `|x| <= 0.8`, above that a rational curve with slope 1 at
/// the knee that approaches (never reaches) 1. Monotonic; guarantees `|y| <= 1`.
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= KNEE {
        return x;
    }
    if !a.is_finite() {
        return if a.is_nan() { 0.0 } else { 1f32.copysign(x) };
    }
    let d = a - KNEE;
    let y = KNEE + HEADROOM * d / (HEADROOM + d);
    y.min(1.0).copysign(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pan_is_equal_power() {
        for i in 0..=100 {
            let x = i as f32 / 100.0;
            for w in [0.0, 0.3, 1.0] {
                let (l, r) = pan_gains(x, true, w);
                assert!((l * l + r * r - 1.0).abs() < 1e-5);
            }
        }
        let (_, r) = pan_gains(0.0, true, 1.0);
        assert!(r.abs() < 1e-6);
        let (l, r) = pan_gains(0.1, false, 1.0);
        assert!((l - r).abs() < 1e-6);
    }

    #[test]
    fn pitch_ratio() {
        assert_eq!(cents_to_ratio(0.0), 1.0);
        assert!((cents_to_ratio(1200.0) - 2.0).abs() < 1e-6);
        assert!((db_to_gain(-6.0) - 0.501).abs() < 1e-3);
    }

    #[test]
    fn hermite_exact_on_samples_and_zero_outside() {
        let d = [0.1, 0.5, -0.3, 0.8];
        for (i, v) in d.iter().enumerate() {
            assert!((hermite(&d, i as f64) - v).abs() < 1e-6);
        }
        assert_eq!(hermite(&d, 10.0), 0.0);
        assert_eq!(hermite(&d, -1.0), 0.0);
        // linear ramp is reproduced exactly in the interior
        let ramp: Vec<f32> = (0..8).map(|i| i as f32).collect();
        assert!((hermite(&ramp, 3.25) - 3.25).abs() < 1e-5);
    }

    #[test]
    fn soft_clip_bounded_monotonic() {
        let mut prev = f32::NEG_INFINITY;
        for i in -4000..=4000 {
            let x = i as f32 / 100.0;
            let y = soft_clip(x);
            assert!(y.abs() <= 1.0);
            assert!(y >= prev);
            prev = y;
        }
        assert_eq!(soft_clip(0.5), 0.5);
        assert_eq!(soft_clip(f32::INFINITY), 1.0);
    }
}
