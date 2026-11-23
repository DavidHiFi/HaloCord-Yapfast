//! HaloCord's Platform indicators: which of Discord's apps each person is on
//! (desktop, phone, web, console) and their status on each, from the `client_status`
//! Discord sends with a presence. The presence readers call [`note`];
//! HaloCord's window reads [`clients`]. Also Fake deafen's switch, so a call
//! is joined showing deafened ([`fake_deafened`]). `docs/halocord.md` lists
//! the hooks.

pub mod presence;

use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use serde_json::Value;

mod invites;
pub use invites::{invites_paused, note_invites};

/// Someone's status on one of Discord's apps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Online,
    Idle,
    DoNotDisturb,
}

/// Where someone is signed in, and as what; none where they aren't.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Clients {
    pub desktop: Option<Status>,
    pub mobile: Option<Status>,
    pub web: Option<Status>,
    /// A console (Xbox, PlayStation), which Discord calls `embedded`.
    pub console: Option<Status>,
}

impl Clients {
    /// Reads Discord's `client_status` object.
    pub fn parse(clients: &Value) -> Self {
        let one = |app: &str| match clients[app].as_str() {
            Some("online") => Some(Status::Online),
            Some("idle") => Some(Status::Idle),
            Some("dnd") => Some(Status::DoNotDisturb),
            _ => None,
        };
        Self {
            desktop: one("desktop"),
            mobile: one("mobile"),
            web: one("web"),
            console: one("embedded"),
        }
    }
}

/// Everyone seen online, by user id. Offline people are left out.
static SEEN: LazyLock<RwLock<HashMap<String, Clients>>> = LazyLock::new(Default::default);

/// Notes `user`'s apps from a presence object as Discord sent it.
pub fn note(user: &str, presence: &Value) {
    let clients = Clients::parse(&presence["client_status"]);
    let Ok(mut seen) = SEEN.write() else {
        return;
    };
    if clients == Clients::default() {
        seen.remove(user);
    } else if seen.get(user) != Some(&clients) {
        seen.insert(user.to_owned(), clients);
    }
}

/// The apps `user` was last seen on.
pub fn clients(user: &str) -> Clients {
    SEEN.read()
        .ok()
        .and_then(|seen| seen.get(user).copied())
        .unwrap_or_default()
}

/// Fake deafen, as the core sets it with the rest of the fake voice state.
static FAKE_DEAF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Shows this account deafened from the moment it joins a call, or not.
pub fn set_fake_deafen(on: bool) {
    FAKE_DEAF.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Whether a join should say deafened: Fake deafen is on. A join otherwise
/// says muted and undeafened until the call's own voice state follows, and
/// others would see the account undeafened meanwhile.
pub fn fake_deafened() -> bool {
    FAKE_DEAF.load(std::sync::atomic::Ordering::Relaxed)
}

/// HaloCord's Show song name: whether a Spotify listener's line under their
/// name shows the song.
#[cfg(not(test))]
static SONG_NAMES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
thread_local! {
    // Each test its own, out of the way of the activity tests beside it.
    static SONG_NAMES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Show song name: switches it on or off.
pub fn set_song_names(on: bool) {
    #[cfg(not(test))]
    SONG_NAMES.store(on, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    SONG_NAMES.with(|cell| cell.set(on));
}

/// Show song name: whether it's on.
pub fn song_names() -> bool {
    #[cfg(not(test))]
    return SONG_NAMES.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    SONG_NAMES.with(std::cell::Cell::get)
}

/// Show song name: the song a Spotify activity's line names while it's on,
/// in place of "Spotify".
pub(crate) fn song_name(activity: &crate::activity::Activity) -> Option<&str> {
    (song_names() && activity.spotify())
        .then_some(activity.details.as_deref())
        .flatten()
        .filter(|song| !song.is_empty())

}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn show_song_name_names_the_song_under_a_name() {
        let music = crate::activity::activities(&json!([{
            "type": 2, "name": "Spotify", "details": "Ashes of the Damned", "state": "Kevin Sherwood",
        }]));
        // Each with the line Discord gives it, as the member list has it.
        let shown = |music: &crate::activity::Activities| {
            let line = music.first().and_then(|first| first.line());
            crate::activity::subline(music, line.as_deref()).name
        };
        assert_eq!(shown(&music).as_deref(), Some("Spotify"), "off");
        set_song_names(true);
        assert_eq!(shown(&music).as_deref(), Some("Ashes of the Damned"));
        // Another app listening, or Spotify with no song, says its name.
        let other = crate::activity::activities(&json!([{
            "type": 2, "name": "Tidal", "details": "A song",
        }]));
        assert_eq!(shown(&other).as_deref(), Some("Tidal"));
        let quiet = crate::activity::activities(&json!([{"type": 2, "name": "Spotify"}]));
        assert_eq!(shown(&quiet).as_deref(), Some("Spotify"));
        set_song_names(false);
    }

    #[test]
    fn apps_are_read_and_forgotten_when_offline() {
        note(
            "halo-test-1",
            &json!({"status": "online", "client_status": {"desktop": "dnd", "mobile": "online"}}),
        );
        assert_eq!(
            clients("halo-test-1"),
            Clients {
                desktop: Some(Status::DoNotDisturb),
                mobile: Some(Status::Online),
                web: None,
                console: None,
            }
        );
        note(
            "halo-test-1",
            &json!({"status": "offline", "client_status": {}}),
        );
        assert_eq!(clients("halo-test-1"), Clients::default());
        assert_eq!(clients("halo-test-nobody"), Clients::default());
    }

    #[test]
    fn a_web_idle_reads_as_idle() {
        assert_eq!(
            Clients::parse(&json!({"web": "idle"})),
            Clients {
                web: Some(Status::Idle),
                ..Clients::default()
            }
        );
    }

    /// A console (Xbox, PlayStation) comes as `embedded`.
    #[test]
    fn a_console_is_read_from_embedded() {
        assert_eq!(
            Clients::parse(&json!({"embedded": "online", "mobile": "dnd"})),
            Clients {
                mobile: Some(Status::DoNotDisturb),
                console: Some(Status::Online),
                ..Clients::default()
            }
        );
    }
}

mod song_link;
pub use song_link::{SongPlatform, song_platforms};
