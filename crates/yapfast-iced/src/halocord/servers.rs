//! Server and home plugins: what a new invite link starts with, the
//! friend request count left off the DM home and Friends, and the
//! conversation kept across restarts.

use yapfast_core::{
    application::{AppState, INVITE_AGES, INVITE_USES},
    halocord as engine,
};

use super::{Kind, Plugin, Setting, id};
use crate::{controls::count_badge, rail::Home, sidebar::SideRow};

pub(super) const INVITE_DEFAULTS: Plugin = Plugin {
    id: id::INVITE_DEFAULTS,
    name: "Invite defaults",
    about: "Picks how long a new invite link lasts and how many times it works, so you don't have to change them in the invite window each time.",
    on_at_first: false,
    settings: &[
        Setting {
            key: "lasts",
            heading: "LINK LASTS",
            note: "How long a new invite link works before it runs out.",
            kind: Kind::Choice {
                names: &[
                    "30 min", "1 hr", "6 hr", "12 hr", "1 day", "7 days", "Never",
                ],
                values: &INVITE_AGES,
                unless_changed: 0,
            },
        },
        Setting {
            key: "uses",
            heading: "TIMES IT WORKS",
            note: "How many people can join with a new invite link.",
            kind: Kind::Choice {
                names: &["No limit", "1", "5", "10", "25", "50", "100"],
                values: &INVITE_USES,
                unless_changed: 0,
            },
        },
    ],
    apply: |kept, plugin| {
        engine::set_first_invite(kept.on(plugin).then(|| {
            (
                kept.value(plugin, &plugin.settings[0]),
                kept.value(plugin, &plugin.settings[1]),
            )
        }));
    },
    hooks: None,
};

pub(super) const NO_PENDING_COUNT: Plugin = Plugin {
    id: id::NO_PENDING_COUNT,
    name: "No pending count",
    about: "Leaves friend requests out of the red count on the DM home and beside Friends. The requests still wait under Pending.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// The DM home's count without the friend requests: its DM mentions only.
pub fn home_mentions(state: &AppState, home: &mut Home) {
    let mentions: u64 = state
        .rooms
        .iter()
        .filter(|room| room.direct)
        .map(|room| room.mentions)
        .sum();
    home.badge = count_badge(mentions);
}

/// Clears the friend request count beside Friends in the channel list.
pub fn no_request_badge(rows: &mut [SideRow]) {
    for row in rows {
        if let SideRow::Screens { badge, .. } = row {
            badge.clear();
        }
    }
}

pub(super) const KEEP_CHANNEL: Plugin = Plugin {
    id: id::KEEP_CHANNEL,
    name: "Keep current channel",
    about: "Opens the channel or DM you were in when Yapfast closed, once it starts again and Discord lists it (within a minute), unless you open something else first.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// How long after starting it waits for the kept conversation to be listed.
const KEEP_WAIT: std::time::Duration = std::time::Duration::from_secs(60);

/// Keep current channel's state, kept on the window (`App::halocord_kept`).
#[derive(Default)]
pub struct KeptChannel {
    /// The file it keeps the conversation in, from the first call on.
    path: Option<std::path::PathBuf>,
    /// The conversation open, as last written.
    written: Option<String>,
    /// The one to open again, until it opens, the wait runs out or someone
    /// opens something else.
    pending: Option<Pending>,
}

/// The kept conversation, waited for since `since`.
struct Pending {
    room: String,
    since: std::time::Instant,
    /// Where the window was when the wait started, or when Yapfast itself
    /// last moved it: a change that isn't Yapfast's own ends the wait.
    navigation: yapfast_core::application::Navigation,
    open: Option<String>,
}

#[cfg(test)]
impl KeptChannel {
    /// Whether it still waits to open the kept conversation.
    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }
}

impl Pending {
    /// Whether someone opened something else while it waited: the window
    /// moved to another server or home, or to a conversation Yapfast
    /// doesn't open by itself there (its first). Yapfast's own picks move
    /// the start along.
    fn overtaken(&mut self, state: &AppState) -> bool {
        if state.navigation != self.navigation {
            return true;
        }
        if state.selected == self.open {
            return false;
        }
        let own_pick = state
            .scoped_rooms()
            .find(|room| !room.request)
            .map(|room| &room.id);
        if state.selected.is_none() || state.selected.as_ref() == own_pick {
            self.open.clone_from(&state.selected);
            return false;
        }
        true
    }
}

/// Hook, after each message. The first call, as the window starts, reads
/// the conversation kept in `halocord-channel` in the app's folder, and
/// while the plugin was on then, opens it once it's listed. While the
/// plugin is on, the one open is written there when it changes. Switching
/// it on later keeps from then on and opens nothing.
pub fn keep_channel(app: &mut crate::app::App) -> iced::Task<crate::app::Message> {
    if app.client.state().preview {
        return iced::Task::none();
    }
    let on = app.layout.halocord.is(id::KEEP_CHANNEL);
    let kept = &mut app.halocord_kept;
    if kept.path.is_none() {
        let path = app.client.root().join("halocord-channel");
        let state = app.client.state();
        kept.pending = std::fs::read_to_string(&path)
            .ok()
            .map(|room| room.trim().to_owned())
            .filter(|room| on && !room.is_empty())
            .map(|room| Pending {
                room,
                since: std::time::Instant::now(),
                navigation: state.navigation.clone(),
                open: state.selected.clone(),
            });
        kept.written = kept.pending.as_ref().map(|pending| pending.room.clone());
        kept.path = Some(path);
    }
    if !on {
        kept.pending = None;
        return iced::Task::none();
    }
    let state = app.client.state();
    if let Some(pending) = &mut kept.pending {
        let room = pending.room.clone();
        if pending.since.elapsed() > KEEP_WAIT || pending.overtaken(state) {
            kept.pending = None;
        } else if state.rooms.iter().any(|known| known.id == room) {
            kept.pending = None;
            if state.selected.as_ref() != Some(&room) {
                app.client
                    .dispatch(yapfast_core::application::Action::Open(room));
                return app.sync();
            }
        } else {
            // Until the kept one opens, what's open at start isn't kept.
            return iced::Task::none();
        }
    }
    let open = app.client.state().selected.clone();
    let kept = &mut app.halocord_kept;
    if open != kept.written
        && let (Some(room), Some(path)) = (open, &kept.path)
    {
        if let Err(error) = std::fs::write(path, &room) {
            tracing::warn!(%error, "Keep current channel can't keep the conversation");
        }
        kept.written = Some(room);
    }
    iced::Task::none()
}
