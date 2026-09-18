//! Pack discovery, loading and import (SPEC §5.1, §5.4).

use std::fs;
use std::path::{Path, PathBuf};

use super::decode::{MAX_PACK_F32_BYTES, decode_file};
use super::manifest::{self, Manifest};
use super::{LoadedPack, PackError, PackErrorKind, PackInfo, PackKind, SampleSet, derive};
use crate::mixer::math::db_to_gain;
use crate::synth::{self, SynthPreset};
use crate::types::KeyClass;

pub const MANIFEST_FILE: &str = "pack.toml";
pub const CREDITS_FILE: &str = "CREDITS.md";
/// Licenses allowed for builtin packs (SPEC §5.3).
pub const BUILTIN_LICENSES: [&str; 2] = ["CC0-1.0", "public-domain"];
/// Extra non-audio files copied on import when present.
const IMPORT_EXTRAS: [&str; 3] = ["LICENSE.txt", "LICENSE", CREDITS_FILE];

pub struct PackRegistry {
    builtin_dir: Option<PathBuf>,
    user_dir: PathBuf,
    infos: Vec<PackInfo>,
}

/// `^[a-z0-9][a-z0-9-]{0,47}$`
pub fn is_valid_dir_name(name: &str) -> bool {
    let b = name.as_bytes();
    !b.is_empty()
        && b.len() <= 48
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

fn prefix(kind: PackKind) -> &'static str {
    match kind {
        PackKind::Synth => "synth",
        PackKind::Builtin => "builtin",
        PackKind::User => "user",
    }
}

fn info_from_manifest(id: String, kind: PackKind, m: &Manifest) -> PackInfo {
    PackInfo {
        id,
        name: m.name.clone(),
        author: m.author.clone(),
        license: m.license.clone(),
        source_url: m.source_url.clone(),
        description: m.description.clone(),
        kind,
        valid: true,
        error: None,
        derived: m.derived_sets(),
    }
}

fn invalid_info(id: String, kind: PackKind, name: &str, err: String) -> PackInfo {
    PackInfo {
        id,
        name: name.to_owned(),
        author: String::new(),
        license: String::new(),
        source_url: None,
        description: None,
        kind,
        valid: false,
        error: Some(err),
        derived: Vec::new(),
    }
}

/// Reads and validates `pack.toml` (plus builtin-only rules). Manifest only, no decoding.
fn read_manifest(dir: &Path, kind: PackKind) -> Result<Manifest, String> {
    let text = fs::read_to_string(dir.join(MANIFEST_FILE))
        .map_err(|e| format!("cannot read {MANIFEST_FILE}: {e}"))?;
    let m = manifest::parse(&text)?;
    if kind == PackKind::Builtin {
        if !BUILTIN_LICENSES.contains(&m.license.as_str()) {
            return Err(format!(
                "builtin pack license must be one of {BUILTIN_LICENSES:?}"
            ));
        }
        if !dir.join(CREDITS_FILE).is_file() {
            return Err(format!("builtin pack is missing {CREDITS_FILE}"));
        }
    }
    Ok(m)
}

fn scan_dir(root: &Path, kind: PackKind) -> Vec<PackInfo> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_valid_dir_name(name) {
            tracing::warn!(dir = %path.display(), "skipping pack dir with invalid name");
            continue;
        }
        let id = format!("{}/{name}", prefix(kind));
        out.push(match read_manifest(&path, kind) {
            Ok(m) => info_from_manifest(id, kind, &m),
            Err(e) => invalid_info(id, kind, name, e),
        });
    }
    out.sort_by(|a, b| (a.name.to_lowercase(), &a.id).cmp(&(b.name.to_lowercase(), &b.id)));
    out
}

/// Fully loads a pack directory: manifest, every file decoded, sets derived.
pub fn load_dir(dir: &Path, id: &str, kind: PackKind) -> Result<LoadedPack, PackError> {
    let inv = |e: String| PackError::invalid(id, e);
    let m = read_manifest(dir, kind).map_err(inv)?;
    let mut sets: [SampleSet; KeyClass::COUNT] = Default::default();
    let mut bytes = 0usize;
    for c in KeyClass::ALL {
        let Some(files) = m.sounds.class(c) else {
            continue;
        };
        for (list, dst) in [(&files.down, 0), (&files.up, 1)] {
            for rel in list {
                let path = manifest::resolve_file(dir, rel).map_err(inv)?;
                let s = decode_file(&path).map_err(|e| inv(format!("{rel}: {e}")))?;
                bytes += s.data.len() * 4;
                if bytes > MAX_PACK_F32_BYTES {
                    return Err(inv("decoded audio exceeds 32 MiB".into()));
                }
                let set = &mut sets[c as usize];
                if dst == 0 {
                    set.down.push(s)
                } else {
                    set.up.push(s)
                }
            }
        }
    }
    let derived = derive::resolve(&mut sets, &m.class_gain_db());
    let mut info = info_from_manifest(id.to_owned(), kind, &m);
    info.derived = derived;
    let pack = LoadedPack::from_sets(info, db_to_gain(m.gain_db()), sets);
    if pack.f32_bytes() > MAX_PACK_F32_BYTES {
        return Err(inv(
            "decoded audio (incl. derived sets) exceeds 32 MiB".into()
        ));
    }
    Ok(pack)
}

impl PackRegistry {
    pub fn new(builtin_dir: Option<PathBuf>, user_dir: PathBuf) -> Self {
        Self {
            builtin_dir,
            user_dir,
            infos: Vec::new(),
        }
    }

    pub fn user_dir(&self) -> &Path {
        &self.user_dir
    }

    /// Manifest-only parse, cheap.
    pub fn rescan(&mut self) -> &[PackInfo] {
        let mut infos: Vec<PackInfo> = SynthPreset::ALL.iter().map(|p| p.info()).collect();
        infos.sort_by(|a, b| a.name.cmp(&b.name));
        if let Some(b) = &self.builtin_dir {
            infos.extend(scan_dir(b, PackKind::Builtin));
        }
        infos.extend(scan_dir(&self.user_dir, PackKind::User));
        self.infos = infos;
        &self.infos
    }

    /// Synth first, then builtin, then user; by name.
    pub fn list(&self) -> &[PackInfo] {
        &self.infos
    }

    fn dir_for(&self, id: &str) -> Result<(PathBuf, PackKind), PackError> {
        let not_found = || PackError::new(PackErrorKind::NotFound, id, "unknown pack id");
        let (kind_str, name) = id.split_once('/').ok_or_else(not_found)?;
        if !is_valid_dir_name(name) {
            return Err(not_found());
        }
        let (root, kind) = match kind_str {
            "builtin" => (
                self.builtin_dir.as_ref().ok_or_else(not_found)?,
                PackKind::Builtin,
            ),
            "user" => (&self.user_dir, PackKind::User),
            _ => return Err(not_found()),
        };
        let dir = root.join(name);
        if !dir.is_dir() {
            return Err(not_found());
        }
        Ok((dir, kind))
    }

    pub fn load(&self, id: &str, synth_rate: u32) -> Result<LoadedPack, PackError> {
        if let Some(p) = SynthPreset::ALL.into_iter().find(|p| p.id() == id) {
            return Ok(synth::generate(p, synth_rate));
        }
        let (dir, kind) = self.dir_for(id)?;
        load_dir(&dir, id, kind)
    }

    /// Validates `src` fully (decode included), then copies into user_dir/<dir name>.
    /// Fails if the id exists. Returns the new info.
    pub fn import_dir(&mut self, src: &Path) -> Result<PackInfo, PackError> {
        let name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        let id = format!("user/{name}");
        if !is_valid_dir_name(&name) {
            return Err(PackError::invalid(
                &id,
                "folder name must be lowercase letters, digits and '-' (max 48)",
            ));
        }
        if !src.is_dir() {
            return Err(PackError::new(
                PackErrorKind::NotFound,
                &id,
                "folder not found",
            ));
        }
        let dst = self.user_dir.join(&name);
        if dst.exists() {
            return Err(PackError::new(
                PackErrorKind::Exists,
                &id,
                "a pack with this id exists",
            ));
        }
        load_dir(src, &id, PackKind::User)?;
        let m = read_manifest(src, PackKind::User).map_err(|e| PackError::invalid(&id, e))?;
        copy_pack(src, &dst, &m).map_err(|e| {
            let _ = fs::remove_dir_all(&dst);
            PackError::new(PackErrorKind::Io, &id, format!("copy failed: {e}"))
        })?;
        self.rescan();
        self.infos
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or_else(|| PackError::new(PackErrorKind::Io, &id, "imported pack not found"))
    }
}

/// Copies `pack.toml`, every referenced audio file (keeping relative paths) and license/credit
/// files. Nothing else from the source folder is copied.
fn copy_pack(src: &Path, dst: &Path, m: &Manifest) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    fs::copy(src.join(MANIFEST_FILE), dst.join(MANIFEST_FILE))?;
    for extra in IMPORT_EXTRAS {
        let p = src.join(extra);
        if p.is_file() {
            fs::copy(&p, dst.join(extra))?;
        }
    }
    for c in KeyClass::ALL {
        let Some(files) = m.sounds.class(c) else {
            continue;
        };
        for rel in files.down.iter().chain(&files.up) {
            let from = manifest::resolve_file(src, rel).map_err(std::io::Error::other)?;
            let to = dst.join(rel);
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(from, to)?;
        }
    }
    Ok(())
}
