//! Clatterbox core: shared contract types, settings, engine params, mixer, packs, synth and
//! keymap. Pure Rust, no I/O beyond pack/settings files, no platform dependencies.
//!
//! Module declarations and re-exports only (WP0).

pub mod keymap;
pub mod mixer;
pub mod pack;
pub mod params;
pub mod repeat;
pub mod settings;
pub mod synth;
pub mod types;

pub use keymap::PhysKey;
pub use mixer::{MAX_VOICES, Mixer, MixerCmd};
pub use pack::registry::PackRegistry;
pub use pack::{LoadedPack, PackError, PackInfo, PackKind, Sample, SampleSet};
pub use params::EngineParams;
pub use repeat::RepeatFilter;
pub use settings::{LoadOutcome, Settings, SettingsPatch};
pub use synth::SynthPreset;
pub use types::{
    AudioStatus, HookStatus, KeyClass, KeyDir, KeySound, PackSlot, Trigger, TriggerTx,
};
