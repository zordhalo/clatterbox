//! Key position map → (class, pan x) (SPEC §7).
//! PRIVACY: no logging, no Debug derives here.
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

use super::PhysKey;
use crate::types::KeyClass;

/// `x` is `0.0..=1.0` (0 = far left). Unknown → `(Default, 0.5)`.
pub fn lookup(key: PhysKey) -> (KeyClass, f32) {
    todo!("WP3")
}
