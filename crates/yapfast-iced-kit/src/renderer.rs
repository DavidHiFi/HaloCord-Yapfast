//! Which renderer draws: tiny-skia on the CPU or wgpu on the GPU, both in
//! the one build, chosen once as Yapfast starts (`choose`), before any
//! picture is made, as each takes its pixels in its own format. The CPU's
//! unless chosen otherwise, as the tests draw.

use std::sync::atomic::{AtomicBool, Ordering};

static GPU: AtomicBool = AtomicBool::new(false);

/// Draws with the GPU, or on the CPU: only that renderer may start when
/// the window opens, so a GPU that can't start is the window's error,
/// not a window drawn on the CPU from pictures made for the GPU.
pub fn choose(gpu: bool) {
    GPU.store(gpu, Ordering::Relaxed);
    iced_wgpu::set_enabled(gpu);
    iced_tiny_skia::set_enabled(!gpu);
}

/// Whether the GPU draws: pictures are then straight RGBA, as wgpu uploads
/// them; on the CPU, premultiplied BGRA, as tiny-skia copies them.
pub fn gpu() -> bool {
    GPU.load(Ordering::Relaxed)
}

/// Asks for a see-through window surface (HaloCord's glass) for the window
/// that opens next. Only the GPU can give one.
pub fn want_glass(want: bool) {
    iced_wgpu::glass::set_wanted(want && gpu());
}

/// Whether the window's surface really is see-through: the GPU started
/// with one. False on the CPU, with glass not asked for, or when the GPU
/// could only start the plain surface.
pub fn glass_active() -> bool {
    gpu() && iced_wgpu::glass::active()
}

/// Whether the window about to open asks for a see-through surface.
pub fn glass_wanted() -> bool {
    iced_wgpu::glass::wanted()
}
