//! Updates: HaloCord's own releases. What the last look found and Check now,
//! the two switches, and the channel the looks follow.

use iced::{
    Element,
    widget::{column, row},
};

use super::{Pref, pref};
use crate::settings::Page;
use crate::{
    app::Message,
    form::{self, switch},
    halocord::updates::Channel,
};

impl<'a> Page<'a> {
    pub(in crate::settings) fn updates_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let prefs = &self.cx.layout.prefs;
        let mut parts = vec![form::title(theme, "Updates")];
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
                "Downloads and checks a new version, then uses it the next time Yapfast opens. \
                 It never stops a call.",
            )
            .view(self.cx.look),
        );
        parts.push(self.channel_picker());
        form::page(parts)
    }

    /// UPDATE CHANNEL: Stable or Nightly, which the next look follows.
    fn channel_picker(self) -> Element<'a, Message> {
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
