mod common;

use clatterbox_core::pack::derive::{class_offset, derive_sample, resolve, resolved_samples};
use clatterbox_core::{KeyClass, Sample, SampleSet};
use common::*;

/// A slow decaying tone with its peak near the start (survives 60 % truncation).
fn source(len: usize) -> Sample {
    let data: Vec<f32> = (0..len)
        .map(|i| {
            let t = i as f32 / 48_000.0;
            0.8 * (-t / 0.02).exp() * (std::f32::consts::TAU * 200.0 * t).sin()
        })
        .collect();
    Sample {
        data: data.into_boxed_slice(),
        rate: 48_000,
    }
}

fn only_default_down() -> [SampleSet; KeyClass::COUNT] {
    let mut sets: [SampleSet; KeyClass::COUNT] = Default::default();
    sets[0].down = vec![source(4800), source(3000)];
    sets
}

#[test]
fn all_sets_filled_and_listed() {
    let mut sets = only_default_down();
    let derived = resolve(&mut sets, &[0.0; KeyClass::COUNT]);
    for s in &sets {
        assert_eq!(s.down.len(), 2);
        assert_eq!(s.up.len(), 2);
        assert!(s.down.iter().chain(&s.up).all(|x| !x.data.is_empty()));
    }
    assert_eq!(
        derived,
        [
            "space.down",
            "enter.down",
            "backspace.down",
            "modifier.down",
            "default.up",
            "space.up",
            "enter.up",
            "backspace.up",
            "modifier.up"
        ]
    );
}

#[test]
fn derived_up_length_and_level() {
    let src = source(4800);
    let up = derive_sample(&src, -300.0, -9.0, Some(0.6), 5.0);
    let expect = 0.6 * 4800.0 * 2f32.powf(300.0 / 1200.0);
    assert!(
        (up.data.len() as f32 - expect).abs() <= 2.0,
        "{} vs {expect}",
        up.data.len()
    );
    let drop = db(peak(&up.data)) - db(peak(&src.data));
    assert!((drop + 9.0).abs() <= 0.5, "drop = {drop}");
    assert_eq!(*up.data.last().unwrap(), 0.0);
}

#[test]
fn class_offsets_give_length_ratios() {
    let mut sets = only_default_down();
    resolve(&mut sets, &[0.0; KeyClass::COUNT]);
    let base = sets[0].down[0].data.len() as f32;
    for c in &KeyClass::ALL[1..] {
        let (cents, _) = class_offset(*c);
        let got = sets[*c as usize].down[0].data.len() as f32;
        let expect = base / 2f32.powf(cents / 1200.0);
        assert!(
            (got - expect).abs() <= 2.0,
            "{}: {got} vs {expect}",
            c.as_str()
        );
    }
}

#[test]
fn explicit_sets_never_replaced() {
    let mut sets = only_default_down();
    sets[KeyClass::Enter as usize].down = vec![source(100)];
    sets[KeyClass::Default as usize].up = vec![source(50)];
    let derived = resolve(&mut sets, &[0.0; KeyClass::COUNT]);
    assert_eq!(sets[KeyClass::Enter as usize].down.len(), 1);
    assert_eq!(sets[KeyClass::Enter as usize].down[0].data.len(), 100);
    assert_eq!(sets[0].up.len(), 1);
    assert_eq!(sets[0].up[0].data.len(), 50);
    // enter.up derives from the explicit enter.down, not from default
    let eu = &sets[KeyClass::Enter as usize].up[0];
    assert!((eu.data.len() as f32 - 0.6 * 100.0 * 2f32.powf(0.25)).abs() <= 2.0);
    assert_eq!(
        derived,
        [
            "space.down",
            "backspace.down",
            "modifier.down",
            "space.up",
            "enter.up",
            "backspace.up",
            "modifier.up"
        ]
    );
}

#[test]
fn class_gain_applied_to_explicit_and_propagated() {
    let mut sets = only_default_down();
    let src_peak = peak(&sets[0].down[0].data);
    let mut gains = [0.0; KeyClass::COUNT];
    gains[0] = -6.0;
    resolve(&mut sets, &gains);
    let d = db(peak(&sets[0].down[0].data)) - db(src_peak);
    assert!((d + 6.0).abs() < 0.01);
    // space.down = default gain (-6) + offset (+1)
    let s = db(peak(&sets[KeyClass::Space as usize].down[0].data)) - db(src_peak);
    assert!((s + 5.0).abs() < 0.5, "space = {s}");
}

#[test]
fn size_estimate_matches_resolution() {
    let mut sets = only_default_down();
    sets[KeyClass::Enter as usize].down = vec![source(700)];
    sets[KeyClass::Space as usize].up = vec![source(90)];
    let estimate = resolved_samples(&sets);
    resolve(&mut sets, &[0.0; KeyClass::COUNT]);
    let actual: usize = sets
        .iter()
        .flat_map(|s| s.down.iter().chain(&s.up))
        .map(|s| s.data.len())
        .sum();
    assert_eq!(estimate, actual);
}
