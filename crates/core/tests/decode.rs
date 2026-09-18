mod common;

use clatterbox_core::pack::decode::decode_file;
use common::*;

#[test]
fn stereo_16bit_to_mono() {
    let s = decode_file(&fixtures().join("audio/stereo-dc.wav")).unwrap();
    assert_eq!(s.rate, 22_050);
    assert_eq!(s.data.len(), 1000);
    assert!(s.data.iter().all(|x| (x - 0.375).abs() < 1e-3));
}

#[test]
fn too_long_rejected() {
    let err = decode_file(&fixtures().join("audio/too-long.wav"))
        .err()
        .unwrap();
    assert!(err.contains("longer"), "{err}");
}

fn first_above_rel_db(d: &[f32], rel_db: f32) -> usize {
    let thr = peak(d) * 10f32.powf(rel_db / 20.0);
    d.iter().position(|x| x.abs() > thr).unwrap()
}

#[test]
fn leading_silence_trimmed() {
    let s = decode_file(&fixtures().join("audio/lead-silence.wav")).unwrap();
    let first = first_above_rel_db(&s.data, -30.0);
    // 20 ms of silence removed, ~1 ms pre-roll kept.
    assert!(first <= (s.rate as usize / 1000) + 2, "first = {first}");
}

#[test]
fn mp3_priming_trimmed() {
    let s = decode_file(&fixtures().join("audio/key.mp3")).unwrap();
    let first = first_above_rel_db(&s.data, -30.0);
    let limit = (1.5e-3 * s.rate as f32) as usize;
    assert!(first <= limit, "first = {first}, limit = {limit}");
    assert!(peak(&s.data) <= 1.0);
}
