//! What a plugin is, its settings, and what `layout.json` keeps of them
//! under `halocord`. The areas' files hold the plugins themselves.

use std::collections::BTreeMap;

use yapfast_core::{application::Action, halocord as engine, voice};

use super::{PLUGINS, id};
use crate::{client::Client, layout::Layout};

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
    pub(super) apply: fn(&Kept, &Plugin),
    /// Plugged into the core while it's on.
    pub(super) hooks: Option<&'static dyn engine::Hooks>,
}

/// A plugin's setting, shown under its switch.
pub struct Setting {
    pub key: &'static str,
    /// Above it; a switch's own label.
    pub heading: &'static str,
    pub note: &'static str,
    pub kind: Kind,
}

/// What a setting is, and its value unless changed.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the slider and text are for plugins still to come"
    )
)]
pub enum Kind {
    /// One choice of a few: the names shown and the values kept, in order.
    Choice {
        names: &'static [&'static str],
        values: &'static [u32],
        unless_changed: u32,
    },
    Switch {
        unless_changed: bool,
    },
    /// A number from 0 to `most`, in steps of `step`, shown with `unit`.
    Slider {
        most: u32,
        step: u32,
        unit: &'static str,
        unless_changed: u32,
    },
    /// A line of text.
    Text {
        placeholder: &'static str,
        unless_changed: &'static str,
    },
}

/// A switch setting that starts off.
pub(super) const fn off(key: &'static str, heading: &'static str, note: &'static str) -> Setting {
    Setting {
        key,
        heading,
        note,
        kind: Kind::Switch {
            unless_changed: false,
        },
    }
}

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
    pub settings: BTreeMap<String, Value>,
}

/// A setting's kept value, as plain JSON: a number, a switch or a text.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Value {
    Flag(bool),
    Number(u32),
    Text(String),
}

/// A change on HaloCord's page, by places in [`PLUGINS`] and the plugin's
/// settings.
#[derive(Clone, Debug)]
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
    Flag {
        plugin: usize,
        setting: usize,
        on: bool,
    },
    Slide {
        plugin: usize,
        setting: usize,
        value: u32,
    },
    Type {
        plugin: usize,
        setting: usize,
        text: String,
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

    fn kept(&self, plugin: &Plugin, setting: &Setting) -> Option<&Value> {
        self.plugins
            .get(plugin.id)
            .and_then(|state| state.settings.get(setting.key))
    }

    /// A choice's or a slider's number; one no longer offered counts as
    /// unchanged.
    pub fn value(&self, plugin: &Plugin, setting: &Setting) -> u32 {
        let kept = match self.kept(plugin, setting) {
            Some(Value::Number(number)) => Some(*number),
            _ => None,
        };
        match &setting.kind {
            Kind::Choice {
                values,
                unless_changed,
                ..
            } => kept
                .filter(|value| values.contains(value))
                .unwrap_or(*unless_changed),
            Kind::Slider {
                most,
                unless_changed,
                ..
            } => kept
                .filter(|value| value <= most)
                .unwrap_or(*unless_changed),
            Kind::Switch { .. } | Kind::Text { .. } => 0,
        }
    }

    /// The place of a choice's value among its names.
    pub fn choice(&self, plugin: &Plugin, setting: &Setting) -> usize {
        let Kind::Choice { values, .. } = &setting.kind else {
            return 0;
        };
        let value = self.value(plugin, setting);
        values
            .iter()
            .position(|offered| *offered == value)
            .unwrap_or(0)
    }

    /// Whether a switch setting is on.
    pub fn flag(&self, plugin: &Plugin, setting: &Setting) -> bool {
        match (self.kept(plugin, setting), &setting.kind) {
            (Some(Value::Flag(on)), Kind::Switch { .. }) => *on,
            (_, Kind::Switch { unless_changed }) => *unless_changed,
            _ => false,
        }
    }

    /// A text setting's text.
    pub fn text<'a>(&'a self, plugin: &Plugin, setting: &'a Setting) -> &'a str {
        match (self.kept(plugin, setting), &setting.kind) {
            (Some(Value::Text(text)), Kind::Text { .. }) => text,
            (_, Kind::Text { unless_changed, .. }) => unless_changed,
            _ => "",
        }
    }

    /// Whether the plugin with `id` is on.
    pub fn is(&self, id: &str) -> bool {
        PLUGINS
            .iter()
            .find(|plugin| plugin.id == id)
            .is_some_and(|plugin| self.on(plugin))
    }

    /// Tells the core what every plugin does, as the window opens. Fake mute
    /// and Fake deafen start off at each launch, as Fake deafen did in
    /// HaloCord: one left on would otherwise carry into the next start, and
    /// the account would join later calls showing muted or deafened without
    /// knowing.
    pub fn apply(&mut self) {
        for id in [id::FAKE_MUTE, id::FAKE_DEAFEN] {
            if let Some(state) = self.plugins.get_mut(id) {
                state.on = None;
            }
        }
        for plugin in PLUGINS {
            self.tell(plugin);
        }
    }

    fn tell(&self, plugin: &Plugin) {
        (plugin.apply)(self, plugin);
        if let Some(hooks) = plugin.hooks {
            engine::plug(hooks, self.on(plugin));
        }
    }

    /// Takes a change from the page and tells the core.
    pub fn change(&mut self, change: Change) {
        if let Some(plugin) = self.change_in(PLUGINS, change) {
            self.tell(plugin);
        }
    }

    /// Keeps a change to one of `plugins`; the plugin it changed.
    fn change_in<'p>(&mut self, plugins: &'p [Plugin], change: Change) -> Option<&'p Plugin> {
        let (Change::Switch { plugin, .. }
        | Change::Choose { plugin, .. }
        | Change::Flag { plugin, .. }
        | Change::Slide { plugin, .. }
        | Change::Type { plugin, .. }) = change;
        let found = plugins.get(plugin)?;
        let state = self.plugins.entry(found.id.to_owned()).or_default();
        let (setting, value) = match change {
            Change::Switch { on, .. } => {
                state.on = Some(on);
                return Some(found);
            }
            Change::Choose {
                setting, choice, ..
            } => {
                let setting = found.settings.get(setting)?;
                let Kind::Choice { values, .. } = &setting.kind else {
                    return None;
                };
                (setting, Value::Number(*values.get(choice)?))
            }
            Change::Flag { setting, on, .. } => (found.settings.get(setting)?, Value::Flag(on)),
            Change::Slide { setting, value, .. } => {
                (found.settings.get(setting)?, Value::Number(value))
            }
            Change::Type { setting, text, .. } => (found.settings.get(setting)?, Value::Text(text)),
        };
        state.settings.insert(setting.key.to_owned(), value);
        Some(found)
    }
}

/// A change from the Plugins page or the user area: kept, told to the core,
/// and, when it changes what others see of a call, the voice state sent
/// again. Whether the core was handed an action.
pub fn changed(layout: &mut Layout, client: &mut Client, change: Change) -> bool {
    let voice = matches!(
        &change,
        Change::Switch { plugin, .. }
            if PLUGINS
                .get(*plugin)
                .is_some_and(|found| found.id == id::FAKE_MUTE || found.id == id::FAKE_DEAFEN)
    );
    let booster = matches!(
        &change,
        Change::Switch { plugin, .. } | Change::Choose { plugin, .. }
            if PLUGINS
                .get(*plugin)
                .is_some_and(|found| found.id == id::VOLUME_BOOSTER)
    );
    layout.halocord.change(change);
    crate::layout::store(client.root(), "halocord", &layout.halocord);
    if booster {
        // People's saved volumes play within the new most.
        layout.voice.apply();
    }
    // Devices is the call's way of sending its voice state once more.
    let resend = voice && client.state().voice.active();
    if resend {
        client.dispatch(Action::Voice(voice::Command::Devices));
    }
    resend
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halocord::place;

    #[test]
    fn a_value_no_longer_offered_counts_as_unchanged() {
        let plugin = &PLUGINS[0];
        let setting = &plugin.settings[0];
        let kept: Kept =
            serde_json::from_str(r#"{"plugins":{"stereo-loader":{"settings":{"bitrate":300}}}}"#)
                .unwrap();
        assert_eq!(kept.value(plugin, setting), engine::DEFAULT_BITRATE);
    }

    #[test]
    fn the_stereo_loaders_kept_bit_rate_still_reads() {
        let json = r#"{"plugins":{"stereo-loader":{"on":true,"settings":{"bitrate":256}}}}"#;
        let kept: Kept = serde_json::from_str(json).unwrap();
        assert_eq!(kept.value(&PLUGINS[0], &PLUGINS[0].settings[0]), 256);
        assert_eq!(serde_json::to_string(&kept).unwrap(), json);
    }

    #[test]
    fn fake_mute_and_deafen_say_so_on_the_user_area() {
        let on = |id: &str| {
            let mut kept = Kept::default();
            kept.change_in(
                PLUGINS,
                Change::Switch {
                    plugin: place(id).unwrap(),
                    on: true,
                },
            );
            kept
        };
        assert_eq!(Kept::default().faking(), None);
        assert_eq!(on(id::FAKE_MUTE).faking(), Some("Faking mute"));
        assert_eq!(on(id::FAKE_DEAFEN).faking(), Some("Faking deafen"));
    }

    fn noop(_: &Kept, _: &Plugin) {}

    static KINDS: &[Plugin] = &[Plugin {
        id: "kinds",
        name: "Kinds",
        about: "",
        on_at_first: false,
        settings: &[
            Setting {
                key: "loud",
                heading: "Loud",
                note: "",
                kind: Kind::Switch {
                    unless_changed: true,
                },
            },
            Setting {
                key: "volume",
                heading: "VOLUME",
                note: "",
                kind: Kind::Slider {
                    most: 100,
                    step: 5,
                    unit: "%",
                    unless_changed: 50,
                },
            },
            Setting {
                key: "word",
                heading: "WORD",
                note: "",
                kind: Kind::Text {
                    placeholder: "",
                    unless_changed: "hi",
                },
            },
        ],
        apply: noop,
        hooks: None,
    }];

    #[test]
    fn switch_slider_and_text_settings_keep_their_values() {
        let plugin = &KINDS[0];
        let [loud, volume, word] = plugin.settings else {
            unreachable!()
        };
        let mut kept = Kept::default();
        assert!(kept.flag(plugin, loud));
        assert_eq!(kept.value(plugin, volume), 50);
        assert_eq!(kept.text(plugin, word), "hi");
        for change in [
            Change::Flag {
                plugin: 0,
                setting: 0,
                on: false,
            },
            Change::Slide {
                plugin: 0,
                setting: 1,
                value: 80,
            },
            Change::Type {
                plugin: 0,
                setting: 2,
                text: "hello".into(),
            },
        ] {
            assert!(kept.change_in(KINDS, change).is_some());
        }
        let kept: Kept = serde_json::from_str(&serde_json::to_string(&kept).unwrap()).unwrap();
        assert!(!kept.flag(plugin, loud));
        assert_eq!(kept.value(plugin, volume), 80);
        assert_eq!(kept.text(plugin, word), "hello");
        // A slider past its end, or a value of the wrong kind, is as it starts.
        let mut wrong = kept;
        let state = wrong.plugins.get_mut("kinds").unwrap();
        state.settings.insert("volume".into(), Value::Number(500));
        state.settings.insert("word".into(), Value::Flag(true));
        assert_eq!(wrong.value(plugin, volume), 50);
        assert_eq!(wrong.text(plugin, word), "hi");
    }
}
