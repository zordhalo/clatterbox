//! Real-time voice mixer (SPEC §4.4). Pure function of its inputs; no I/O, never allocates in
//! `trigger`, `render`, `apply` or `take_retired`.

pub mod math;
mod voice;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::pack::LoadedPack;
use crate::params::EngineParams;
use crate::types::{KeyClass, KeyDir, PackSlot, Trigger};
use voice::{FADE_LEN, Voice};

pub const MAX_VOICES: usize = 32;

/// Pitch variation range at `pitch_variation = 1.0`.
const MAX_CENTS: f32 = 150.0;
/// Gain variation range at `pitch_variation = 1.0`.
const MAX_GAIN_DB: f32 = 1.0;
/// Fixed master headroom factor.
const MASTER_TRIM: f32 = 0.9;

pub enum MixerCmd {
    SetPack {
        slot: PackSlot,
        pack: Arc<LoadedPack>,
    },
}

pub struct Mixer {
    voices: [Voice; MAX_VOICES],
    /// Indexed by `PackSlot as usize`.
    packs: [Option<Arc<LoadedPack>>; 2],
    /// Previous pack of each slot, kept alive while its voices fade out after a swap.
    retiring: [Option<Arc<LoadedPack>>; 2],
    /// Last variation per `[slot][class][dir]` (avoid immediate repeats).
    last_var: [[[u8; 2]; KeyClass::COUNT]; 2],
    rng: fastrand::Rng,
    out_rate: u32,
    params: Arc<EngineParams>,
}

impl Mixer {
    pub fn new(params: Arc<EngineParams>, out_rate: u32, seed: u64) -> Self {
        Self {
            voices: [Voice::IDLE; MAX_VOICES],
            packs: [None, None],
            retiring: [None, None],
            last_var: [[[u8::MAX; 2]; KeyClass::COUNT]; 2],
            rng: fastrand::Rng::with_seed(seed),
            out_rate: out_rate.max(1),
            params,
        }
    }

    pub fn set_output_rate(&mut self, out_rate: u32) {
        let out_rate = out_rate.max(1);
        let k = f64::from(self.out_rate) / f64::from(out_rate);
        for v in &mut self.voices {
            v.step *= k;
        }
        self.out_rate = out_rate;
    }

    pub fn output_rate(&self) -> u32 {
        self.out_rate
    }

    /// Returns the retired pack (caller routes it to the garbage queue). A pack that still has
    /// sounding voices is kept until its 64-sample fade ends; collect it with
    /// [`Mixer::take_retired`] after `render`.
    pub fn apply(&mut self, cmd: MixerCmd) -> Option<Arc<LoadedPack>> {
        let MixerCmd::SetPack { slot, pack } = cmd;
        let s = slot as usize;
        let old = self.packs[s].replace(pack)?;
        // A pack still fading from an earlier swap is cut now (two swaps within one fade).
        let mut out = None;
        if let Some(prev) = self.retiring[s].take() {
            for v in self
                .voices
                .iter_mut()
                .filter(|v| v.active && v.slot == slot && v.retiring)
            {
                v.active = false;
            }
            out = Some(prev);
        }
        let mut sounding = false;
        for v in self
            .voices
            .iter_mut()
            .filter(|v| v.active && v.slot == slot)
        {
            v.retiring = true;
            v.fade = FADE_LEN;
            sounding = true;
        }
        if sounding {
            self.retiring[s] = Some(old);
            out
        } else if out.is_some() {
            // Two packs to hand back but one return value: keep `old` as retiring (no voices
            // reference it), it is returned by the next `take_retired`.
            self.retiring[s] = Some(old);
            out
        } else {
            Some(old)
        }
    }

    /// Returns a retiring pack whose voices have all finished, if any. Call after `render`
    /// until it returns `None`.
    pub fn take_retired(&mut self) -> Option<Arc<LoadedPack>> {
        for s in 0..2 {
            if self.retiring[s].is_some()
                && !self
                    .voices
                    .iter()
                    .any(|v| v.active && v.retiring && v.slot as usize == s)
            {
                return self.retiring[s].take();
            }
        }
        None
    }

    pub fn trigger(&mut self, t: Trigger) {
        let (class, dir, x) = (t.sound.class, t.sound.dir, t.sound.x);
        let p = &self.params;
        if t.slot == PackSlot::Main
            && (!p.enabled.load(Ordering::Relaxed)
                || (dir == KeyDir::Up && !p.key_up.load(Ordering::Relaxed)))
        {
            return;
        }
        let Some(pack) = self.packs[t.slot as usize].as_ref() else {
            return;
        };
        let samples = pack.samples(class, dir);
        let n = samples.len().min(usize::from(u8::MAX));
        if n == 0 {
            return;
        }
        let last = &mut self.last_var[t.slot as usize][class as usize][dir as usize];
        let var = if n == 1 {
            0
        } else {
            let r = self.rng.usize(0..n - 1);
            if r >= usize::from(*last) { r + 1 } else { r }
        };
        *last = var as u8;
        let sample = &samples[var];

        let pv = p.pitch_variation();
        let tri = self.rng.f32() + self.rng.f32() - 1.0;
        let ratio = math::cents_to_ratio(MAX_CENTS * pv * tri);
        let gain = math::db_to_gain(MAX_GAIN_DB * pv * tri) * pack.gain;
        let (gl, gr) = math::pan_gains(x, p.spatial.load(Ordering::Relaxed), p.spatial_width());
        let step = f64::from(ratio) * f64::from(sample.rate) / f64::from(self.out_rate);

        let idx = self.alloc_voice();
        self.voices[idx] = Voice {
            active: true,
            slot: t.slot,
            retiring: false,
            class,
            dir,
            var: var as u8,
            pos: 0.0,
            step,
            gain_l: gl * gain,
            gain_r: gr * gain,
            fade: 0,
        };
    }

    /// First idle voice, else the one furthest through its sample.
    fn alloc_voice(&self) -> usize {
        if let Some(i) = self.voices.iter().position(|v| !v.active) {
            return i;
        }
        let mut best = (0, f64::NEG_INFINITY);
        for (i, v) in self.voices.iter().enumerate() {
            let prog = self
                .voice_data(v)
                .map_or(f64::INFINITY, |d| v.progress(d.len()));
            if prog > best.1 {
                best = (i, prog);
            }
        }
        best.0
    }

    fn voice_data<'a>(&'a self, v: &Voice) -> Option<&'a [f32]> {
        let s = v.slot as usize;
        let pack = if v.retiring {
            &self.retiring[s]
        } else {
            &self.packs[s]
        };
        let set = pack.as_ref()?.samples(v.class, v.dir);
        set.get(usize::from(v.var)).map(|smp| &*smp.data)
    }

    /// Interleaved output, overwrites `out`. Never allocates.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        out.fill(0.0);
        if channels == 0 {
            return;
        }
        let usable = out.len() - out.len() % channels;
        let out = &mut out[..usable];
        let vol = self.params.volume();
        let master = vol * vol * MASTER_TRIM;
        for i in 0..MAX_VOICES {
            if !self.voices[i].active {
                continue;
            }
            let mut v = self.voices[i];
            match self.voice_data(&v) {
                Some(data) => v.render(data, out, channels, master),
                None => v.active = false,
            }
            self.voices[i] = v;
        }
        // With > 2 channels only ch0/ch1 carry signal; the rest stays zero.
        for x in out.iter_mut() {
            *x = math::soft_clip(*x);
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}
