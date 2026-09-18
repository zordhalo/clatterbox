//! macOS backend: listen-only CGEventTap on a CFRunLoop thread (SPEC §3.3).

pub(crate) mod keycode;
pub(crate) mod permission;

use std::ffi::c_void;
use std::ptr::NonNull;

use clatterbox_core::{HookStatus, KeyDir};
use objc2_core_foundation::{
    CFMachPort, CFRetained, CFRunLoop, kCFRunLoopCommonModes, kCFRunLoopDefaultMode,
};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventMask, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType,
};

use crate::dispatch::Dispatcher;
use crate::shared::Shared;

pub(crate) const BACKEND: &str = "event_tap";

/// Shown with `HookStatus::NeedsPermission` on macOS.
const PERMISSION_HINT: &str = "Allow Clatterbox under System Settings → Privacy & Security → \
Input Monitoring.";

/// Preflight re-check interval while waiting for the grant.
const PERMISSION_POLL_MS: u64 = 2000;

/// Run-loop slice; bounds how long a stop requested before `CFRunLoopRun` can go unseen.
const RUN_SLICE_S: f64 = 1.0;

struct TapCtx {
    dispatcher: Dispatcher,
    tap: Option<CFRetained<CFMachPort>>,
}

/// `CFRunLoopStop` is thread-safe; the retained ref is only used for that.
struct RunLoopRef(CFRetained<CFRunLoop>);
// SAFETY: CFRunLoop is a thread-safe CF object; we only call `CFRunLoopStop` on it.
unsafe impl Send for RunLoopRef {}

impl RunLoopRef {
    // A method (not `self.0.stop()` in the closure) so the closure captures the Send wrapper.
    fn stop(&self) {
        self.0.stop();
    }
}

pub(crate) fn run(d: Dispatcher, shared: &Shared) {
    let mut waited = false;
    if !permission::preflight() {
        shared.set_status(HookStatus::NeedsPermission {
            hint: PERMISSION_HINT.into(),
        });
        waited = true;
        loop {
            if shared.sleep(PERMISSION_POLL_MS) {
                return;
            }
            if permission::preflight() {
                break;
            }
        }
    }

    // Owned through a raw pointer for the tap's lifetime; reclaimed at the end of `run`.
    let ctx = Box::into_raw(Box::new(TapCtx {
        dispatcher: d,
        tap: None,
    }));
    let mask: CGEventMask = (1 << CGEventType::KeyDown.0)
        | (1 << CGEventType::KeyUp.0)
        | (1 << CGEventType::FlagsChanged.0);
    // SAFETY: `callback` matches CGEventTapCallBack; `ctx` outlives the tap (it is invalidated
    // before `ctx` is dropped at the end of this function).
    let tap = unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::HIDEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            mask,
            Some(callback),
            ctx.cast::<c_void>(),
        )
    };
    let Some(tap) = tap else {
        // SAFETY: no tap exists, so nothing else refers to `ctx`.
        drop(unsafe { Box::from_raw(ctx) });
        // macOS often needs a relaunch before a freshly granted tap can be created.
        if waited {
            shared.set_status(HookStatus::NeedsRestart);
        } else {
            shared.failed("CGEventTapCreate failed");
        }
        return;
    };
    // SAFETY: the tap is not attached to a run loop yet, so the callback cannot run.
    unsafe { (*ctx).tap = Some(tap.clone()) };

    let (Some(source), Some(rl)) = (
        CFMachPort::new_run_loop_source(None, Some(&tap), 0),
        CFRunLoop::current(),
    ) else {
        tap.invalidate();
        // SAFETY: the tap is invalidated and never ran.
        drop(unsafe { Box::from_raw(ctx) });
        return shared.failed("could not attach event tap to run loop");
    };
    // SAFETY: CF-provided static mode constants.
    rl.add_source(Some(&source), unsafe { kCFRunLoopCommonModes });

    let stopper = RunLoopRef(rl.clone());
    if shared.arm(Box::new(move || stopper.stop())) {
        shared.set_status(HookStatus::Running {
            backend: BACKEND.into(),
        });
        while !shared.stop_requested() {
            // SAFETY: CF-provided static mode constant.
            CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, RUN_SLICE_S, false);
        }
    }
    shared.disarm();
    CGEvent::tap_enable(&tap, false);
    tap.invalidate();
    // SAFETY: the tap is invalidated and its run loop is not running, so the callback is done.
    drop(unsafe { Box::from_raw(ctx) });
}

unsafe extern "C-unwind" fn callback(
    _proxy: CGEventTapProxy,
    ty: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    // SAFETY: `user_info` is the `TapCtx` registered in `run`, alive while the tap exists; the
    // callback only runs on the tap's run-loop thread.
    let ctx = unsafe { &mut *user_info.cast::<TapCtx>() };
    if ty == CGEventType::TapDisabledByTimeout || ty == CGEventType::TapDisabledByUserInput {
        if let Some(tap) = &ctx.tap {
            CGEvent::tap_enable(tap, true);
        }
        return event.as_ptr();
    }
    // SAFETY: the system passes a valid event for the duration of the callback.
    let ev = unsafe { event.as_ref() };
    let kvk = CGEvent::integer_value_field(Some(ev), CGEventField::KeyboardEventKeycode) as u16;
    let d = &mut ctx.dispatcher;
    let now = d.now_ms();
    if ty == CGEventType::KeyDown {
        d.dispatch(keycode::to_phys(kvk), KeyDir::Down, now);
    } else if ty == CGEventType::KeyUp {
        d.dispatch(keycode::to_phys(kvk), KeyDir::Up, now);
    } else if ty == CGEventType::FlagsChanged {
        if kvk == keycode::KVK_CAPS_LOCK {
            let key = keycode::to_phys(kvk);
            d.dispatch(key, KeyDir::Down, now);
            d.dispatch(key, KeyDir::Up, now);
        } else if let Some(bit) = keycode::modifier_mask(kvk) {
            let flags = CGEvent::flags(Some(ev)).0;
            let dir = if flags & bit != 0 {
                KeyDir::Down
            } else {
                KeyDir::Up
            };
            d.dispatch(keycode::to_phys(kvk), dir, now);
        }
    }
    // Listen-only: pass the event through unmodified.
    event.as_ptr()
}
