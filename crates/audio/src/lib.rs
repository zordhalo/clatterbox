//! Clatterbox audio engine (SPEC §4): cpal output stream owned by a dedicated thread, fed by
//! wait-free queues into the mixer.
// WP0 stub: remove this allow once implemented (WP2).
#![allow(unused_variables, dead_code)]

mod owner;
mod stream;

use std::sync::Arc;
use std::thread::JoinHandle;

use clatterbox_core::{EngineParams, LoadedPack, MixerCmd, PackSlot, Trigger};
use parking_lot::Mutex;

pub use clatterbox_core::{AudioStatus, TriggerTx};
pub use stream::{Choice, choose_config};

/// Hook queue capacity (SPEC §4.3).
pub const HOOK_QUEUE: usize = 256;
pub const PREVIEW_QUEUE: usize = 64;
pub const CONTROL_QUEUE: usize = 16;
pub const GARBAGE_QUEUE: usize = 16;

pub struct AudioEngine {
    owner: Option<JoinHandle<()>>,
    owner_tx: std::sync::mpsc::Sender<owner::OwnerMsg>,
    control_tx: Mutex<rtrb::Producer<MixerCmd>>,
    preview_tx: Mutex<TriggerTx>,
    status: Arc<Mutex<AudioStatus>>,
}

impl AudioEngine {
    /// Spawns the owner thread and opens the default device. Returns the Hook producer.
    /// Never fails hard: device problems are reported via status + on_status.
    pub fn start(
        params: Arc<EngineParams>,
        on_status: Box<dyn Fn(AudioStatus) + Send + Sync>,
    ) -> (AudioEngine, TriggerTx) {
        todo!("WP2")
    }

    pub fn set_pack(&self, slot: PackSlot, pack: Arc<LoadedPack>) {
        todo!("WP2")
    }

    /// `false` if the preview queue is full.
    pub fn preview(&self, t: Trigger) -> bool {
        todo!("WP2")
    }

    pub fn status(&self) -> AudioStatus {
        self.status.lock().clone()
    }

    pub fn shutdown(self) {
        todo!("WP2")
    }
}
