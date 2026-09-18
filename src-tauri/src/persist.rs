//! Debounced background settings writer (SPEC §8.1): 500 ms debounce, last write wins.
// WP0 stub: remove this allow once implemented (WP1).
#![allow(unused_variables, dead_code)]

use std::path::PathBuf;
use std::time::Duration;

use clatterbox_core::Settings;

pub const DEBOUNCE: Duration = Duration::from_millis(500);

pub struct Persister {
    path: PathBuf,
}

impl Persister {
    pub fn new(path: PathBuf) -> Persister {
        todo!("WP1")
    }

    pub fn schedule(&self, settings: Settings) {
        todo!("WP1")
    }

    /// Blocks until any pending write is on disk (called on quit).
    pub fn flush(&self) {
        todo!("WP1")
    }
}
