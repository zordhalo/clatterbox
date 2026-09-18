//! Pack discovery, loading and import (SPEC §5.1, §5.4).
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

use std::path::{Path, PathBuf};

use super::{LoadedPack, PackError, PackInfo};

pub struct PackRegistry {
    builtin_dir: Option<PathBuf>,
    user_dir: PathBuf,
    infos: Vec<PackInfo>,
}

impl PackRegistry {
    pub fn new(builtin_dir: Option<PathBuf>, user_dir: PathBuf) -> Self {
        Self {
            builtin_dir,
            user_dir,
            infos: Vec::new(),
        }
    }

    /// Manifest-only parse, cheap.
    pub fn rescan(&mut self) -> &[PackInfo] {
        todo!("WP2")
    }

    /// Synth first, then builtin, then user; by name.
    pub fn list(&self) -> &[PackInfo] {
        &self.infos
    }

    pub fn load(&self, id: &str, synth_rate: u32) -> Result<LoadedPack, PackError> {
        todo!("WP2")
    }

    /// Validates `src` fully (decode included), then copies into user_dir/<dir name>.
    /// Fails if the id exists. Returns the new info.
    pub fn import_dir(&mut self, src: &Path) -> Result<PackInfo, PackError> {
        todo!("WP2")
    }
}
