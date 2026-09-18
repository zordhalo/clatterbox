//! Audio-owner thread: owns the (`!Send`) `cpal::Stream`, rebuilds on error / default-device
//! change, drops retired packs, reports `AudioStatus` (SPEC §4.2, §4.3).

use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use clatterbox_core::{AudioStatus, LoadedPack};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;

use crate::stream::{Choice, MixerHost, build_stream, choose_config};

/// Device-watch tick.
pub(crate) const TICK: Duration = Duration::from_secs(3);
/// Delay before rebuilding after a stream error.
pub(crate) const REBUILD_DELAY: Duration = Duration::from_millis(250);
/// How often retired packs are dropped.
const GARBAGE_TICK: Duration = Duration::from_millis(500);

pub(crate) enum OwnerMsg {
    /// From the cpal error callback of the stream with this `generation`.
    StreamError {
        generation: u64,
        message: String,
    },
    /// A retired pack was queued; drop it soon.
    Wake,
    Shutdown,
}

pub(crate) type StatusFn = Box<dyn Fn(AudioStatus) + Send + Sync>;

struct Live {
    _stream: cpal::Stream,
    device_id: Option<cpal::DeviceId>,
    generation: u64,
}

pub(crate) struct Owner {
    pub(crate) host: Arc<Mutex<MixerHost>>,
    pub(crate) rx: Receiver<OwnerMsg>,
    pub(crate) tx: Sender<OwnerMsg>,
    pub(crate) garbage_rx: rtrb::Consumer<Arc<LoadedPack>>,
    pub(crate) status: Arc<Mutex<AudioStatus>>,
    pub(crate) on_status: StatusFn,
}

impl Owner {
    fn set_status(&self, s: AudioStatus) {
        let mut cur = self.status.lock();
        if *cur != s {
            *cur = s.clone();
            drop(cur);
            (self.on_status)(s);
        }
    }

    fn drain_garbage(&mut self) {
        while let Ok(p) = self.garbage_rx.pop() {
            drop(p);
        }
    }

    /// Opens the default device. `Err` carries the status to report.
    fn open(&self, generation: u64) -> Result<Live, AudioStatus> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioStatus::NoDevice)?;
        let failed = |e: String| AudioStatus::Failed { reason: e };
        let supported: Vec<_> = device
            .supported_output_configs()
            .map_err(|e| failed(format!("cannot query output formats: {e}")))?
            .collect();
        let default_rate = device
            .default_output_config()
            .map(|c| c.sample_rate())
            .unwrap_or(48_000);
        let mut choice = choose_config(&supported, default_rate)
            .ok_or_else(|| failed("no supported output format (need f32, i32 or i16)".into()))?;
        {
            // No stream is alive here, so the lock is uncontended. Triggers queued while there
            // was no stream are stale: drop them rather than play them as one burst.
            let mut host = self.host.lock();
            host.mixer.set_output_rate(choice.config.sample_rate);
            host.discard_triggers();
        }

        let build =
            |c: &Choice| build_stream(&device, c, self.host.clone(), self.tx.clone(), generation);
        let stream = match build(&choice) {
            Ok(s) => s,
            Err(e) if choice.fixed_buffer => {
                tracing::info!("fixed 256-frame buffer rejected ({e}); using device default");
                choice.config.buffer_size = cpal::BufferSize::Default;
                choice.fixed_buffer = false;
                build(&choice).map_err(|e| failed(format!("cannot open output stream: {e}")))?
            }
            Err(e) => return Err(failed(format!("cannot open output stream: {e}"))),
        };
        stream
            .play()
            .map_err(|e| failed(format!("cannot start output stream: {e}")))?;
        let name = device.to_string();
        tracing::info!(
            device = %name,
            rate = choice.config.sample_rate,
            channels = choice.config.channels,
            format = %choice.format,
            fixed_buffer = choice.fixed_buffer,
            "audio stream running"
        );
        self.set_status(AudioStatus::Running {
            device: name,
            sample_rate: choice.config.sample_rate,
            buffer_frames: choice
                .fixed_buffer
                .then_some(crate::stream::TARGET_BUFFER_FRAMES),
        });
        Ok(Live {
            _stream: stream,
            device_id: device.id().ok(),
            generation,
        })
    }

    fn default_device_id() -> Option<cpal::DeviceId> {
        cpal::default_host()
            .default_output_device()
            .and_then(|d| d.id().ok())
    }

    pub(crate) fn run(mut self) {
        let mut live: Option<Live> = None;
        let mut generation = 0u64;
        let mut retry_at = Instant::now();
        let mut next_watch = Instant::now() + TICK;
        let mut last_error: Option<AudioStatus> = None;
        loop {
            let now = Instant::now();
            if live.is_none() && now >= retry_at {
                generation += 1;
                match self.open(generation) {
                    Ok(l) => {
                        live = Some(l);
                        last_error = None;
                    }
                    Err(s) => {
                        if last_error.as_ref() != Some(&s) {
                            tracing::warn!(status = ?s, "audio output unavailable; retrying");
                        }
                        self.set_status(s.clone());
                        last_error = Some(s);
                        retry_at = now + TICK;
                    }
                }
            }
            self.drain_garbage();

            let wait = if live.is_none() {
                retry_at
                    .saturating_duration_since(Instant::now())
                    .min(GARBAGE_TICK)
            } else {
                GARBAGE_TICK
            };
            match self.rx.recv_timeout(wait) {
                Ok(OwnerMsg::StreamError {
                    generation: g,
                    message,
                }) => {
                    if live.as_ref().is_some_and(|l| l.generation == g) {
                        tracing::warn!(error = %message, "audio stream error; rebuilding");
                        live = None;
                        self.set_status(AudioStatus::Starting);
                        retry_at = Instant::now() + REBUILD_DELAY;
                    }
                }
                Ok(OwnerMsg::Wake) | Err(RecvTimeoutError::Timeout) => {}
                Ok(OwnerMsg::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            }

            let now = Instant::now();
            if now >= next_watch {
                next_watch = now + TICK;
                if let Some(l) = &live {
                    let current = Self::default_device_id();
                    if current.is_some() && current != l.device_id {
                        tracing::info!("default output device changed; rebuilding stream");
                        live = None;
                        retry_at = now;
                    }
                }
            }
        }
        drop(live);
        self.drain_garbage();
    }
}
