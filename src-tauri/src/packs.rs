//! PackCache LRU + load-with-fallback + preview scheduler (SPEC §5.4, §8.3).
// WP0 stub: remove this allow once implemented (WP1).
#![allow(unused_variables, dead_code)]

use std::sync::Arc;
use std::time::Duration;

use clatterbox_core::{KeyClass, LoadedPack};

pub const CACHE_CAPACITY: usize = 3;

/// Preview phrase (class, x); Down then Up 55 ms later; 110 ms between keys.
pub const PREVIEW_PHRASE: [(KeyClass, f32); 7] = [
    (KeyClass::Default, 0.25),
    (KeyClass::Default, 0.45),
    (KeyClass::Default, 0.60),
    (KeyClass::Space, 0.47),
    (KeyClass::Default, 0.70),
    (KeyClass::Backspace, 0.93),
    (KeyClass::Enter, 0.90),
];
pub const PREVIEW_UP_DELAY: Duration = Duration::from_millis(55);
pub const PREVIEW_KEY_INTERVAL: Duration = Duration::from_millis(110);

/// LRU of at most [`CACHE_CAPACITY`] loaded packs (current + recent previews).
pub struct PackCache {
    entries: Vec<Arc<LoadedPack>>,
}

impl PackCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(CACHE_CAPACITY),
        }
    }

    pub fn get(&mut self, id: &str) -> Option<Arc<LoadedPack>> {
        todo!("WP1")
    }

    pub fn insert(&mut self, pack: Arc<LoadedPack>) {
        todo!("WP1")
    }
}

impl Default for PackCache {
    fn default() -> Self {
        Self::new()
    }
}
