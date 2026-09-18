//! Micro benchmark: mixer render cost per 256-frame stereo block with all 32 voices active.
//! `cargo run --release -p clatterbox-audio --example mixer_bench`

use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

use clatterbox_core::synth::generate;
use clatterbox_core::{
    EngineParams, KeyClass, KeyDir, KeySound, MAX_VOICES, Mixer, MixerCmd, PackSlot, Settings,
    SynthPreset, Trigger,
};

const FRAMES: usize = 256;
const BLOCKS: usize = 20_000;

fn main() {
    let params = Arc::new(EngineParams::from_settings(&Settings::default()));
    // 44.1 kHz output forces fractional resampling of the 48 kHz synth samples.
    let mut mixer = Mixer::new(params, 44_100, 1);
    let pack = Arc::new(generate(SynthPreset::Thock, 48_000));
    let _ = mixer.apply(MixerCmd::SetPack {
        slot: PackSlot::Main,
        pack,
    });
    let mut out = vec![0.0f32; FRAMES * 2];
    let mut total = 0.0f64;
    let mut worst = 0.0f64;
    let mut voice_blocks = 0usize;
    for b in 0..BLOCKS {
        // Keep the voice pool saturated (stealing kicks in once full).
        while mixer.active_voices() < MAX_VOICES {
            let x = (b % 17) as f32 / 16.0;
            mixer.trigger(Trigger::key(KeySound {
                class: KeyClass::ALL[b % KeyClass::COUNT],
                dir: KeyDir::Down,
                x,
            }));
        }
        voice_blocks += mixer.active_voices();
        let t = Instant::now();
        mixer.render(black_box(&mut out), 2);
        let dt = t.elapsed().as_secs_f64();
        total += dt;
        worst = worst.max(dt);
        black_box(&out);
    }
    let mean_us = total / BLOCKS as f64 * 1e6;
    let budget_us = FRAMES as f64 / 44_100.0 * 1e6;
    println!(
        "mixer render, {FRAMES} frames x 2 ch, avg {:.1} voices: mean {mean_us:.2} us, worst {:.2} us \
         ({:.3}% of the {budget_us:.0} us block)",
        voice_blocks as f64 / BLOCKS as f64,
        worst * 1e6,
        mean_us / budget_us * 100.0
    );
}
