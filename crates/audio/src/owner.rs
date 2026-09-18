//! Audio-owner thread: owns the (`!Send`) `cpal::Stream`, rebuilds on error / default-device
//! change, drops retired packs, reports `AudioStatus` (SPEC §4.2, §4.3).
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

use std::time::Duration;

/// Device-watch tick.
pub(crate) const TICK: Duration = Duration::from_secs(3);
/// Delay before rebuilding after a stream error.
pub(crate) const REBUILD_DELAY: Duration = Duration::from_millis(250);

pub(crate) enum OwnerMsg {
    StreamError(String),
    Shutdown,
}
