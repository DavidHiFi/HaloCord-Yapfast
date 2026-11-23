//! MochaCord's options, in the dialog the Themes page opens from the
//! MochaCord tile's cog. Each is applied at once where it can be and kept in
//! `layout.json` under `mochacord`. The options and their rules are
//! [`crate::mochacord`].

use iced::Element;

use super::{Pref, pref};
use crate::{
    app::Message,
    controls::{choices, slider},
    form,
    mochacord::{Change, FONTS, MOST_CORNERS, WEIGHTS},
    settings::Page,
};

/// What the weight row calls each of [`WEIGHTS`].
const WEIGHT_NAMES: [&str; 6] = ["400", "450", "500", "600", "700", "800"];

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
            .unwrap_or(2);
        let font_now = FONTS
            .iter()
            .position(|family| *family == kept.font)
            .unwrap_or(0);
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
                form::heading(theme, "Font"),
                form::options(theme, font_choices(), Some(font_now), true, move |index| {
                    change(Change::Font(FONTS[index].to_owned()))
                }),
                form::heading(theme, "Weight"),
                choices(theme, &WEIGHT_NAMES, weight_now, move |index| {
                    change(Change::Weight(WEIGHTS[index]))
                }),
                form::note(
                    theme,
                    "A font that isn't installed falls back to Yapfast's own. Font changes show after a restart.",
                ),
            ]),
        ]
    }
}
