//! Button plugins: the user area's look and buttons, and the top bar's
//! Restart and Read all, Close all DMs, and Jump to start.

use iced::Task;
use yapfast_core::application::Action;

use super::{Kept, PLUGINS, Plugin, id, off};
use crate::{
    app::{App, Message},
    pictures::Icon,
};

pub(super) const USER_AREA: Plugin = Plugin {
    id: id::USER_AREA,
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
        off(
            "game_button",
            "Game activity button",
            "A button that switches sharing the game you play on and off.",
        ),
    ],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const RESTART: Plugin = Plugin {
    id: id::RESTART,
    name: "Restart button",
    about: "A button in the top bar (beside Settings when the bar is the system's) that closes Yapfast and starts it again the way it was started.",
    on_at_first: true,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const READ_ALL: Plugin = Plugin {
    id: id::READ_ALL,
    name: "Read all button",
    about: "A button in the top bar (beside Settings when the bar is the system's) that marks every server and message you have unread as read, in one press.",
    on_at_first: true,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const CLOSE_DMS: Plugin = Plugin {
    id: id::CLOSE_DMS,
    name: "Close all DMs",
    about: "A small X beside Direct Messages that closes every one-to-one conversation in the list at once. Group chats stay. The messages aren't deleted, and a new message from someone opens theirs again.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const JUMP_TO_START: Plugin = Plugin {
    id: id::JUMP_TO_START,
    name: "Jump to start",
    about: "A button in the channel's top bar that jumps to the first message of the channel or DM you're in.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// How the user area looks and which buttons it has, from its plugin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dock {
    pub compact: bool,
    pub arrows_hidden: bool,
    pub fake_mute: bool,
    pub fake_deafen: bool,
    pub stats: bool,
    pub stereo: bool,
    pub game: bool,
}

impl Kept {
    /// The user area's options; as it started while its plugin is off.
    pub fn dock(&self) -> Dock {
        let Some(area) = PLUGINS
            .iter()
            .find(|plugin| plugin.id == id::USER_AREA)
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
            game: flag("game_button"),
        }
    }
}

/// A button a plugin adds to the top bar or the user area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Restart,
    ReadAll,
    CloseDms,
    JumpStart,
}

/// The buttons the plugins that are on add: icon, tip and what they do.
pub fn presses(kept: &Kept) -> Vec<(Icon, &'static str, Press)> {
    let mut buttons = Vec::new();
    if kept.is(id::RESTART) {
        buttons.push((Icon::Refresh, "Restart Yapfast", Press::Restart));
    }
    if kept.is(id::READ_ALL) {
        buttons.push((Icon::MarkRead, "Mark everything read", Press::ReadAll));
    }
    buttons
}

/// Jump to start's button in the channel's top bar: icon, tip and what it
/// does, over a Discord channel or DM's messages (and the preview's).
pub fn start_button(cx: crate::ctx::Ctx<'_>) -> Option<(Icon, &'static str, Message)> {
    let discord = cx
        .state
        .selected
        .as_deref()
        .and_then(yapfast_core::discord::parse_room)
        .is_some();
    let shows = cx.layout.halocord.is(id::JUMP_TO_START)
        && cx.state.shows_text()
        && (cx.state.preview || discord);
    shows.then(|| {
        (
            Icon::ChevronUp,
            "Jump to start",
            Message::Bar(crate::window_bar::Event::Halo(Press::JumpStart)),
        )
    })
}

/// What marks everything read: every server, and each conversation with
/// something unread. The core drops those with nothing to mark.
pub(crate) fn read_all_actions(state: &yapfast_core::application::AppState) -> Vec<Action> {
    let servers = state.servers.iter().map(|server| server.id.clone());
    let chats = state
        .rooms
        .iter()
        .filter(|room| room.direct && (room.unread > 0 || room.mentions > 0))
        .map(|room| room.id.clone());
    servers.chain(chats).map(Action::MarkRead).collect()
}

/// What closes every conversation in the list: a close for each direct
/// one. The core drops the groups and anything that can't be closed.
pub(crate) fn close_dm_actions(state: &yapfast_core::application::AppState) -> Vec<Action> {
    state
        .rooms
        .iter()
        .filter(|room| room.direct)
        .map(|room| Action::CloseDm(room.id.clone()))
        .collect()
}

/// A plugin's button was pressed.
pub fn press(app: &mut App, press: Press) -> Task<Message> {
    match press {
        Press::Restart => app.requests.push(crate::ctx::Request::Restart),
        Press::CloseDms => {
            for action in close_dm_actions(app.client.state()) {
                app.client.dispatch(action);
            }
        }
        Press::JumpStart => app.client.dispatch(Action::JumpToStart),
        Press::ReadAll => {
            for action in read_all_actions(app.client.state()) {
                app.client.dispatch(action);
            }
        }
    }
    app.sync()
}
