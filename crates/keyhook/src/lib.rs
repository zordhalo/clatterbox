//! Clatterbox global keyboard hook (SPEC §3). One thin backend per OS, all feeding
//! [`dispatch::Dispatcher`].
//!
//! PRIVACY (SPEC §0): nothing in this crate logs, stores, or transmits key identity. No logging
//! or print macros and no Debug derives anywhere in this crate (scripts/check-privacy.sh).
//!
//! Note: the platform modules are named `windows`/`macos`/`linux`; at the crate root `windows`
//! would be ambiguous with the `windows` crate, so refer to the crate as `::windows::…`.
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

mod dispatch;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use clatterbox_core::{EngineParams, HookStatus, TriggerTx};

pub struct HookHandle {
    thread: Option<JoinHandle<()>>,
    status: Arc<Mutex<HookStatus>>,
}

impl HookHandle {
    pub fn status(&self) -> HookStatus {
        match self.status.lock() {
            Ok(s) => s.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn stop(self) {
        todo!("WP3")
    }
}

/// Starts the platform backend on its own thread. Never panics; failures surface as status.
pub fn start(
    tx: TriggerTx,
    params: Arc<EngineParams>,
    on_status: Box<dyn Fn(HookStatus) + Send + Sync>,
) -> HookHandle {
    todo!("WP3")
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

pub const LINUX_INPUT_GROUP_HINT: &str = "On Wayland, Clatterbox needs read access to keyboard \
devices. Run `sudo usermod -aG input $USER`, then log out and back in. Note: membership in \
`input` lets any program you run read input devices.";
