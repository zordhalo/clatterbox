//! PackCache LRU + load-with-fallback + preview scheduler (SPEC §5.4, §8.3).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use clatterbox_core::{
    KeyClass, KeyDir, KeySound, LoadedPack, PackError, PackRegistry, Settings, Trigger,
};
use tauri::{AppHandle, Manager};

use crate::state::AppState;

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
    /// Most-recently-used at the end.
    entries: Vec<Arc<LoadedPack>>,
}

impl PackCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(CACHE_CAPACITY),
        }
    }

    pub fn get(&mut self, id: &str) -> Option<Arc<LoadedPack>> {
        let idx = self.entries.iter().position(|p| p.info.id == id)?;
        let pack = self.entries.remove(idx);
        self.entries.push(pack.clone());
        Some(pack)
    }

    pub fn insert(&mut self, pack: Arc<LoadedPack>) {
        self.entries.retain(|p| p.info.id != pack.info.id);
        if self.entries.len() >= CACHE_CAPACITY {
            self.entries.remove(0);
        }
        self.entries.push(pack);
    }
}

impl Default for PackCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Loads `id` from the cache or registry; on failure (or if `id` is empty) falls back to
/// [`Settings::FALLBACK_PACK`], which always succeeds (procedural, no I/O).
pub fn load_with_fallback(
    registry: &PackRegistry,
    cache: &mut PackCache,
    id: &str,
    synth_rate: u32,
) -> (Arc<LoadedPack>, Option<PackError>) {
    if let Some(pack) = cache.get(id) {
        return (pack, None);
    }
    match registry.load(id, synth_rate) {
        Ok(pack) => {
            let pack = Arc::new(pack);
            cache.insert(pack.clone());
            (pack, None)
        }
        Err(err) => {
            if let Some(pack) = cache.get(Settings::FALLBACK_PACK) {
                return (pack, Some(err));
            }
            match registry.load(Settings::FALLBACK_PACK, synth_rate) {
                Ok(fallback) => {
                    let fallback = Arc::new(fallback);
                    cache.insert(fallback.clone());
                    (fallback, Some(err))
                }
                Err(_) => {
                    // The synth pack must never fail to load; if it somehow does there is
                    // nothing sane left to play, so surface the original error only.
                    unreachable!("synth fallback pack must always load")
                }
            }
        }
    }
}

/// Monotonically increasing preview generation; starting a new preview invalidates any
/// in-flight one (the spawned thread checks this before every push).
static PREVIEW_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Cancels any in-flight preview and schedules a fresh one on a background thread. Looks the
/// engine up through `app` on each step so the thread never needs to outlive `AppState`.
pub fn schedule_preview(app: AppHandle) {
    let generation = PREVIEW_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    thread::spawn(move || {
        for (i, (class, x)) in PREVIEW_PHRASE.iter().enumerate() {
            if PREVIEW_GENERATION.load(Ordering::SeqCst) != generation {
                return;
            }
            if i > 0 {
                thread::sleep(PREVIEW_KEY_INTERVAL);
            }
            if PREVIEW_GENERATION.load(Ordering::SeqCst) != generation {
                return;
            }
            let state = app.state::<AppState>();
            state.engine.preview(Trigger::preview(KeySound {
                class: *class,
                dir: KeyDir::Down,
                x: *x,
            }));

            thread::sleep(PREVIEW_UP_DELAY);
            if PREVIEW_GENERATION.load(Ordering::SeqCst) != generation {
                return;
            }
            let state = app.state::<AppState>();
            state.engine.preview(Trigger::preview(KeySound {
                class: *class,
                dir: KeyDir::Up,
                x: *x,
            }));
        }
    });
}
