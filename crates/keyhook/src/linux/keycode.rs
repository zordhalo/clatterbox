//! evdev `KEY_*` code → PhysKey (SPEC §3.5); shared by X11 via `detail - 8`.
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

use clatterbox_core::PhysKey;

pub(crate) fn to_phys(code: u16) -> PhysKey {
    todo!("WP3")
}
