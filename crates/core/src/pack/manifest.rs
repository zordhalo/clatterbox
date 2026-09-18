//! `pack.toml` schema + path validation (SPEC §5.2, §5.3).

use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use super::decode::MAX_FILES_PER_SET;
use crate::types::KeyClass;

pub const SCHEMA_VERSION: u32 = 1;
pub const GAIN_DB_MIN: f32 = -24.0;
pub const GAIN_DB_MAX: f32 = 12.0;
pub const AUDIO_EXTENSIONS: [&str; 4] = ["wav", "ogg", "flac", "mp3"];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub name: String,
    pub author: String,
    pub license: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub gain_db: Option<f32>,
    pub sounds: Sounds,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sounds {
    pub default: SoundFiles,
    #[serde(default)]
    pub space: Option<SoundFiles>,
    #[serde(default)]
    pub enter: Option<SoundFiles>,
    #[serde(default)]
    pub backspace: Option<SoundFiles>,
    #[serde(default)]
    pub modifier: Option<SoundFiles>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundFiles {
    #[serde(default)]
    pub down: Vec<String>,
    #[serde(default)]
    pub up: Vec<String>,
    /// Optional per-class trim, clamped to `-24.0..=12.0` (loudness-match mixed sources).
    #[serde(default)]
    pub gain_db: Option<f32>,
}

impl Sounds {
    /// The `[sounds.<class>]` table, if present.
    pub fn class(&self, class: KeyClass) -> Option<&SoundFiles> {
        match class {
            KeyClass::Default => Some(&self.default),
            KeyClass::Space => self.space.as_ref(),
            KeyClass::Enter => self.enter.as_ref(),
            KeyClass::Backspace => self.backspace.as_ref(),
            KeyClass::Modifier => self.modifier.as_ref(),
        }
    }
}

impl Manifest {
    /// Pack gain in dB (0 if absent; already clamped by `parse`).
    pub fn gain_db(&self) -> f32 {
        self.gain_db.unwrap_or(0.0)
    }

    /// Per-class gain in dB, indexed by `KeyClass as usize`.
    pub fn class_gain_db(&self) -> [f32; KeyClass::COUNT] {
        KeyClass::ALL.map(|c| self.sounds.class(c).and_then(|f| f.gain_db).unwrap_or(0.0))
    }

    /// Names of sets that will be derived at load (SPEC §5.2.1), e.g. `["space.down"]`.
    pub fn derived_sets(&self) -> Vec<String> {
        let has = |c: KeyClass, up: bool| {
            self.sounds
                .class(c)
                .is_some_and(|f| !if up { &f.up } else { &f.down }.is_empty())
        };
        let mut out = Vec::new();
        for up in [false, true] {
            for c in KeyClass::ALL {
                if !has(c, up) {
                    out.push(format!("{}.{}", c.as_str(), if up { "up" } else { "down" }));
                }
            }
        }
        out
    }
}

fn clamp_gain(g: &mut Option<f32>) {
    if let Some(v) = g {
        *v = if v.is_finite() {
            v.clamp(GAIN_DB_MIN, GAIN_DB_MAX)
        } else {
            0.0
        };
    }
}

fn check_len(field: &str, v: &str, min: usize, max: usize) -> Result<(), String> {
    let n = v.chars().count();
    if n < min || n > max || (min > 0 && v.trim().is_empty()) {
        return Err(format!("`{field}` must be {min}..={max} characters"));
    }
    Ok(())
}

/// SPDX-like expression charset, or "public-domain".
fn check_license(l: &str) -> Result<(), String> {
    let ok = !l.is_empty()
        && l.len() <= 64
        && l.chars()
            .all(|c| c.is_ascii_alphanumeric() || " .+-()".contains(c));
    if ok {
        Ok(())
    } else {
        Err(format!("invalid `license` {l:?}"))
    }
}

/// Syntactic checks on a manifest-relative path: relative, no `..`, no root/drive prefix,
/// supported extension.
pub fn check_rel_path(rel: &str) -> Result<(), String> {
    if rel.is_empty() || rel.contains('\0') {
        return Err("empty file path".into());
    }
    if rel.starts_with('/') || rel.starts_with('\\') || rel.contains(':') {
        return Err(format!("absolute path not allowed: {rel:?}"));
    }
    for comp in Path::new(rel).components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            _ => return Err(format!("path escapes pack dir: {rel:?}")),
        }
    }
    if rel.split(['/', '\\']).any(|seg| seg == "..") {
        return Err(format!("path escapes pack dir: {rel:?}"));
    }
    let ext = Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!("unsupported file type: {rel:?}"));
    }
    Ok(())
}

/// Parse + validate field rules (lengths, schema, source_url scheme, gain clamp).
pub fn parse(text: &str) -> Result<Manifest, String> {
    let mut m: Manifest = toml::from_str(text).map_err(|e| format!("pack.toml: {e}"))?;
    if m.schema != SCHEMA_VERSION {
        return Err(format!("unsupported schema {} (expected 1)", m.schema));
    }
    check_len("name", &m.name, 1, 48)?;
    check_len("author", &m.author, 1, 96)?;
    check_license(&m.license)?;
    if let Some(u) = &m.source_url
        && (!u.starts_with("https://") || u.len() > 512 || u.chars().any(char::is_whitespace))
    {
        return Err("`source_url` must be an https:// URL".into());
    }
    if let Some(d) = &m.description {
        check_len("description", d, 0, 200)?;
    }
    clamp_gain(&mut m.gain_db);
    if m.sounds.default.down.is_empty() {
        return Err("`[sounds.default] down` must not be empty".into());
    }
    for c in KeyClass::ALL {
        let files = match c {
            KeyClass::Default => Some(&mut m.sounds.default),
            KeyClass::Space => m.sounds.space.as_mut(),
            KeyClass::Enter => m.sounds.enter.as_mut(),
            KeyClass::Backspace => m.sounds.backspace.as_mut(),
            KeyClass::Modifier => m.sounds.modifier.as_mut(),
        };
        let Some(files) = files else { continue };
        clamp_gain(&mut files.gain_db);
        for (dir, list) in [("down", &files.down), ("up", &files.up)] {
            if list.len() > MAX_FILES_PER_SET {
                return Err(format!(
                    "{}.{dir}: at most {MAX_FILES_PER_SET} files",
                    c.as_str()
                ));
            }
            for rel in list {
                check_rel_path(rel)?;
            }
        }
    }
    Ok(m)
}

/// Resolve a manifest-relative file path; must stay under the canonicalized `pack_dir`
/// (blocks `..`, absolute paths and symlink escapes). The file must exist.
pub fn resolve_file(pack_dir: &Path, rel: &str) -> Result<PathBuf, String> {
    check_rel_path(rel)?;
    let root = pack_dir
        .canonicalize()
        .map_err(|e| format!("pack dir: {e}"))?;
    let full = root
        .join(rel)
        .canonicalize()
        .map_err(|_| format!("missing file: {rel:?}"))?;
    if !full.starts_with(&root) {
        return Err(format!("path escapes pack dir: {rel:?}"));
    }
    if !full.is_file() {
        return Err(format!("not a file: {rel:?}"));
    }
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: &str = r#"
schema = 1
name = "Min"
author = "Me"
license = "CC0-1.0"
[sounds.default]
down = ["a.wav"]
"#;

    #[test]
    fn parses_min_and_lists_derived() {
        let m = parse(MIN).unwrap();
        assert_eq!(m.derived_sets().len(), 9);
        assert_eq!(m.derived_sets()[0], "space.down");
    }

    #[test]
    fn rejects_bad_fields() {
        let bad = |from: &str, to: &str| parse(&MIN.replace(from, to)).is_err();
        assert!(bad("schema = 1", "schema = 2"));
        assert!(bad("name = \"Min\"", "name = \"\""));
        assert!(bad(
            "[sounds.default]",
            "[sounds.tab]\ndown=[\"a.wav\"]\n[sounds.default]"
        ));
        assert!(bad("schema = 1", "schema = 1\nextra = 3"));
        assert!(bad("down = [\"a.wav\"]", "down = []"));
        assert!(bad("a.wav", "../a.wav"));
        assert!(bad("a.wav", "/etc/a.wav"));
        assert!(bad("a.wav", "C:\\\\a.wav"));
        assert!(bad("a.wav", "a.txt"));
        assert!(!bad("a.wav", "sub/A.WAV"));
        assert!(bad("license", "source_url = \"http://x\"\nlicense"));
        assert!(!bad("license", "source_url = \"https://x.org/a\"\nlicense"));
    }

    #[test]
    fn clamps_gain() {
        let m = parse(&MIN.replace("schema = 1", "schema = 1\ngain_db = 40.0")).unwrap();
        assert_eq!(m.gain_db(), GAIN_DB_MAX);
        let m = parse(&MIN.replace("down = [\"a.wav\"]", "down = [\"a.wav\"]\ngain_db = -99.0"))
            .unwrap();
        assert_eq!(m.class_gain_db()[0], GAIN_DB_MIN);
    }
}
