//! HaloCord's plugins in its copy of Yapfast: what each is, its settings,
//! and what it tells the core. Settings' HaloCord page draws [`PLUGINS`], and
//! `layout.json` keeps each plugin under `halocord`, beside Yapfast's own
//! settings, so it stays through updates. `docs/halocord.md` lists the few
//! places Yapfast's own files call in here.
//!
//! Each area has its own file, matching the core's `halocord/`:
//!
//! - `kept.rs`: what a plugin is, its settings, and what is kept of them.
//! - `voice.rs`: voice plugins and the voice marks in the lists.
//! - `chat.rs`: tags by names, the character counter and what a message
//!   or file goes up as.
//! - `menus.rs`: right-click menu rows and what a pick does.
//! - `buttons.rs`: the user area and the top bar's buttons.
//! - `members.rs`: the member list's count and platform marks, timeouts
//!   and a profile's exact dates and full size pictures.
//! - `presence.rs`: Custom RPC and Animated status.
//! - `theme_files.rs`: the Themes page's Save to file and Load from file (not
//!   a plugin).
//! - `areas.rs`: which part of Discord each plugin works on, for the chips
//!   on the Plugins page.
//! - `deck.rs`: Stream Deck keys, through a small server on this computer.
//! - `backup.rs`: Backup & Restore's file of the plugins, the look and the
//!   person's own themes (not a plugin).
//!
//! Adding a plugin: its id in [`id`], one `const` in its area's file, and its
//! place in [`PLUGINS`]. `docs/halocord.md` has the template. A plugin that
//! needs messages of its own (a press on something it drew, a reply from a
//! task it started) adds a variant to [`Event`] and its arm in [`update`].

mod areas;
pub mod backup;
mod buttons;
mod chat;
pub mod deck;
mod clicks;
mod fake_dialog;
mod kept;
mod members;
mod menus;
mod presence;
mod messages;
mod servers;
pub mod theme_files;
mod voice;

pub use areas::{Area, area};
pub use buttons::{Press, press, presses, start_button};
use chat::thousands;
pub use chat::{
    chat_buttons, count_color, count_words, double_click, inserts_gif, tag, upload_button,
    uploads_first,
};
pub use clicks::{Clicks, click_keys, click_row};
pub use fake_dialog::{FakeDialog, FakeEvent, view as fake_view};
use kept::off;
pub use kept::{Change, Kept, Kind, Plugin, Setting, changed};
#[cfg(test)]
pub use members::time_left;
pub use members::{
    Place, add_member_count, dm_marks, exact_day, full_picture, name_marks, platform_marks,
    timeout_clock, timeout_note, tip_member_count,
};
pub use members::{message_timeout, role_press, server_tip, zoom_size, zoomable};
pub use menus::{
    attachment_menu, chosen, copied_text, mention_button, message_rows, person_rows, search_result,
    song_links, song_page, sticker_tile,
};
pub use messages::{collapsed_row, fake_button, is_fake_row, kept_deleted, message_ink, notes};
pub use servers::{
    KeptChannel, home_face, home_mentions, keep_channel, no_request_badge,
};
pub use voice::{in_voice_mark, unmute, unmute_mark, voice_count};

// What only the tests reach from outside.
#[cfg(test)]
pub use buttons::Dock;
#[cfg(test)]
pub(crate) use buttons::{close_dm_actions, read_all_actions};
#[cfg(test)]
pub use chat::{Fill, chat_buttons_hidden, counter_words};
#[cfg(test)]
pub use clicks::presses_so_far;
#[cfg(test)]
pub use members::member_totals;
#[cfg(test)]
pub use voice::voice_room_of;

pub use id::*;

/// The plugins' ids, as `layout.json` keeps them, for the places that look one up.
pub mod id {
    pub const STEREO: &str = "stereo-loader";
    pub const FAKE_MUTE: &str = "fake-mute";
    pub const FAKE_DEAFEN: &str = "fake-deafen";
    pub const STAFF_TAG: &str = "staff-tag";
    pub const USER_AREA: &str = "user-area";
    pub const COPY_MENTION: &str = "copy-user-mention";
    pub const COPY_MEDIA_LINK: &str = "copy-media-link";
    pub const COPY_FILE_TEXT: &str = "copy-file-contents";
    pub const COPY_STICKER_LINK: &str = "copy-sticker-links";
    pub const QUICK_MENTION: &str = "quick-mention";
    pub const COPY_PROFILE_LINK: &str = "copy-user-urls";
    pub const RESTART: &str = "restart-button";
    pub const READ_ALL: &str = "read-all-button";
    pub const COUNTER: &str = "character-counter";
    pub const NEW_USER: &str = "new-user-tag";
    pub const CLOSE_DMS: &str = "close-all-dms";
    pub const MEMBER_COUNT: &str = "member-count";
    pub const VOLUME_BOOSTER: &str = "volume-booster";
    pub const FORCE_MONO: &str = "force-mono";
    pub const CLICK_TO_UNMUTE: &str = "click-to-unmute";
    pub const USER_VOICE_SHOW: &str = "user-voice-show";
    pub const VOICE_COUNT: &str = "voice-count";
    pub const PLATFORMS: &str = "platform-indicators";
    pub const ALWAYS_TRUST: &str = "always-trust";
    pub const REVEAL_SPOILERS: &str = "reveal-all-spoilers";
    pub const PLAIN_FOLDER: &str = "plain-folder-icon";
    pub const INVITE_DEFAULTS: &str = "invite-defaults";
    pub const NO_PENDING_COUNT: &str = "no-pending-count";
    pub const SHOW_TIMEOUT: &str = "show-timeout-duration";
    pub const SEND_TIMESTAMPS: &str = "send-timestamps";
    pub const FIX_FILE_EXTENSIONS: &str = "fix-file-extensions";
    pub const BETTER_JOINED_DATE: &str = "better-joined-date";
    pub const VIEW_ICONS: &str = "view-icons";
    pub const JUMP_TO_START: &str = "jump-to-start";
    pub const CLICK_ACTIONS: &str = "message-click-actions";
    pub const CLICKABLE_ROLES: &str = "clickable-roles";
    pub const COPY_STATUS_URLS: &str = "copy-status-urls";
    pub const COPY_PROFILE_COLORS: &str = "copy-profile-colors";
    pub const FIND_REPLY: &str = "find-reply";
    pub const IMAGE_ZOOM: &str = "image-zoom";
    pub const CUSTOM_RPC: &str = "custom-rpc";
    pub const ANIMATED_STATUS: &str = "animated-status";
    pub const FAKE_DMS: &str = "fake-dms";
    pub const MESSAGE_LOGGER: &str = "message-logger";
    pub const STREAM_DECK: &str = "stream-deck";
    pub const EMBEDDED_URLS: &str = "embedded-urls";
    pub const AUTO_ZIPPER: &str = "auto-zipper";
    pub const BETTER_FORWARDS: &str = "better-forwards";
    pub const GIF_PASTE: &str = "gif-paste";
    pub const HIDE_CHAT_BUTTONS: &str = "hide-chat-buttons";
    pub const HOME_TYPING: &str = "home-typing";
    pub const SEARCH_MENU: &str = "full-search-context";
    pub const UNIVERSAL_MENTION: &str = "universal-mention";
    pub const KEEP_CHANNEL: &str = "keep-current-channel";
    pub const BETTER_UPLOAD_BUTTON: &str = "better-upload-button";
    pub const NOTIFICATION_TITLE: &str = "notification-title";
    pub const SHOW_HIDDEN: &str = "show-hidden-things";
    pub const SHOW_SONG_NAME: &str = "show-song-name";
    pub const SONG_LINK: &str = "song-link";
    pub const FAVORITE_ANYTHING: &str = "favorite-anything";

}

/// The plugins, in the page's order. Each lives in its area's file.
pub static PLUGINS: &[Plugin] = &[
    voice::STEREO,
    voice::FAKE_MUTE,
    voice::FAKE_DEAFEN,
    chat::STAFF_TAG,
    buttons::USER_AREA,
    menus::COPY_MENTION,
    menus::COPY_MEDIA_LINK,
    menus::COPY_FILE_TEXT,
    menus::COPY_STICKER_LINK,
    menus::QUICK_MENTION,
    menus::COPY_PROFILE_LINK,
    buttons::RESTART,
    buttons::READ_ALL,
    chat::COUNTER,
    chat::NEW_USER,
    buttons::CLOSE_DMS,
    members::MEMBER_COUNT,
    voice::VOLUME_BOOSTER,
    voice::FORCE_MONO,
    voice::CLICK_TO_UNMUTE,
    voice::USER_VOICE_SHOW,
    voice::VOICE_COUNT,
    members::PLATFORMS,
    chat::ALWAYS_TRUST,
    chat::REVEAL_SPOILERS,
    chat::PLAIN_FOLDER,
    servers::INVITE_DEFAULTS,
    servers::NO_PENDING_COUNT,
    members::SHOW_TIMEOUT,
    chat::SEND_TIMESTAMPS,
    chat::FIX_FILE_EXTENSIONS,
    members::BETTER_JOINED_DATE,
    members::VIEW_ICONS,
    buttons::JUMP_TO_START,
    chat::CLICK_ACTIONS,
    members::CLICKABLE_ROLES,
    menus::COPY_STATUS_URLS,
    menus::COPY_PROFILE_COLORS,
    menus::FIND_REPLY,
    members::IMAGE_ZOOM,
    presence::CUSTOM_RPC,
    presence::ANIMATED_STATUS,
    messages::FAKE_DMS,
    messages::MESSAGE_LOGGER,
    deck::STREAM_DECK,
    chat::EMBEDDED_URLS,
    chat::AUTO_ZIPPER,
    chat::BETTER_FORWARDS,
    chat::GIF_PASTE,
    chat::HIDE_CHAT_BUTTONS,
    servers::HOME_TYPING,
    menus::SEARCH_MENU,
    chat::UNIVERSAL_MENTION,
    servers::KEEP_CHANNEL,
    chat::BETTER_UPLOAD_BUTTON,
    chat::NOTIFICATION_TITLE,
    members::SHOW_HIDDEN,
    members::SHOW_SONG_NAME,
    menus::SONG_LINK,
    chat::FAVORITE_ANYTHING,

];

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

/// Hook: Send timestamps' clock before the message box's buttons, while its
/// picker is on (`chat.rs`).
pub fn timestamp_button<'a>(
    cx: crate::ctx::Ctx<'a>,
    buttons: impl Into<iced::Element<'a, crate::app::Message>>,
) -> iced::Element<'a, crate::app::Message> {
    chat::timestamp_button(cx, buttons)
}

/// The window's plugins' own messages, carried by `Message::HaloCord`, so a
/// plugin's presses and replies need no event of Yapfast's own.
#[derive(Clone, Debug)]
pub enum Event {
    /// Send timestamps' picker, from its clock by the message box.
    TimestampMenu,
    /// Click to unmute: this person, unmuted for this account.
    Unmute(String),
    /// Copy sticker link: a sticker picker tile right clicked, by its picture.
    StickerMenu(String),
    /// Quick mention's hover-bar button: this Discord user, mentioned in
    /// the composer.
    Mention(String),
    /// Message click actions, while it counts presses: a press on a
    /// message's row (with what its double-click does), a double click's
    /// wait run out, and Backspace held or let go.
    Press(String, Option<crate::menus::MenuAct>),
    DoubleClickDue(String, u64, Option<crate::menus::MenuAct>),
    Backspace(bool),
    /// Search result menu: a right click on a search result, by its room
    /// and item.
    SearchMenu(String, String),
}

/// Carries out a plugin's message.
pub fn update(app: &mut crate::app::App, event: Event) -> iced::Task<crate::app::Message> {
    match event {
        Event::TimestampMenu => return chat::timestamp_menu(app),
        Event::Unmute(person) => voice::unmute(app, &person),
        Event::StickerMenu(picture) => return menus::sticker_menu(app, &picture),
        // As Quick mention's menu row does: the composer takes the mention
        // in and has the focus.
        Event::Mention(user) => {
            return menus::done(
                app,
                yapfast_core::halocord::Picked {
                    actions: vec![yapfast_core::application::Action::MentionPerson(user)],
                    ..Default::default()
                },
            );
        }
        Event::Press(item, double) => return clicks::press(app, item, double),
        Event::DoubleClickDue(item, press, double) => {
            return clicks::double_click_due(app, &item, press, double);
        }
        Event::Backspace(held) => clicks::backspace(app, held),
        Event::SearchMenu(room, item) => return menus::search_menu(app, room, item),
    }
    iced::Task::none()
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
    fn the_message_plugins_exist_with_their_settings_and_a_search_finds_them() {
        let find = |query: &str| -> Vec<&str> {
            PLUGINS
                .iter()
                .filter(|plugin| found(plugin, query))
                .map(|plugin| plugin.name)
                .collect()
        };
        let fake = PLUGINS.iter().find(|p| p.id == FAKE_DMS).unwrap();
        assert_eq!(fake.settings.len(), 1);
        let logger = PLUGINS.iter().find(|p| p.id == MESSAGE_LOGGER).unwrap();
        let headings: Vec<_> = logger.settings.iter().map(|s| s.heading).collect();
        assert_eq!(
            headings,
            [
                "Ignore bots",
                "Ignore your own messages",
                "Keep deleted messages",
                "Keep edit history",
                "Collapse deleted messages"
            ]
        );
        assert!(find("fake dm").contains(&"Fake DMs"));
        assert!(find("local mark").contains(&"Fake DMs"));
        assert!(find("edit history").contains(&"Message logger"));
        assert!(find("collapse").contains(&"Message logger"));
        // Both are added last, so the page's order before them is untouched.
        let last: Vec<_> = PLUGINS.iter().rev().take(2).map(|p| p.id).collect();
        assert_eq!(last, [MESSAGE_LOGGER, FAKE_DMS]);
    }

    /// Files HaloCord owns outright; `docs/halocord.md` lists them by area.
    fn own(path: &str) -> bool {
        ["/halocord/", "/halocord.rs", "/mochacord", "/halocord_"]
            .iter()
            .any(|part| path.contains(part))
            || path.ends_with("fonts/pick.rs")
            || path.ends_with("discord_audio/flat.rs")
    }

    fn rust_files(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_files(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                found.push(path);
            }
        }
    }

    /// Every Yapfast file that calls into HaloCord or MochaCord is on the
    /// rebase list in `docs/halocord.md`, so a Yapfast update never drops a
    /// hook nobody wrote down.
    #[test]
    fn every_hook_is_in_the_docs() {
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let docs = std::fs::read_to_string(crates.join("../docs/halocord.md")).unwrap();
        let mut missing = Vec::new();
        for krate in [
            "yapfast-core",
            "yapfast-discord",
            "yapfast-iced",
            "yapfast-iced-kit",
        ] {
            let src = crates.join(krate).join("src");
            let mut files = Vec::new();
            rust_files(&src, &mut files);
            for file in files {
                let path = file
                    .strip_prefix(&src)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = std::fs::read_to_string(&file).unwrap().to_lowercase();
                if own(&format!("/{path}"))
                    || !(text.contains("halocord") || text.contains("mochacord"))
                {
                    continue;
                }
                let parts: Vec<&str> = path.split('/').collect();
                let named = (0..parts.len())
                    .any(|from| docs.contains(&format!("`{}`", parts[from..].join("/"))));
                if !named {
                    missing.push(format!("{krate}/{path}"));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "add these to docs/halocord.md: {missing:?}"
        );
    }
}
