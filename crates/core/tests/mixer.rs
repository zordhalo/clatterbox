mod common;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use clatterbox_core::{
    EngineParams, KeyClass, KeyDir, KeySound, MAX_VOICES, Mixer, MixerCmd, PackSlot, Settings,
    Trigger,
};
use common::*;

fn params(f: impl FnOnce(&mut Settings)) -> Arc<EngineParams> {
    let mut s = Settings {
        volume: 1.0,
        pitch_variation: 0.0,
        spatial_width: 1.0,
        ..Settings::default()
    };
    f(&mut s);
    Arc::new(EngineParams::from_settings(&s))
}

fn mixer_with(p: Arc<EngineParams>, len: usize) -> Mixer {
    let mut m = Mixer::new(p, 48_000, 7);
    for slot in [PackSlot::Main, PackSlot::Preview] {
        let r = m.apply(MixerCmd::SetPack {
            slot,
            pack: Arc::new(sine_pack("t", len, 3)),
        });
        assert!(r.is_none());
    }
    m
}

fn key(x: f32) -> KeySound {
    KeySound {
        class: KeyClass::Default,
        dir: KeyDir::Down,
        x,
    }
}

fn energy(buf: &[f32], ch: usize, c: usize) -> f32 {
    buf.chunks_exact(ch).map(|f| f[c] * f[c]).sum()
}

#[test]
fn trigger_renders_sound() {
    let mut m = mixer_with(params(|_| {}), 4800);
    let mut out = vec![0.0; 512];
    m.render(&mut out, 2);
    assert_eq!(peak(&out), 0.0);
    m.trigger(Trigger::key(key(0.5)));
    assert_eq!(m.active_voices(), 1);
    m.render(&mut out, 2);
    assert!(peak(&out) > 0.1);
    assert!(peak(&out) <= 1.0);
}

#[test]
fn left_pan() {
    let mut m = mixer_with(params(|_| {}), 4800);
    m.trigger(Trigger::key(key(0.0)));
    let mut out = vec![0.0; 1024];
    m.render(&mut out, 2);
    assert!(energy(&out, 2, 0) > 4.0 * energy(&out, 2, 1));
}

#[test]
fn voice_cap_and_stealing() {
    let mut m = mixer_with(params(|_| {}), 48_000);
    let mut out = vec![0.0; 64];
    for i in 0..100 {
        m.trigger(Trigger::key(key((i % 10) as f32 / 10.0)));
        if i % 7 == 0 {
            m.render(&mut out, 2);
        }
        assert!(m.active_voices() <= MAX_VOICES);
    }
    assert_eq!(m.active_voices(), MAX_VOICES);
    m.render(&mut out, 2);
    assert!(out.iter().all(|x| x.abs() <= 1.0));
}

#[test]
fn disabled_main_silent_preview_audible() {
    let p = params(|s| {
        s.enabled = false;
        s.key_up_enabled = false;
    });
    let mut m = mixer_with(p.clone(), 4800);
    m.trigger(Trigger::key(key(0.5)));
    assert_eq!(m.active_voices(), 0);
    m.trigger(Trigger::preview(key(0.5)));
    let up = KeySound {
        dir: KeyDir::Up,
        ..key(0.5)
    };
    m.trigger(Trigger::preview(up));
    assert_eq!(m.active_voices(), 2);
    p.enabled.store(true, Ordering::Relaxed);
    m.trigger(Trigger::key(up));
    assert_eq!(m.active_voices(), 2, "key-up disabled for Main");
}

#[test]
fn pack_swap_fades_without_click() {
    let mut m = mixer_with(params(|s| s.spatial_enabled = false), 48_000);
    m.trigger(Trigger::key(key(0.5)));
    let mut out = vec![0.0; 2 * 256];
    m.render(&mut out, 2);
    let mut prev = out[out.len() - 2];
    let retired = m.apply(MixerCmd::SetPack {
        slot: PackSlot::Main,
        pack: Arc::new(sine_pack("new", 480, 1)),
    });
    assert!(
        retired.is_none(),
        "old pack must stay alive during the fade"
    );
    assert!(m.take_retired().is_none());
    m.render(&mut out, 2);
    // 440 Hz at amplitude ~0.32 moves ≤ ~0.02 per sample; a hard cut would jump by ~0.3.
    let mut max_delta = 0f32;
    for &l in out.iter().step_by(2) {
        max_delta = max_delta.max((l - prev).abs());
        prev = l;
    }
    assert!(max_delta < 0.05, "max delta {max_delta}");
    assert_eq!(m.active_voices(), 0);
    let old = m.take_retired().expect("retired after fade");
    assert_eq!(old.info.id, "t");
    assert!(m.take_retired().is_none());
}

#[test]
fn swap_without_voices_returns_immediately() {
    let mut m = mixer_with(params(|_| {}), 480);
    let r = m.apply(MixerCmd::SetPack {
        slot: PackSlot::Preview,
        pack: Arc::new(sine_pack("p2", 480, 1)),
    });
    assert_eq!(r.unwrap().info.id, "t");
}

#[test]
fn mono_and_multichannel() {
    let mut m = mixer_with(params(|_| {}), 4800);
    m.trigger(Trigger::key(key(0.2)));
    let mut mono = vec![0.0; 256];
    m.render(&mut mono, 1);
    assert!(peak(&mono) > 0.05);
    m.trigger(Trigger::key(key(0.8)));
    let mut six = vec![0.0; 6 * 256];
    m.render(&mut six, 6);
    assert!(energy(&six, 6, 0) > 0.0 && energy(&six, 6, 1) > 0.0);
    for c in 2..6 {
        assert_eq!(energy(&six, 6, c), 0.0);
    }
}

#[test]
fn step_follows_rate_and_pitch() {
    // 48 kHz sample into a 24 kHz output plays twice as fast: a 4800-sample sine ends after
    // 2400 frames.
    let mut m = mixer_with(params(|_| {}), 4800);
    m.set_output_rate(24_000);
    m.trigger(Trigger::key(key(0.5)));
    let mut out = vec![0.0; 2 * 2399];
    m.render(&mut out, 2);
    assert_eq!(m.active_voices(), 1);
    let mut out = vec![0.0; 2 * 2];
    m.render(&mut out, 2);
    assert_eq!(m.active_voices(), 0);
}
