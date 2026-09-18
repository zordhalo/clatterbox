//! Synth building blocks (SPEC §6.1).

use std::f32::consts::TAU;

/// White noise, xorshift32.
pub struct Noise {
    state: u32,
}

impl Noise {
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9 } else { seed },
        }
    }

    /// Uniform in `-1.0..1.0`.
    pub fn sample(&mut self) -> f32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        (x >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
}

/// RBJ cookbook biquad (transposed direct form II).
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn from_coeffs(b: [f32; 3], a: [f32; 3]) -> Self {
        Self {
            b0: b[0] / a[0],
            b1: b[1] / a[0],
            b2: b[2] / a[0],
            a1: a[1] / a[0],
            a2: a[2] / a[0],
            z1: 0.0,
            z2: 0.0,
        }
    }

    /// `(cos w0, alpha)`; `fc` is clamped below Nyquist.
    fn prewarp(rate: u32, fc: f32, q: f32) -> (f32, f32) {
        let fc = fc.clamp(1.0, rate as f32 * 0.49);
        let w0 = TAU * fc / rate as f32;
        (w0.cos(), w0.sin() / (2.0 * q.max(0.01)))
    }

    /// Constant 0 dB peak gain bandpass.
    pub fn bandpass(rate: u32, fc: f32, q: f32) -> Self {
        let (c, a) = Self::prewarp(rate, fc, q);
        Self::from_coeffs([a, 0.0, -a], [1.0 + a, -2.0 * c, 1.0 - a])
    }

    pub fn lowpass(rate: u32, fc: f32, q: f32) -> Self {
        let (c, a) = Self::prewarp(rate, fc, q);
        let b1 = 1.0 - c;
        Self::from_coeffs([b1 / 2.0, b1, b1 / 2.0], [1.0 + a, -2.0 * c, 1.0 - a])
    }

    pub fn highpass(rate: u32, fc: f32, q: f32) -> Self {
        let (c, a) = Self::prewarp(rate, fc, q);
        let b1 = 1.0 + c;
        Self::from_coeffs([b1 / 2.0, -b1, b1 / 2.0], [1.0 + a, -2.0 * c, 1.0 - a])
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    /// Filters `input` into a new buffer.
    pub fn run(&mut self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&x| self.process(x)).collect()
    }
}

/// Exponential decay envelope of `len` samples.
pub fn exp_env(len: usize, tau_ms: f32, rate: u32) -> Vec<f32> {
    let k = -1.0 / (tau_ms.max(0.001) * 0.001 * rate as f32);
    (0..len).map(|i| (i as f32 * k).exp()).collect()
}

/// Exponential sine sweep `f0 → f1` with time constant `tau` seconds.
pub struct SineSweep {
    pub f0: f32,
    pub f1: f32,
    pub tau: f32,
}

impl SineSweep {
    pub fn render(&self, len: usize, rate: u32) -> Vec<f32> {
        let dt = 1.0 / rate as f32;
        let mut phase = 0.0f32;
        (0..len)
            .map(|i| {
                let t = i as f32 * dt;
                let f = self.f1 + (self.f0 - self.f1) * (-t / self.tau.max(1e-6)).exp();
                let s = phase.sin();
                phase = (phase + TAU * f * dt) % TAU;
                s
            })
            .collect()
    }
}

pub fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0f32, |m, x| m.max(x.abs()))
}

pub fn normalize_peak(buf: &mut [f32], dbfs: f32) {
    let p = peak(buf);
    if p > 0.0 {
        let g = 10f32.powf(dbfs / 20.0) / p;
        buf.iter_mut().for_each(|x| *x *= g);
    }
}

pub fn fade_out(buf: &mut [f32], ms: f32, rate: u32) {
    let n = ((ms * 0.001 * rate as f32) as usize).min(buf.len());
    let len = buf.len();
    for (i, x) in buf[len - n..].iter_mut().enumerate() {
        *x *= 1.0 - (i + 1) as f32 / n as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_bounded_and_deterministic() {
        let mut a = Noise::new(7);
        let mut b = Noise::new(7);
        for _ in 0..1000 {
            let x = a.sample();
            assert!((-1.0..1.0).contains(&x));
            assert_eq!(x, b.sample());
        }
    }

    #[test]
    fn bandpass_passes_center_rejects_far() {
        let rate = 48_000;
        let tone = |f: f32| -> f32 {
            let mut bp = Biquad::bandpass(rate, 1000.0, 2.0);
            let s: Vec<f32> = (0..4800)
                .map(|i| (TAU * f * i as f32 / rate as f32).sin())
                .collect();
            peak(&bp.run(&s)[2400..])
        };
        assert!((tone(1000.0) - 1.0).abs() < 0.05);
        assert!(tone(50.0) < 0.1);
    }

    #[test]
    fn normalize_and_fade() {
        let mut v = vec![0.5, -0.25, 0.1, 0.3];
        normalize_peak(&mut v, 0.0);
        assert!((peak(&v) - 1.0).abs() < 1e-6);
        fade_out(&mut v, 1000.0, 4);
        assert_eq!(*v.last().unwrap(), 0.0);
    }
}
