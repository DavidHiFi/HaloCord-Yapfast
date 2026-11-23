//! HaloCord's Show hidden things: the servers whose invites are paused, as
//! their `INVITES_DISABLED` feature says, which Discord's client shows only
//! to moderators. The server reader ([`crate::ready::server`]) calls
//! [`note_invites`]; HaloCord's window reads [`invites_paused`].

use std::{
    collections::HashSet,
    sync::{LazyLock, RwLock},
};

/// The servers whose invites are paused, by id.
static PAUSED: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(Default::default);

/// Notes whether `server`'s invites are paused, from its features.
pub fn note_invites(server: &str, paused: bool) {
    let Ok(mut known) = PAUSED.write() else {
        return;
    };
    if paused {
        known.insert(server.to_owned());
    } else {
        known.remove(server);
    }
}

/// Whether `server`'s invites were paused when Discord last said.
pub fn invites_paused(server: &str) -> bool {
    PAUSED.read().is_ok_and(|known| known.contains(server))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::invites_paused;

    #[test]
    fn paused_invites_are_noted_as_a_server_is_read() {
        let read = |features: serde_json::Value| {
            crate::ready::server(&json!({
                "id": "halo-test-invites", "name": "Test", "features": features,
            }))
            .expect("a server")
        };
        read(json!(["COMMUNITY", "INVITES_DISABLED"]));
        assert!(invites_paused("halo-test-invites"));
        // An update without it: running again.
        read(json!(["COMMUNITY"]));
        assert!(!invites_paused("halo-test-invites"));
        assert!(!invites_paused("halo-test-never-seen"));
    }
}
