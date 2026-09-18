//! Procedural synth packs (SPEC §6). Always available: zero-asset operation.

pub mod dsp;
mod presets;

use crate::mixer::math::db_to_gain;
use crate::pack::{LoadedPack, PackInfo, PackKind, Sample, SampleSet};
use crate::types::{KeyClass, KeyDir};
use dsp::{Biquad, Noise, SineSweep, exp_env, fade_out, normalize_peak};
use presets::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynthPreset {
    Thock,
    Click,
}

impl SynthPreset {
    pub const ALL: [SynthPreset; 2] = [Self::Thock, Self::Click];

    /// `"synth/thock"` | `"synth/click"`
    pub fn id(self) -> &'static str {
        match self {
            Self::Thock => "synth/thock",
            Self::Click => "synth/click",
        }
    }

    /// license "CC0-1.0", author "Clatterbox contributors".
    pub fn info(self) -> PackInfo {
        let (name, description) = match self {
            Self::Thock => (
                "Synth Thock",
                "Procedural muted linear with a deep bottom-out.",
            ),
            Self::Click => (
                "Synth Click",
                "Procedural tactile click with a bright leaf snap.",
            ),
        };
        PackInfo {
            id: self.id().to_owned(),
            name: name.to_owned(),
            author: "Clatterbox contributors".to_owned(),
            license: "CC0-1.0".to_owned(),
            source_url: None,
            description: Some(description.to_owned()),
            kind: PackKind::Synth,
            valid: true,
            error: None,
            derived: Vec::new(),
        }
    }
}

fn ms(ms: f32, rate: u32) -> usize {
    (ms * 0.001 * rate as f32).round() as usize
}

/// Multiplies by a uniform factor in `1 ± frac`.
fn jitter(rng: &mut fastrand::Rng, frac: f32) -> f32 {
    1.0 + frac * (rng.f32() * 2.0 - 1.0)
}

/// Scales `buf` to unit peak (no-op for silence).
fn unit_peak(mut buf: Vec<f32>) -> Vec<f32> {
    normalize_peak(&mut buf, 0.0);
    buf
}

fn add_scaled(dst: &mut [f32], src: &[f32], g: f32) {
    for (d, s) in dst.iter_mut().zip(src) {
        *d += s * g;
    }
}

/// Renders one variation (SPEC §6.2).
fn render_voice(p: &PresetParams, class: KeyClass, dir: KeyDir, var: usize, rate: u32) -> Vec<f32> {
    let cm = class_mod(class);
    let up = dir == KeyDir::Up;
    let seed = SEED_BASE ^ ((class as u64) << 8) ^ ((dir as u64) << 4) ^ var as u64;
    let mut rng = fastrand::Rng::with_seed(seed);
    let len = if up {
        ms(UP_LENGTH_MS, rate)
    } else {
        ms((p.length_ms * cm.decay).clamp(90.0, 160.0), rate)
    };
    let gain_jitter = |rng: &mut fastrand::Rng| db_to_gain(1.5 * (rng.f32() * 2.0 - 1.0));

    // Excitation: short noise impulse (~4 ms).
    let mut noise = Noise::new(rng.u32(..));
    let ex_len = ms(4.0, rate).min(len);
    let env = exp_env(ex_len, p.excite_tau_ms * jitter(&mut rng, 0.1), rate);
    let mut excite = vec![0.0f32; len];
    for (x, e) in excite.iter_mut().zip(&env) {
        *x = noise.sample() * e;
    }

    // Click band (plus optional second leaf impulse).
    let mut click_in = excite.clone();
    if let Some((delay_ms, level_db)) = p.second_click {
        let d = ms(delay_ms, rate);
        if d < len {
            let g = db_to_gain(level_db);
            for (i, e) in env.iter().enumerate().take(len - d) {
                click_in[d + i] += noise.sample() * e * g;
            }
        }
    }
    let click = unit_peak(Biquad::bandpass(rate, p.click_hz, p.click_q).run(&click_in));
    let mut g_click = p.g_click * gain_jitter(&mut rng);
    if up {
        g_click *= UP_CLICK_GAIN;
    }
    let mut out = vec![0.0f32; len];
    add_scaled(&mut out, &click, g_click);

    // Body: modal resonances.
    let mode_f = cm.freq * if up { UP_MODE_FREQ } else { 1.0 };
    let mut bank = std::array::from_fn::<_, 3, _>(|i| {
        let fc = p.modes_hz[i] * mode_f * jitter(&mut rng, 0.05);
        let q = p.modes_q[i] * cm.decay * jitter(&mut rng, 0.15);
        Biquad::bandpass(rate, fc, q)
    });
    for (i, mode) in Biquad::run_bank(&mut bank, &excite).into_iter().enumerate() {
        add_scaled(
            &mut out,
            &unit_peak(mode),
            p.modes_g[i] * gain_jitter(&mut rng),
        );
    }

    // Thump: bottom-out sweep (down strokes only).
    if !up {
        let t_hz = p.thump_hz * cm.freq;
        let sweep = SineSweep {
            f0: t_hz * 1.6,
            f1: t_hz,
            tau: 0.006,
        }
        .render(len, rate);
        let env = exp_env(len, p.thump_tau_ms * cm.decay, rate);
        let thump: Vec<f32> = sweep.iter().zip(&env).map(|(s, e)| s * e).collect();
        add_scaled(&mut out, &thump, p.g_thump * gain_jitter(&mut rng));
    }

    // Stabilizer rattle (space).
    if cm.rattle {
        let d = ms(RATTLE_DELAY_MS, rate);
        let burst_len = ms(15.0, rate).min(len.saturating_sub(d));
        let benv = exp_env(burst_len, 3.0, rate);
        let mut burst = vec![0.0f32; len];
        for (i, e) in benv.iter().enumerate() {
            burst[d + i] = noise.sample() * e;
        }
        let r = unit_peak(Biquad::bandpass(rate, RATTLE_HZ, RATTLE_Q).run(&burst));
        let sum_peak = dsp::peak(&out);
        add_scaled(&mut out, &r, sum_peak * db_to_gain(RATTLE_LEVEL_DB));
    }

    normalize_peak(&mut out, 0.0);
    for x in out.iter_mut() {
        *x = (1.4 * *x).tanh();
    }
    let mut out = Biquad::highpass(rate, 40.0, std::f32::consts::FRAC_1_SQRT_2).run(&out);
    fade_out(&mut out, 3.0, rate);
    let level = cm.level_db + if up { UP_LEVEL_DB } else { 0.0 };
    normalize_peak(&mut out, PEAK_DBFS - MAX_CLASS_LEVEL_DB + level);
    out
}

/// Deterministic generation at `rate` Hz (budget < 10 ms total in release).
pub fn generate(preset: SynthPreset, rate: u32) -> LoadedPack {
    let rate = rate.clamp(8_000, 192_000);
    let p = params(preset);
    let sets = KeyClass::ALL.map(|class| {
        let make = |dir| {
            (0..VARIATIONS)
                .map(|var| Sample {
                    data: render_voice(&p, class, dir, var, rate).into_boxed_slice(),
                    rate,
                })
                .collect()
        };
        SampleSet {
            down: make(KeyDir::Down),
            up: make(KeyDir::Up),
        }
    });
    LoadedPack::from_sets(preset.info(), 1.0, sets)
}
