//! macOS virtual keycode (`kVK_*`) → PhysKey (SPEC §3.5).

use clatterbox_core::PhysKey;

/// `kVK_CapsLock`: FlagsChanged has no reliable release, so it plays Down then Up.
pub(crate) const KVK_CAPS_LOCK: u16 = 0x39;

/// Device-dependent `CGEventFlags` bit whose state gives the direction of a FlagsChanged
/// event for this modifier (`NX_DEVICE*KEYMASK`; Fn uses `kCGEventFlagMaskSecondaryFn`).
pub(crate) fn modifier_mask(kvk: u16) -> Option<u64> {
    Some(match kvk {
        0x38 => 0x02,      // Shift
        0x3C => 0x04,      // RightShift
        0x3B => 0x01,      // Control
        0x3E => 0x2000,    // RightControl
        0x3A => 0x20,      // Option
        0x3D => 0x40,      // RightOption
        0x37 => 0x08,      // Command
        0x36 => 0x10,      // RightCommand
        0x3F => 0x80_0000, // Function
        _ => return None,
    })
}

pub(crate) fn to_phys(kvk: u16) -> PhysKey {
    use PhysKey as K;
    match kvk {
        0x00 => K::A,
        0x01 => K::S,
        0x02 => K::D,
        0x03 => K::F,
        0x04 => K::H,
        0x05 => K::G,
        0x06 => K::Z,
        0x07 => K::X,
        0x08 => K::C,
        0x09 => K::V,
        0x0A => K::IntlBackslash, // ISO_Section
        0x0B => K::B,
        0x0C => K::Q,
        0x0D => K::W,
        0x0E => K::E,
        0x0F => K::R,
        0x10 => K::Y,
        0x11 => K::T,
        0x12 => K::Digit1,
        0x13 => K::Digit2,
        0x14 => K::Digit3,
        0x15 => K::Digit4,
        0x16 => K::Digit6,
        0x17 => K::Digit5,
        0x18 => K::Equal,
        0x19 => K::Digit9,
        0x1A => K::Digit7,
        0x1B => K::Minus,
        0x1C => K::Digit8,
        0x1D => K::Digit0,
        0x1E => K::BracketR,
        0x1F => K::O,
        0x20 => K::U,
        0x21 => K::BracketL,
        0x22 => K::I,
        0x23 => K::P,
        0x24 => K::Enter,
        0x25 => K::L,
        0x26 => K::J,
        0x27 => K::Quote,
        0x28 => K::K,
        0x29 => K::Semicolon,
        0x2A => K::Backslash,
        0x2B => K::Comma,
        0x2C => K::Slash,
        0x2D => K::N,
        0x2E => K::M,
        0x2F => K::Period,
        0x30 => K::Tab,
        0x31 => K::Space,
        0x32 => K::Grave,
        0x33 => K::Backspace, // kVK_Delete
        0x35 => K::Escape,
        0x36 => K::MetaR,
        0x37 => K::MetaL,
        0x38 => K::ShiftL,
        0x39 => K::CapsLock,
        0x3A => K::AltL,
        0x3B => K::CtrlL,
        0x3C => K::ShiftR,
        0x3D => K::AltR,
        0x3E => K::CtrlR,
        0x3F => K::Fn,
        0x41 => K::NumpadDecimal,
        0x43 => K::NumpadMultiply,
        0x45 => K::NumpadAdd,
        0x47 => K::NumLock, // Keypad Clear
        0x4B => K::NumpadDivide,
        0x4C => K::NumpadEnter,
        0x4E => K::NumpadSubtract,
        0x52 => K::Numpad0,
        0x53 => K::Numpad1,
        0x54 => K::Numpad2,
        0x55 => K::Numpad3,
        0x56 => K::Numpad4,
        0x57 => K::Numpad5,
        0x58 => K::Numpad6,
        0x59 => K::Numpad7,
        0x5B => K::Numpad8,
        0x5C => K::Numpad9,
        0x5D => K::IntlYen,
        0x5E => K::IntlRo, // JIS_Underscore
        0x60 => K::F5,
        0x61 => K::F6,
        0x62 => K::F7,
        0x63 => K::F3,
        0x64 => K::F8,
        0x65 => K::F9,
        0x67 => K::F11,
        0x69 => K::PrintScreen, // F13 on Apple extended keyboards
        0x6B => K::ScrollLock,  // F14
        0x6D => K::F10,
        0x6F => K::F12,
        0x71 => K::Pause,  // F15
        0x72 => K::Insert, // Help
        0x73 => K::Home,
        0x74 => K::PageUp,
        0x75 => K::Delete, // ForwardDelete
        0x76 => K::F4,
        0x77 => K::End,
        0x78 => K::F2,
        0x79 => K::PageDown,
        0x7A => K::F1,
        0x7B => K::ArrowLeft,
        0x7C => K::ArrowRight,
        0x7D => K::ArrowDown,
        0x7E => K::ArrowUp,
        _ => K::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes() {
        assert!(to_phys(0x00) == PhysKey::A);
        assert!(to_phys(0x31) == PhysKey::Space);
        assert!(to_phys(0x24) == PhysKey::Enter);
        assert!(to_phys(0x33) == PhysKey::Backspace);
        assert!(to_phys(0x7F) == PhysKey::Unknown);
        assert!(modifier_mask(0x38) == Some(0x02));
        assert!(modifier_mask(0x00).is_none());
    }

    #[test]
    fn no_duplicate_mappings() {
        let mut seen = [0u8; PhysKey::COUNT];
        for code in 0..=0xFFu16 {
            let k = to_phys(code);
            if k != PhysKey::Unknown {
                seen[k as usize] += 1;
            }
        }
        assert!(seen.iter().all(|&n| n <= 1));
    }
}
