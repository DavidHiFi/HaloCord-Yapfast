//! HaloCord's plugins in its copy of Yapfast: what each is, its settings,
//! and what it tells the core. Settings' HaloCord page draws this list, and
//! `layout.json` keeps each plugin under `halocord`, beside Yapfast's own
//! settings, so it stays through updates. `docs/halocord.md` lists the few
//! places Yapfast's own files call in here.
//!
//! A plugin is one entry in [`PLUGINS`]: a switch, the settings shown under
//! it while it's on, an `apply` that tells the core, and the hooks it plugs
//! in while it's on. Nothing else needs to change to add one.

use std::collections::BTreeMap;

use iced::{
    Alignment, Element, Task,
    widget::{container, text},
};
use yapfast_core::{
    application::Action,
    chat::ChatMessage,
    halocord::{self as engine, Tag},
    voice,
};

use crate::{
    app::{App, Message},
    client::Client,
    ctx::Ctx,
    fonts::BOLD,
    layout::Layout,
    menus::{MenuAct, MenuRow},
    pictures::Icon,
    style::{faded, hex, rounded},
    widgets::Pic,
};

/// The plugins' ids, for the places that look one up.
pub const STEREO: &str = "stereo-loader";
pub const FAKE_MUTE: &str = "fake-mute";
pub const FAKE_DEAFEN: &str = "fake-deafen";
pub const USER_AREA: &str = "user-area";

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
    /// Plugged into the core while it's on.
    hooks: Option<&'static dyn engine::Hooks>,
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
const fn off(key: &'static str, heading: &'static str, note: &'static str) -> Setting {
    Setting {
        key,
        heading,
        note,
        kind: Kind::Switch {
            unless_changed: false,
        },
    }
}

/// The plugins, in the page's order.
pub static PLUGINS: &[Plugin] = &[
    Plugin {
        id: STEREO,
        name: "Stereo loader",
        about: "Sends your microphone in stereo, just as it comes in: no echo cancelling, noise suppression or leveling. Best with headphones.",
        on_at_first: true,
        settings: &[Setting {
            key: "bitrate",
            heading: "MICROPHONE BIT RATE",
            note: "How much detail your microphone keeps in calls. 510 kbit/s is the most Discord calls carry. With this plugin off, calls send at Yapfast's own 128 kbit/s.",
            kind: Kind::Choice {
                names: &[
                    "64 kbit/s",
                    "128 kbit/s",
                    "256 kbit/s",
                    "384 kbit/s",
                    "510 kbit/s",
                ],
                values: &engine::BITRATES,
                unless_changed: engine::DEFAULT_BITRATE,
            },
        }],
        apply: |kept, plugin| {
            let on = kept.on(plugin);
            engine::set_flat(on);
            engine::set_bitrate(on.then(|| kept.value(plugin, &plugin.settings[0])));
        },
        hooks: None,
    },
    Plugin {
        id: FAKE_MUTE,
        name: "Fake mute",
        about: "Others see you muted in calls while you can still talk. Your own microphone stays as you set it.",
        on_at_first: false,
        settings: &[],
        apply: tell_voice,
        hooks: None,
    },
    Plugin {
        id: FAKE_DEAFEN,
        name: "Fake deafen",
        about: "Others see you deafened in calls while you still hear everyone. They see you muted too, as a real deafen shows.",
        on_at_first: false,
        settings: &[],
        apply: tell_voice,
        hooks: None,
    },
    Plugin {
        id: "staff-tag",
        name: "Staff tag",
        about: "Tags the owner, the admins and the mods of a server by their names in chat.",
        on_at_first: false,
        settings: &[
            off("crown", "Crown", "Draw a crown in place of the words."),
            Setting {
                key: "role_color",
                heading: "Role colour",
                note: "Draw the tag in the member's role colour, where they have one.",
                kind: Kind::Switch {
                    unless_changed: true,
                },
            },
        ],
        apply: |kept, plugin| {
            engine::staff::set_crown(kept.flag(plugin, &plugin.settings[0]));
            engine::staff::set_role_color(kept.flag(plugin, &plugin.settings[1]));
        },
        hooks: Some(&engine::staff::Staff),
    },
    Plugin {
        id: USER_AREA,
        name: "User area",
        about: "How the card at the bottom of the channel list looks, and the buttons on it.",
        on_at_first: true,
        settings: &[
            off(
                "compact",
                "Compact mode",
                "A smaller, tighter card that stays inside the channel list.",
            ),
            off(
                "hide_arrows",
                "Hide the dropdown arrows",
                "Takes the little arrows off the mute and deafen buttons.",
            ),
            off(
                "fake_mute_button",
                "Fake mute button",
                "A button that switches Fake mute on and off.",
            ),
            off(
                "fake_deafen_button",
                "Fake deafen button",
                "A button that switches Fake deafen on and off.",
            ),
            off(
                "stats_button",
                "Stats button",
                "A button that opens the call's connection details.",
            ),
            off(
                "stereo_button",
                "Stereo button",
                "A button that switches the Stereo loader on and off.",
            ),
        ],
        apply: |_, _| {},
        hooks: None,
    },
    Plugin {
        id: "copy-user-mention",
        name: "Copy mention",
        about: "Adds Copy mention to someone's right-click menu. It copies the text that pings them when you send it.",
        on_at_first: false,
        settings: &[],
        apply: |_, _| {},
        hooks: Some(&engine::copy_user_mention::CopyUserMention),
    },
    Plugin {
        id: "copy-user-urls",
        name: "Copy profile link",
        about: "Adds Copy profile link to someone's right-click menu. It copies the web link to their profile.",
        on_at_first: false,
        settings: &[],
        apply: |_, _| {},
        hooks: Some(&engine::copy_user_url::CopyUserUrl),
    },
];

/// Tells the core what Fake mute and Fake deafen do; both plugins say it.
fn tell_voice(kept: &Kept, _: &Plugin) {
    engine::set_fake_voice(kept.is(FAKE_MUTE), kept.is(FAKE_DEAFEN));
}

/// Whether a search (lowercase, not empty) finds `plugin` by its name, what
/// it says it does, or the names of its settings.
pub fn found(plugin: &Plugin, query: &str) -> bool {
    let has = |words: &str| words.to_lowercase().contains(query);
    has(plugin.name)
        || has(plugin.about)
        || plugin.settings.iter().any(|setting| has(setting.heading))
}

/// Whether a search finds any plugin, so Settings' list keeps the page.
pub fn any_found(query: &str) -> bool {
    PLUGINS.iter().any(|plugin| found(plugin, query))
}

/// The place of the plugin with `id` in [`PLUGINS`].
pub fn place(id: &str) -> Option<usize> {
    PLUGINS.iter().position(|plugin| plugin.id == id)
}

/// How the user area looks and which buttons it has, from its plugin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dock {
    pub compact: bool,
    pub arrows_hidden: bool,
    pub fake_mute: bool,
    pub fake_deafen: bool,
    pub stats: bool,
    pub stereo: bool,
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

    /// What Fake mute or Fake deafen is doing, to say on the user area while
    /// in a call; deafen wins, as it shows both.
    pub fn faking(&self) -> Option<&'static str> {
        if self.is(FAKE_DEAFEN) {
            Some("Faking deafen")
        } else if self.is(FAKE_MUTE) {
            Some("Faking mute")
        } else {
            None
        }
    }

    /// The user area's options; as it started while its plugin is off.
    pub fn dock(&self) -> Dock {
        let Some(area) = PLUGINS
            .iter()
            .find(|plugin| plugin.id == USER_AREA)
            .filter(|plugin| self.on(plugin))
        else {
            return Dock::default();
        };
        let flag = |key: &str| {
            area.settings
                .iter()
                .find(|setting| setting.key == key)
                .is_some_and(|setting| self.flag(area, setting))
        };
        Dock {
            compact: flag("compact"),
            arrows_hidden: flag("hide_arrows"),
            fake_mute: flag("fake_mute_button"),
            fake_deafen: flag("fake_deafen_button"),
            stats: flag("stats_button"),
            stereo: flag("stereo_button"),
        }
    }

    /// Tells the core what every plugin does.
    pub fn apply(&self) {
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
                .is_some_and(|found| found.id == FAKE_MUTE || found.id == FAKE_DEAFEN)
    );
    layout.halocord.change(change);
    crate::layout::store(client.root(), "halocord", &layout.halocord);
    // Devices is the call's way of sending its voice state once more.
    let resend = voice && client.state().voice.active();
    if resend {
        client.dispatch(Action::Voice(voice::Command::Devices));
    }
    resend
}

/// 4. A plugin's tag after a message's author: its words in a chip, or a
/// crown, in the colour it sets.
pub fn tag<'a>(ctx: Ctx<'a>, tag: Tag) -> Element<'a, Message> {
    let Some(color) = tag.color else {
        return ctx.blurple_tag(tag.text, 1.0);
    };
    if tag.crown {
        return Pic::square(ctx.look.icon(Icon::Crown, 14, color), 14.0).into();
    }
    let ink = hex(color);
    container(text(tag.text).size(10).font(BOLD).color(ink))
        .padding([0, 4])
        .height(16)
        .align_y(Alignment::Center)
        .style(move |_| rounded(faded(ink, 0.18), 4))
        .into()
}

/// 5. HaloCord's plugins' rows for a message's menu, before Delete.
pub fn menu_rows(message: &ChatMessage) -> Vec<MenuRow> {
    keyed(engine::menu(message))
}

/// 5. HaloCord's plugins' rows for a person's menu, after its own.
pub fn person_rows(user: &str) -> Vec<MenuRow> {
    keyed(engine::person_menu(user))
}

/// Menu rows from the engine's keys and labels, the first set apart.
fn keyed(rows: Vec<(String, &'static str)>) -> Vec<MenuRow> {
    let mut rows: Vec<MenuRow> = rows
        .into_iter()
        .map(|(key, label)| MenuRow::plain(MenuAct::Plugin, label).key(key))
        .collect();
    if let Some(first) = rows.first_mut() {
        first.separated = true;
    }
    rows
}

/// 5. A plugin's menu row was chosen: in a person's menu (`place` is the
/// person) it's theirs, else it's for the message `item`.
pub fn chosen(app: &mut App, key: &str, item: &str, place: Option<&str>) -> Task<Message> {
    match place {
        Some(user) => done(app, engine::person_pick(key, user)),
        None => pick(app, key, item),
    }
}

/// 5. A plugin's menu row was chosen for the message `item`.
pub fn pick(app: &mut App, key: &str, item: &str) -> Task<Message> {
    let Some(message) = app.client.state().message(item).cloned() else {
        return Task::none();
    };
    done(app, engine::pick(key, &message))
}

/// What a chosen row gave: its actions, and its text to copy.
fn done(app: &mut App, picked: engine::Picked) -> Task<Message> {
    for action in picked.actions {
        app.client.dispatch(action);
    }
    let copied = picked.copy.map_or_else(Task::none, iced::clipboard::write);
    Task::batch([app.sync(), copied])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_names_each_value() {
        for plugin in PLUGINS {
            for setting in plugin.settings {
                if let Kind::Choice {
                    names,
                    values,
                    unless_changed,
                } = &setting.kind
                {
                    assert_eq!(names.len(), values.len(), "{}", setting.key);
                    assert!(values.contains(unless_changed));
                }
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
    fn a_search_finds_plugins_by_name_words_and_settings() {
        let names = |query: &str| -> Vec<&str> {
            PLUGINS
                .iter()
                .filter(|plugin| found(plugin, query))
                .map(|plugin| plugin.name)
                .collect()
        };
        assert_eq!(names("stereo"), ["Stereo loader", "User area"]);
        assert_eq!(names("fake mute"), ["Fake mute", "User area"]);
        assert_eq!(names("crown"), ["Staff tag"]);
        assert!(names("zzz").is_empty());
        assert!(any_found("staff") && !any_found("zzz"));
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
        assert_eq!(on(FAKE_MUTE).faking(), Some("Faking mute"));
        assert_eq!(on(FAKE_DEAFEN).faking(), Some("Faking deafen"));
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
        let mut wrong = kept.clone();
        let state = wrong.plugins.get_mut("kinds").unwrap();
        state.settings.insert("volume".into(), Value::Number(500));
        state.settings.insert("word".into(), Value::Flag(true));
        assert_eq!(wrong.value(plugin, volume), 50);
        assert_eq!(wrong.text(plugin, word), "hi");
    }

    struct Rows;

    impl engine::Hooks for Rows {
        fn id(&self) -> &'static str {
            "rows-test"
        }
        fn menu(&self, message: &ChatMessage) -> Vec<(&'static str, &'static str)> {
            // Only for this test's message: the window's tests share the core.
            if message.body == "halocord rows test" {
                vec![("raw", "Copy raw"), ("source", "View source")]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn plugin_rows_join_a_messages_menu() {
        let message = ChatMessage {
            body: "halocord rows test".into(),
            ..ChatMessage::default()
        };
        engine::plug(&Rows, true);
        let rows = menu_rows(&message);
        engine::plug(&Rows, false);
        let keys: Vec<_> = rows.iter().map(|row| row.key.as_str()).collect();
        assert_eq!(keys, ["rows-test/raw", "rows-test/source"]);
        assert!(rows.iter().all(|row| row.act == MenuAct::Plugin));
        assert!(rows[0].separated && !rows[1].separated);
        assert!(menu_rows(&message).is_empty());
    }
}
