//! How quick popups, menus and dialogs open and close: the person's choice
//! in the MochaCord dialog, over the speed every one of DavidHiFi's apps
//! shares (`HKCU\Software\DavidHiFi\Motion`: `Enabled` 0 turns it off,
//! `Speed` is a level's name or a percent of normal), and still while
//! Windows' animation effects are off. The kit does the moving
//! (`widgets::motion`); this only says how quick.

/// The speeds the dialog offers: the first follows the shared one.
pub const SPEEDS: [&str; 6] = [
    "Follow the global speed",
    "Very quick",
    "Quick",
    "Normal",
    "Relaxed",
    "Slow",
];
/// What each speed after the first is, in percent of the normal times.
const PERCENT: [u32; 5] = [50, 75, 100, 150, 200];

/// What Windows and the shared setting say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct System {
    /// Windows' animation effects are off, or the shared setting is.
    pub still: bool,
    /// The shared speed, in percent of normal.
    pub percent: u32,
}

impl System {
    /// Nothing set: on, at the normal speed.
    pub const NORMAL: Self = Self {
        still: false,
        percent: 100,
    };
}

/// The speed in percent of normal, 0 for still: off when `on` is off or
/// the system says still, else the shared speed for the first of
/// [`SPEEDS`] and the level chosen for the rest.
pub fn percent(on: bool, speed: usize, system: System) -> u32 {
    if !on || system.still {
        return 0;
    }
    match speed {
        0 => system.percent,
        chosen => PERCENT.get(chosen - 1).copied().unwrap_or(100),
    }
}

/// A level's name as the shared setting may hold it ("relaxed",
/// "Very quick", "very-quick"), in percent of normal.
pub fn level(name: &str) -> Option<u32> {
    let squash = |text: &str| {
        text.chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase()
    };
    let name = squash(name);
    SPEEDS[1..]
        .iter()
        .position(|each| squash(each) == name)
        .map(|at| PERCENT[at])
}

#[cfg(windows)]
pub use windows::system;

/// Off Windows nothing is shared: on, at the normal speed.
#[cfg(not(windows))]
pub fn system() -> System {
    System::NORMAL
}

#[cfg(windows)]
mod windows {
    use super::{System, level};

    // The same shape `blur.rs` declares, so the two never clash.
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

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SystemParametersInfoW(action: u32, param: u32, data: *mut i32, ini: u32) -> i32;
    }

    const CURRENT_USER: isize = 0x8000_0001_u32 as i32 as isize;
    const DWORD: u32 = 0x10;
    const TEXT: u32 = 0x02;
    /// Windows' "Animation effects" switch.
    const CLIENT_AREA_ANIMATION: u32 = 0x1042;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    /// A number under `key`, if it's there as one.
    fn number(key: &[u16], value: &str) -> Option<u32> {
        let value = wide(value);
        let (mut data, mut size) = (0_u32, 4_u32);
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
        (found == 0).then_some(data)
    }

    /// Words under `key`, if they're there as words.
    fn words(key: &[u16], value: &str) -> Option<String> {
        let value = wide(value);
        let mut data = [0_u32; 32];
        let mut size = (data.len() * 4) as u32;
        // SAFETY: both names end in a null; `data` is `size` bytes and,
        // like `size`, a local that outlives the call.
        let found = unsafe {
            RegGetValueW(
                CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                TEXT,
                std::ptr::null_mut(),
                data.as_mut_ptr(),
                &mut size,
            )
        };
        if found != 0 {
            return None;
        }
        let units: Vec<u16> = data
            .iter()
            .flat_map(|pair| [*pair as u16, (*pair >> 16) as u16])
            .take_while(|unit| *unit != 0)
            .collect();
        Some(String::from_utf16_lossy(&units))
    }

    /// What Windows and the shared setting say now: a missing key is on
    /// at the normal speed.
    pub fn system() -> System {
        let key = wide(r"Software\DavidHiFi\Motion");
        let off = number(&key, "Enabled") == Some(0);
        let percent = number(&key, "Speed")
            .or_else(|| words(&key, "Speed").and_then(|name| level(&name)))
            .map_or(100, |percent| percent.clamp(25, 400));
        let mut animates = 1_i32;
        // SAFETY: the call fills the one BOOL `animates` points to.
        let asked = unsafe { SystemParametersInfoW(CLIENT_AREA_ANIMATION, 0, &mut animates, 0) };
        System {
            still: off || (asked != 0 && animates == 0),
            percent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_speed_follows_the_shared_one_unless_a_level_is_chosen() {
        let relaxed = System {
            still: false,
            percent: 150,
        };
        assert_eq!(percent(true, 0, System::NORMAL), 100);
        assert_eq!(percent(true, 0, relaxed), 150);
        assert_eq!(percent(true, 1, relaxed), 50);
        assert_eq!(percent(true, 5, relaxed), 200);
        // Off, or Windows' animation effects off: still.
        assert_eq!(percent(false, 3, relaxed), 0);
        let still = System {
            still: true,
            ..relaxed
        };
        assert_eq!(percent(true, 3, still), 0);
    }

    #[test]
    fn a_level_is_read_by_its_name_however_it_is_written() {
        assert_eq!(level("relaxed"), Some(150));
        assert_eq!(level("Very quick"), Some(50));
        assert_eq!(level("very-quick"), Some(50));
        assert_eq!(level("NORMAL"), Some(100));
        assert_eq!(level("Follow the global speed"), None);
        assert_eq!(level("fast"), None);
    }
}
