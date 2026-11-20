//! HaloCord's settings for Yapfast's core, kept here so Yapfast's own files
//! carry only the hooks that read them. `docs/halocord.md` lists every hook.

use std::sync::atomic::{AtomicBool, Ordering};

/// The bit rates HaloCord's page offers, in kbit/s.
pub const BITRATES: [u32; 5] = [64, 128, 256, 384, 510];
/// The bit rate HaloCord sends unless changed, in kbit/s: the most Opus sends.
pub const DEFAULT_BITRATE: u32 = 510;

static FLAT: AtomicBool = AtomicBool::new(false);

/// Sends every call's microphone flat, in stereo: it opens with both its
/// channels and skips echo cancellation, the rumble filter, noise
/// suppression, gain control and leveling, so only its volume and the voice
/// gate (or push to talk) apply. A call going on switches within a block.
/// For music, instruments, or a good microphone with headphones.
pub fn set_flat(on: bool) {
    FLAT.store(on, Ordering::Relaxed);
}

/// Whether the microphone goes flat.
pub fn flat() -> bool {
    FLAT.load(Ordering::Relaxed)
}

/// Sets the bit rate Discord calls send the microphone at, in kbit/s, or
/// Yapfast's own for `None`; a call going on takes it at its next frame.
pub fn set_bitrate(kbps: Option<u32>) {
    #[cfg(feature = "discord")]
    {
        use yapfast_discord::voice;
        voice::set_microphone_bitrate(kbps.map_or(voice::DEFAULT_MICROPHONE_BITRATE, |kbps| {
            kbps.saturating_mul(1000)
        }));
    }
    #[cfg(not(feature = "discord"))]
    let _ = kbps;
}
