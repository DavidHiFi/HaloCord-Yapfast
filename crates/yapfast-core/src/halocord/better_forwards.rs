//! Better forwards: a message forwarded to one conversation leaves the one
//! open as it is, rather than opening where it went.

use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(false);

/// Turns the plugin on or off.
pub fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Whether a forward to one conversation stays where it is.
pub fn stays() -> bool {
    ON.load(Ordering::Relaxed)
}
