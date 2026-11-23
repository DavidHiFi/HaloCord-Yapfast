//! MochaCord: HaloCord's look for Yapfast. The theme itself is the core's
//! (`yapfast_core::theme`, id `mochacord`); this file keeps what the person
//! can change about the look and applies it: the window's glass, the
//! scrollbars' gradient, how round the corners are, and the font. Settings'
//! MochaCord page draws the options (`settings/pages/mochacord.rs`) and
//! `docs/halocord.md` lists the few places Yapfast's own files call in here.
//!
//! Glass is real blur or nothing: the window's surface is see-through and
//! Windows blurs the desktop behind it (acrylic), and the docks' fills carry
//! alpha over that. The glass is the same whether the window is in front or
//! not. Anywhere the blur can't be had (the software renderer, the option
//! off, Windows' transparency effects off, battery saver on) the fills are
//! the opaque Mocha ones (`blur.rs`).

mod blur;

use std::{
    path::Path,
    sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering},
};

use iced::Color;
use yapfast_iced_kit::{fonts, renderer, style, widgets};

use crate::layout::{self, Layout};

/// The theme's id in the core's list.
pub const THEME: &str = "mochacord";
/// Where the options sit in `layout.json`.
pub const KEY: &str = "mochacord";
/// The font the look starts with; Yapfast's own is used when it's missing.
pub const FONT: &str = "FiraCode Nerd Font";

/// The ends of the scrollbar's gradient and the accent line: Mocha blue and
/// mauve.
const BLUE: Color = Color::from_rgb(
    0x89 as f32 / 255.0,
    0xb4 as f32 / 255.0,
    0xfa as f32 / 255.0,
);
const MAUVE: Color = Color::from_rgb(
    0xcb as f32 / 255.0,
    0xa6 as f32 / 255.0,
    0xf7 as f32 / 255.0,
);
/// The scrollbars' lane: Mocha's surface 0.
const LANE: Color = Color::from_rgb(
    0x31 as f32 / 255.0,
    0x32 as f32 / 255.0,
    0x44 as f32 / 255.0,
);

/// What `layout.json` keeps of the look, under `mochacord`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Kept {
    /// Whether the one-time switch to MochaCord happened.
    pub switched: bool,
    /// Whether the window is glass, where it can be.
    pub glass: bool,
    /// How much of the desktop shows through, 0 to 100.
    pub glass_strength: u32,
    /// Whether scrollbars and accents run blue to mauve.
    pub accent_gradient: bool,
    /// How round the corners are, in percent of how they're drawn.
    pub corners: u32,
    /// The font's family name; empty is Yapfast's own.
    pub font: String,
    /// The weight of regular text, 100 to 900.
    pub weight: u32,
}

impl Default for Kept {
    fn default() -> Self {
        Self {
            switched: false,
            glass: true,
            glass_strength: 75,
            accent_gradient: true,
            corners: 100,
            font: FONT.to_owned(),
            weight: 500,
        }
    }
}

/// A change on the MochaCord page.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Glass(bool),
    Strength(u32),
    Gradient(bool),
    Corners(u32),
    Font(String),
    Weight(u32),
    /// Every option back to how it starts.
    Reset,
}

/// The most a corner setting may be, in percent.
pub const MOST_CORNERS: u32 = 200;
/// The weights the page offers.
pub const WEIGHTS: [u32; 6] = [400, 450, 500, 600, 700, 800];
/// The font families the page offers; empty is Yapfast's own.
pub const FONTS: [&str; 5] = [
    FONT,
    "JetBrainsMono Nerd Font",
    "Cascadia Code",
    "Segoe UI Variable",
    "",
];

impl Kept {
    /// Tells the kit about the options that apply live.
    pub fn apply(&self) {
        GLASS_ON.store(self.glass, Ordering::Relaxed);
        STRENGTH.store(self.glass_strength.min(100), Ordering::Relaxed);
        style::set_corner_scale(self.corners);
        widgets::set_bar_look(self.accent_gradient.then_some(widgets::BarGradient {
            from: BLUE,
            to: MAUVE,
            track: LANE,
        }));
    }

    /// Takes a change from the page.
    pub fn change(&mut self, change: Change) {
        match change {
            Change::Glass(on) => self.glass = on,
            Change::Strength(value) => self.glass_strength = value.min(100),
            Change::Gradient(on) => self.accent_gradient = on,
            Change::Corners(value) => self.corners = value.min(MOST_CORNERS),
            Change::Font(family) => self.font = family,
            Change::Weight(value) => self.weight = value.clamp(100, 900),
            Change::Reset => {
                *self = Self {
                    switched: self.switched,
                    ..Self::default()
                };
            }
        }
        self.apply();
    }

    /// The font choice the kit loads at start; none keeps Yapfast's own.
    pub fn font_choice(&self) -> Option<fonts::Choice> {
        (!self.font.trim().is_empty()).then(|| fonts::Choice {
            family: self.font.clone(),
            weight: self.weight.clamp(100, 900) as u16,
        })
    }
}

/// The one-time switch: on the first start of this look, the person's theme
/// becomes MochaCord, kept in `layout.json` beside the flag. After that the
/// choice in Appearance is theirs. Returns whether it switched.
pub fn first_start(layout: &mut Layout, root: &Path) -> bool {
    if layout.mochacord.switched {
        return false;
    }
    layout.mochacord.switched = true;
    layout.theme = Some(THEME.to_owned());
    layout::store(root, "theme", THEME);
    layout::store(root, KEY, &layout.mochacord);
    true
}

/// Before the window opens: asks for the see-through surface and sets the
/// font the window draws with. The surface and the fonts are made once, so
/// glass and font changes show at the next start.
pub fn prepare(root: &Path) {
    let kept = Layout::read(root).mochacord;
    renderer::want_glass(kept.glass);
    fonts::choose(kept.font_choice());
}

static GLASS_ON: AtomicBool = AtomicBool::new(false);
static STRENGTH: AtomicU32 = AtomicU32::new(75);
/// What Windows last said about its effects: bit 0 transparency off, bit 1
/// battery saver on. All clear until asked.
static EFFECTS: AtomicU8 = AtomicU8::new(0);

/// Asks Windows whether it blurs now (transparency effects, battery saver),
/// at start and each time the window takes or loses the focus, which is
/// where someone coming back from Windows' settings lands.
pub fn refresh_effects() {
    let now = blur::effects();
    EFFECTS.store(
        u8::from(!now.transparency) | u8::from(now.battery_saver) << 1,
        Ordering::Relaxed,
    );
}

fn effects_now() -> blur::Effects {
    let bits = EFFECTS.load(Ordering::Relaxed);
    blur::Effects {
        transparency: bits & 1 == 0,
        battery_saver: bits & 2 != 0,
    }
}

/// Which dock a fill is for; each has its own most see-through value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dock {
    /// The server rail and the window's title bar.
    Rail,
    /// The channel list, laid over the rail's glass.
    Sidebar,
    /// The chat's ground.
    Chat,
}

impl Dock {
    /// The least opaque a fill gets, at full strength.
    fn most_clear(self) -> f32 {
        match self {
            Self::Rail => 0.40,
            Self::Sidebar => 0.25,
            Self::Chat => 0.50,
        }
    }
}

/// Whether glass is showing: asked for, got from the GPU, and Windows blurs.
/// Focus has no part in it.
pub fn live() -> bool {
    blur::glass_shows(
        GLASS_ON.load(Ordering::Relaxed),
        renderer::glass_active(),
        effects_now(),
    )
}

/// `color` for `dock`: itself while glass isn't live, else with the alpha
/// the strength gives, never under the dock's own floor.
pub fn glass(color: Color, dock: Dock) -> Color {
    if !live() {
        return color;
    }
    glassy(color, dock, STRENGTH.load(Ordering::Relaxed))
}

/// The alpha math of [`glass`], for a strength from 0 to 100.
fn glassy(color: Color, dock: Dock, strength: u32) -> Color {
    let share = strength.min(100) as f32 / 100.0;
    Color {
        a: color.a * (1.0 - (1.0 - dock.most_clear()) * share),
        ..color
    }
}

/// Puts the window's backdrop on: Windows' acrylic blur behind a
/// see-through window (the accent policy's, which keeps blurring while the
/// window is behind another; the system backdrop goes flat then), and the
/// frame extended over all of it. Off puts both back.
#[cfg(windows)]
pub fn backdrop(window: isize, on: bool) {
    #[repr(C)]
    struct Margins {
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
    }
    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            window: isize,
            attribute: u32,
            value: *const i32,
            size: u32,
        ) -> i32;
        fn DwmExtendFrameIntoClientArea(window: isize, margins: *const Margins) -> i32;
    }
    const DARK: u32 = 20;
    const BACKDROP: u32 = 38;
    // The system backdrop stays off either way: the accent policy blurs.
    const NONE: i32 = 1;
    let (kind, edge) = (NONE, if on { -1 } else { 0 });
    let dark = 1_i32;
    let margins = Margins {
        left: edge,
        right: edge,
        top: edge,
        bottom: edge,
    };
    // SAFETY: `window` is the live window's handle iced gave this call, and
    // each pointer is to a local that outlives the call.
    let results = unsafe {
        [
            DwmSetWindowAttribute(window, DARK, &dark, 4),
            DwmSetWindowAttribute(window, BACKDROP, &kind, 4),
            DwmExtendFrameIntoClientArea(window, &margins),
        ]
    };
    blur::accent(window, on);
    tracing::info!(?results, on, "window backdrop");
}

/// Puts the backdrop behind the window that just opened, when its surface
/// is see-through and glass is on.
pub fn frame(window: &dyn iced::window::Window) {
    use iced::window::raw_window_handle::RawWindowHandle;
    refresh_effects();
    if !(GLASS_ON.load(Ordering::Relaxed) && renderer::glass_active()) {
        return;
    }
    if let Ok(handle) = window.window_handle()
        && let RawWindowHandle::Win32(win32) = handle.as_raw()
    {
        backdrop(win32.hwnd.get(), true);
    }
}

/// There's no backdrop to ask for off Windows.
#[cfg(not(windows))]
pub fn backdrop(_window: isize, _on: bool) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_start_as_the_rice_has_them() {
        let kept = Kept::default();
        assert!(kept.glass && kept.accent_gradient && !kept.switched);
        assert_eq!(
            (kept.glass_strength, kept.corners, kept.weight),
            (75, 100, 500)
        );
        assert_eq!(kept.font, "FiraCode Nerd Font");
    }

    #[test]
    fn the_options_round_trip_through_layout_json() {
        let folder = tempfile::tempdir().unwrap();
        let mut kept = Kept::default();
        kept.change(Change::Glass(false));
        kept.change(Change::Strength(40));
        kept.change(Change::Corners(150));
        kept.change(Change::Font("Cascadia Code".into()));
        kept.change(Change::Weight(600));
        std::fs::write(
            folder.path().join(layout::LAYOUT),
            serde_json::json!({ KEY: &kept, "zoom": 1.2 }).to_string(),
        )
        .unwrap();
        let read = Layout::read(folder.path()).mochacord;
        assert_eq!(read, kept);
        assert_eq!(read.font_choice().unwrap().weight, 600);
        // A partial file takes the starting value for what's missing.
        std::fs::write(
            folder.path().join(layout::LAYOUT),
            r#"{"mochacord": {"glass": false}}"#,
        )
        .unwrap();
        let partial = Layout::read(folder.path()).mochacord;
        assert!(!partial.glass);
        assert_eq!(partial.glass_strength, 75);
    }

    #[test]
    fn changes_stay_in_range_and_reset_keeps_the_first_start_flag() {
        let mut kept = Kept::default();
        kept.change(Change::Strength(900));
        kept.change(Change::Corners(900));
        kept.change(Change::Weight(5));
        assert_eq!(
            (kept.glass_strength, kept.corners, kept.weight),
            (100, 200, 100)
        );
        kept.switched = true;
        kept.change(Change::Reset);
        assert!(kept.switched);
        assert_eq!(kept.glass_strength, 75);
        style::set_corner_scale(100);
    }

    #[test]
    fn the_first_start_switches_to_mochacord_once_and_leaves_the_choice_after() {
        let folder = tempfile::tempdir().unwrap();
        let mut layout = Layout::read(folder.path());
        layout.theme = Some("bonfire".into());
        assert!(first_start(&mut layout, folder.path()));
        assert_eq!(layout.theme.as_deref(), Some("mochacord"));
        assert!(layout.mochacord.switched);
        // The person picks another theme; the next start leaves it.
        layout.theme = Some("nord".into());
        assert!(!first_start(&mut layout, folder.path()));
        assert_eq!(layout.theme.as_deref(), Some("nord"));
    }

    #[test]
    fn glass_alpha_follows_the_strength_and_never_goes_under_the_floor() {
        let base = Color::from_rgb(0.1, 0.1, 0.2);
        assert_eq!(glassy(base, Dock::Rail, 0).a, 1.0);
        assert!((glassy(base, Dock::Rail, 100).a - 0.40).abs() < 1e-6);
        assert!((glassy(base, Dock::Chat, 100).a - 0.50).abs() < 1e-6);
        let default = glassy(base, Dock::Rail, 75).a;
        assert!((0.40..0.65).contains(&default), "{default}");
        // Past the top it holds at the floor.
        assert!((glassy(base, Dock::Sidebar, 500).a - 0.25).abs() < 1e-6);
    }

    #[test]
    fn without_a_see_through_surface_every_fill_stays_opaque() {
        // Tests run on no GPU, so glass can't be live whatever is asked.
        GLASS_ON.store(true, Ordering::Relaxed);
        let base = Color::from_rgb(0.1, 0.1, 0.2);
        assert!(!live());
        assert_eq!(glass(base, Dock::Rail), base);
        GLASS_ON.store(false, Ordering::Relaxed);
    }
}
