//! Which part of Discord each plugin works on, for the chips over the
//! Plugins page. The plugins themselves stay as they are: this is a table
//! beside them, by id, and an id it doesn't know is [`Area::Other`].

use super::id;

/// A part of the app a plugin belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Voice,
    Chat,
    Menus,
    Buttons,
    Members,
    Servers,
    Status,
    Other,
}

impl Area {
    /// Every area, in the order the chips show.
    pub const ALL: [Area; 8] = [
        Area::Voice,
        Area::Chat,
        Area::Menus,
        Area::Buttons,
        Area::Members,
        Area::Servers,
        Area::Status,
        Area::Other,
    ];

    /// The chip's words.
    pub fn name(self) -> &'static str {
        match self {
            Area::Voice => "Voice",
            Area::Chat => "Chat",
            Area::Menus => "Menus",
            Area::Buttons => "Buttons",
            Area::Members => "Members",
            Area::Servers => "Servers",
            Area::Status => "Status",
            Area::Other => "Other",
        }
    }
}

/// The area of the plugin with `id`.
pub fn area(id: &str) -> Area {
    match id {
        id::STEREO
        | id::FAKE_MUTE
        | id::FAKE_DEAFEN
        | id::FAKE_CAMERA
        | id::FAKE_STREAM
        | id::VOLUME_BOOSTER
        | id::FORCE_MONO
        | id::CLICK_TO_UNMUTE
        | id::USER_VOICE_SHOW
        | id::VOICE_COUNT
        | id::FOLLOW_MAJORITY
        | id::STREAM_DECK => Area::Voice,
        id::COUNTER
        | id::NEW_USER
        | id::ALWAYS_TRUST
        | id::REVEAL_SPOILERS
        | id::SEND_TIMESTAMPS
        | id::FIX_FILE_EXTENSIONS
        | id::CLICK_ACTIONS
        | id::IMAGE_ZOOM
        | id::EMBEDDED_URLS
        | id::AUTO_ZIPPER
        | id::BETTER_FORWARDS
        | id::GIF_PASTE
        | id::UNIVERSAL_MENTION
        | id::NOTIFICATION_TITLE
        | id::FAVORITE_ANYTHING
        | id::SHOW_BADGES
        | "fake-dms"
        | "message-logger" => Area::Chat,
        id::COPY_MENTION
        | id::COPY_MEDIA_LINK
        | id::COPY_FILE_TEXT
        | id::COPY_STICKER_LINK
        | id::QUICK_MENTION
        | id::COPY_PROFILE_LINK
        | id::COPY_STATUS_URLS
        | id::COPY_PROFILE_COLORS
        | id::FIND_REPLY
        | id::SEARCH_MENU
        | id::SONG_LINK => Area::Menus,
        id::USER_AREA
        | id::RESTART
        | id::READ_ALL
        | id::CLOSE_DMS
        | id::JUMP_TO_START
        | id::HIDE_CHAT_BUTTONS
        | id::BETTER_UPLOAD_BUTTON
        | id::USER_TOOLS => Area::Buttons,
        id::STAFF_TAG
        | id::MEMBER_COUNT
        | id::PLATFORMS
        | id::SHOW_TIMEOUT
        | id::BETTER_JOINED_DATE
        | id::VIEW_ICONS
        | id::CLICKABLE_ROLES
        | id::SHOW_HIDDEN
        | id::SHOW_SONG_NAME
        | id::SHOW_CONNECTIONS
        | id::LAST_ONLINE
        | id::USER_COLORS => Area::Members,
        id::PLAIN_FOLDER
        | id::INVITE_DEFAULTS
        | id::NO_PENDING_COUNT
        | id::HOME_TYPING
        | id::KEEP_CHANNEL
        | id::SERVER_COUNTS
        | id::SERVER_SIDEBAR => Area::Servers,
        "custom-rpc" | "animated-status" => Area::Status,
        _ => Area::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halocord::PLUGINS;

    #[test]
    fn every_plugin_has_a_real_area() {
        for plugin in PLUGINS {
            assert_ne!(area(plugin.id), Area::Other, "{} needs an area", plugin.id);
        }
    }

    #[test]
    fn plugins_still_to_come_are_placed_and_strangers_are_other() {
        assert_eq!(area("custom-rpc"), Area::Status);
        assert_eq!(area("animated-status"), Area::Status);
        assert_eq!(area("fake-dms"), Area::Chat);
        assert_eq!(area("message-logger"), Area::Chat);
        assert_eq!(area("not-a-plugin"), Area::Other);
        assert_eq!(area(id::STEREO), Area::Voice);
    }

    #[test]
    fn the_chips_name_each_area_once() {
        let mut names: Vec<_> = Area::ALL.iter().map(|area| area.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Area::ALL.len());
    }
}
