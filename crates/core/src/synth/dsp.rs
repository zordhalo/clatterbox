//! Synth building blocks (SPEC §6.1).
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

/// White noise, xorshift32.
pub struct Noise {
    state: u32,
}

impl Noise {
    pub fn new(seed: u32) -> Self {
        todo!("WP2")
    }

    pub fn sample(&mut self) -> f32 {
        todo!("WP2")
    }
}

/// RBJ cookbook biquad.
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
    /// Constant 0 dB peak gain bandpass.
    pub fn bandpass(rate: u32, fc: f32, q: f32) -> Self {
        todo!("WP2")
    }

    pub fn lowpass(rate: u32, fc: f32, q: f32) -> Self {
        todo!("WP2")
    }

    pub fn highpass(rate: u32, fc: f32, q: f32) -> Self {
        todo!("WP2")
    }

    pub fn process(&mut self, x: f32) -> f32 {
        todo!("WP2")
    }
}

/// Exponential decay envelope of `len` samples.
pub fn exp_env(len: usize, tau_ms: f32, rate: u32) -> Vec<f32> {
    todo!("WP2")
}

/// Exponential sine sweep `f0 → f1` with time constant `tau` seconds.
pub struct SineSweep {
    pub f0: f32,
    pub f1: f32,
    pub tau: f32,
}

impl SineSweep {
    pub fn render(&self, len: usize, rate: u32) -> Vec<f32> {
        todo!("WP2")
    }
}

pub fn normalize_peak(buf: &mut [f32], dbfs: f32) {
    todo!("WP2")
}

pub fn fade_out(buf: &mut [f32], ms: f32, rate: u32) {
    todo!("WP2")
}
