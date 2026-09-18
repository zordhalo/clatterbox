//! Debounced background settings writer (SPEC §8.1): 500 ms debounce, last write wins.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use clatterbox_core::Settings;
use parking_lot::Mutex;

pub const DEBOUNCE: Duration = Duration::from_millis(500);

enum Msg {
    Write(Settings),
    Flush(Sender<()>),
    Shutdown,
}

pub struct Persister {
    tx: Sender<Msg>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Persister {
    pub fn new(path: PathBuf) -> Persister {
        let (tx, rx) = mpsc::channel::<Msg>();
        let thread = std::thread::Builder::new()
            .name("clatterbox-persist".into())
            .spawn(move || {
                let mut pending: Option<Settings> = None;
                loop {
                    let timeout = if pending.is_some() {
                        DEBOUNCE
                    } else {
                        Duration::from_secs(3600)
                    };
                    match rx.recv_timeout(timeout) {
                        Ok(Msg::Write(s)) => pending = Some(s),
                        Ok(Msg::Flush(done)) => {
                            if let Some(s) = pending.take() {
                                let _ = clatterbox_core::settings::save_atomic(&path, &s);
                            }
                            let _ = done.send(());
                        }
                        Ok(Msg::Shutdown) => {
                            if let Some(s) = pending.take() {
                                let _ = clatterbox_core::settings::save_atomic(&path, &s);
                            }
                            return;
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            if let Some(s) = pending.take() {
                                let _ = clatterbox_core::settings::save_atomic(&path, &s);
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            if let Some(s) = pending.take() {
                                let _ = clatterbox_core::settings::save_atomic(&path, &s);
                            }
                            return;
                        }
                    }
                }
            })
            .expect("failed to spawn persist thread");

        Persister {
            tx,
            thread: Mutex::new(Some(thread)),
        }
    }

    /// Debounced write; last write wins.
    pub fn schedule(&self, settings: Settings) {
        let _ = self.tx.send(Msg::Write(settings));
    }

    /// Blocks until any pending write is on disk (called on quit).
    pub fn flush(&self) {
        let (done_tx, done_rx) = mpsc::channel();
        if self.tx.send(Msg::Flush(done_tx)).is_ok() {
            let _ = done_rx.recv_timeout(Duration::from_secs(2));
        }
    }
}

impl Drop for Persister {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Shutdown);
        if let Some(handle) = self.thread.lock().take() {
            let _ = handle.join();
        }
    }
}
