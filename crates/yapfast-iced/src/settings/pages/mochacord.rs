//! MochaCord's page: the look's options, each applied at once where it can
//! be and kept in `layout.json` under `mochacord`. The options and their
//! rules are [`crate::mochacord`].

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

/// What the font choice row calls each family; the empty one is Yapfast's.
fn font_names() -> Vec<&'static str> {
    FONTS
        .iter()
        .map(|family| {
            if family.is_empty() {
                "Yapfast's own"
            } else {
                family
            }
        })
        .collect()
}

impl<'a> Page<'a> {
    pub(in crate::settings) fn mochacord_page(self) -> Element<'a, Message> {
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
        let names = font_names();
        let page = vec![
            form::title(theme, "MochaCord"),
            form::note(
                theme,
                "How this copy of Yapfast looks. Pick the MochaCord theme in Appearance to see it whole; these options work with any theme.",
            ),
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
                choices(theme, &names, font_now, move |index| {
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
        ];
        form::page(page)
    }
}
