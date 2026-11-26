//! About: the build, its updates and Yapfast's links.

use iced::{
    Element,
    widget::{column, row, text},
};

use super::{AccountEvent, Pref, account_message, pref};
use crate::settings::{Event, Page, build_tint};
use crate::{
    app::Message,
    controls::{Kind, action},
    fonts::BOLD,
    form::{self, switch},
    pictures::Icon,
};

/// About's links.
pub(super) const WEBSITE: &str = "https://yapfast.com";

pub(super) const GITHUB: &str = "https://github.com/yapfast/yapfast";

pub(super) const DISCORD: &str = "https://yapfast.com/discord-invite";

impl<'a> Page<'a> {
    pub(in crate::settings) fn about_page(self) -> Element<'a, Message> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        let build = self.cx.build;
        let prefs = &self.cx.layout.prefs;
        let mut channel = row![
            text(build.channel.as_str())
                .size(13)
                .font(BOLD)
                .color(build_tint(theme, build.tone))
        ];
        if !build.details.is_empty() {
            channel = channel.push(
                text(format!(" · {}", build.details))
                    .size(13)
                    .color(theme.muted)
                    .wrapping(text::Wrapping::None),
            );
        }
        let card = form::card(
            theme,
            form::beside(
                column![
                    text(build.headline.as_str())
                        .size(20)
                        .font(BOLD)
                        .color(theme.text)
                        .wrapping(text::Wrapping::None),
                    channel
                ]
                .spacing(4),
                action(
                    theme,
                    if settings.copied {
                        "Copied"
                    } else {
                        "Copy version"
                    },
                    Kind::Plain,
                    Some(Message::Settings(Event::CopyVersion)),
                ),
            ),
        );
        let mut parts = vec![form::title(theme, "About"), card];
        parts.extend(self.updates_part());
        let checks = prefs.checks_for_updates();
        parts.push(
            switch(
                "Check for updates automatically",
                checks,
                Some(pref(Pref::CheckUpdates(!checks))),
            )
            .description("Looks for a newer HaloCord every 6 hours, and when Yapfast starts.")
            .fresh(self.fresh("check-updates"))
            .view(self.cx.look),
        );
        let installs = prefs.installs_updates();
        parts.push(
            switch(
                "Install updates automatically",
                installs,
                Some(pref(Pref::AutoInstall(!installs))),
            )
            .description(
                "Downloads and checks a new version, then uses it the next time Yapfast opens.                  It never stops a call.",
            )
            .view(self.cx.look),
        );
        parts.push(self.channel_picker());
        parts.push(form::heading(theme, "LINKS"));
        let ink = crate::theme::rgb(theme.text);
        let link = |label, icon, page| -> Element<'a, Message> {
            action(
                theme,
                label,
                Kind::Plain,
                Some(account_message(AccountEvent::Open(page))),
            )
            .icon(self.cx.look.icon(icon, 18, ink))
            .into()
        };
        parts.push(form::buttons([
            link("Website", Icon::Logo, WEBSITE),
            link("GitHub", Icon::GitHub, GITHUB),
            link("Discord", Icon::Discord, DISCORD),
        ]));
        form::page(parts)
    }

    /// About's UPDATE CHANNEL: Stable or Nightly, which the next look
    /// follows.
    fn channel_picker(self) -> Element<'a, Message> {
        use crate::halocord::updates::Channel;
        let theme = self.cx.look.theme;
        let mut heading = row![form::heading(theme, "UPDATE CHANNEL")]
            .spacing(8)
            .align_y(iced::Alignment::Center);
        if self.fresh("update-nightlies") {
            heading = heading.push(form::new_tag(theme));
        }
        let choices = Channel::ALL
            .iter()
            .map(|channel| form::Choice {
                title: channel.label().into(),
                description: channel.description().into(),
                colour: None,
                offered: true,
            })
            .collect();
        let chosen = self.cx.layout.prefs.update_channel.index();
        column![
            heading,
            form::options(theme, choices, Some(chosen), true, |index| {
                pref(Pref::UpdateChannel(Channel::ALL[index]))
            })
        ]
        .spacing(8)
        .into()
    }
}
