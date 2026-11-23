//! Favorite anything: still pictures and attached videos in messages can be
//! favourites too, not only pictures that move.

use crate::chat::Gif;

#[cfg(not(test))]
static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
thread_local! {
    // Each test its own, out of the way of the GIF tests beside it.
    static ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Turns the plugin on or off.
pub fn set(on: bool) {
    #[cfg(not(test))]
    ON.store(on, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(|cell| cell.set(on));
}

/// Whether the plugin is on.
pub fn is_on() -> bool {
    #[cfg(not(test))]
    return ON.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(std::cell::Cell::get)
}

/// The id a still picture's favourite carries on its message, which tells
/// it from a moving one's: a message's GIF has none. It stays on the
/// message: a favourite kept on Discord carries no id.
pub const STILL: &str = "halocord-still";

/// Whether a message's GIF is a still picture's, which Favorite anything
/// offered: it has no GIF corner.
pub fn still(gif: &Gif) -> bool {
    gif.id.as_deref() == Some(STILL)
}
