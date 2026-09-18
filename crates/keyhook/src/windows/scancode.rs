//! Set-1 scan code (+E0) → PhysKey (SPEC §3.5).

use clatterbox_core::{KeyDir, PhysKey};

/// `RAWKEYBOARD.Flags` bits.
const RI_KEY_BREAK: u16 = 1;
const RI_KEY_E0: u16 = 2;
const RI_KEY_E1: u16 = 4;
/// `VK_PAUSE`; Pause arrives as an E1-prefixed sequence and is identified by its VKey.
const VK_PAUSE: u16 = 0x13;
/// VKey of synthetic halves of multi-code sequences.
const VK_FAKE: u16 = 0xFF;

/// Reduces one `RAWKEYBOARD` record to `(key, dir)`, or `None` for overrun / fake / synthetic
/// sequence codes that must not make a sound.
#[inline]
pub(crate) fn classify(make_code: u16, flags: u16, vkey: u16) -> Option<(PhysKey, KeyDir)> {
    let dir = if flags & RI_KEY_BREAK != 0 {
        KeyDir::Up
    } else {
        KeyDir::Down
    };
    if vkey == VK_PAUSE {
        return Some((PhysKey::Pause, dir));
    }
    if make_code == 0 || make_code == 0xFF || vkey == VK_FAKE || flags & RI_KEY_E1 != 0 {
        return None;
    }
    let e0 = flags & RI_KEY_E0 != 0;
    // E0-prefixed shifts are fake shifts the keyboard sends around navigation keys.
    if e0 && (make_code == 0x2A || make_code == 0x36) {
        return None;
    }
    Some((to_phys(make_code, e0), dir))
}

pub(crate) fn to_phys(make_code: u16, e0: bool) -> PhysKey {
    use PhysKey as K;
    if e0 {
        return match make_code {
            0x1C => K::NumpadEnter,
            0x1D => K::CtrlR,
            0x35 => K::NumpadDivide,
            0x37 => K::PrintScreen,
            0x38 => K::AltR,
            0x45 => K::NumLock,
            0x46 => K::Pause, // Ctrl+Break
            0x47 => K::Home,
            0x48 => K::ArrowUp,
            0x49 => K::PageUp,
            0x4B => K::ArrowLeft,
            0x4D => K::ArrowRight,
            0x4F => K::End,
            0x50 => K::ArrowDown,
            0x51 => K::PageDown,
            0x52 => K::Insert,
            0x53 => K::Delete,
            0x5B => K::MetaL,
            0x5C => K::MetaR,
            0x5D => K::Menu,
            _ => K::Unknown,
        };
    }
    match make_code {
        0x01 => K::Escape,
        0x02 => K::Digit1,
        0x03 => K::Digit2,
        0x04 => K::Digit3,
        0x05 => K::Digit4,
        0x06 => K::Digit5,
        0x07 => K::Digit6,
        0x08 => K::Digit7,
        0x09 => K::Digit8,
        0x0A => K::Digit9,
        0x0B => K::Digit0,
        0x0C => K::Minus,
        0x0D => K::Equal,
        0x0E => K::Backspace,
        0x0F => K::Tab,
        0x10 => K::Q,
        0x11 => K::W,
        0x12 => K::E,
        0x13 => K::R,
        0x14 => K::T,
        0x15 => K::Y,
        0x16 => K::U,
        0x17 => K::I,
        0x18 => K::O,
        0x19 => K::P,
        0x1A => K::BracketL,
        0x1B => K::BracketR,
        0x1C => K::Enter,
        0x1D => K::CtrlL,
        0x1E => K::A,
        0x1F => K::S,
        0x20 => K::D,
        0x21 => K::F,
        0x22 => K::G,
        0x23 => K::H,
        0x24 => K::J,
        0x25 => K::K,
        0x26 => K::L,
        0x27 => K::Semicolon,
        0x28 => K::Quote,
        0x29 => K::Grave,
        0x2A => K::ShiftL,
        0x2B => K::Backslash,
        0x2C => K::Z,
        0x2D => K::X,
        0x2E => K::C,
        0x2F => K::V,
        0x30 => K::B,
        0x31 => K::N,
        0x32 => K::M,
        0x33 => K::Comma,
        0x34 => K::Period,
        0x35 => K::Slash,
        0x36 => K::ShiftR,
        0x37 => K::NumpadMultiply,
        0x38 => K::AltL,
        0x39 => K::Space,
        0x3A => K::CapsLock,
        0x3B => K::F1,
        0x3C => K::F2,
        0x3D => K::F3,
        0x3E => K::F4,
        0x3F => K::F5,
        0x40 => K::F6,
        0x41 => K::F7,
        0x42 => K::F8,
        0x43 => K::F9,
        0x44 => K::F10,
        0x45 => K::NumLock,
        0x46 => K::ScrollLock,
        0x47 => K::Numpad7,
        0x48 => K::Numpad8,
        0x49 => K::Numpad9,
        0x4A => K::NumpadSubtract,
        0x4B => K::Numpad4,
        0x4C => K::Numpad5,
        0x4D => K::Numpad6,
        0x4E => K::NumpadAdd,
        0x4F => K::Numpad1,
        0x50 => K::Numpad2,
        0x51 => K::Numpad3,
        0x52 => K::Numpad0,
        0x53 => K::NumpadDecimal,
        0x54 => K::PrintScreen, // Alt+SysRq
        0x56 => K::IntlBackslash,
        0x57 => K::F11,
        0x58 => K::F12,
        0x73 => K::IntlRo,
        0x7D => K::IntlYen,
        _ => K::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes() {
        assert!(to_phys(0x1E, false) == PhysKey::A);
        assert!(to_phys(0x39, false) == PhysKey::Space);
        assert!(to_phys(0x1C, false) == PhysKey::Enter);
        assert!(to_phys(0x1C, true) == PhysKey::NumpadEnter);
        assert!(to_phys(0x4B, false) == PhysKey::Numpad4);
        assert!(to_phys(0x4B, true) == PhysKey::ArrowLeft);
        assert!(to_phys(0x5B, true) == PhysKey::MetaL);
        assert!(to_phys(0x7F, false) == PhysKey::Unknown);
    }

    #[test]
    fn classify_filters_fake_and_sets_direction() {
        let a_down = classify(0x1E, 0, 0x41).expect("A down");
        assert!(a_down.0 == PhysKey::A && a_down.1 == KeyDir::Down);
        let a_up = classify(0x1E, RI_KEY_BREAK, 0x41).expect("A up");
        assert!(a_up.1 == KeyDir::Up);
        assert!(classify(0x00, 0, 0x41).is_none());
        assert!(classify(0xFF, 0, 0x41).is_none());
        assert!(classify(0x2A, RI_KEY_E0, 0x10).is_none());
        assert!(classify(0x45, 0, VK_FAKE).is_none());
        assert!(classify(0x1D, RI_KEY_E1, 0x11).is_none());
        let pause = classify(0x1D, RI_KEY_E1, VK_PAUSE).expect("pause");
        assert!(pause.0 == PhysKey::Pause);
    }

    #[test]
    fn no_duplicate_mappings() {
        // Aliases that legitimately share a PhysKey.
        let aliases = [PhysKey::PrintScreen, PhysKey::NumLock, PhysKey::Pause];
        let mut seen = [0u8; PhysKey::COUNT];
        for e0 in [false, true] {
            for code in 0..=0xFFu16 {
                let k = to_phys(code, e0);
                if k != PhysKey::Unknown {
                    seen[k as usize] += 1;
                }
            }
        }
        for (i, &n) in seen.iter().enumerate() {
            let key = PhysKey::from_index(i).expect("in range");
            assert!(n <= 1 || aliases.contains(&key));
        }
    }
}
