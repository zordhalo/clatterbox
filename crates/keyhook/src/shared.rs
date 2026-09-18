//! State shared between a [`crate::HookHandle`] and its backend thread: status reporting and a
//! race-free stop signal.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use clatterbox_core::HookStatus;

/// Interrupts a backend's blocking loop (posts WM_CLOSE, stops a CFRunLoop, writes a pipe…).
pub(crate) type Waker = Box<dyn Fn() + Send>;

struct StopState {
    requested: bool,
    waker: Option<Waker>,
}

pub(crate) struct Shared {
    status: Mutex<HookStatus>,
    on_status: Box<dyn Fn(HookStatus) + Send + Sync>,
    stop: Mutex<StopState>,
    stop_cv: Condvar,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Shared {
    pub(crate) fn new(on_status: Box<dyn Fn(HookStatus) + Send + Sync>) -> Self {
        Self {
            status: Mutex::new(HookStatus::Starting),
            on_status,
            stop: Mutex::new(StopState {
                requested: false,
                waker: None,
            }),
            stop_cv: Condvar::new(),
        }
    }

    pub(crate) fn status(&self) -> HookStatus {
        lock(&self.status).clone()
    }

    /// Stores `s` and notifies the app if it changed.
    pub(crate) fn set_status(&self, s: HookStatus) {
        let mut cur = lock(&self.status);
        if *cur == s {
            return;
        }
        *cur = s.clone();
        drop(cur);
        (self.on_status)(s);
    }

    pub(crate) fn failed(&self, reason: impl Into<String>) {
        self.set_status(HookStatus::Failed {
            reason: reason.into(),
        });
    }

    #[cfg_attr(windows, allow(dead_code))]
    pub(crate) fn stop_requested(&self) -> bool {
        lock(&self.stop).requested
    }

    /// Registers how to interrupt the backend's blocking wait. Returns `false` if a stop was
    /// already requested (the backend must then exit without blocking).
    pub(crate) fn arm(&self, waker: Waker) -> bool {
        let mut st = lock(&self.stop);
        if st.requested {
            return false;
        }
        st.waker = Some(waker);
        true
    }

    /// Drops the registered waker (call before releasing what it refers to).
    pub(crate) fn disarm(&self) {
        lock(&self.stop).waker = None;
    }

    pub(crate) fn request_stop(&self) {
        let mut st = lock(&self.stop);
        st.requested = true;
        if let Some(w) = st.waker.take() {
            w();
        }
        drop(st);
        self.stop_cv.notify_all();
    }

    /// Sleeps up to `ms`; returns `true` if a stop was requested (immediately or meanwhile).
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) fn sleep(&self, ms: u64) -> bool {
        let st = lock(&self.stop);
        let (st, _) = self
            .stop_cv
            .wait_timeout_while(st, Duration::from_millis(ms), |s| !s.requested)
            .unwrap_or_else(PoisonError::into_inner);
        st.requested
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn status_changes_notify_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let s = Shared::new(Box::new(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        }));
        s.set_status(HookStatus::NeedsRestart);
        s.set_status(HookStatus::NeedsRestart);
        s.failed("x");
        assert!(calls.load(Ordering::SeqCst) == 2);
        assert!(s.status() == HookStatus::Failed { reason: "x".into() });
    }

    #[test]
    fn stop_runs_waker_or_refuses_arming() {
        let s = Shared::new(Box::new(|_| {}));
        let woke = Arc::new(AtomicUsize::new(0));
        let w = woke.clone();
        assert!(s.arm(Box::new(move || {
            w.fetch_add(1, Ordering::SeqCst);
        })));
        s.request_stop();
        assert!(woke.load(Ordering::SeqCst) == 1);
        assert!(s.stop_requested());
        assert!(!s.arm(Box::new(|| {})));
        assert!(s.sleep(10_000));
    }
}
