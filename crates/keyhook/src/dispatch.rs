//! The single point where a key identity is reduced to a `KeySound` (SPEC §3.6).
//! PRIVACY: must not log, store, or clone `key`.
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

use std::sync::Arc;

use clatterbox_core::{EngineParams, KeyDir, PhysKey, RepeatFilter, TriggerTx};

pub(crate) struct Dispatcher {
    filter: RepeatFilter,
    params: Arc<EngineParams>,
    tx: TriggerTx,
}

impl Dispatcher {
    pub(crate) fn new(params: Arc<EngineParams>, tx: TriggerTx) -> Self {
        Self {
            filter: RepeatFilter::new(),
            params,
            tx,
        }
    }

    /// The ONLY place a key identity is converted. Must not log, store, or clone `key`.
    #[inline]
    pub(crate) fn dispatch(&mut self, key: PhysKey, dir: KeyDir, now_ms: u64) {
        todo!("WP3")
    }
}
