//! HaloCord's page: a switch for each of its plugins, with that plugin's
//! settings under it while it's on. The list is [`crate::halocord::PLUGINS`].

use iced::Element;

use super::{Pref, pref};
use crate::{
    app::Message,
    controls::choices,
    form,
    halocord::{Change, PLUGINS},
    settings::Page,
};

impl<'a> Page<'a> {
    pub(in crate::settings) fn halocord_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let kept = &self.cx.layout.halocord;
        let mut page = vec![
            form::title(theme, "HaloCord"),
            form::note(
                theme,
                "HaloCord's plugins, built into this copy of Yapfast. They stay when HaloCord brings Yapfast up to date.",
            ),
        ];
        for (index, plugin) in PLUGINS.iter().enumerate() {
            let on = kept.on(plugin);
            let mut group = vec![
                form::switch(
                    plugin.name,
                    on,
                    Some(pref(Pref::HaloCord(Change::Switch {
                        plugin: index,
                        on: !on,
                    }))),
                )
                .description(plugin.about)
                .view(self.cx.look),
            ];
            if on {
                for (place, setting) in plugin.settings.iter().enumerate() {
                    group.push(form::heading(theme, setting.heading));
                    group.push(choices(
                        theme,
                        setting.names,
                        kept.choice(plugin, setting),
                        move |choice| {
                            pref(Pref::HaloCord(Change::Choose {
                                plugin: index,
                                setting: place,
                                choice,
                            }))
                        },
                    ));
                    group.push(form::note(theme, setting.note));
                }
            }
            page.push(form::group(group));
        }
        form::page(page)
    }
}
