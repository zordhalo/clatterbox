//! Input Monitoring permission (kTCCServiceListenEvent) (SPEC §3.3).
// WP0 stub: remove this allow once implemented (WP3).
#![allow(unused_variables, dead_code)]

/// `CGPreflightListenEventAccess()`.
pub(crate) fn preflight() -> bool {
    todo!("WP3")
}

/// `CGRequestListenEventAccess()`; returns whether access is granted now.
pub(crate) fn request() -> bool {
    todo!("WP3")
}
