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
    assert_eq!(&ids[..2], ["synth/click", "synth/thock"]);
    // user packs sorted by name: "Bad Missing File", "bad-schema" (invalid → dir name), "Valid Full", "Valid Min"
    assert_eq!(
        &ids[2..],
        [
            "user/bad-missing-file",
            "user/bad-schema",
            "user/valid-full",
            "user/valid-min"
        ]
    );
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
