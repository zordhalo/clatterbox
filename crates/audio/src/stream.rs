//! Stream config negotiation, the RT data callback and sample-format conversion (SPEC §4.2).
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

use std::sync::Arc;

use clatterbox_core::{LoadedPack, Mixer, MixerCmd, Trigger};

/// Requested fixed buffer size in frames.
pub const TARGET_BUFFER_FRAMES: u32 = 256;

/// Result of config negotiation. Pure data so it is unit-testable without a device.
pub struct Choice {
    pub config: cpal::StreamConfig,
    pub format: cpal::SampleFormat,
    /// `true` if `config.buffer_size` is `Fixed(TARGET_BUFFER_FRAMES)`.
    pub fixed_buffer: bool,
}

/// Prefer F32, 2 channels, default rate; Fixed(256) if within range.
pub fn choose_config(supported: &[cpal::SupportedStreamConfigRange]) -> Option<Choice> {
    todo!("WP2")
}

/// Everything the data callback touches, behind `Arc<parking_lot::Mutex<_>>` (`try_lock` only).
pub(crate) struct MixerHost {
    pub(crate) mixer: Mixer,
    pub(crate) hook_rx: rtrb::Consumer<Trigger>,
    pub(crate) preview_rx: rtrb::Consumer<Trigger>,
    pub(crate) control_rx: rtrb::Consumer<MixerCmd>,
    pub(crate) garbage_tx: rtrb::Producer<Arc<LoadedPack>>,
}
