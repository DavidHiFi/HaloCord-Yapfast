//! Server and home plugins: what a new invite link starts with, and the
//! friend request count left off the DM home and Friends.

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
