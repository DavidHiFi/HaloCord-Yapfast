//! When the window's glass shows, and how Windows is asked to blur.
//!
//! The glass is the same focused or not. Windows' own backdrop for a window
//! (the system acrylic) goes flat grey once the window is behind another,
//! so the window asks for the accent policy's acrylic blur instead, which
//! keeps blurring while the window isn't in front. Nothing about focus
//! reaches the window's messages, so the app never thinks it has the
//! keyboard when it hasn't.
//!
//! The fills go opaque only when Windows draws no blur at all: its
//! transparency effects are off, or battery saver is on.

/// What Windows says about its own effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effects {
    /// Settings > Personalization > Colors > Transparency effects.
    pub transparency: bool,
    /// Battery saver is on, which turns the blur off.
    pub battery_saver: bool,
}

impl Effects {
    /// Nothing known to switch the blur off.
    #[cfg(any(test, not(windows)))]
    pub const ALL_ON: Self = Self {
        transparency: true,
        battery_saver: false,
    };

    /// Whether Windows draws a blur behind the window.
    pub fn blurs(self) -> bool {
        self.transparency && !self.battery_saver
    }
}

/// Whether the glass fills show: glass asked for, the GPU gave a see-through
/// surface, and Windows blurs. Whether the window is in front is not a
/// part of it.
pub fn glass_shows(asked: bool, surface: bool, effects: Effects) -> bool {
    asked && surface && effects.blurs()
}

#[cfg(windows)]
pub use windows::{accent, effects};

/// Off Windows nothing switches the blur off, and none is asked for.
#[cfg(not(windows))]
pub fn effects() -> Effects {
    Effects::ALL_ON
}

#[cfg(not(windows))]
pub fn accent(_window: isize, _on: bool) {}

#[cfg(windows)]
mod windows {
    use super::Effects;

    #[repr(C)]
    struct SystemPowerStatus {
        line: u8,
        battery: u8,
        percent: u8,
        /// Bit 0: battery saver is on.
        system: u8,
        life: u32,
        full: u32,
    }

    #[repr(C)]
    struct AccentPolicy {
        state: u32,
        flags: u32,
        tint: u32,
        animation: u32,
    }

    #[repr(C)]
    struct Attribute {
        kind: u32,
        data: *mut AccentPolicy,
        size: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> i32;
        fn LoadLibraryW(name: *const u16) -> isize;
        fn GetProcAddress(module: isize, name: *const u8) -> *const core::ffi::c_void;
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegGetValueW(
            key: isize,
            sub_key: *const u16,
            value: *const u16,
            flags: u32,
            kind: *mut u32,
            data: *mut u32,
            size: *mut u32,
        ) -> i32;
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    /// What Windows says now: the transparency switch (on when the key is
    /// missing, as Windows has it) and battery saver (off when unknown).
    pub fn effects() -> Effects {
        const CURRENT_USER: isize = 0x8000_0001_u32 as i32 as isize;
        const DWORD: u32 = 0x10;
        let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        let value = wide("EnableTransparency");
        let mut data = 1_u32;
        let mut size = 4_u32;
        // SAFETY: both names end in a null, and `data` and `size` are
        // locals that outlive the call.
        let found = unsafe {
            RegGetValueW(
                CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                DWORD,
                std::ptr::null_mut(),
                &mut data,
                &mut size,
            )
        };
        let mut power = SystemPowerStatus {
            line: 0,
            battery: 0,
            percent: 0,
            system: 0,
            life: 0,
            full: 0,
        };
        // SAFETY: `power` is a local of the shape the call fills.
        let known = unsafe { GetSystemPowerStatus(&mut power) } != 0;
        Effects {
            transparency: found != 0 || data != 0,
            battery_saver: known && power.system & 1 != 0,
        }
    }

    /// Asks for Windows' acrylic blur behind the window by the accent
    /// policy, or takes it back. The call isn't in the import library, so
    /// it's looked up by name; if it isn't there, nothing is asked.
    pub fn accent(window: isize, on: bool) {
        const POLICY: u32 = 19;
        const ACRYLIC_BLUR_BEHIND: u32 = 4;
        const DISABLED: u32 = 0;
        let user = wide("user32.dll");
        // SAFETY: the name ends in a null, and the library is Windows' own.
        let module = unsafe { LoadLibraryW(user.as_ptr()) };
        if module == 0 {
            return;
        }
        // SAFETY: the name ends in a null.
        let found =
            unsafe { GetProcAddress(module, c"SetWindowCompositionAttribute".as_ptr().cast()) };
        if found.is_null() {
            return;
        }
        type Set = unsafe extern "system" fn(isize, *mut Attribute) -> i32;
        // SAFETY: this is the call's documented-by-use signature.
        let set: Set = unsafe { std::mem::transmute(found) };
        // The tint is clear: the fills carry the colour and its alpha.
        let mut policy = AccentPolicy {
            state: if on { ACRYLIC_BLUR_BEHIND } else { DISABLED },
            flags: 0,
            tint: 0,
            animation: 0,
        };
        let mut attribute = Attribute {
            kind: POLICY,
            data: &mut policy,
            size: std::mem::size_of::<AccentPolicy>(),
        };
        // SAFETY: `window` is the live window's handle iced gave, and
        // `attribute` points at `policy`, both alive for the call.
        let result = unsafe { set(window, &mut attribute) };
        tracing::info!(result, on, "window acrylic blur");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ON: Effects = Effects::ALL_ON;

    #[test]
    fn glass_shows_whenever_windows_blurs() {
        assert!(glass_shows(true, true, ON));
    }

    #[test]
    fn the_window_not_being_in_front_changes_nothing() {
        // The decision has no part for focus, so the same inputs give the
        // same answer focused or not: the glass doesn't flatten behind
        // another window.
        for _focused in [true, false] {
            assert!(glass_shows(true, true, ON));
        }
    }

    #[test]
    fn transparency_effects_off_makes_the_fills_opaque() {
        let off = Effects {
            transparency: false,
            ..ON
        };
        assert!(!off.blurs());
        assert!(!glass_shows(true, true, off));
    }

    #[test]
    fn battery_saver_makes_the_fills_opaque() {
        let saver = Effects {
            battery_saver: true,
            ..ON
        };
        assert!(!saver.blurs());
        assert!(!glass_shows(true, true, saver));
    }

    #[test]
    fn both_off_and_the_one_that_blurs_are_told_apart() {
        let both = Effects {
            transparency: false,
            battery_saver: true,
        };
        assert!(!glass_shows(true, true, both));
        assert!(ON.blurs());
    }

    #[test]
    fn no_glass_asked_or_no_see_through_surface_stays_opaque() {
        assert!(!glass_shows(false, true, ON));
        assert!(!glass_shows(true, false, ON));
    }

    #[cfg(windows)]
    #[test]
    fn windows_answers_about_its_effects() {
        // Whatever this machine says, asking doesn't fail or panic.
        let _ = effects();
    }
}
