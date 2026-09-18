//! Load-time audio decoding via symphonia (SPEC §5.3). Never used on the audio thread.
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

use std::path::Path;

use super::Sample;

pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_DURATION_S: f32 = 2.0;
pub const MIN_RATE: u32 = 8_000;
pub const MAX_RATE: u32 = 192_000;
pub const MAX_FILES_PER_SET: usize = 16;
pub const MAX_PACK_F32_BYTES: usize = 32 * 1024 * 1024;
/// Allowed file extensions, compared case-insensitively.
pub const EXTENSIONS: [&str; 4] = ["wav", "ogg", "flac", "mp3"];

/// Decode → mono f32 → trim leading silence (−30 dB rel. peak, 1 ms pre-roll) → trim tail
/// (−60 dB rel. peak, keep 10 ms, 5 ms fade) → peak-normalize only if clipping.
pub fn decode_file(path: &Path) -> Result<Sample, String> {
    todo!("WP2")
}
