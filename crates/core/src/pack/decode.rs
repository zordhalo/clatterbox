//! Load-time audio decoding via symphonia (SPEC §5.3). Never used on the audio thread.

use std::fs::File;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use super::Sample;
use crate::mixer::math::db_to_gain;

pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_DURATION_S: f32 = 2.0;
pub const MIN_RATE: u32 = 8_000;
pub const MAX_RATE: u32 = 192_000;
pub const MAX_FILES_PER_SET: usize = 16;
pub const MAX_PACK_F32_BYTES: usize = 32 * 1024 * 1024;

/// Leading-silence threshold relative to the file's peak.
pub const LEAD_TRIM_DB: f32 = -30.0;
/// Pre-roll kept before the first sample above the lead threshold.
pub const LEAD_PREROLL_MS: f32 = 1.0;
/// Tail threshold relative to the file's peak.
pub const TAIL_TRIM_DB: f32 = -60.0;
pub const TAIL_KEEP_MS: f32 = 10.0;
pub const TAIL_FADE_MS: f32 = 5.0;
/// Peak level applied only to files that clip (peak > 0 dBFS).
pub const CLIP_NORMALIZE_DBFS: f32 = -1.0;

/// Decode → mono f32 → trim leading silence → trim tail → peak-normalize only if clipping.
/// Decoder panics are caught and reported as errors. Error messages do not include the path.
pub fn decode_file(path: &Path) -> Result<Sample, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_FILE_BYTES {
        return Err("larger than 4 MiB".into());
    }
    let (mono, rate) = catch_unwind(AssertUnwindSafe(|| decode_mono(path)))
        .map_err(|_| "decoder panicked".to_owned())??;
    let data = process(mono, rate)?;
    Ok(Sample {
        data: data.into_boxed_slice(),
        rate,
    })
}

/// Raw decode of the first audio track, channels averaged to mono.
fn decode_mono(path: &Path) -> Result<(Vec<f32>, u32), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("unrecognized audio: {e}"))?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or("no audio track")?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or("no audio codec parameters")?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| format!("unsupported codec: {e}"))?;

    let mut mono: Vec<f32> = Vec::new();
    let mut rate = 0u32;
    let mut scratch: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(SymError::ResetRequired) => break,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("read error: {e}")),
        };
        if packet.track_id != track_id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(b) => b,
            Err(SymError::DecodeError(_)) => continue,
            Err(e) => return Err(format!("decode error: {e}")),
        };
        let spec = buf.spec();
        rate = spec.rate();
        if !(MIN_RATE..=MAX_RATE).contains(&rate) {
            return Err(format!(
                "sample rate {rate} outside {MIN_RATE}..={MAX_RATE}"
            ));
        }
        let ch = spec.channels().count().max(1);
        buf.copy_to_vec_interleaved(&mut scratch);
        let inv = 1.0 / ch as f32;
        mono.extend(
            scratch
                .chunks_exact(ch)
                .map(|f| f.iter().sum::<f32>() * inv),
        );
        if mono.len() as f32 > MAX_DURATION_S * rate as f32 {
            return Err(format!("longer than {MAX_DURATION_S} s"));
        }
    }
    if mono.is_empty() || rate == 0 {
        return Err("no audio decoded".into());
    }
    Ok((mono, rate))
}

fn ms_to_samples(ms: f32, rate: u32) -> usize {
    (ms * 0.001 * rate as f32).round() as usize
}

/// Trim + normalize a decoded mono buffer (SPEC §5.3).
pub(crate) fn process(mut data: Vec<f32>, rate: u32) -> Result<Vec<f32>, String> {
    for x in data.iter_mut() {
        if !x.is_finite() {
            *x = 0.0;
        }
    }
    let peak = data.iter().fold(0f32, |m, x| m.max(x.abs()));
    if peak <= 0.0 {
        return Err("file is silent".into());
    }
    let lead = peak * db_to_gain(LEAD_TRIM_DB);
    let first = data.iter().position(|x| x.abs() > lead).unwrap_or(0);
    let start = first.saturating_sub(ms_to_samples(LEAD_PREROLL_MS, rate));

    let tail = peak * db_to_gain(TAIL_TRIM_DB);
    let last = data
        .iter()
        .rposition(|x| x.abs() > tail)
        .unwrap_or(data.len() - 1);
    let end = (last + 1 + ms_to_samples(TAIL_KEEP_MS, rate)).min(data.len());
    let trimmed_tail = end < data.len();

    data.truncate(end);
    data.drain(..start);
    if trimmed_tail {
        fade_out_linear(&mut data, ms_to_samples(TAIL_FADE_MS, rate));
    }
    if peak > 1.0 {
        let g = db_to_gain(CLIP_NORMALIZE_DBFS) / peak;
        data.iter_mut().for_each(|x| *x *= g);
    }
    Ok(data)
}

/// Linear fade to zero over the last `n` samples.
pub(crate) fn fade_out_linear(data: &mut [f32], n: usize) {
    let n = n.min(data.len());
    let len = data.len();
    for (i, x) in data[len - n..].iter_mut().enumerate() {
        *x *= 1.0 - (i + 1) as f32 / n as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_lead_and_tail() {
        let rate = 48_000;
        let mut v = vec![0.0f32; 4800]; // 100 ms silence
        v.extend((0..480).map(|i| (i as f32 * 0.3).sin() * 0.5));
        v.extend(vec![0.0; 9600]);
        let out = process(v, rate).unwrap();
        let first = out
            .iter()
            .position(|x| x.abs() > 0.5 * db_to_gain(-30.0))
            .unwrap();
        assert!(first <= 48 + 1, "first = {first}");
        assert!(out.len() < 480 + 48 + 480 + 10);
        assert_eq!(*out.last().unwrap(), 0.0);
    }

    #[test]
    fn normalizes_only_clipping() {
        let out = process(vec![0.2, -0.5, 0.1], 8000).unwrap();
        assert!(out.iter().any(|&x| x == -0.5));
        let out = process(vec![0.2, -2.0, 0.1], 8000).unwrap();
        let peak = out.iter().fold(0f32, |m, x| m.max(x.abs()));
        assert!((peak - db_to_gain(-1.0)).abs() < 1e-4);
        assert!(process(vec![0.0; 10], 8000).is_err());
    }
}
