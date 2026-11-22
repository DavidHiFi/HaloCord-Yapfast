//! Yapfast port: a see-through window surface, for HaloCord's glass.
//!
//! A normal wgpu window on Windows has no alpha: its swap chain is made
//! straight from the window handle. Glass needs one made from a
//! DirectComposition visual, with a premultiplied alpha mode, so what the
//! app draws with alpha lets Windows' own backdrop (acrylic) show through.
//!
//! The app asks before the window opens ([`set_wanted`]). The compositor
//! then tries that surface first and says whether it got it ([`active`]);
//! if it didn't, it starts as before, opaque, so the GPU is not lost.

use std::sync::atomic::{AtomicBool, Ordering};

static WANTED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Asks for a see-through surface for the windows made from now on.
pub fn set_wanted(wanted: bool) {
    WANTED.store(wanted, Ordering::Relaxed);
}

/// Whether the next compositor should try for a see-through surface.
pub fn wanted() -> bool {
    WANTED.load(Ordering::Relaxed)
}

/// Whether the compositor got one: the surface takes the alpha the app
/// draws. False until a window has started, and when it fell back.
pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub(crate) fn mark_active(active: bool) {
    ACTIVE.store(active, Ordering::Relaxed);
}
