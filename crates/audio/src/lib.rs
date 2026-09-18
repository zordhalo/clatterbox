//! Clatterbox audio engine (SPEC §4): cpal output stream owned by a dedicated thread, fed by
//! wait-free queues into the mixer.

mod owner;
mod stream;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread::JoinHandle;

use clatterbox_core::{EngineParams, LoadedPack, Mixer, MixerCmd, PackSlot, Trigger};
use parking_lot::Mutex;

pub use clatterbox_core::{AudioStatus, TriggerTx};
pub use stream::{Choice, TARGET_BUFFER_FRAMES, choose_config, f32_to_i16, f32_to_i32};

use owner::{Owner, OwnerMsg};
use stream::MixerHost;

/// Hook queue capacity (SPEC §4.3).
pub const HOOK_QUEUE: usize = 256;
pub const PREVIEW_QUEUE: usize = 64;
pub const CONTROL_QUEUE: usize = 16;
pub const GARBAGE_QUEUE: usize = 16;

/// Initial mixer rate; replaced by the device rate before the first stream is built.
const INITIAL_RATE: u32 = 48_000;

pub struct AudioEngine {
    owner: Option<JoinHandle<()>>,
    owner_tx: mpsc::Sender<OwnerMsg>,
    control_tx: Mutex<rtrb::Producer<MixerCmd>>,
    preview_tx: Mutex<TriggerTx>,
    status: Arc<Mutex<AudioStatus>>,
    host: Arc<Mutex<MixerHost>>,
}

impl AudioEngine {
    /// Spawns the owner thread and opens the default device. Returns the Hook producer.
    /// Never fails hard: device problems are reported via status + on_status.
    pub fn start(
        params: Arc<EngineParams>,
        on_status: Box<dyn Fn(AudioStatus) + Send + Sync>,
    ) -> (AudioEngine, TriggerTx) {
        let (hook_tx, hook_rx) = rtrb::RingBuffer::new(HOOK_QUEUE);
        let (preview_tx, preview_rx) = rtrb::RingBuffer::new(PREVIEW_QUEUE);
        let (control_tx, control_rx) = rtrb::RingBuffer::new(CONTROL_QUEUE);
        let (garbage_tx, garbage_rx) = rtrb::RingBuffer::new(GARBAGE_QUEUE);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x5EED, |d| d.as_nanos() as u64);
        let mixer = Mixer::new(params, INITIAL_RATE, seed);
        let host = Arc::new(Mutex::new(MixerHost::new(
            mixer, hook_rx, preview_rx, control_rx, garbage_tx,
        )));
        let status = Arc::new(Mutex::new(AudioStatus::Starting));
        let (owner_tx, owner_rx) = mpsc::channel();
        let owner = Owner {
            host: host.clone(),
            rx: owner_rx,
            tx: owner_tx.clone(),
            garbage_rx,
            status: status.clone(),
            on_status,
        };
        let handle = std::thread::Builder::new()
            .name("clatterbox-audio".into())
            .spawn(move || owner.run());
        let owner = match handle {
            Ok(h) => Some(h),
            Err(e) => {
                tracing::error!("cannot spawn audio owner thread: {e}");
                *status.lock() = AudioStatus::Failed {
                    reason: format!("cannot spawn audio thread: {e}"),
                };
                None
            }
        };
        let engine = AudioEngine {
            owner,
            owner_tx,
            control_tx: Mutex::new(control_tx),
            preview_tx: Mutex::new(TriggerTx(preview_tx)),
            status,
            host,
        };
        (engine, TriggerTx(hook_tx))
    }

    /// Queues a pack swap for the callback. If the control queue is full (no stream is draining
    /// it), the swap is applied directly under the mixer lock and retired packs drop here.
    pub fn set_pack(&self, slot: PackSlot, pack: Arc<LoadedPack>) {
        let cmd = MixerCmd::SetPack { slot, pack };
        let Err(rtrb::PushError::Full(cmd)) = self.control_tx.lock().push(cmd) else {
            return;
        };
        let mut retired = Vec::new();
        self.host.lock().apply_now(cmd, &mut retired);
        drop(retired);
        let _ = self.owner_tx.send(OwnerMsg::Wake);
    }

    /// `false` if the preview queue is full.
    pub fn preview(&self, t: Trigger) -> bool {
        self.preview_tx.lock().push(t)
    }

    pub fn status(&self) -> AudioStatus {
        self.status.lock().clone()
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        let _ = self.owner_tx.send(OwnerMsg::Shutdown);
        if let Some(h) = self.owner.take() {
            let _ = h.join();
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop();
    }
}
