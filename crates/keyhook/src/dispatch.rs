//! The single point where a key identity is reduced to a `KeySound` (SPEC §3.6).
//! PRIVACY: must not log, store, or clone `key`.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use clatterbox_core::keymap::layout;
use clatterbox_core::{EngineParams, KeyDir, KeySound, PhysKey, RepeatFilter, Trigger, TriggerTx};

pub(crate) struct Dispatcher {
    filter: RepeatFilter,
    params: Arc<EngineParams>,
    tx: TriggerTx,
    epoch: Instant,
}

impl Dispatcher {
    pub(crate) fn new(params: Arc<EngineParams>, tx: TriggerTx) -> Self {
        Self {
            filter: RepeatFilter::new(),
            params,
            tx,
            epoch: Instant::now(),
        }
    }

    /// Monotonic milliseconds since this dispatcher was created.
    #[inline]
    pub(crate) fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    /// The ONLY place a key identity is converted. Must not log, store, or clone `key`.
    #[inline]
    pub(crate) fn dispatch(&mut self, key: PhysKey, dir: KeyDir, now_ms: u64) {
        if !self.filter.accept(key, dir, now_ms) {
            return;
        }
        if !self.params.enabled.load(Ordering::Relaxed) {
            return;
        }
        if dir == KeyDir::Up && !self.params.key_up.load(Ordering::Relaxed) {
            return;
        }
        let (class, x) = layout::lookup(key);
        // Full queue: drop silently.
        let _ = self.tx.push(Trigger::key(KeySound { class, dir, x }));
    }

    /// Forgets held keys (a backend restarted, so key-ups may have been missed).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub(crate) fn reset(&mut self) {
        self.filter.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clatterbox_core::{KeyClass, PackSlot, Settings};

    fn setup(enabled: bool, key_up: bool) -> (Dispatcher, rtrb::Consumer<Trigger>) {
        let s = Settings {
            enabled,
            key_up_enabled: key_up,
            ..Settings::default()
        };
        let params = Arc::new(EngineParams::from_settings(&s));
        let (prod, cons) = rtrb::RingBuffer::new(4);
        (Dispatcher::new(params, TriggerTx(prod)), cons)
    }

    #[test]
    fn down_up_become_key_sounds() {
        let (mut d, mut rx) = setup(true, true);
        d.dispatch(PhysKey::Space, KeyDir::Down, 0);
        d.dispatch(PhysKey::Space, KeyDir::Down, 30); // repeat
        d.dispatch(PhysKey::Space, KeyDir::Up, 60);
        let t = rx.pop().expect("down");
        assert!(t.slot == PackSlot::Main);
        assert!(t.sound.class == KeyClass::Space && t.sound.dir == KeyDir::Down);
        assert!((t.sound.x - 0.458).abs() < 0.01);
        assert!(rx.pop().expect("up").sound.dir == KeyDir::Up);
        assert!(rx.pop().is_err());
    }

    #[test]
    fn disabled_and_key_up_off_are_silent() {
        let (mut d, mut rx) = setup(false, true);
        d.dispatch(PhysKey::A, KeyDir::Down, 0);
        d.dispatch(PhysKey::A, KeyDir::Up, 10);
        assert!(rx.pop().is_err());

        let (mut d, mut rx) = setup(true, false);
        d.dispatch(PhysKey::A, KeyDir::Down, 0);
        d.dispatch(PhysKey::A, KeyDir::Up, 10);
        assert!(rx.pop().expect("down").sound.dir == KeyDir::Down);
        assert!(rx.pop().is_err());
    }

    #[test]
    fn full_queue_drops_without_panic() {
        let (mut d, mut rx) = setup(true, true);
        for i in 0..PhysKey::COUNT {
            let key = PhysKey::from_index(i).expect("in range");
            d.dispatch(key, KeyDir::Down, 0);
        }
        let mut n = 0;
        while rx.pop().is_ok() {
            n += 1;
        }
        assert!(n == 4);
    }
}
