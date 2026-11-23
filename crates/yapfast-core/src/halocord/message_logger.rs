//! Message logger: a message deleted while it's loaded stays where it was,
//! marked deleted, and an edited message keeps the text it had before each
//! edit. Only what this app saw while it ran: the store is in memory, holds
//! the last [`MESSAGES`] messages it marked, and is never put on disk
//! (`open_channel.rs` leaves kept deleted messages out of the channels it
//! saves). It asks Discord nothing.

use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

/// What the plugin keeps and leaves alone, as its settings say.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub deletes: bool,
    pub edits: bool,
    pub ignore_bots: bool,
    /// Discord ids of people, channels and servers to leave alone.
    pub ignore_people: Vec<String>,
    pub ignore_channels: Vec<String>,
    pub ignore_servers: Vec<String>,
}

impl Settings {
    /// Ids from a list separated by commas, spaces or lines.
    pub fn ids(list: &str) -> Vec<String> {
        list.split([',', ' ', '\n'])
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

/// The most messages the store marks: the oldest goes first.
pub const MESSAGES: usize = 2000;
/// The most earlier texts one message keeps.
pub const VERSIONS: usize = 20;

/// What the store knows of one message.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Logged {
    pub deleted: bool,
    /// Its texts before each edit, the oldest first.
    pub edits: Vec<String>,
}

#[derive(Default)]
struct Store {
    settings: Option<Settings>,
    by_id: HashMap<String, Logged>,
    order: VecDeque<String>,
}

#[cfg(not(test))]
static STORE: Mutex<Option<Store>> = Mutex::new(None);

#[cfg(test)]
thread_local! {
    // Each test keeps its own, out of the way of the tests beside it.
    static STORE: Mutex<Option<Store>> = const { Mutex::new(None) };
}

fn with<T>(work: impl FnOnce(&mut Store) -> T) -> T {
    let run = |store: &Mutex<Option<Store>>| {
        let mut held = store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        work(held.get_or_insert_with(Store::default))
    };
    #[cfg(not(test))]
    return run(&STORE);
    #[cfg(test)]
    STORE.with(run)
}

/// Sets what it keeps, or switches it off for `None`, which forgets what it
/// kept (kept deleted messages go at their channel's next load).
pub fn set(settings: Option<Settings>) {
    with(|store| {
        if settings.is_none() {
            store.by_id.clear();
            store.order.clear();
        }
        store.settings = settings;
    });
}

/// Whether the plugin keeps deleted messages now.
pub fn keeps_deletions() -> bool {
    with(|store| {
        store
            .settings
            .as_ref()
            .is_some_and(|settings| settings.deletes)
    })
}

/// Whether the plugin kept the message with this Discord id, deleted.
pub fn deleted(id: &str) -> bool {
    with(|store| store.by_id.get(id).is_some_and(|logged| logged.deleted))
}

/// What it knows of the message with this Discord id.
pub fn logged(id: &str) -> Option<Logged> {
    with(|store| store.by_id.get(id).cloned())
}

impl Store {
    // Only live events reach it, and they're Discord's.
    #[cfg_attr(not(feature = "discord"), allow(dead_code))]
    fn entry(&mut self, id: &str) -> &mut Logged {
        if !self.by_id.contains_key(id) {
            if self.order.len() >= MESSAGES
                && let Some(oldest) = self.order.pop_front()
            {
                self.by_id.remove(&oldest);
            }
            self.order.push_back(id.to_owned());
        }
        self.by_id.entry(id.to_owned()).or_default()
    }
}

/// A live event in `channel` of `guild`, before the open channel applies it,
/// with the messages it has loaded. An edit's earlier text is kept. A
/// deletion of a message it keeps is kept in place of it: this gives the
/// message's place (its snowflake), and the channel keeps it.
#[cfg(feature = "discord")]
pub fn live<'a>(
    guild: &str,
    channel: &str,
    event: &yapfast_discord::service::Live,
    known: impl Fn(u64) -> Option<&'a yapfast_discord::message::Message>,
) -> Option<u64> {
    use yapfast_discord::service::Live;
    let (id, edit) = match event {
        Live::Deleted { channel: of, id } if of == channel => (id, None),
        Live::Updated(message) if message.channel_id == channel => {
            (&message.id, Some(message.content.as_str()))
        }
        _ => return None,
    };
    let order = id.parse().ok()?;
    let known = known(order)?;
    with(|store| {
        let settings = store.settings.as_ref()?;
        if known.ephemeral
            || (settings.ignore_bots && known.bot)
            || settings.ignore_people.contains(&known.author_id)
            || settings.ignore_channels.iter().any(|id| id == channel)
            || settings.ignore_servers.iter().any(|id| id == guild)
        {
            return None;
        }
        match edit {
            None if settings.deletes => {
                store.entry(id).deleted = true;
                Some(order)
            }
            // The same edit reaches each channel holding the message, and an
            // update that leaves the text as it was (an embed coming) is no
            // edit.
            Some(text) if settings.edits && text != known.content => {
                let edits = &mut store.entry(id).edits;
                if edits.last() != Some(&known.content) {
                    edits.push(known.content.clone());
                    if edits.len() > VERSIONS {
                        edits.remove(0);
                    }
                }
                None
            }
            _ => None,
        }
    })
}

#[cfg(all(test, feature = "discord"))]
mod tests {
    use yapfast_discord::{message::Message, service::Live};

    use super::*;

    fn message(id: &str, author: &str, content: &str) -> Message {
        let data = serde_json::json!({
            "id": id, "channel_id": "20", "content": content,
            "author": {"id": author, "username": "sam"},
            "timestamp": "2026-11-21T00:00:00Z",
        });
        yapfast_discord::message::parse(&data).expect("a message")
    }

    fn deletion(id: &str) -> Live {
        Live::Deleted {
            channel: "20".into(),
            id: id.into(),
        }
    }

    #[test]
    fn deletions_and_edits_are_kept_while_on() {
        let first = message("100", "7", "first");
        let known = |order: u64| (order == 100).then_some(&first);
        assert_eq!(live("10", "20", &deletion("100"), known), None, "off");
        set(Some(Settings {
            deletes: true,
            edits: true,
            ..Settings::default()
        }));
        let edit = Live::Updated(message("100", "7", "second"));
        assert_eq!(live("10", "20", &edit, known), None);
        assert_eq!(live("10", "20", &edit, known), None, "a second channel");
        let same = Live::Updated(message("100", "7", "first"));
        assert_eq!(live("10", "20", &same, known), None, "no new text");
        assert_eq!(live("10", "20", &deletion("100"), known), Some(100));
        assert_eq!(
            logged("100"),
            Some(Logged {
                deleted: true,
                edits: vec!["first".into()]
            })
        );
        assert!(deleted("100"));
        // Not loaded, or another channel's: left to the channel.
        assert_eq!(live("10", "20", &deletion("101"), known), None);
        assert_eq!(live("10", "21", &deletion("100"), known), None);
        set(None);
        assert!(!deleted("100"), "forgotten when off");
    }

    #[test]
    fn ignored_people_channels_servers_and_bots_are_left_alone() {
        let first = message("100", "7", "first");
        let known = |order: u64| (order == 100).then_some(&first);
        let on = |settings: Settings| {
            set(Some(Settings {
                deletes: true,
                ..settings
            }));
        };
        on(Settings {
            ignore_people: Settings::ids("5, 7"),
            ..Settings::default()
        });
        assert_eq!(live("10", "20", &deletion("100"), known), None, "a person");
        on(Settings {
            ignore_channels: Settings::ids("20"),
            ..Settings::default()
        });
        assert_eq!(live("10", "20", &deletion("100"), known), None, "a channel");
        on(Settings {
            ignore_servers: Settings::ids("9\n10"),
            ..Settings::default()
        });
        assert_eq!(live("10", "20", &deletion("100"), known), None, "a server");
        on(Settings::default());
        assert_eq!(live("10", "20", &deletion("100"), known), Some(100));
        // Edits are off here: the earlier text isn't kept.
        let edit = Live::Updated(message("100", "7", "second"));
        assert_eq!(live("10", "20", &edit, known), None);
        assert_eq!(logged("100").map(|logged| logged.edits), Some(Vec::new()));
        set(None);
    }

    #[test]
    fn the_store_keeps_the_newest() {
        set(Some(Settings {
            deletes: true,
            ..Settings::default()
        }));
        let kept: Vec<_> = (0..=MESSAGES as u64)
            .map(|id| message(&(id + 1).to_string(), "7", "x"))
            .collect();
        for message in &kept {
            live("10", "20", &deletion(&message.id), |_| Some(message));
        }
        assert!(!deleted("1"), "the oldest went");
        assert!(deleted("2") && deleted(&(MESSAGES + 1).to_string()));
        set(None);
    }
}
