//! The Plugins page: a switch for each of HaloCord's plugins, with that
//! plugin's settings under it while it's on. The list is [`crate::halocord::PLUGINS`].

use iced::Element;

use super::{Pref, pref};
use crate::{
    app::Message,
    controls::{choices, slider},
    form,
    halocord::{Change, Kind, PLUGINS, found},
    settings::Page,
};

impl<'a> Page<'a> {
    pub(in crate::settings) fn halocord_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let kept = &self.cx.layout.halocord;
        let mut page = vec![
            form::title(theme, "Plugins"),
            form::note(
                theme,
                "Each plugin has a switch, and its settings show under it while it is on. They stay when HaloCord brings Yapfast up to date.",
            ),
        ];
        // Settings' search box narrows the list to the plugins it finds.
        let query = self.s.query.to_lowercase();
        let all = query.is_empty() || "plugins".contains(&query);
        for (index, plugin) in PLUGINS.iter().enumerate() {
            if !all && !found(plugin, &query) {
                continue;
            }
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
                    let change = move |change| pref(Pref::HaloCord(change));
                    match &setting.kind {
                        Kind::Choice { names, .. } => {
                            group.push(form::heading(theme, setting.heading));
                            group.push(choices(
                                theme,
                                names,
                                kept.choice(plugin, setting),
                                move |choice| {
                                    change(Change::Choose {
                                        plugin: index,
                                        setting: place,
                                        choice,
                                    })
                                },
                            ));
                            group.push(form::note(theme, setting.note));
                        }
                        Kind::Switch { .. } => {
                            let set = kept.flag(plugin, setting);
                            group.push(
                                form::switch(
                                    setting.heading,
                                    set,
                                    Some(change(Change::Flag {
                                        plugin: index,
                                        setting: place,
                                        on: !set,
                                    })),
                                )
                                .description(setting.note)
                                .view(self.cx.look),
                            );
                        }
                        Kind::Slider {
                            most, step, unit, ..
                        } => {
                            let value = kept.value(plugin, setting);
                            group.push(
                                slider(setting.heading, value, *most, move |value| {
                                    change(Change::Slide {
                                        plugin: index,
                                        setting: place,
                                        value,
                                    })
                                })
                                .step(*step)
                                .shown(format!("{value}{unit}"))
                                .mark(false)
                                .view(theme),
                            );
                            group.push(form::note(theme, setting.note));
                        }
                        Kind::Text { placeholder, .. } => {
                            group.push(form::heading(theme, setting.heading));
                            group.push(
                                form::field(
                                    theme,
                                    setting.key,
                                    placeholder,
                                    kept.text(plugin, setting),
                                )
                                .on_input(move |text| {
                                    change(Change::Type {
                                        plugin: index,
                                        setting: place,
                                        text,
                                    })
                                })
                                .into(),
                            );
                            group.push(form::note(theme, setting.note));
                        }
                    }
                }
            }
            page.push(form::group(group));
        }
        form::page(page)
    }
}
