mod common;

use std::path::Path;

use clatterbox_core::pack::PackErrorKind;
use clatterbox_core::{KeyClass, KeyDir, PackKind, PackRegistry};
use common::*;

/// User dir with the fixture packs plus one invalidly named dir.
fn fixture_user_dir() -> TempDir {
    let tmp = TempDir::new("registry");
    for name in ["valid-min", "valid-full", "bad-schema", "bad-missing-file"] {
        copy_dir(&pack_fixture(name), &tmp.0.join(name));
    }
    copy_dir(&pack_fixture("valid-min"), &tmp.0.join("Bad_Name"));
    tmp
}

#[test]
fn scan_ordering_and_invalid_dir_skipped() {
    let user = fixture_user_dir();
    let mut reg = PackRegistry::new(None, user.0.clone());
    let ids: Vec<String> = reg.rescan().iter().map(|i| i.id.clone()).collect();
    // user packs sorted by name: "Bad Missing File", "bad-schema" (invalid → dir name), "Valid Full", "Valid Min"
    assert_eq!(
        &ids[..4],
        [
            "user/bad-missing-file",
            "user/bad-schema",
            "user/valid-full",
            "user/valid-min"
        ]
    );
    assert_eq!(&ids[4..], ["synth/click", "synth/thock"]);
    let bad = reg
        .list()
        .iter()
        .find(|i| i.id == "user/bad-schema")
        .unwrap();
    assert!(!bad.valid && bad.error.is_some());
    // manifest-only scan: a missing audio file is only caught at load
    let missing = reg
        .list()
        .iter()
        .find(|i| i.id == "user/bad-missing-file")
        .unwrap();
    assert!(missing.valid);
    let err = reg.load("user/bad-missing-file", 48_000).err().unwrap();
    assert_eq!(err.kind, PackErrorKind::Invalid);
    assert!(err.message.contains("missing"), "{}", err.message);
}

#[test]
fn load_full_pack_resolves_all_sets() {
    let user = fixture_user_dir();
    let reg = PackRegistry::new(None, user.0.clone());
    let pack = reg.load("user/valid-full", 48_000).unwrap();
    assert_eq!(pack.info.kind, PackKind::User);
    assert!((pack.gain - 10f32.powf(-2.0 / 20.0)).abs() < 1e-5);
    for c in KeyClass::ALL {
        assert!(!pack.samples(c, KeyDir::Down).is_empty());
        assert!(!pack.samples(c, KeyDir::Up).is_empty());
    }
    assert_eq!(pack.samples(KeyClass::Default, KeyDir::Down).len(), 2);
    assert!(pack.info.derived.contains(&"modifier.down".to_string()));
    assert!(!pack.info.derived.contains(&"space.down".to_string()));
}

#[test]
fn load_errors() {
    let reg = PackRegistry::new(None, pack_fixture(""));
    assert_eq!(
        reg.load("user/nope", 48_000).err().unwrap().kind,
        PackErrorKind::NotFound
    );
    assert_eq!(
        reg.load("weird/x", 48_000).err().unwrap().kind,
        PackErrorKind::NotFound
    );
    assert_eq!(
        reg.load("user/../x", 48_000).err().unwrap().kind,
        PackErrorKind::NotFound
    );
    let e = reg.load("user/bad-too-long", 48_000).err().unwrap();
    assert!(e.message.contains("longer"), "{}", e.message);
    assert!(reg.load("user/bad-traversal", 48_000).is_err());
    assert!(reg.load("synth/click", 44_100).is_ok());
}

#[test]
fn import_copies_and_refuses_duplicate() {
    let user = TempDir::new("import");
    let mut reg = PackRegistry::new(None, user.0.clone());
    reg.rescan();
    let info = reg.import_dir(&pack_fixture("valid-full")).unwrap();
    assert_eq!(info.id, "user/valid-full");
    assert!(user.0.join("valid-full/sub/space-down-1.wav").is_file());
    assert!(reg.list().iter().any(|i| i.id == "user/valid-full"));
    assert!(reg.load("user/valid-full", 48_000).is_ok());
    let dup = reg.import_dir(&pack_fixture("valid-full")).err().unwrap();
    assert_eq!(dup.kind, PackErrorKind::Exists);
    // invalid packs are never copied
    assert!(reg.import_dir(&pack_fixture("bad-too-long")).is_err());
    assert!(!user.0.join("bad-too-long").exists());
}

/// Integration: every bundled pack in `src-tauri/resources/packs` is valid, CC0/public-domain,
/// credited, and fully loads. Skipped while the directory is empty.
#[test]
fn builtin_packs_valid() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/packs");
    let has_packs = std::fs::read_dir(&dir)
        .map(|rd| rd.flatten().any(|e| e.path().is_dir()))
        .unwrap_or(false);
    if !has_packs {
        eprintln!("no builtin packs at {}; skipping", dir.display());
        return;
    }
    let empty_user = TempDir::new("builtin");
    let mut reg = PackRegistry::new(Some(dir), empty_user.0.clone());
    let builtins: Vec<_> = reg
        .rescan()
        .iter()
        .filter(|i| i.kind == PackKind::Builtin)
        .cloned()
        .collect();
    assert!(!builtins.is_empty());
    for info in builtins {
        assert!(info.valid, "{}: {:?}", info.id, info.error);
        let pack = reg.load(&info.id, 48_000).unwrap_or_else(|e| panic!("{e}"));
        for c in KeyClass::ALL {
            assert!(
                !pack.samples(c, KeyDir::Down).is_empty(),
                "{} {}",
                info.id,
                c.as_str()
            );
        }
    }
}

/// Minimal 16-bit mono PCM WAV writer.
fn write_wav(path: &std::path::Path, rate: u32, samples: &[i16]) {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
}

#[test]
fn oversized_derived_pack_rejected_before_derivation() {
    // 16 x 1.9 s at 192 kHz decodes to ~23 MiB (under the 32 MiB decode cap); the derived sets
    // would push it far over, which must be caught from the lengths alone.
    let tmp = TempDir::new("oversize");
    let dir = tmp.0.join("big");
    std::fs::create_dir_all(&dir).unwrap();
    let n = (192_000.0 * 1.9) as usize;
    let samples: Vec<i16> = (0..n)
        .map(|i| ((i as f32 * 0.05).sin() * 8000.0) as i16)
        .collect();
    write_wav(&dir.join("a.wav"), 192_000, &samples);
    let list = vec!["\"a.wav\""; 16].join(", ");
    std::fs::write(
        dir.join("pack.toml"),
        format!(
            "schema = 1\nname = \"Big\"\nauthor = \"t\"\nlicense = \"CC0-1.0\"\n\
             [sounds.default]\ndown = [{list}]\n"
        ),
    )
    .unwrap();
    let reg = PackRegistry::new(None, tmp.0.clone());
    let err = reg.load("user/big", 48_000).err().unwrap();
    assert!(err.message.contains("derived"), "{}", err.message);
}

#[test]
fn oversized_manifest_rejected() {
    let tmp = TempDir::new("bigmanifest");
    let dir = tmp.0.join("p");
    copy_dir(&pack_fixture("valid-min"), &dir);
    let mut text = std::fs::read_to_string(dir.join("pack.toml")).unwrap();
    text.push_str(&format!("# {}\n", "x".repeat(70 * 1024)));
    std::fs::write(dir.join("pack.toml"), text).unwrap();
    let mut reg = PackRegistry::new(None, tmp.0.clone());
    let info = reg
        .rescan()
        .iter()
        .find(|i| i.id == "user/p")
        .cloned()
        .unwrap();
    assert!(!info.valid);
    assert!(info.error.unwrap().contains("64 KiB"));
}

#[test]
fn import_extras_checked() {
    let src_root = TempDir::new("extras-src");
    let src = src_root.0.join("with-extras");
    copy_dir(&pack_fixture("valid-min"), &src);
    std::fs::write(src.join("LICENSE.txt"), "CC0").unwrap();
    std::fs::write(src.join("CREDITS.md"), "x".repeat(2 * 1024 * 1024)).unwrap();
    let user = TempDir::new("extras-user");
    let mut reg = PackRegistry::new(None, user.0.clone());
    let err = reg.import_dir(&src).err().unwrap();
    assert!(err.message.contains("1 MiB"), "{}", err.message);
    assert!(!user.0.join("with-extras").exists());

    std::fs::remove_file(src.join("CREDITS.md")).unwrap();
    reg.import_dir(&src).unwrap();
    assert!(user.0.join("with-extras/LICENSE.txt").is_file());
    assert!(!user.0.join("with-extras/CREDITS.md").exists());
}

#[test]
fn import_refuses_symlinked_extra() {
    let src_root = TempDir::new("extras-link");
    let src = src_root.0.join("linked");
    copy_dir(&pack_fixture("valid-min"), &src);
    let target = src_root.0.join("secret.txt");
    std::fs::write(&target, "secret").unwrap();
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&target, src.join("LICENSE")).is_ok();
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(&target, src.join("LICENSE")).is_ok();
    if !made {
        eprintln!("symlink creation not permitted here; skipping");
        return;
    }
    let user = TempDir::new("extras-link-user");
    let mut reg = PackRegistry::new(None, user.0.clone());
    let err = reg.import_dir(&src).err().unwrap();
    assert!(err.message.contains("symlink"), "{}", err.message);
    assert!(!user.0.join("linked").exists());
}
