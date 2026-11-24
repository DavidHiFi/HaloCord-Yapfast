//! Last online: a DM's row says how long ago its person went offline, as
//! this app saw them go. Nothing is asked of Discord: someone who went
//! offline while Yapfast was closed shows nothing until they come and go
//! again. The times are kept in `halocord-last-online.json` in the app's
//! folder while "Kept across restarts" is on.

use std::{collections::BTreeMap, path::Path, sync::Mutex};

use yapfast_core::{application::AppState, chat::Presence};

use super::{Kept, Kind, Plugin, Setting, id};
use crate::ctx::Ctx;

pub(super) const LAST_ONLINE: Plugin = Plugin {
    id: id::LAST_ONLINE,
    name: "Last online",
    about: "A DM's row says how long ago its person went offline, as Yapfast saw them go. Nothing is asked of Discord: someone who went offline while Yapfast was closed shows nothing until they come and go again.",
    on_at_first: false,
    settings: &[Setting {
        key: "kept",
        heading: "Kept across restarts",
        note: "In halocord-last-online.json in Yapfast's folder.",
        kind: Kind::Switch {
            unless_changed: true,
        },
    }],
    apply: |kept, plugin| {
        if !kept.on(plugin) {
            seen(|seen| *seen = Seen::default());
        }
    },
    hooks: None,
};

/// Whether each DM's person was online at the last look, and when each was
/// last seen going offline, in Unix milliseconds, by the DM's room id.
#[derive(Default)]
struct Seen {
    read: bool,
    online: BTreeMap<String, bool>,
    last: BTreeMap<String, u64>,
}

/// As a static, like Hide chat buttons' state: the DM rows draw from it.
static SEEN: Mutex<Option<Seen>> = Mutex::new(None);

fn seen<T>(with: impl FnOnce(&mut Seen) -> T) -> T {
    let mut held = SEEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    with(held.get_or_insert_with(Seen::default))
}

const FILE: &str = "halocord-last-online.json";

/// Hook, as the window makes its lists again: notes who of the DMs' people
/// went offline since the last look, and keeps it while it's on.
pub fn note_presences(kept: &Kept, root: &Path, state: &AppState) {
    if !kept.is(id::LAST_ONLINE) {
        return;
    }
    let keeps = kept.flag(&LAST_ONLINE, &LAST_ONLINE.settings[0]);
    let file = root.join(FILE);
    let people = state
        .rooms
        .iter()
        .filter(|room| room.direct)
        .filter_map(|room| Some((room.id.as_str(), room.presence?)));
    let json = seen(|held| {
        if !held.read {
            held.read = true;
            if keeps && let Ok(read) = std::fs::read(&file) {
                held.last = serde_json::from_slice(&read).unwrap_or_default();
            }
        }
        let went = note(held, people, yapfast_core::clock::now_ms());
        (went && keeps)
            .then(|| serde_json::to_vec(&held.last).ok())
            .flatten()
    });
    if let Some(json) = json
        && let Err(error) = std::fs::write(&file, json)
    {
        tracing::warn!(%error, "Last online can't keep its times");
    }
}

/// Notes each person's presence; whether someone went offline.
fn note<'p>(seen: &mut Seen, people: impl Iterator<Item = (&'p str, Presence)>, now: u64) -> bool {
    let mut went = false;
    for (person, presence) in people {
        let online = presence != Presence::Offline;
        if seen.online.insert(person.to_owned(), online) == Some(true) && !online {
            seen.last.insert(person.to_owned(), now);
            went = true;
        }
    }
    went
}

/// "Online 5m ago" for someone offline now who was seen going, at `now`.
fn line(seen: &Seen, person: &str, now: u64) -> Option<String> {
    if seen.online.get(person) == Some(&true) {
        return None;
    }
    let minutes = now.saturating_sub(*seen.last.get(person)?) / 60_000;
    Some(match minutes {
        0 => "Online just now".to_owned(),
        1..60 => format!("Online {minutes}m ago"),
        60..1440 => format!("Online {}h ago", minutes / 60),
        _ => format!("Online {}d ago", minutes / 1440),
    })
}

/// Hook: Last online's line under the name of the DM `room`.
pub fn dm_line(cx: Ctx<'_>, room: &str) -> Option<String> {
    if !cx.layout.halocord.is(id::LAST_ONLINE) {
        return None;
    }
    seen(|seen| line(seen, room, yapfast_core::clock::now_ms()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_who_went_offline_and_says_how_long_ago() {
        let mut seen = Seen::default();
        let look = |seen: &mut Seen, presence, now| note(seen, [("77", presence)].into_iter(), now);
        assert!(!look(&mut seen, Presence::Online, 0), "online: nothing yet");
        assert_eq!(line(&seen, "77", 0), None);
        assert!(look(&mut seen, Presence::Offline, 1_000), "went offline");
        assert_eq!(line(&seen, "77", 1_000).as_deref(), Some("Online just now"));
        assert_eq!(
            line(&seen, "77", 1_000 + 5 * 60_000).as_deref(),
            Some("Online 5m ago")
        );
        assert_eq!(
            line(&seen, "77", 1_000 + 3 * 3_600_000).as_deref(),
            Some("Online 3h ago")
        );
        assert_eq!(
            line(&seen, "77", 1_000 + 2 * 86_400_000).as_deref(),
            Some("Online 2d ago")
        );
        assert!(
            !look(&mut seen, Presence::Offline, 9_000),
            "still offline: kept as it was"
        );
        assert_eq!(line(&seen, "77", 1_000).as_deref(), Some("Online just now"));
        look(&mut seen, Presence::Idle, 10_000);
        assert_eq!(line(&seen, "77", 10_000), None, "back: no line");
        assert_eq!(line(&seen, "never", 0), None, "never seen going");
    }
}
