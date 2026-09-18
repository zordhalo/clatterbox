//! Physical key positions (SPEC §3.5).
//! PRIVACY: no logging, no Debug derives under `keymap/` (enforced by scripts/check-privacy.sh).
//! Tests compare with `assert!(a == b)`, not `assert_eq!`.

pub mod layout;

/// Physical positions on an ANSI/ISO 105-key board, named by US-ANSI legends.
/// Discriminants must stay `< 128` (`RepeatFilter` bitset). Intentionally NOT Debug.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysKey {
    // Function row
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // Number row
    Grave,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Digit0,
    Minus,
    Equal,
    Backspace,
    // Top row
    Tab,
    Q,
    W,
    E,
    R,
    T,
    Y,
    U,
    I,
    O,
    P,
    BracketL,
    BracketR,
    Backslash,
    // Home row
    CapsLock,
    A,
    S,
    D,
    F,
    G,
    H,
    J,
    K,
    L,
    Semicolon,
    Quote,
    Enter,
    // Bottom row
    ShiftL,
    Z,
    X,
    C,
    V,
    B,
    N,
    M,
    Comma,
    Period,
    Slash,
    ShiftR,
    // Space row
    CtrlL,
    MetaL,
    AltL,
    Space,
    AltR,
    MetaR,
    Menu,
    CtrlR,
    Fn,
    // Navigation cluster
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PageUp,
    Delete,
    End,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    // Numpad
    NumLock,
    NumpadDivide,
    NumpadMultiply,
    NumpadSubtract,
    NumpadAdd,
    NumpadEnter,
    NumpadDecimal,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    // ISO extras
    IntlBackslash,
    IntlRo,
    IntlYen,
    /// Unmapped code: plays `Default` class at x = 0.5.
    Unknown,
}

impl PhysKey {
    /// Number of variants (including `Unknown`).
    pub const COUNT: usize = PhysKey::Unknown as usize + 1;

    /// Inverse of `key as usize`; `None` if `i >= COUNT`.
    pub const fn from_index(i: usize) -> Option<PhysKey> {
        if i < Self::COUNT {
            // SAFETY: `PhysKey` is `repr(u8)` with implicit, contiguous discriminants
            // `0..COUNT`, and `i < COUNT <= 128` fits in `u8`.
            Some(unsafe { std::mem::transmute::<u8, PhysKey>(i as u8) })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PhysKey;

    #[test]
    fn index_round_trips() {
        const { assert!(PhysKey::COUNT <= 128) };
        for i in 0..PhysKey::COUNT {
            let key = PhysKey::from_index(i).expect("in range");
            assert!(key as usize == i);
        }
        assert!(PhysKey::from_index(PhysKey::COUNT).is_none());
    }
}
