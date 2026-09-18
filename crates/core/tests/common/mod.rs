//! Shared helpers for core integration tests.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use clatterbox_core::{KeyClass, LoadedPack, PackInfo, PackKind, Sample, SampleSet};

pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn pack_fixture(name: &str) -> PathBuf {
    fixtures().join("packs").join(name)
}

/// Fresh empty directory under the system temp dir (removed on drop).
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "clatterbox-test-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap().flatten() {
        let to = dst.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &to);
        } else {
            std::fs::copy(e.path(), to).unwrap();
        }
    }
}

pub fn sine(len: usize, freq: f32, rate: u32, amp: f32) -> Sample {
    let data: Vec<f32> = (0..len)
        .map(|i| amp * (std::f32::consts::TAU * freq * i as f32 / rate as f32).sin())
        .collect();
    Sample {
        data: data.into_boxed_slice(),
        rate,
    }
}

pub fn test_info(id: &str) -> PackInfo {
    PackInfo {
        id: id.into(),
        name: id.into(),
        author: "t".into(),
        license: "CC0-1.0".into(),
        source_url: None,
        description: None,
        kind: PackKind::User,
        valid: true,
        error: None,
        derived: Vec::new(),
    }
}

/// Pack whose every set holds `vars` sines of `len` samples at 48 kHz (ups are half as long).
pub fn sine_pack(id: &str, len: usize, vars: usize) -> LoadedPack {
    let sets = KeyClass::ALL.map(|_| SampleSet {
        down: (0..vars).map(|_| sine(len, 440.0, 48_000, 0.5)).collect(),
        up: (0..vars)
            .map(|_| sine(len / 2, 880.0, 48_000, 0.5))
            .collect(),
    });
    LoadedPack::from_sets(test_info(id), 1.0, sets)
}

pub fn peak(d: &[f32]) -> f32 {
    d.iter().fold(0f32, |m, x| m.max(x.abs()))
}

pub fn db(x: f32) -> f32 {
    20.0 * x.log10()
}
