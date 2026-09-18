//! Persistent settings: JSON on disk and over IPC (SPEC §8.1).

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Missing fields → defaults; unknown fields ignored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Always `1` (future migrations switch on it).
    pub version: u32,
    pub enabled: bool,
    /// `0.0..=1.0`
    pub volume: f32,
    /// Pack id, e.g. `"builtin/classic"`, `"synth/thock"`.
    pub pack: String,
    pub key_up_enabled: bool,
    /// `0.0..=1.0`
    pub pitch_variation: f32,
    pub spatial_enabled: bool,
    /// `0.0..=1.0`
    pub spatial_width: f32,
    pub launch_at_login: bool,
}

impl Settings {
    pub const VERSION: u32 = 1;
    /// Default pack per SPEC §16.3. Falls back to [`Settings::FALLBACK_PACK`] if it fails to load.
    pub const DEFAULT_PACK: &'static str = "builtin/classic";
    pub const FALLBACK_PACK: &'static str = "synth/thock";

    /// Clamp ranges, NaN → default, version → 1.
    pub fn sanitize(&mut self) {
        self.version = Self::VERSION;
        self.volume = sanitize_unit(self.volume, Self::default().volume);
        self.pitch_variation = sanitize_unit(self.pitch_variation, Self::default().pitch_variation);
        self.spatial_width = sanitize_unit(self.spatial_width, Self::default().spatial_width);
        if self.pack.trim().is_empty() {
            self.pack = Self::DEFAULT_PACK.to_owned();
        }
    }

    /// Applies only the `Some` fields of `patch`; returns a sanitized copy.
    pub fn merged(&self, patch: &SettingsPatch) -> Settings {
        let mut next = self.clone();
        if let Some(v) = patch.enabled {
            next.enabled = v;
        }
        if let Some(v) = patch.volume {
            next.volume = v;
        }
        if let Some(v) = &patch.pack {
            next.pack = v.clone();
        }
        if let Some(v) = patch.key_up_enabled {
            next.key_up_enabled = v;
        }
        if let Some(v) = patch.pitch_variation {
            next.pitch_variation = v;
        }
        if let Some(v) = patch.spatial_enabled {
            next.spatial_enabled = v;
        }
        if let Some(v) = patch.spatial_width {
            next.spatial_width = v;
        }
        if let Some(v) = patch.launch_at_login {
            next.launch_at_login = v;
        }
        next.sanitize();
        next
    }
}

/// Clamps to `0.0..=1.0`; NaN (or otherwise non-finite) falls back to `default`.
fn sanitize_unit(value: f32, default: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        default
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            enabled: true,
            volume: 0.6,
            pack: Self::DEFAULT_PACK.to_owned(),
            key_up_enabled: true,
            pitch_variation: 0.35,
            spatial_enabled: true,
            spatial_width: 0.6,
            launch_at_login: false,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsPatch {
    pub enabled: Option<bool>,
    pub volume: Option<f32>,
    pub pack: Option<String>,
    pub key_up_enabled: Option<bool>,
    pub pitch_variation: Option<f32>,
    pub spatial_enabled: Option<bool>,
    pub spatial_width: Option<f32>,
    pub launch_at_login: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadOutcome {
    Loaded,
    FirstRun,
    Recovered,
}

/// Missing → default + `FirstRun`; parse error → rename to `settings.json.corrupt-<unix_ts>`,
/// default + `Recovered`.
pub fn load(path: &Path) -> (Settings, LoadOutcome) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return (Settings::default(), LoadOutcome::FirstRun);
        }
        Err(_) => {
            recover_corrupt(path);
            return (Settings::default(), LoadOutcome::Recovered);
        }
    };

    match serde_json::from_str::<Settings>(&content) {
        Ok(mut settings) => {
            settings.sanitize();
            (settings, LoadOutcome::Loaded)
        }
        Err(_) => {
            recover_corrupt(path);
            (Settings::default(), LoadOutcome::Recovered)
        }
    }
}

/// Renames the unreadable/corrupt file out of the way; best-effort (a failed rename still
/// leaves the caller free to write a fresh default file over `path`).
fn recover_corrupt(path: &Path) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let corrupt_path = path.with_file_name(format!(
        "{}.corrupt-{ts}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("settings.json")
    ));
    let _ = fs::rename(path, corrupt_path);
}

/// Write `settings.json.tmp`, fsync, rename.
pub fn save_atomic(path: &Path, s: &Settings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_file_name(format!(
        "{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("settings.json")
    ));
    let json = serde_json::to_string_pretty(s)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert!(back == s);
    }

    #[test]
    fn missing_fields_become_defaults() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert!(s == Settings::default());
    }

    #[test]
    fn unknown_fields_ignored() {
        let s: Settings = serde_json::from_str(r#"{"enabled": false, "bogus": 42}"#).unwrap();
        assert!(!s.enabled);
    }

    #[test]
    fn sanitize_clamps_and_fixes_nan() {
        let mut s = Settings {
            version: 99,
            volume: 5.0,
            pitch_variation: f32::NAN,
            spatial_width: -3.0,
            pack: "   ".to_owned(),
            ..Settings::default()
        };
        s.sanitize();
        assert!(s.version == Settings::VERSION);
        assert!(s.volume == 1.0);
        assert!(s.pitch_variation == Settings::default().pitch_variation);
        assert!(s.spatial_width == 0.0);
        assert!(s.pack == Settings::DEFAULT_PACK);
    }

    #[test]
    fn merged_applies_only_some_fields() {
        let base = Settings::default();
        let patch = SettingsPatch {
            volume: Some(0.9),
            ..SettingsPatch::default()
        };
        let merged = base.merged(&patch);
        assert!(merged.volume == 0.9);
        assert!(merged.enabled == base.enabled);
        assert!(merged.pack == base.pack);
    }

    #[test]
    fn load_missing_file_is_first_run() {
        let dir = std::env::temp_dir().join(format!("clatterbox-test-{}", fastrand_seed()));
        let path = dir.join("settings.json");
        let (settings, outcome) = load(&path);
        assert!(outcome == LoadOutcome::FirstRun);
        assert!(settings == Settings::default());
    }

    #[test]
    fn load_corrupt_file_renames_and_recovers() {
        let dir = std::env::temp_dir().join(format!("clatterbox-test-{}", fastrand_seed()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        fs::write(&path, b"{not valid json").unwrap();

        let (settings, outcome) = load(&path);
        assert!(outcome == LoadOutcome::Recovered);
        assert!(settings == Settings::default());
        assert!(!path.exists());

        let mut renamed = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned());
        assert!(renamed.any(|n| n.starts_with("settings.json.corrupt-")));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_atomic_leaves_no_tmp_file() {
        let dir = std::env::temp_dir().join(format!("clatterbox-test-{}", fastrand_seed()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        save_atomic(&path, &Settings::default()).unwrap();

        assert!(path.exists());
        assert!(!path.with_file_name("settings.json.tmp").exists());

        let (loaded, outcome) = load(&path);
        assert!(outcome == LoadOutcome::Loaded);
        assert!(loaded == Settings::default());

        let _ = fs::remove_dir_all(&dir);
    }

    fn fastrand_seed() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
            ^ (std::process::id() as u64)
    }
}
