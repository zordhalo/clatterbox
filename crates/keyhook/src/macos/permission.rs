//! Input Monitoring permission (kTCCServiceListenEvent) (SPEC §3.3).

use objc2_core_graphics::{CGPreflightListenEventAccess, CGRequestListenEventAccess};

/// `CGPreflightListenEventAccess()`.
pub(crate) fn preflight() -> bool {
    CGPreflightListenEventAccess()
}

/// `CGRequestListenEventAccess()`; returns whether access is granted now.
pub(crate) fn request() -> bool {
    CGRequestListenEventAccess() || preflight()
}
