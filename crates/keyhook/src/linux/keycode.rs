//! evdev `KEY_*` code → PhysKey (SPEC §3.5); shared by X11 via `detail - 8`.

use clatterbox_core::PhysKey;

pub(crate) fn to_phys(code: u16) -> PhysKey {
    use PhysKey as K;
    match code {
        1 => K::Escape,
        2 => K::Digit1,
        3 => K::Digit2,
        4 => K::Digit3,
        5 => K::Digit4,
        6 => K::Digit5,
        7 => K::Digit6,
        8 => K::Digit7,
        9 => K::Digit8,
        10 => K::Digit9,
        11 => K::Digit0,
        12 => K::Minus,
        13 => K::Equal,
        14 => K::Backspace,
        15 => K::Tab,
        16 => K::Q,
        17 => K::W,
        18 => K::E,
        19 => K::R,
        20 => K::T,
        21 => K::Y,
        22 => K::U,
        23 => K::I,
        24 => K::O,
        25 => K::P,
        26 => K::BracketL,
        27 => K::BracketR,
        28 => K::Enter,
        29 => K::CtrlL,
        30 => K::A,
        31 => K::S,
        32 => K::D,
        33 => K::F,
        34 => K::G,
        35 => K::H,
        36 => K::J,
        37 => K::K,
        38 => K::L,
        39 => K::Semicolon,
        40 => K::Quote,
        41 => K::Grave,
        42 => K::ShiftL,
        43 => K::Backslash,
        44 => K::Z,
        45 => K::X,
        46 => K::C,
        47 => K::V,
        48 => K::B,
        49 => K::N,
        50 => K::M,
        51 => K::Comma,
        52 => K::Period,
        53 => K::Slash,
        54 => K::ShiftR,
        55 => K::NumpadMultiply,
        56 => K::AltL,
        57 => K::Space,
        58 => K::CapsLock,
        59 => K::F1,
        60 => K::F2,
        61 => K::F3,
        62 => K::F4,
        63 => K::F5,
        64 => K::F6,
        65 => K::F7,
        66 => K::F8,
        67 => K::F9,
        68 => K::F10,
        69 => K::NumLock,
        70 => K::ScrollLock,
        71 => K::Numpad7,
        72 => K::Numpad8,
        73 => K::Numpad9,
        74 => K::NumpadSubtract,
        75 => K::Numpad4,
        76 => K::Numpad5,
        77 => K::Numpad6,
        78 => K::NumpadAdd,
        79 => K::Numpad1,
        80 => K::Numpad2,
        81 => K::Numpad3,
        82 => K::Numpad0,
        83 => K::NumpadDecimal,
        86 => K::IntlBackslash, // KEY_102ND
        87 => K::F11,
        88 => K::F12,
        89 => K::IntlRo,
        96 => K::NumpadEnter,
        97 => K::CtrlR,
        98 => K::NumpadDivide,
        99 => K::PrintScreen, // KEY_SYSRQ
        100 => K::AltR,
        102 => K::Home,
        103 => K::ArrowUp,
        104 => K::PageUp,
        105 => K::ArrowLeft,
        106 => K::ArrowRight,
        107 => K::End,
        108 => K::ArrowDown,
        109 => K::PageDown,
        110 => K::Insert,
        111 => K::Delete,
        119 => K::Pause,
        124 => K::IntlYen,
        125 => K::MetaL,
        126 => K::MetaR,
        127 => K::Menu, // KEY_COMPOSE
        464 => K::Fn,
        _ => K::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes() {
        assert!(to_phys(30) == PhysKey::A);
        assert!(to_phys(57) == PhysKey::Space);
        assert!(to_phys(28) == PhysKey::Enter);
        assert!(to_phys(96) == PhysKey::NumpadEnter);
        assert!(to_phys(0) == PhysKey::Unknown);
        // X11 keycode = evdev code + 8.
        assert!(to_phys((38 - super::super::x11::X_KEYCODE_OFFSET) as u16) == PhysKey::A);
    }

    #[test]
    fn no_duplicate_mappings() {
        let mut seen = [0u8; PhysKey::COUNT];
        for code in 0..=0x2FFu16 {
            let k = to_phys(code);
            if k != PhysKey::Unknown {
                seen[k as usize] += 1;
            }
        }
        assert!(seen.iter().all(|&n| n <= 1));
    }
}
