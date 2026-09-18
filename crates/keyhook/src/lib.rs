//! Clatterbox global keyboard hook (SPEC §3). One thin backend per OS, all feeding
//! [`dispatch::Dispatcher`].
//!
//! PRIVACY (SPEC §0): nothing in this crate logs, stores, or transmits key identity. No logging
//! or print macros and no Debug derives anywhere in this crate (scripts/check-privacy.sh).
//!
//! Note: the platform modules are named `windows`/`macos`/`linux`; at the crate root `windows`
//! would be ambiguous with the `windows` crate, so refer to the crate as `::windows::…`.

mod dispatch;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod shared;
#[cfg(windows)]
mod windows;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use clatterbox_core::{EngineParams, HookStatus, TriggerTx};

use dispatch::Dispatcher;
use shared::Shared;

pub struct HookHandle {
    thread: Option<JoinHandle<()>>,
    shared: Arc<Shared>,
}

impl HookHandle {
    pub fn status(&self) -> HookStatus {
        self.shared.status()
    }

    /// Stops the backend and joins its thread.
    pub fn stop(self) {
        drop(self);
    }
}

impl Drop for HookHandle {
    fn drop(&mut self) {
        self.shared.request_stop();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Starts the platform backend on its own thread. Never panics; failures surface as status.
///
/// Windows ordering constraint: Raw Input keeps one registration per (usage page, usage) per
/// process, and the last `RegisterRawInputDevices` call wins. tao registers keyboard Raw Input
/// for its own window in `EventLoop::new` / `EventLoopBuilder::build`, so call `start` after
/// the event loop exists (inside Tauri's `setup`), and never call
/// `set_device_event_filter` afterwards: it re-registers and takes the keyboard away from
/// this hook.
pub fn start(
    tx: TriggerTx,
    params: Arc<EngineParams>,
    on_status: Box<dyn Fn(HookStatus) + Send + Sync>,
) -> HookHandle {
    let shared = Arc::new(Shared::new(on_status));
    let dispatcher = Dispatcher::new(params, tx);
    let worker = shared.clone();
    let spawned = thread::Builder::new()
        .name("clatterbox-keyhook".into())
        .spawn(move || {
            let run = catch_unwind(AssertUnwindSafe(|| run_backend(dispatcher, &worker)));
            if run.is_err() {
                worker.failed("keyboard hook thread panicked");
            }
        });
    let thread = match spawned {
        Ok(t) => Some(t),
        Err(e) => {
            shared.failed(format!("could not spawn keyboard hook thread: {e}"));
            None
        }
    };
    HookHandle { thread, shared }
}

#[cfg(windows)]
fn run_backend(d: Dispatcher, shared: &Shared) {
    windows::run(d, shared);
}

#[cfg(target_os = "macos")]
fn run_backend(d: Dispatcher, shared: &Shared) {
    macos::run(d, shared);
}

#[cfg(target_os = "linux")]
fn run_backend(d: Dispatcher, shared: &Shared) {
    linux::run(d, shared);
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn run_backend(d: Dispatcher, shared: &Shared) {
    drop(d);
    shared.set_status(HookStatus::Unsupported {
        reason: "no keyboard backend for this operating system".into(),
    });
}

/// macOS: CGRequestListenEventAccess + returns whether granted now. Others: true.
pub fn request_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::permission::request()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub const MACOS_PRIVACY_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent";

/// UI copy for `HookStatus::NeedsPermission` on Wayland (SPEC §3.4). Informational only.
pub const LINUX_INPUT_GROUP_HINT: &str = "On Wayland, Clatterbox can only hear keys if your user \
can read keyboard devices (the `input` group). Clatterbox will not change this for you. Be aware \
that `input` group membership lets any program you run read all input devices. See the README \
for details.";
