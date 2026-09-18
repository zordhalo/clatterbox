mod common;

use clatterbox_core::pack::manifest::{self, GAIN_DB_MAX};
use common::*;

fn parse_fixture(name: &str) -> Result<manifest::Manifest, String> {
    manifest::parse(&std::fs::read_to_string(pack_fixture(name).join("pack.toml")).unwrap())
}

const HEADER: &str = "schema = 1\nname = \"x\"\nauthor = \"y\"\nlicense = \"CC0-1.0\"\n";

#[test]
fn valid_fixtures_parse() {
    let m = parse_fixture("valid-min").unwrap();
    assert_eq!(m.name, "Valid Min");
    let m = parse_fixture("valid-full").unwrap();
    assert_eq!(m.gain_db(), -2.0);
    assert_eq!(m.class_gain_db(), [0.0, 1.5, -3.0, 0.0, 0.0]);
    assert_eq!(
        m.derived_sets(),
        [
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
fn bad_schema_and_traversal_rejected() {
    assert!(parse_fixture("bad-schema").unwrap_err().contains("schema"));
    assert!(parse_fixture("bad-traversal").is_err());
}

#[test]
fn missing_default_down_and_unknown_class_rejected() {
    assert!(manifest::parse(&format!("{HEADER}[sounds.default]\nup = [\"a.wav\"]\n")).is_err());
    let unknown =
        format!("{HEADER}[sounds.default]\ndown = [\"a.wav\"]\n[sounds.tab]\ndown = [\"b.wav\"]\n");
    assert!(manifest::parse(&unknown).is_err());
    let unknown_key = format!("{HEADER}color = 1\n[sounds.default]\ndown = [\"a.wav\"]\n");
    assert!(manifest::parse(&unknown_key).is_err());
}

#[test]
fn gain_clamped() {
    let text = format!("{HEADER}gain_db = 99.0\n[sounds.default]\ndown = [\"a.wav\"]\n");
    assert_eq!(manifest::parse(&text).unwrap().gain_db(), GAIN_DB_MAX);
}

#[test]
fn bad_source_url_rejected() {
    let text = format!("{HEADER}source_url = \"ftp://a\"\n[sounds.default]\ndown = [\"a.wav\"]\n");
    assert!(manifest::parse(&text).is_err());
}

#[test]
fn resolve_file_checks() {
    let dir = pack_fixture("valid-full");
    assert!(manifest::resolve_file(&dir, "sub/space-down-1.wav").is_ok());
    assert!(manifest::resolve_file(&dir, "nope.wav").is_err());
    assert!(manifest::resolve_file(&dir, "../valid-min/a.wav").is_err());
    let abs = pack_fixture("valid-min").join("a.wav");
    assert!(manifest::resolve_file(&dir, abs.to_str().unwrap()).is_err());
}

#[test]
fn symlink_escape_rejected() {
    let tmp = TempDir::new("symlink");
    let pack = tmp.0.join("p");
    std::fs::create_dir_all(&pack).unwrap();
    let target = pack_fixture("valid-min").join("a.wav");
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&target, pack.join("link.wav")).is_ok();
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(&target, pack.join("link.wav")).is_ok();
    if !made {
        eprintln!("symlink creation not permitted here; skipping");
        return;
    }
    let err = manifest::resolve_file(&pack, "link.wav").unwrap_err();
    assert!(err.contains("escapes"), "{err}");
}
