//! evdev `/dev/input/event*` reader, poll(2) over non-blocking fds, never grabs (SPEC §3.4).
//! Note: this module shadows the `evdev` crate inside `linux::`; use `::evdev::…` for the crate.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ::evdev::{Device, EventType, KeyCode};
use clatterbox_core::{HookStatus, KeyDir};

use super::keycode;
use crate::LINUX_INPUT_GROUP_HINT;
use crate::dispatch::Dispatcher;
use crate::shared::Shared;

pub(crate) const BACKEND: &str = "evdev";

/// Hot-plug rescan interval / poll timeout.
pub(crate) const RESCAN_MS: i32 = 3000;

const INPUT_DIR: &str = "/dev/input";

struct Keyboard {
    path: PathBuf,
    dev: Device,
}

pub(crate) fn run(d: &mut Dispatcher, shared: &Shared) {
    let (wake_rx, wake_tx) = match wake_pipe() {
        Ok(p) => p,
        Err(e) => return shared.failed(format!("pipe: {e}")),
    };
    let wake_tx = Arc::new(wake_tx);
    let armed = shared.arm(Box::new(move || {
        let byte = 1u8;
        // SAFETY: writes one byte from a valid buffer to a pipe we own (kept alive by the Arc).
        let _ = unsafe { libc::write(wake_tx.as_raw_fd(), (&byte as *const u8).cast(), 1) };
    }));
    if !armed {
        return;
    }

    let mut kbds: Vec<Keyboard> = Vec::new();
    let mut denied = rescan(&mut kbds);
    let mut last_scan = Instant::now();
    let mut pollfds: Vec<libc::pollfd> = Vec::new();
    loop {
        report(shared, &kbds, denied);
        pollfds.clear();
        pollfds.push(pollfd(wake_rx.as_raw_fd()));
        pollfds.extend(kbds.iter().map(|k| pollfd(k.dev.as_raw_fd())));
        // SAFETY: `pollfds` is a valid, initialised slice of pollfd for the duration of the call.
        let n = unsafe {
            libc::poll(
                pollfds.as_mut_ptr(),
                pollfds.len() as libc::nfds_t,
                RESCAN_MS,
            )
        };
        if shared.stop_requested() {
            break;
        }
        if n < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            shared.failed(format!("poll: {err}"));
            break;
        }
        let mut lost: Vec<usize> = Vec::new();
        for (i, (pfd, kbd)) in pollfds[1..].iter().zip(kbds.iter_mut()).enumerate() {
            let gone = pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0
                || (pfd.revents & libc::POLLIN != 0 && read_events(d, &mut kbd.dev).is_err());
            if gone {
                lost.push(i);
            }
        }
        if !lost.is_empty() {
            // Unplugged: drop dead devices; their held keys may never see a key-up.
            for &i in lost.iter().rev() {
                kbds.swap_remove(i);
            }
            d.reset();
        }
        if last_scan.elapsed() >= Duration::from_millis(RESCAN_MS as u64) {
            denied = rescan(&mut kbds);
            last_scan = Instant::now();
        }
    }
    shared.disarm();
}

/// Reads every pending event; `Err` means the device is gone.
#[inline]
fn read_events(d: &mut Dispatcher, dev: &mut Device) -> io::Result<()> {
    let events = match dev.fetch_events() {
        Ok(ev) => ev,
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
        Err(e) => return Err(e),
    };
    for ev in events {
        if ev.event_type() != EventType::KEY {
            continue;
        }
        let dir = match ev.value() {
            1 => KeyDir::Down,
            0 => KeyDir::Up,
            _ => continue, // 2 = OS auto-repeat
        };
        let now = d.now_ms();
        d.dispatch(keycode::to_phys(ev.code()), dir, now);
    }
    Ok(())
}

/// Opens keyboards not already open. Returns whether any device was refused with EACCES.
fn rescan(kbds: &mut Vec<Keyboard>) -> bool {
    let Ok(entries) = std::fs::read_dir(INPUT_DIR) else {
        return false;
    };
    let mut denied = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_event_node(&path) || kbds.iter().any(|k| k.path == path) {
            continue;
        }
        match Device::open(&path) {
            Ok(dev) if is_keyboard(&dev) && dev.set_nonblocking(true).is_ok() => {
                kbds.push(Keyboard { path, dev });
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => denied = true,
            Err(_) => {}
        }
    }
    denied
}

fn is_event_node(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("event"))
}

/// Keyboards have both A and Space (skips mice, power buttons, lid switches).
fn is_keyboard(dev: &Device) -> bool {
    dev.supported_keys()
        .is_some_and(|k| k.contains(KeyCode::KEY_A) && k.contains(KeyCode::KEY_SPACE))
}

fn report(shared: &Shared, kbds: &[Keyboard], denied: bool) {
    let status = if !kbds.is_empty() {
        HookStatus::Running {
            backend: BACKEND.into(),
        }
    } else if denied {
        HookStatus::NeedsPermission {
            hint: LINUX_INPUT_GROUP_HINT.into(),
        }
    } else {
        HookStatus::Unsupported {
            reason: "no keyboard device found under /dev/input".into(),
        }
    };
    shared.set_status(status);
}

fn pollfd(fd: RawFd) -> libc::pollfd {
    libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    }
}

/// Non-blocking self-pipe `(read, write)` that wakes `poll` on stop.
fn wake_pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0; 2];
    // SAFETY: `fds` is a valid out-array of two ints.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: pipe2 just returned two fresh descriptors that nothing else owns.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}
