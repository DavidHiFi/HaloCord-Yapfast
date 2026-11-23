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
    about: "Opens the channel or DM you were in when Yapfast closed, once it starts again and Discord lists it (within a minute).",
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
    /// The file it keeps the conversation in, once it started.
    path: Option<std::path::PathBuf>,
    /// The conversation open, as last written.
    written: Option<String>,
    /// The one to open again, until it opens or the wait runs out.
    pending: Option<(String, std::time::Instant)>,
}

/// Hook, after each message: the first time while on, the conversation kept
/// is read from `halocord-channel` in the app's folder, and opens once it's
/// listed; after that, the one open is written there when it changes.
pub fn keep_channel(app: &mut crate::app::App) -> iced::Task<crate::app::Message> {
    if !app.layout.halocord.is(id::KEEP_CHANNEL) || app.client.state().preview {
        return iced::Task::none();
    }
    let kept = &mut app.halocord_kept;
    if kept.path.is_none() {
        let path = app.client.root().join("halocord-channel");
        kept.pending = std::fs::read_to_string(&path)
            .ok()
            .map(|room| room.trim().to_owned())
            .filter(|room| !room.is_empty())
            .map(|room| (room, std::time::Instant::now()));
        kept.written = kept.pending.as_ref().map(|(room, _)| room.clone());
        kept.path = Some(path);
    }
    let state = app.client.state();
    if let Some((room, since)) = kept.pending.clone() {
        if since.elapsed() > KEEP_WAIT {
            kept.pending = None;
        } else if state.rooms.iter().any(|known| known.id == room) {
            kept.pending = None;
            if state.selected.as_ref() != Some(&room) {
                app.client
                    .dispatch(yapfast_core::application::Action::Open(room));
                return app.sync();
            }
        }
        // Until the kept one opens, what's open at start isn't kept.
        return iced::Task::none();
    }
    let open = state.selected.clone();
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
