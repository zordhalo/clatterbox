//! Managed application state (SPEC §8.2).
// WP0 stub: remove this allow once wired (WP1).
#![allow(dead_code)]

use std::sync::Arc;

use clatterbox_audio::AudioEngine;
use clatterbox_core::{EngineParams, PackRegistry, Settings};
use clatterbox_keyhook::HookHandle;
use parking_lot::Mutex;

use crate::packs::PackCache;
use crate::persist::Persister;

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub params: Arc<EngineParams>,
    pub engine: AudioEngine,
    pub hook: Mutex<Option<HookHandle>>,
    pub registry: Mutex<PackRegistry>,
    /// LRU(3) of `Arc<LoadedPack>`.
    pub loaded: Mutex<PackCache>,
    pub persister: Persister,
}
