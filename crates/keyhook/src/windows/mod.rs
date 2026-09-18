//! Windows backend: Raw Input on a message-only window (SPEC §3.2).

pub(crate) mod scancode;

use std::mem::{MaybeUninit, size_of};

use ::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use ::windows::Win32::System::LibraryLoader::GetModuleHandleW;
use ::windows::Win32::UI::Input::{
    GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICE_FLAGS, RAWINPUTHEADER,
    RID_INPUT, RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEKEYBOARD, RegisterRawInputDevices,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG, PostMessageW,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_INPUT,
};
use ::windows::core::w;
use clatterbox_core::HookStatus;

use crate::dispatch::Dispatcher;
use crate::shared::Shared;

pub(crate) const BACKEND: &str = "raw_input";

/// HID generic desktop / keyboard.
const USAGE_PAGE_GENERIC: u16 = 0x01;
const USAGE_KEYBOARD: u16 = 0x06;

pub(crate) fn run(mut d: Dispatcher, shared: &Shared) {
    // SAFETY: plain Win32 calls on this thread; the window is used and destroyed only here.
    let hwnd = match unsafe { create_window() } {
        Ok(h) => h,
        Err(e) => return shared.failed(format!("raw input window: {e}")),
    };
    if let Err(e) = register(hwnd, RIDEV_INPUTSINK) {
        // SAFETY: `hwnd` was created on this thread.
        let _ = unsafe { DestroyWindow(hwnd) };
        return shared.failed(format!("RegisterRawInputDevices: {e}"));
    }

    // HWND is not Send; carry it as an integer into the waker.
    let target = hwnd.0 as isize;
    let armed = shared.arm(Box::new(move || {
        // SAFETY: posting to a stale handle fails harmlessly.
        let _ =
            unsafe { PostMessageW(Some(HWND(target as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }));
    if armed {
        shared.set_status(HookStatus::Running {
            backend: BACKEND.into(),
        });
        message_loop(&mut d);
    }
    shared.disarm();
    let _ = register(HWND::default(), RIDEV_REMOVE);
    // SAFETY: fails harmlessly if WM_CLOSE already destroyed it.
    let _ = unsafe { DestroyWindow(hwnd) };
}

fn message_loop(d: &mut Dispatcher) {
    let mut msg = MSG::default();
    loop {
        // SAFETY: `msg` is a valid out-pointer; 0 = WM_QUIT, -1 = error.
        let r = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if r.0 <= 0 {
            return;
        }
        match msg.message {
            // Posted by the stop waker.
            WM_CLOSE => return,
            WM_INPUT => on_input(d, msg.lParam),
            _ => {}
        }
        // Lets DefWindowProc clean up the raw input handle.
        // SAFETY: `msg` came from GetMessageW.
        unsafe { DispatchMessageW(&msg) };
    }
}

#[inline]
fn on_input(d: &mut Dispatcher, lparam: LPARAM) {
    let mut raw = MaybeUninit::<RAWINPUT>::zeroed();
    let mut size = size_of::<RAWINPUT>() as u32;
    // SAFETY: the buffer is a full RAWINPUT (fixed-size for keyboards) and `size` says so.
    let n = unsafe {
        GetRawInputData(
            HRAWINPUT(lparam.0 as *mut _),
            RID_INPUT,
            Some(raw.as_mut_ptr().cast()),
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    if n == 0 || n == u32::MAX {
        return;
    }
    // SAFETY: zero-initialised POD, partially filled by the OS.
    let raw = unsafe { raw.assume_init() };
    if raw.header.dwType != RIM_TYPEKEYBOARD.0 {
        return;
    }
    // SAFETY: dwType says the union holds a keyboard record.
    let kb = unsafe { raw.data.keyboard };
    if let Some((key, dir)) = scancode::classify(kb.MakeCode, kb.Flags, kb.VKey) {
        let now = d.now_ms();
        d.dispatch(key, dir, now);
    }
}

fn register(hwnd: HWND, flags: RAWINPUTDEVICE_FLAGS) -> ::windows::core::Result<()> {
    let dev = RAWINPUTDEVICE {
        usUsagePage: USAGE_PAGE_GENERIC,
        usUsage: USAGE_KEYBOARD,
        dwFlags: flags,
        hwndTarget: hwnd,
    };
    // SAFETY: one valid, fully initialised device record.
    unsafe { RegisterRawInputDevices(&[dev], size_of::<RAWINPUTDEVICE>() as u32) }
}

unsafe fn create_window() -> ::windows::core::Result<HWND> {
    // SAFETY: the system "Message" class exists for message-only windows; its window procedure
    // is DefWindowProc. All messages we care about are posted and handled in `message_loop`.
    unsafe {
        let hinstance = GetModuleHandleW(None)?;
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Message"),
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinstance.into()),
            None,
        )
    }
}
