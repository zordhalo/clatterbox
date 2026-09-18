//! Stream config negotiation, the RT data callback and sample-format conversion (SPEC §4.2).

use std::sync::Arc;
use std::sync::mpsc::Sender;

use clatterbox_core::{LoadedPack, Mixer, MixerCmd, Trigger};
use cpal::traits::DeviceTrait;
use cpal::{BufferSize, SampleFormat, SizedSample, StreamConfig, SupportedBufferSize};
use parking_lot::Mutex;

use crate::owner::OwnerMsg;

/// Requested fixed buffer size in frames.
pub const TARGET_BUFFER_FRAMES: u32 = 256;
/// Fallback when the device's default rate is unsupported by a range.
const PREFERRED_RATE: u32 = 48_000;
/// Scratch buffer the callback renders into before format conversion (samples).
const SCRATCH_SAMPLES: usize = 8192;
/// Retired packs the callback can hold while the garbage queue is full.
const PENDING: usize = 4;

/// Result of config negotiation. Pure data so it is unit-testable without a device.
pub struct Choice {
    pub config: cpal::StreamConfig,
    pub format: cpal::SampleFormat,
    /// `true` if `config.buffer_size` is `Fixed(TARGET_BUFFER_FRAMES)`.
    pub fixed_buffer: bool,
}

fn format_rank(f: SampleFormat) -> Option<u8> {
    match f {
        SampleFormat::F32 => Some(0),
        SampleFormat::I32 => Some(1),
        SampleFormat::I16 => Some(2),
        _ => None,
    }
}

fn channel_rank(ch: u16) -> Option<u8> {
    match ch {
        0 => None,
        2 => Some(0),
        1 => Some(2),
        _ => Some(1),
    }
}

/// Prefer F32 (then I32, I16), 2 channels (then more, then mono), the device's default rate
/// (then 48 kHz, then the nearest supported); `Fixed(256)` if within the supported range.
pub fn choose_config(
    supported: &[cpal::SupportedStreamConfigRange],
    default_rate: u32,
) -> Option<Choice> {
    let rate_for = |r: &cpal::SupportedStreamConfigRange| -> (u8, u32) {
        let (lo, hi) = (r.min_sample_rate(), r.max_sample_rate());
        if (lo..=hi).contains(&default_rate) {
            (0, default_rate)
        } else if (lo..=hi).contains(&PREFERRED_RATE) {
            (1, PREFERRED_RATE)
        } else {
            (2, PREFERRED_RATE.clamp(lo, hi))
        }
    };
    let best = supported
        .iter()
        .filter_map(|r| {
            let key = (
                format_rank(r.sample_format())?,
                channel_rank(r.channels())?,
                rate_for(r).0,
            );
            Some((key, r))
        })
        .min_by_key(|(key, _)| *key)?
        .1;
    let fixed_buffer = matches!(best.buffer_size(),
        SupportedBufferSize::Range { min, max } if (*min..=*max).contains(&TARGET_BUFFER_FRAMES));
    Some(Choice {
        config: StreamConfig {
            channels: best.channels(),
            sample_rate: rate_for(best).1,
            buffer_size: if fixed_buffer {
                BufferSize::Fixed(TARGET_BUFFER_FRAMES)
            } else {
                BufferSize::Default
            },
        },
        format: best.sample_format(),
        fixed_buffer,
    })
}

#[inline]
pub fn f32_to_i16(x: f32) -> i16 {
    (x.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
}

#[inline]
pub fn f32_to_i32(x: f32) -> i32 {
    (f64::from(x.clamp(-1.0, 1.0)) * f64::from(i32::MAX)).round() as i32
}

/// Everything the data callback touches, behind `Arc<parking_lot::Mutex<_>>` (`try_lock` only).
pub(crate) struct MixerHost {
    pub(crate) mixer: Mixer,
    pub(crate) hook_rx: rtrb::Consumer<Trigger>,
    pub(crate) preview_rx: rtrb::Consumer<Trigger>,
    pub(crate) control_rx: rtrb::Consumer<MixerCmd>,
    pub(crate) garbage_tx: rtrb::Producer<Arc<LoadedPack>>,
    /// Retired packs waiting for garbage-queue space (never dropped in the callback).
    pending: [Option<Arc<LoadedPack>>; PENDING],
    scratch: Box<[f32]>,
}

impl MixerHost {
    pub(crate) fn new(
        mixer: Mixer,
        hook_rx: rtrb::Consumer<Trigger>,
        preview_rx: rtrb::Consumer<Trigger>,
        control_rx: rtrb::Consumer<MixerCmd>,
        garbage_tx: rtrb::Producer<Arc<LoadedPack>>,
    ) -> Self {
        Self {
            mixer,
            hook_rx,
            preview_rx,
            control_rx,
            garbage_tx,
            pending: Default::default(),
            scratch: vec![0.0; SCRATCH_SAMPLES].into_boxed_slice(),
        }
    }

    fn pending_free(&self) -> usize {
        self.pending.iter().filter(|p| p.is_none()).count()
    }

    /// Hands a retired pack to the owner thread, or parks it. Callers guarantee a free slot.
    fn retire(&mut self, pack: Arc<LoadedPack>) {
        if let Err(rtrb::PushError::Full(pack)) = self.garbage_tx.push(pack)
            && let Some(slot) = self.pending.iter_mut().find(|p| p.is_none())
        {
            *slot = Some(pack);
        }
    }

    /// Applies queued commands and triggers, renders `out`, and retires finished packs.
    /// Realtime-safe: no allocation, no locking, no logging.
    pub(crate) fn process<T: Copy>(&mut self, out: &mut [T], channels: usize, conv: fn(f32) -> T) {
        for i in 0..PENDING {
            if let Some(p) = self.pending[i].take()
                && let Err(rtrb::PushError::Full(p)) = self.garbage_tx.push(p)
            {
                self.pending[i] = Some(p);
                break;
            }
        }
        // A command can retire at most one pack immediately; stop while no slot is free.
        while self.pending_free() > 0 {
            let Ok(cmd) = self.control_rx.pop() else {
                break;
            };
            if let Some(old) = self.mixer.apply(cmd) {
                self.retire(old);
            }
        }
        while let Ok(t) = self.preview_rx.pop() {
            self.mixer.trigger(t);
        }
        while let Ok(t) = self.hook_rx.pop() {
            self.mixer.trigger(t);
        }
        let channels = channels.max(1);
        let chunk = (SCRATCH_SAMPLES / channels).max(1) * channels;
        for dst in out.chunks_mut(chunk) {
            let buf = &mut self.scratch[..dst.len()];
            self.mixer.render(buf, channels);
            for (o, x) in dst.iter_mut().zip(buf.iter()) {
                *o = conv(*x);
            }
        }
        while self.pending_free() > 0 {
            let Some(p) = self.mixer.take_retired() else {
                break;
            };
            self.retire(p);
        }
    }

    /// Drops queued key and preview triggers. Called by the owner thread before a stream is
    /// (re)built, so keystrokes from a device outage do not replay as one burst.
    pub(crate) fn discard_triggers(&mut self) -> usize {
        let mut n = 0;
        while self.hook_rx.pop().is_ok() {
            n += 1;
        }
        while self.preview_rx.pop().is_ok() {
            n += 1;
        }
        n
    }

    /// Main-thread fallback when the control queue is full (e.g. no stream is draining it).
    /// Returns every retired pack so the caller can drop it off the audio thread.
    pub(crate) fn apply_now(&mut self, cmd: MixerCmd, dropped: &mut Vec<Arc<LoadedPack>>) {
        while let Ok(queued) = self.control_rx.pop() {
            dropped.extend(self.mixer.apply(queued));
        }
        dropped.extend(self.mixer.apply(cmd));
        while let Some(p) = self.mixer.take_retired() {
            dropped.push(p);
        }
        dropped.extend(self.pending.iter_mut().filter_map(Option::take));
    }
}

/// Builds (does not start) an output stream for `choice` feeding from `host`.
pub(crate) fn build_stream(
    device: &cpal::Device,
    choice: &Choice,
    host: Arc<Mutex<MixerHost>>,
    owner_tx: Sender<OwnerMsg>,
    generation: u64,
) -> Result<cpal::Stream, cpal::Error> {
    fn typed<T: SizedSample + Send + 'static>(
        device: &cpal::Device,
        config: StreamConfig,
        host: Arc<Mutex<MixerHost>>,
        owner_tx: Sender<OwnerMsg>,
        generation: u64,
        conv: fn(f32) -> T,
        silence: T,
    ) -> Result<cpal::Stream, cpal::Error> {
        let channels = usize::from(config.channels);
        device.build_output_stream::<T, _, _>(
            config,
            move |data: &mut [T], _| match host.try_lock() {
                Some(mut h) => h.process(data, channels, conv),
                None => data.fill(silence),
            },
            move |err| {
                let _ = owner_tx.send(OwnerMsg::StreamError {
                    generation,
                    message: err.to_string(),
                });
            },
            None,
        )
    }
    let (d, c, h, t, g) = (device, choice.config, host, owner_tx, generation);
    match choice.format {
        SampleFormat::F32 => typed::<f32>(d, c, h, t, g, |x| x, 0.0),
        SampleFormat::I32 => typed::<i32>(d, c, h, t, g, f32_to_i32, 0),
        SampleFormat::I16 => typed::<i16>(d, c, h, t, g, f32_to_i16, 0),
        other => Err(cpal::Error::with_message(
            cpal::ErrorKind::UnsupportedConfig,
            format!("sample format {other} not supported"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpal::SupportedStreamConfigRange as Range;

    fn range(ch: u16, lo: u32, hi: u32, buf: SupportedBufferSize, f: SampleFormat) -> Range {
        Range::new(ch, lo, hi, buf, f)
    }

    const ANY_BUF: SupportedBufferSize = SupportedBufferSize::Range { min: 64, max: 4096 };

    #[test]
    fn prefers_f32_stereo_default_rate_fixed() {
        let s = [
            range(2, 44_100, 48_000, ANY_BUF, SampleFormat::I16),
            range(6, 44_100, 48_000, ANY_BUF, SampleFormat::F32),
            range(2, 44_100, 96_000, ANY_BUF, SampleFormat::F32),
            range(2, 44_100, 96_000, ANY_BUF, SampleFormat::U8),
        ];
        let c = choose_config(&s, 44_100).unwrap();
        assert_eq!(c.format, SampleFormat::F32);
        assert_eq!(c.config.channels, 2);
        assert_eq!(c.config.sample_rate, 44_100);
        assert!(c.fixed_buffer);
        assert_eq!(c.config.buffer_size, BufferSize::Fixed(256));
    }

    #[test]
    fn falls_back_on_format_rate_and_buffer() {
        let s = [
            range(
                1,
                8_000,
                22_050,
                SupportedBufferSize::Unknown,
                SampleFormat::I16,
            ),
            range(
                2,
                96_000,
                96_000,
                SupportedBufferSize::Range {
                    min: 512,
                    max: 1024,
                },
                SampleFormat::I32,
            ),
        ];
        let c = choose_config(&s, 44_100).unwrap();
        assert_eq!(c.format, SampleFormat::I32);
        assert_eq!(c.config.sample_rate, 96_000);
        assert!(!c.fixed_buffer);
        assert_eq!(c.config.buffer_size, BufferSize::Default);
        let c = choose_config(&s[..1], 48_000).unwrap();
        assert_eq!(c.config.channels, 1);
        assert_eq!(c.config.sample_rate, 22_050);
        assert!(choose_config(&[range(2, 1, 2, ANY_BUF, SampleFormat::U8)], 48_000).is_none());
        assert!(choose_config(&[], 48_000).is_none());
    }

    #[test]
    fn discard_triggers_empties_queues() {
        use clatterbox_core::{EngineParams, KeyClass, KeyDir, KeySound, Settings};
        let params = Arc::new(EngineParams::from_settings(&Settings::default()));
        let (mut hook_tx, hook_rx) = rtrb::RingBuffer::new(8);
        let (mut prev_tx, prev_rx) = rtrb::RingBuffer::new(8);
        let (_ctl_tx, ctl_rx) = rtrb::RingBuffer::new(2);
        let (garbage_tx, _garbage_rx) = rtrb::RingBuffer::new(2);
        let mut host = MixerHost::new(
            Mixer::new(params, 48_000, 1),
            hook_rx,
            prev_rx,
            ctl_rx,
            garbage_tx,
        );
        let t = Trigger::key(KeySound {
            class: KeyClass::Default,
            dir: KeyDir::Down,
            x: 0.5,
        });
        for _ in 0..5 {
            hook_tx.push(t).ok().unwrap();
        }
        prev_tx.push(Trigger::preview(t.sound)).ok().unwrap();
        assert_eq!(host.discard_triggers(), 6);
        assert_eq!(host.discard_triggers(), 0);
    }

    #[test]
    fn int_conversion() {
        assert_eq!(f32_to_i16(1.0), i16::MAX);
        assert_eq!(f32_to_i16(-2.0), -i16::MAX);
        assert_eq!(f32_to_i16(0.0), 0);
        assert_eq!(f32_to_i32(1.0), i32::MAX);
        assert_eq!(f32_to_i32(-1.0), -i32::MAX);
        assert_eq!(f32_to_i32(0.5), 1_073_741_824);
    }
}
