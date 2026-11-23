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
//! - `theme_files.rs`: Appearance's Save to file and Load from file (not a
//!   plugin).
//!
//! Adding a plugin: its id in [`id`], one `const` in its area's file, and its
//! place in [`PLUGINS`]. `docs/halocord.md` has the template.

mod buttons;
mod chat;
mod fake_dialog;
mod kept;
mod members;
mod menus;
mod messages;
mod servers;
pub mod theme_files;
mod voice;

pub use buttons::{Press, press, presses, start_button};
use chat::thousands;
pub use chat::{count_color, count_words, double_click, tag};
pub use fake_dialog::{FakeDialog, FakeEvent, view as fake_view};
use kept::off;
pub use kept::{Change, Kept, Kind, Plugin, Setting, changed};
#[cfg(test)]
pub use members::time_left;
pub use members::{
    add_member_count, exact_day, full_picture, platform_marks, timeout_clock, timeout_note,
};
pub use members::{role_press, zoom_size, zoomable};
pub use menus::{attachment_menu, chosen, copied_text, message_rows, person_rows};
pub use messages::{collapsed_row, fake_button, is_fake_row, kept_deleted, message_ink, notes};
pub use servers::{home_mentions, no_request_badge};
pub use voice::{in_voice_mark, unmute, unmute_mark, voice_count};

// What only the tests reach from outside.
#[cfg(test)]
pub use buttons::Dock;
#[cfg(test)]
pub(crate) use buttons::{close_dm_actions, read_all_actions};
#[cfg(test)]
pub use chat::{Fill, counter_words};
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
    pub const FAKE_DMS: &str = "fake-dms";
    pub const MESSAGE_LOGGER: &str = "message-logger";
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
    messages::FAKE_DMS,
    messages::MESSAGE_LOGGER,
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
