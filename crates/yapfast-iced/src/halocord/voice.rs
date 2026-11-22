//! Voice plugins: the stereo loader, fake mute and deafen, the volume
//! booster, force mono, and the voice marks in the channel and member lists.

use iced::{Element, widget::text};
use yapfast_core::halocord as engine;

use super::{Kept, Kind, PLUGINS, Plugin, Setting, id, off, place};
use crate::{
    app::{App, Message},
    ctx::Ctx,
    fonts::BOLD,
    pictures::Icon,
};

pub(super) const STEREO: Plugin = Plugin {
    id: id::STEREO,
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
};

pub(super) const FAKE_MUTE: Plugin = Plugin {
    id: id::FAKE_MUTE,
    name: "Fake mute",
    about: "Others see you muted in calls while you can still talk. Your own microphone stays as you set it.",
    on_at_first: false,
    settings: &[],
    apply: tell_voice,
    hooks: None,
};

pub(super) const FAKE_DEAFEN: Plugin = Plugin {
    id: id::FAKE_DEAFEN,
    name: "Fake deafen",
    about: "Others see you deafened in calls while you still hear everyone. They see you muted too, as a real deafen shows.",
    on_at_first: false,
    settings: &[],
    apply: tell_voice,
    hooks: None,
};

pub(super) const VOLUME_BOOSTER: Plugin = Plugin {
    id: id::VOLUME_BOOSTER,
    name: "Volume booster",
    about: "Lets a person's volume and a stream's volume go past 200%, up to the most you pick.",
    on_at_first: true,
    settings: &[Setting {
        key: "cap",
        heading: "MOST VOLUME",
        note: "How far the sliders in someone's voice menu go. 200% is where Yapfast stops on its own.",
        kind: Kind::Choice {
            names: &["200%", "400%", "600%", "800%", "1000%"],
            values: &engine::VOLUME_CAPS,
            unless_changed: 1000,
        },
    }],
    apply: |kept, plugin| {
        let on = kept.on(plugin);
        engine::set_volume_cap(on.then(|| kept.value(plugin, &plugin.settings[0])));
    },
    hooks: None,
};

pub(super) const FORCE_MONO: Plugin = Plugin {
    id: id::FORCE_MONO,
    name: "Force mono",
    about: "Plays everyone's voice the same in both ears, for one-sided headphones or a single speaker.",
    on_at_first: false,
    settings: &[],
    apply: |kept, plugin| engine::set_mono(kept.on(plugin)),
    hooks: None,
};

pub(super) const CLICK_TO_UNMUTE: Plugin = Plugin {
    id: id::CLICK_TO_UNMUTE,
    name: "Click to unmute",
    about: "Shows a mark beside someone you've muted for yourself in a voice channel. Click it to unmute them.",
    on_at_first: true,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const USER_VOICE_SHOW: Plugin = Plugin {
    id: id::USER_VOICE_SHOW,
    name: "User voice show",
    about: "Marks a member in the member list while they are in a voice channel, and says which one.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const VOICE_COUNT: Plugin = Plugin {
    id: id::VOICE_COUNT,
    name: "Voice count",
    about: "Shows how many people are in each voice channel, at the end of its row in the channel list.",
    on_at_first: false,
    settings: &[off(
        "empty",
        "Show empty channels",
        "Write 0 beside a voice channel nobody is in.",
    )],
    apply: |_, _| {},
    hooks: None,
};

/// Tells the core what Fake mute and Fake deafen do; both plugins say it.
fn tell_voice(kept: &Kept, _: &Plugin) {
    engine::set_fake_voice(kept.is(id::FAKE_MUTE), kept.is(id::FAKE_DEAFEN));
}

impl Kept {
    /// What Fake mute or Fake deafen is doing, to say on the user area while
    /// in a call; deafen wins, as it shows both.
    pub fn faking(&self) -> Option<&'static str> {
        if self.is(id::FAKE_DEAFEN) {
            Some("Faking deafen")
        } else if self.is(id::FAKE_MUTE) {
            Some("Faking mute")
        } else {
            None
        }
    }
}

/// The voice channel `user` is in, by the rooms' lists of who is in them.
pub fn voice_room_of<'a>(
    rooms: &'a [std::sync::Arc<yapfast_core::chat::ChatRoom>],
    user: &str,
) -> Option<&'a str> {
    rooms
        .iter()
        .find(|room| room.voice && room.people.iter().any(|person| person.id == user))
        .map(|room| room.name.as_str())
}

/// The mark on a member's row while they're in a voice channel (User voice
/// show), saying which one when it's pointed at.
pub fn in_voice_mark<'a>(cx: Ctx<'a>, user: &str) -> Option<Element<'a, Message>> {
    if !cx.layout.halocord.is(id::USER_VOICE_SHOW) {
        return None;
    }
    let channel = voice_room_of(&cx.state.rooms, user)?;
    let theme = cx.look.theme;
    Some(
        iced::widget::tooltip(
            cx.look.glyph(Icon::Volume, 14, theme.muted),
            crate::controls::tip(theme, format!("In {channel}")),
            iced::widget::tooltip::Position::Top,
        )
        .gap(6)
        .into(),
    )
}

/// How many people are in a voice channel, for the end of its row, while
/// Voice count is on (an empty one only when the plugin's setting says so).
pub fn voice_count<'a>(cx: Ctx<'a>, people: usize) -> Option<Element<'a, Message>> {
    let kept = &cx.layout.halocord;
    let plugin = place(id::VOICE_COUNT).map(|at| &PLUGINS[at])?;
    if !kept.on(plugin) || (people == 0 && !kept.flag(plugin, &plugin.settings[0])) {
        return None;
    }
    let theme = cx.look.theme;
    Some(
        text(people.to_string())
            .size(12)
            .font(BOLD)
            .color(theme.muted)
            .into(),
    )
}

/// The mark beside someone in a voice channel who is muted for this account,
/// while Click to unmute is on; a click on it unmutes them.
pub fn unmute_mark<'a>(
    cx: crate::ctx::Ctx<'a>,
    person: &yapfast_core::chat::VoicePerson,
    live: Option<&yapfast_core::voice::Participant>,
) -> Option<Element<'a, Message>> {
    if !cx.layout.halocord.is(id::CLICK_TO_UNMUTE) {
        return None;
    }
    // Volumes are kept by the network id, which the call knows them by.
    let key = live.map_or_else(|| person.id.clone(), |live| live.person.clone());
    if !yapfast_core::volume::muted(&key) {
        return None;
    }
    let theme = cx.look.theme;
    let mark = cx.look.glyph(Icon::VolumeOff, 16, theme.danger);
    let area = iced::widget::mouse_area(mark)
        .on_press(Message::Call(crate::call::Event::Unmute(key)))
        .interaction(iced::mouse::Interaction::Pointer);
    Some(
        iced::widget::tooltip(
            area,
            crate::controls::tip(theme, "Muted for you. Click to unmute"),
            iced::widget::tooltip::Position::Top,
        )
        .gap(4)
        .into(),
    )
}

/// Unmutes `person` for this account, as their voice menu's Mute box does,
/// and keeps it (Click to unmute).
pub fn unmute(app: &mut App, person: &str) {
    yapfast_core::volume::mute(person, false);
    let people = &mut app.layout.voice.muted_people;
    people.remove(person);
    crate::layout::store(app.client.root(), "muted_people", &*people);
}
