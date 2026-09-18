//! Key position map → (class, pan x) (SPEC §7).
//! PRIVACY: no logging, no Debug derives here.

use super::PhysKey;
use crate::types::KeyClass;

/// Width of the main block in key units.
const MAIN_WIDTH_U: f32 = 15.0;

/// `(key, width in u)` from the left edge, one table per main-block row (ANSI).
const NUMBER_ROW: &[(PhysKey, f32)] = &[
    (PhysKey::Grave, 1.0),
    (PhysKey::Digit1, 1.0),
    (PhysKey::Digit2, 1.0),
    (PhysKey::Digit3, 1.0),
    (PhysKey::Digit4, 1.0),
    (PhysKey::Digit5, 1.0),
    (PhysKey::Digit6, 1.0),
    (PhysKey::Digit7, 1.0),
    (PhysKey::Digit8, 1.0),
    (PhysKey::Digit9, 1.0),
    (PhysKey::Digit0, 1.0),
    (PhysKey::Minus, 1.0),
    (PhysKey::Equal, 1.0),
    (PhysKey::Backspace, 2.0),
];

const TOP_ROW: &[(PhysKey, f32)] = &[
    (PhysKey::Tab, 1.5),
    (PhysKey::Q, 1.0),
    (PhysKey::W, 1.0),
    (PhysKey::E, 1.0),
    (PhysKey::R, 1.0),
    (PhysKey::T, 1.0),
    (PhysKey::Y, 1.0),
    (PhysKey::U, 1.0),
    (PhysKey::I, 1.0),
    (PhysKey::O, 1.0),
    (PhysKey::P, 1.0),
    (PhysKey::BracketL, 1.0),
    (PhysKey::BracketR, 1.0),
    (PhysKey::Backslash, 1.5),
];

const HOME_ROW: &[(PhysKey, f32)] = &[
    (PhysKey::CapsLock, 1.75),
    (PhysKey::A, 1.0),
    (PhysKey::S, 1.0),
    (PhysKey::D, 1.0),
    (PhysKey::F, 1.0),
    (PhysKey::G, 1.0),
    (PhysKey::H, 1.0),
    (PhysKey::J, 1.0),
    (PhysKey::K, 1.0),
    (PhysKey::L, 1.0),
    (PhysKey::Semicolon, 1.0),
    (PhysKey::Quote, 1.0),
    (PhysKey::Enter, 2.25),
];

const BOTTOM_ROW: &[(PhysKey, f32)] = &[
    (PhysKey::ShiftL, 2.25),
    (PhysKey::Z, 1.0),
    (PhysKey::X, 1.0),
    (PhysKey::C, 1.0),
    (PhysKey::V, 1.0),
    (PhysKey::B, 1.0),
    (PhysKey::N, 1.0),
    (PhysKey::M, 1.0),
    (PhysKey::Comma, 1.0),
    (PhysKey::Period, 1.0),
    (PhysKey::Slash, 1.0),
    (PhysKey::ShiftR, 2.75),
];

const SPACE_ROW: &[(PhysKey, f32)] = &[
    (PhysKey::CtrlL, 1.25),
    (PhysKey::MetaL, 1.25),
    (PhysKey::AltL, 1.25),
    (PhysKey::Space, 6.25),
    (PhysKey::AltR, 1.25),
    (PhysKey::MetaR, 1.25),
    (PhysKey::Menu, 1.25),
    (PhysKey::CtrlR, 1.25),
];

/// Keys placed by explicit key-centre (in u), not by row walking.
const CENTRES: &[(PhysKey, f32)] = &[
    // Function row: Esc, gap, F1–F4, half gap, F5–F8, half gap, F9–F12.
    (PhysKey::Escape, 0.5),
    (PhysKey::F1, 2.5),
    (PhysKey::F2, 3.5),
    (PhysKey::F3, 4.5),
    (PhysKey::F4, 5.5),
    (PhysKey::F5, 7.0),
    (PhysKey::F6, 8.0),
    (PhysKey::F7, 9.0),
    (PhysKey::F8, 10.0),
    (PhysKey::F9, 11.5),
    (PhysKey::F10, 12.5),
    (PhysKey::F11, 13.5),
    (PhysKey::F12, 14.5),
    // ISO extras: 1.25u key right of a 1.25u left shift; Ro right of Slash; JIS Yen left of
    // a 1u Backspace.
    (PhysKey::IntlBackslash, 1.875),
    (PhysKey::IntlRo, 12.75),
    (PhysKey::IntlYen, 13.5),
    // Laptop Fn sits at the bottom-left corner.
    (PhysKey::Fn, 0.625),
    (PhysKey::Unknown, 7.5),
];

/// Navigation cluster, arrows and numpad sit right of the main block.
const RIGHT_OF_MAIN: f32 = 1.0;

/// Pan position per `PhysKey as usize`, computed at compile time.
const X_TABLE: [f32; PhysKey::COUNT] = build_x_table();

const fn build_x_table() -> [f32; PhysKey::COUNT] {
    let mut t = [RIGHT_OF_MAIN; PhysKey::COUNT];
    let rows = [NUMBER_ROW, TOP_ROW, HOME_ROW, BOTTOM_ROW, SPACE_ROW];
    let mut r = 0;
    while r < rows.len() {
        let row = rows[r];
        let mut left = 0.0;
        let mut i = 0;
        while i < row.len() {
            let (key, width) = row[i];
            t[key as usize] = clamp01((left + width / 2.0) / MAIN_WIDTH_U);
            left += width;
            i += 1;
        }
        r += 1;
    }
    let mut i = 0;
    while i < CENTRES.len() {
        let (key, centre) = CENTRES[i];
        t[key as usize] = clamp01(centre / MAIN_WIDTH_U);
        i += 1;
    }
    t
}

const fn clamp01(v: f32) -> f32 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

fn class_of(key: PhysKey) -> KeyClass {
    use PhysKey as K;
    match key {
        K::Space => KeyClass::Space,
        K::Enter | K::NumpadEnter => KeyClass::Enter,
        K::Backspace | K::Delete => KeyClass::Backspace,
        K::ShiftL
        | K::ShiftR
        | K::CtrlL
        | K::CtrlR
        | K::AltL
        | K::AltR
        | K::MetaL
        | K::MetaR
        | K::CapsLock
        | K::Tab
        | K::Fn
        | K::Menu => KeyClass::Modifier,
        _ => KeyClass::Default,
    }
}

/// `x` is `0.0..=1.0` (0 = far left). Unknown → `(Default, 0.5)`.
#[inline]
pub fn lookup(key: PhysKey) -> (KeyClass, f32) {
    (class_of(key), X_TABLE[key as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn x(key: PhysKey) -> f32 {
        lookup(key).1
    }

    fn all_keys() -> impl Iterator<Item = PhysKey> {
        (0..PhysKey::COUNT).filter_map(PhysKey::from_index)
    }

    #[test]
    fn every_x_in_unit_range() {
        for key in all_keys() {
            let v = x(key);
            assert!((0.0..=1.0).contains(&v), "x out of range: {v}");
        }
    }

    #[test]
    fn rows_fill_main_block() {
        for row in [NUMBER_ROW, TOP_ROW, HOME_ROW, BOTTOM_ROW, SPACE_ROW] {
            let total: f32 = row.iter().map(|&(_, w)| w).sum();
            assert!((total - MAIN_WIDTH_U).abs() < 1e-6, "row width {total}");
        }
    }

    #[test]
    fn home_row_strictly_increasing() {
        use PhysKey as K;
        let keys = [K::A, K::S, K::D, K::F, K::G, K::H, K::J, K::K, K::L];
        for w in keys.windows(2) {
            assert!(x(w[0]) < x(w[1]));
        }
    }

    #[test]
    fn known_positions() {
        assert!((x(PhysKey::Space) - 0.5).abs() <= 0.05);
        assert!((x(PhysKey::Escape) - 0.033).abs() < 0.001);
        assert!((x(PhysKey::F12) - 0.967).abs() < 0.001);
        assert!((x(PhysKey::IntlBackslash) - 1.875 / 15.0).abs() < 1e-6);
        assert!((x(PhysKey::IntlRo) - (x(PhysKey::Slash) + 1.0 / 15.0)).abs() < 1e-6);
        assert!(x(PhysKey::Unknown) == 0.5);
        for key in [
            PhysKey::ArrowLeft,
            PhysKey::Numpad5,
            PhysKey::Insert,
            PhysKey::Pause,
        ] {
            assert!(x(key) == 1.0);
        }
        assert!(x(PhysKey::ShiftL) < 0.1 && x(PhysKey::ShiftR) > 0.85);
    }

    #[test]
    fn class_mapping() {
        use PhysKey as K;
        let cases = [
            (K::Space, KeyClass::Space),
            (K::Enter, KeyClass::Enter),
            (K::NumpadEnter, KeyClass::Enter),
            (K::Backspace, KeyClass::Backspace),
            (K::Delete, KeyClass::Backspace),
            (K::ShiftL, KeyClass::Modifier),
            (K::CtrlR, KeyClass::Modifier),
            (K::MetaL, KeyClass::Modifier),
            (K::CapsLock, KeyClass::Modifier),
            (K::Tab, KeyClass::Modifier),
            (K::Fn, KeyClass::Modifier),
            (K::Menu, KeyClass::Modifier),
            (K::A, KeyClass::Default),
            (K::F5, KeyClass::Default),
            (K::Numpad7, KeyClass::Default),
            (K::Unknown, KeyClass::Default),
        ];
        for (key, class) in cases {
            assert!(lookup(key).0 == class);
        }
    }
}
