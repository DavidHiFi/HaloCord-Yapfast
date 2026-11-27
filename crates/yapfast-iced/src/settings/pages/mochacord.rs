//! MochaCord's options, in the dialog the Themes page opens from the
//! MochaCord tile's cog. Each is applied at once where it can be and kept in
//! `layout.json` under `mochacord`. The options and their rules are
//! [`crate::mochacord`].

use iced::{Alignment, Color, Element, widget::row};
use yapfast_iced_kit::{theme::Theme, theme_page as kit};

use super::{Pref, pref};
use crate::{
    app::Message,
    controls::{choices, slider},
    form,
    mochacord::{Change, FONTS, MOST_CORNERS, SPEEDS, WEIGHTS, colour},
    settings::{Drop, Event, Page},
};

/// What the weight row calls each of [`WEIGHTS`].
const WEIGHT_NAMES: [&str; 6] = ["400", "450", "500", "600", "700", "800"];

/// The lightest and heaviest weight the free slider reaches.
const MIN_WEIGHT: u32 = 100;
const MAX_WEIGHT: u32 = 900;

/// Mocha's accents, as codes the colour fields take.
const SWATCHES: [&str; 10] = [
    "#89B4FA", "#CBA6F7", "#B4BEFE", "#74C7EC", "#94E2D5", "#A6E3A1", "#FAB387", "#F5C2E7",
    "#F38BA8", "#F9E2AF",
];

/// Whether a typed code is there but isn't a colour.
fn wrong(text: &str) -> bool {
    !text.trim().is_empty() && colour(text).is_none()
}

/// A row of Mocha's accents to pick from; the one in use is ringed.
fn swatch_row<'a>(
    theme: &Theme,
    now: &str,
    pick: impl Fn(String) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let now = colour(now);
    let mut swatches = row![].spacing(4).align_y(Alignment::Center);
    for code in SWATCHES {
        let shade = colour(code).unwrap_or(Color::BLACK);
        swatches = swatches.push(kit::swatch(
            theme,
            shade,
            now == Some(shade),
            pick(code.to_owned()),
        ));
    }
    swatches.into()
}

/// What the font list says under each of [`FONTS`].
const FONT_NOTES: [&str; 5] = [
    "MochaCord's own: a coding font with icons built in.",
    "A coding font with icons built in.",
    "Windows' own coding font.",
    "Windows' own font.",
    "The font Yapfast comes with.",
];

/// The font list, one row each, so long names never crowd: the empty
/// family is Yapfast's own.
fn font_choices() -> Vec<form::Choice<'static>> {
    FONTS
        .iter()
        .zip(FONT_NOTES)
        .map(|(family, note)| form::Choice {
            title: if family.is_empty() {
                "Yapfast's own"
            } else {
                family
            }
            .into(),
            description: note.into(),
            colour: None,
            offered: true,
        })
        .collect()
}

impl<'a> Page<'a> {
    pub(in crate::settings) fn mochacord_options(self) -> Vec<Element<'a, Message>> {
        let theme = self.cx.look.theme;
        let kept = &self.cx.layout.mochacord;
        let change = |change| pref(Pref::MochaCord(change));
        let glass_live = yapfast_iced_kit::renderer::glass_active();
        let glass_note = if glass_live {
            "Windows blurs the desktop behind the window. The strength is how much of it shows through."
        } else if kept.glass {
            "Glass is on but not showing: it needs the graphics card to draw the window and Windows' transparency effects on. The window is solid until then. A change here shows after a restart."
        } else {
            "The window is solid. A change here shows after a restart."
        };
        let weight_now = WEIGHTS
            .iter()
            .position(|weight| *weight == kept.weight)
            .unwrap_or(usize::MAX);
        let font_now = FONTS.iter().position(|family| *family == kept.font);
        vec![
            form::group(vec![
                form::switch(
                    "Glass window",
                    kept.glass,
                    Some(change(Change::Glass(!kept.glass))),
                )
                .description(glass_note)
                .view(self.cx.look),
                slider("Glass strength", kept.glass_strength, 100, move |value| {
                    change(Change::Strength(value))
                })
                .step(5)
                .shown(format!("{}%", kept.glass_strength))
                .mark(false)
                .view(theme),
            ]),
            form::group(vec![
                form::switch(
                    "Blue to mauve accents",
                    kept.accent_gradient,
                    Some(change(Change::Gradient(!kept.accent_gradient))),
                )
                .description("The scrollbars run from blue to mauve, with a dark lane behind them.")
                .view(self.cx.look),
                slider(
                    "Corner roundness",
                    kept.corners,
                    MOST_CORNERS,
                    move |value| change(Change::Corners(value)),
                )
                .step(10)
                .shown(format!("{}%", kept.corners))
                .mark(false)
                .view(theme),
                form::note(
                    theme,
                    "How round cards, menus and buttons are. 100% is as drawn.",
                ),
            ]),
            form::group(vec![
                form::switch(
                    "Open and close animation",
                    kept.motion,
                    Some(change(Change::Motion(!kept.motion))),
                )
                .description(
                    "Menus, pop-ups and dialogs fade and slide a little as they open and close.",
                )
                .view(self.cx.look),
                form::heading(theme, "Animation speed"),
                form::dropdown(
                    SPEEDS.iter().map(|speed| (*speed).to_owned()).collect(),
                    Some(kept.motion_speed.min(SPEEDS.len() - 1)),
                    kept.motion && self.s.open == Some(Drop::AnimationSpeed),
                    crate::app::settings(Event::Drop(Drop::AnimationSpeed)),
                    move |index| change(Change::MotionSpeed(index)),
                )
                .enabled(kept.motion)
                .view(self.cx.look),
                form::note(
                    theme,
                    "The global speed is the one all of DavidHiFi's apps share. Nothing moves while Windows' animation effects are off.",
                ),
            ]),
            form::group(vec![
                form::heading(theme, "Font"),
                form::options(theme, font_choices(), font_now, true, move |index| {
                    change(Change::Font(FONTS[index].to_owned()))
                }),
                form::heading(theme, "Any installed font"),
                form::entry(theme, "mochacord-font", "Type a font's name", &kept.font)
                    .on_input(move |text| change(Change::Font(text)))
                    .into(),
                form::heading(theme, "Weight"),
                choices(theme, &WEIGHT_NAMES, weight_now, move |index| {
                    change(Change::Weight(WEIGHTS[index]))
                }),
                slider(
                    "Any weight",
                    kept.weight.clamp(MIN_WEIGHT, MAX_WEIGHT) - MIN_WEIGHT,
                    MAX_WEIGHT - MIN_WEIGHT,
                    move |value| change(Change::Weight(value + MIN_WEIGHT)),
                )
                .step(10)
                .shown(kept.weight.to_string())
                .mark(false)
                .view(theme),
                form::note(
                    theme,
                    "A font that isn't installed falls back to Yapfast's own. Font changes show after a restart.",
                ),
            ]),
            form::group(vec![
                form::heading(theme, "Accent"),
                swatch_row(theme, &kept.accent, move |text| {
                    change(Change::Accent(text))
                }),
                form::colour_row(
                    theme,
                    "Accent",
                    colour(&kept.accent).unwrap_or(theme.edge_from),
                    &kept.accent,
                    wrong(&kept.accent),
                    move |text| change(Change::Accent(text)),
                )
                .view(),
                form::heading(theme, "Accent 2, the end of the run"),
                swatch_row(theme, &kept.accent_two, move |text| {
                    change(Change::AccentTwo(text))
                }),
                form::colour_row(
                    theme,
                    "Accent 2",
                    colour(&kept.accent_two).unwrap_or(theme.edge_to),
                    &kept.accent_two,
                    wrong(&kept.accent_two),
                    move |text| change(Change::AccentTwo(text)),
                )
                .view(),
                form::heading(theme, "Edge at rest"),
                swatch_row(theme, &kept.edge, move |text| change(Change::Edge(text))),
                form::colour_row(
                    theme,
                    "Edge",
                    colour(&kept.edge).unwrap_or(theme.edge),
                    &kept.edge,
                    wrong(&kept.edge),
                    move |text| change(Change::Edge(text)),
                )
                .view(),
                form::note(
                    theme,
                    "Type a code like #89B4FA or pick a swatch. Empty is Mocha's own. The accents are the two ends of the hover edge and the scrollbars.",
                ),
            ]),
        ]
    }
}
