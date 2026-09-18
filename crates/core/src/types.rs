//! Shared contract types (SPEC §4.6). FROZEN: changes need lead approval.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyClass {
    Default = 0,
    Space = 1,
    Enter = 2,
    Backspace = 3,
    Modifier = 4,
}

impl KeyClass {
    pub const COUNT: usize = 5;
    pub const ALL: [KeyClass; 5] = [
        Self::Default,
        Self::Space,
        Self::Enter,
        Self::Backspace,
        Self::Modifier,
    ];

    /// "default" | "space" | "enter" | "backspace" | "modifier"
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Space => "space",
            Self::Enter => "enter",
            Self::Backspace => "backspace",
            Self::Modifier => "modifier",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyDir {
    Down,
    Up,
}

/// Privacy boundary: carries no key identity. Intentionally NOT Debug.
#[derive(Clone, Copy)]
pub struct KeySound {
    pub class: KeyClass,
    pub dir: KeyDir,
    /** 0.0 = far left .. 1.0 = far right */
    pub x: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PackSlot {
    Main = 0,
    Preview = 1,
}

/// Intentionally NOT Debug.
#[derive(Clone, Copy)]
pub struct Trigger {
    pub sound: KeySound,
    pub slot: PackSlot,
}

impl Trigger {
    pub fn key(sound: KeySound) -> Self {
        Self {
            sound,
            slot: PackSlot::Main,
        }
    }
    pub fn preview(sound: KeySound) -> Self {
        Self {
            sound,
            slot: PackSlot::Preview,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum HookStatus {
    Starting,
    /// backend: "raw_input" | "event_tap" | "x11_xi2" | "evdev"
    Running {
        backend: String,
    },
    NeedsPermission {
        hint: String,
    },
    /// macOS: granted, relaunch required
    NeedsRestart,
    Unsupported {
        reason: String,
    },
    Failed {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AudioStatus {
    Starting,
    Running {
        device: String,
        sample_rate: u32,
        buffer_frames: Option<u32>,
    },
    NoDevice,
    Failed {
        reason: String,
    },
}

/// Wait-free producer end of a `Trigger` queue (SPEC §4.3). Lives in core so that keyhook does
/// not depend on audio. Intentionally NOT Debug.
pub struct TriggerTx(pub rtrb::Producer<Trigger>);

impl TriggerTx {
    /// Returns `false` if the queue is full (the trigger is dropped).
    #[inline]
    pub fn push(&mut self, t: Trigger) -> bool {
        self.0.push(t).is_ok()
    }
}
