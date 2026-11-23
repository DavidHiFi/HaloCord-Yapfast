//! HaloCord's Platform indicators: which of Discord's apps each person is on
//! (desktop, phone, web, console) and their status on each, from the `client_status`
//! Discord sends with a presence. The presence readers call [`note`];
//! HaloCord's window reads [`clients`]. `docs/halocord.md` lists the hooks.

use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use serde_json::Value;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
