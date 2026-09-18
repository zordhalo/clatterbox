//! Set-1 scan code (+E0) → PhysKey (SPEC §3.5).
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

use clatterbox_core::PhysKey;

pub(crate) fn to_phys(make_code: u16, e0: bool) -> PhysKey {
    todo!("WP3")
}
