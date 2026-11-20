//! HaloCord's plugins in its copy of Yapfast: what each is, its settings,
//! and what it tells the core. Settings' HaloCord page draws this list, and
//! `layout.json` keeps each plugin under `halocord`, beside Yapfast's own
//! settings, so it stays through updates. `docs/halocord.md` lists the few
//! places Yapfast's own files call in here.
//!
//! A plugin is one entry in [`PLUGINS`]: a switch, the settings shown under
//! it while it's on, and an `apply` that tells the core. Nothing else needs
//! to change to add one.

use std::collections::BTreeMap;

use yapfast_core::halocord as engine;

/// One plugin: its switch on HaloCord's page and what it does.
pub struct Plugin {
    /// Its key in `layout.json`; never changes once shipped.
    pub id: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    /// Whether it's on before it's ever switched.
    pub on_at_first: bool,
    /// Shown under its switch while it's on.
    pub settings: &'static [Setting],
    /// Tells the core, at start and on every change.
    apply: fn(&Kept, &Plugin),
}

/// A plugin's setting: one choice of a few, kept as its value.
pub struct Setting {
    pub key: &'static str,
    pub heading: &'static str,
    pub note: &'static str,
    /// The names shown and the values kept, in the order shown.
    pub names: &'static [&'static str],
    pub values: &'static [u32],
    pub unless_changed: u32,
}

/// The plugins, in the page's order.
pub static PLUGINS: &[Plugin] = &[Plugin {
    id: "stereo-loader",
    name: "Stereo loader",
    about: "Sends your microphone in stereo, just as it comes in, like TestCord's stereo loader. No echo cancelling, noise suppression, gain control or leveling, so the Voice page's sound cleanup is skipped while this is on: only your microphone volume and the voice gate or push to talk apply. Best with headphones. A call going on switches straight away.",
    on_at_first: true,
    settings: &[Setting {
        key: "bitrate",
        heading: "MICROPHONE BIT RATE",
        note: "How much detail your microphone keeps in calls. 510 kbit/s is the most Discord calls carry. With this plugin off, calls send at Yapfast's own 128 kbit/s.",
        names: &[
            "64 kbit/s",
            "128 kbit/s",
            "256 kbit/s",
            "384 kbit/s",
            "510 kbit/s",
        ],
        values: &engine::BITRATES,
        unless_changed: engine::DEFAULT_BITRATE,
    }],
    apply: |kept, plugin| {
        let on = kept.on(plugin);
        engine::set_flat(on);
        engine::set_bitrate(on.then(|| kept.value(plugin, &plugin.settings[0])));
    },
}];

/// What `layout.json` keeps of HaloCord: each plugin's state by its id.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Kept {
    pub plugins: BTreeMap<String, PluginState>,
}

/// A plugin switched, or a setting changed; unset is as it starts.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct PluginState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub settings: BTreeMap<String, u32>,
}

/// A change on HaloCord's page, by places in [`PLUGINS`].
#[derive(Clone, Copy, Debug)]
pub enum Change {
    Switch {
        plugin: usize,
        on: bool,
    },
    Choose {
        plugin: usize,
        setting: usize,
        choice: usize,
    },
}

impl Kept {
    /// Whether `plugin` is on.
    pub fn on(&self, plugin: &Plugin) -> bool {
        self.plugins
            .get(plugin.id)
            .and_then(|state| state.on)
            .unwrap_or(plugin.on_at_first)
    }

    /// `setting`'s value for `plugin`; one no longer offered counts as
    /// unchanged.
    pub fn value(&self, plugin: &Plugin, setting: &Setting) -> u32 {
        self.plugins
            .get(plugin.id)
            .and_then(|state| state.settings.get(setting.key))
            .copied()
            .filter(|value| setting.values.contains(value))
            .unwrap_or(setting.unless_changed)
    }

    /// The place of `setting`'s value among its choices.
    pub fn choice(&self, plugin: &Plugin, setting: &Setting) -> usize {
        let value = self.value(plugin, setting);
        setting
            .values
            .iter()
            .position(|offered| *offered == value)
            .unwrap_or(0)
    }

    /// Tells the core what every plugin does.
    pub fn apply(&self) {
        for plugin in PLUGINS {
            (plugin.apply)(self, plugin);
        }
    }

    /// Takes a change from the page and tells the core.
    pub fn change(&mut self, change: Change) {
        let (Change::Switch { plugin, .. } | Change::Choose { plugin, .. }) = change;
        let Some(found) = PLUGINS.get(plugin) else {
            return;
        };
        let state = self.plugins.entry(found.id.to_owned()).or_default();
        match change {
            Change::Switch { on, .. } => state.on = Some(on),
            Change::Choose {
                setting, choice, ..
            } => {
                let Some(setting) = found.settings.get(setting) else {
                    return;
                };
                let Some(value) = setting.values.get(choice) else {
                    return;
                };
                state.settings.insert(setting.key.to_owned(), *value);
            }
        }
        (found.apply)(self, found);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_names_each_value() {
        for plugin in PLUGINS {
            for setting in plugin.settings {
                assert_eq!(setting.names.len(), setting.values.len(), "{}", setting.key);
                assert!(setting.values.contains(&setting.unless_changed));
            }
        }
    }

    #[test]
    fn plugin_ids_differ() {
        let mut ids: Vec<_> = PLUGINS.iter().map(|plugin| plugin.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PLUGINS.len());
    }

    #[test]
    fn a_value_no_longer_offered_counts_as_unchanged() {
        let plugin = &PLUGINS[0];
        let setting = &plugin.settings[0];
        let mut kept = Kept::default();
        kept.plugins
            .entry(plugin.id.into())
            .or_default()
            .settings
            .insert(setting.key.into(), 300);
        assert_eq!(kept.value(plugin, setting), setting.unless_changed);
    }
}
