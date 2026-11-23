//! Presence plugins: an activity you write yourself, and a custom status
//! that turns through a list of lines. They tell the Discord service what to
//! send; `yapfast-discord`'s `halocord/presence.rs` builds the presence.

use yapfast_core::yapfast_discord::halocord::presence::{
    Animated, LEAST_EVERY, Rpc, Started, set_animated, set_custom_rpc,
};

use super::{Kept, Kind, Plugin, Setting, id};

/// The text of the plugin's setting at `place`.
fn text<'a>(kept: &'a Kept, plugin: &'a Plugin, place: usize) -> &'a str {
    kept.text(plugin, &plugin.settings[place])
}

/// The number of the plugin's choice or slider at `place`.
fn value(kept: &Kept, plugin: &Plugin, place: usize) -> u32 {
    kept.value(plugin, &plugin.settings[place])
}

const fn field(
    key: &'static str,
    heading: &'static str,
    note: &'static str,
    placeholder: &'static str,
) -> Setting {
    Setting {
        key,
        heading,
        note,
        kind: Kind::Text {
            placeholder,
            unless_changed: "",
        },
    }
}

pub(super) const CUSTOM_RPC: Plugin = Plugin {
    id: id::CUSTOM_RPC,
    name: "Custom RPC",
    about: "Shows an activity you write yourself on your profile, beside any game or music Yapfast finds.",
    on_at_first: false,
    settings: &[
        field(
            "application",
            "APPLICATION ID",
            "Needed for pictures and buttons to show. Make a free application at discord.com/developers and paste its Application ID here.",
            "123456789012345678",
        ),
        Setting {
            key: "type",
            heading: "TYPE",
            note: "Streaming needs a Twitch or YouTube link below, or it shows as Playing.",
            kind: Kind::Choice {
                names: &[
                    "Playing",
                    "Listening to",
                    "Watching",
                    "Competing in",
                    "Streaming",
                ],
                values: &[0, 2, 3, 5, 1],
                unless_changed: 0,
            },
        },
        field(
            "name",
            "NAME",
            "Needed. Nothing shows without a name.",
            "My game",
        ),
        field("details", "DETAILS", "The first line under the name.", ""),
        field("state", "STATE", "The second line under the name.", ""),
        field(
            "big-image",
            "BIG PICTURE",
            "An asset key from your application, or a link to a picture.",
            "",
        ),
        field(
            "big-text",
            "BIG PICTURE TEXT",
            "Shown when someone points at the big picture.",
            "",
        ),
        field(
            "small-image",
            "SMALL PICTURE",
            "An asset key from your application, or a link to a picture.",
            "",
        ),
        field(
            "small-text",
            "SMALL PICTURE TEXT",
            "Shown when someone points at the small picture.",
            "",
        ),
        Setting {
            key: "started",
            heading: "STARTED",
            note: "Shows a timer of how long it has been going. \"When you turned this on\" counts from this run of Yapfast.",
            kind: Kind::Choice {
                names: &["None", "When Yapfast started", "When you turned this on"],
                values: &[0, 1, 2],
                unless_changed: 0,
            },
        },
        field(
            "button-1-text",
            "BUTTON 1 TEXT",
            "The words on the first button.",
            "",
        ),
        field(
            "button-1-link",
            "BUTTON 1 LINK",
            "Where the first button goes. It needs the text too.",
            "https://",
        ),
        field(
            "button-2-text",
            "BUTTON 2 TEXT",
            "The words on the second button.",
            "",
        ),
        field(
            "button-2-link",
            "BUTTON 2 LINK",
            "Where the second button goes. It needs the text too.",
            "https://",
        ),
        field(
            "stream-link",
            "STREAM LINK",
            "Used with Streaming. It has to be a Twitch or YouTube link.",
            "https://twitch.tv/",
        ),
    ],
    apply: |kept, plugin| {
        let at = |place: usize| text(kept, plugin, place).to_owned();
        set_custom_rpc(kept.on(plugin).then(|| Rpc {
            application: at(0),
            kind: value(kept, plugin, 1),
            name: at(2),
            details: at(3),
            state: at(4),
            large_image: at(5),
            large_text: at(6),
            small_image: at(7),
            small_text: at(8),
            started: Started::from_number(value(kept, plugin, 9)),
            buttons: [(at(10), at(11)), (at(12), at(13))],
            stream_link: at(14),
        }));
    },
    hooks: None,
};

pub(super) const ANIMATED_STATUS: Plugin = Plugin {
    id: id::ANIMATED_STATUS,
    name: "Animated status",
    about: "Turns your custom status through a list of lines. Only what others see changes; your saved status stays as it is.",
    on_at_first: false,
    settings: &[
        field(
            "lines",
            "LINES",
            "Put a | between lines. Empty ones are skipped.",
            "Working | Playing | Away for lunch",
        ),
        Setting {
            key: "every",
            heading: "EVERY",
            note: "How long each line stays. At least 30 seconds.",
            kind: Kind::Slider {
                most: 600,
                step: 5,
                unit: " s",
                unless_changed: 60,
            },
        },
        field(
            "emoji",
            "EMOJI",
            "One emoji shown beside every line. Leave empty for none.",
            "",
        ),
    ],
    apply: |kept, plugin| {
        set_animated(kept.on(plugin).then(|| Animated {
            lines: text(kept, plugin, 0).to_owned(),
            every_seconds: value(kept, plugin, 1).max(LEAST_EVERY),
            emoji: text(kept, plugin, 2).to_owned(),
        }));
    },
    hooks: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_are_in_the_order_apply_reads() {
        let keys = |plugin: &Plugin| -> Vec<&str> {
            plugin.settings.iter().map(|setting| setting.key).collect()
        };
        assert_eq!(
            keys(&CUSTOM_RPC),
            [
                "application",
                "type",
                "name",
                "details",
                "state",
                "big-image",
                "big-text",
                "small-image",
                "small-text",
                "started",
                "button-1-text",
                "button-1-link",
                "button-2-text",
                "button-2-link",
                "stream-link"
            ]
        );
        assert_eq!(keys(&ANIMATED_STATUS), ["lines", "every", "emoji"]);
    }
}
