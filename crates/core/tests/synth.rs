mod common;

use std::hash::{Hash, Hasher};
use std::time::Instant;

use clatterbox_core::synth::generate;
use clatterbox_core::{KeyClass, KeyDir, LoadedPack, SynthPreset};
use common::*;

fn hash(pack: &LoadedPack) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for c in KeyClass::ALL {
        for d in [KeyDir::Down, KeyDir::Up] {
            for s in pack.samples(c, d) {
                s.data.iter().for_each(|x| x.to_bits().hash(&mut h));
            }
        }
    }
    h.finish()
}

#[test]
fn every_buffer_is_sane() {
    for preset in SynthPreset::ALL {
        let pack = generate(preset, 48_000);
        assert_eq!(pack.info.id, preset.id());
        assert!(pack.info.derived.is_empty());
        for c in KeyClass::ALL {
            for d in [KeyDir::Down, KeyDir::Up] {
                let set = pack.samples(c, d);
                assert_eq!(set.len(), 5);
                for s in set {
                    assert!(!s.data.is_empty());
                    assert!(s.data.iter().all(|x| x.is_finite()));
                    let rms =
                        (s.data.iter().map(|x| x * x).sum::<f32>() / s.data.len() as f32).sqrt();
                    assert!(
                        db(rms) > -40.0,
                        "{:?} {} rms {}",
                        preset,
                        c.as_str(),
                        db(rms)
                    );
                    assert!(db(peak(&s.data)) <= -2.9, "peak {}", db(peak(&s.data)));
                }
            }
        }
    }
}

#[test]
fn deterministic() {
    for preset in SynthPreset::ALL {
        assert_eq!(
            hash(&generate(preset, 48_000)),
            hash(&generate(preset, 48_000))
        );
    }
    assert_ne!(
        hash(&generate(SynthPreset::Thock, 48_000)),
        hash(&generate(SynthPreset::Click, 48_000))
    );
}

/// SPEC §6 / §11: < 10 ms per preset in release, < 50 ms in debug (the dev profile builds
/// `clatterbox-core` optimized). Best of 3 runs to be robust against a loaded CI machine.
#[test]
fn fast_enough() {
    let budget_ms = if cfg!(debug_assertions) { 50.0 } else { 10.0 };
    for preset in SynthPreset::ALL {
        let best = (0..3)
            .map(|_| {
                let t = Instant::now();
                generate(preset, 48_000);
                t.elapsed().as_secs_f64() * 1000.0
            })
            .fold(f64::INFINITY, f64::min);
        println!("{}: {best:.1} ms", preset.id());
        assert!(best < budget_ms, "{} took {best:.1} ms", preset.id());
    }
}
