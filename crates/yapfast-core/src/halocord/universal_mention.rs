//! Universal mention: `@` on Discord also suggests the other person of each
//! of this account's one-to-one DMs, wherever it's typed, after the people
//! the conversation already offers. Nothing is asked of Discord.

#[cfg(test)]
use std::cell::Cell;
use std::collections::HashSet;

use crate::{application::AppState, mention::Candidate, network::Network};

#[cfg(not(test))]
static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
thread_local! {
    // Each test its own, out of the way of the mention tests beside it.
    static ON: Cell<bool> = const { Cell::new(false) };
}

/// Turns the plugin on or off.
pub fn set(on: bool) {
    #[cfg(not(test))]
    ON.store(on, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(|cell| cell.set(on));
}

fn on() -> bool {
    #[cfg(not(test))]
    return ON.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(Cell::get)
}

/// Adds the people of the DMs not `seen` already to `candidates`, while the
/// plugin is on.
pub fn add(state: &AppState, seen: &mut HashSet<String>, candidates: &mut Vec<Candidate>) {
    if !on() {
        return;
    }
    for room in &state.rooms {
        let Some(person) = room
            .person
            .as_ref()
            .filter(|_| room.direct && !room.request)
        else {
            continue;
        };
        if room.network != Network::Discord || room.name.is_empty() || !seen.insert(person.clone())
        {
            continue;
        }
        candidates.push(Candidate {
            name: room.name.clone(),
            username: None,
            mention: format!("<@{person}>"),
            avatar: room.avatar.clone(),
            color: None,
            group: false,
            channel: false,
            voice: false,
        });
    }
}
