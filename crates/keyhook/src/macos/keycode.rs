//! macOS virtual keycode (`kVK_*`) → PhysKey (SPEC §3.5).
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

use clatterbox_core::PhysKey;

pub(crate) fn to_phys(kvk: u16) -> PhysKey {
    todo!("WP3")
}
