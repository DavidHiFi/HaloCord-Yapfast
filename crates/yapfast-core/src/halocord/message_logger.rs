//! The Message Logger plugin: a message someone deletes stays in the chat,
//! marked as deleted, and one they edit keeps its earlier wordings. Only
//! what Yapfast had already seen can be kept, and only in memory, up to
//! [`CAP`] messages, the oldest going first.
//!
//! The channel asks here, where it handles a deleted or an edited message
//! (`discord_text/open_channel.rs`), and marks the rows it draws.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use super::{Hooks, Picked};
use crate::{application::Action, chat::ChatMessage};

/// The most messages the log holds.
pub const CAP: usize = 2000;
/// The most earlier wordings kept of one message.
pub const MOST_EDITS: usize = 20;

/// What the plugin's page says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The plugin is on at all.
    pub on: bool,
    pub ignore_bots: bool,
    pub ignore_own: bool,
    pub keep_deleted: bool,
    pub keep_edits: bool,
    pub collapse: bool,
    /// Discord ids of people, channels and servers to leave alone.
    pub ignore_people: Vec<String>,
    pub ignore_channels: Vec<String>,
    pub ignore_servers: Vec<String>,
}

impl Settings {
    /// Nothing kept: what the core does before the page tells it.
    pub const OFF: Self = Self {
        on: false,
        ignore_bots: false,
        ignore_own: false,
        keep_deleted: true,
        keep_edits: true,
        collapse: false,
        ignore_people: Vec::new(),
        ignore_channels: Vec::new(),
        ignore_servers: Vec::new(),
    };

    /// Ids from a list separated by commas, spaces or lines.
    pub fn ids(list: &str) -> Vec<String> {
        list.split([',', ' ', '
'])
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn skips(&self, me: &str, author: &str, bot: bool) -> bool {
        (self.ignore_bots && bot)
            || (self.ignore_own && !me.is_empty() && author == me)
            || self.ignore_people.iter().any(|id| id == author)
    }

    /// Whether the page says to leave this channel, or its server, alone.
    fn skips_place(&self, guild: &str, channel: &str) -> bool {
        self.ignore_channels.iter().any(|id| id == channel)
            || (!guild.is_empty() && self.ignore_servers.iter().any(|id| id == guild))
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Entry {
    deleted: bool,
    /// Earlier wordings, oldest first.
    edits: Vec<String>,
}

/// What the logger remembers.
#[derive(Debug, Default)]
pub struct Log {
    entries: HashMap<String, Entry>,
    /// Message ids, in the order the log first heard of them.
    order: VecDeque<String>,
    cap: usize,
}

impl Log {
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            ..Self::default()
        }
    }

    fn entry(&mut self, id: &str) -> &mut Entry {
        if !self.entries.contains_key(id) {
            self.order.push_back(id.to_owned());
            self.entries.insert(id.to_owned(), Entry::default());
            while self.order.len() > self.cap.max(1) {
                if let Some(oldest) = self.order.pop_front() {
                    self.entries.remove(&oldest);
                }
            }
        }
        self.entries.get_mut(id).expect("just inserted")
    }

    /// A message was deleted: whether the log keeps it in the chat.
    pub fn delete(&mut self, set: &Settings, me: &str, author: &str, bot: bool, id: &str) -> bool {
        if !set.on || !set.keep_deleted || set.skips(me, author, bot) {
            return false;
        }
        self.entry(id).deleted = true;
        true
    }

    /// A message was edited from `old` to `new`; it adds the old wording,
    /// once, however many places hear of the edit.
    #[allow(
        clippy::too_many_arguments,
        reason = "who wrote it, and the edit's before and after"
    )]
    pub fn edit(
        &mut self,
        set: &Settings,
        me: &str,
        author: &str,
        bot: bool,
        id: &str,
        old: &str,
        new: &str,
    ) {
        if !set.on || !set.keep_edits || set.skips(me, author, bot) || old == new {
            return;
        }
        if self.entries.get(id).is_some_and(|entry| entry.deleted) {
            return;
        }
        let entry = self.entry(id);
        if entry.edits.last().is_some_and(|last| last == old) {
            return;
        }
        entry.edits.push(old.to_owned());
        if entry.edits.len() > MOST_EDITS {
            entry.edits.remove(0);
        }
    }

    pub fn is_deleted(&self, id: &str) -> bool {
        self.entries.get(id).is_some_and(|entry| entry.deleted)
    }

    /// The earlier wordings of a message, oldest first.
    pub fn history(&self, id: &str) -> Vec<String> {
        self.entries
            .get(id)
            .map(|entry| entry.edits.clone())
            .unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

struct Held {
    settings: Settings,
    me: String,
    log: Log,
    /// Counts clears and changes, so the window knows to draw again.
    changes: u64,
}

impl Held {
    fn new() -> Self {
        Self {
            settings: Settings::OFF,
            me: String::new(),
            log: Log::new(CAP),
            changes: 0,
        }
    }
}

#[cfg(not(test))]
static HELD: Mutex<Option<Held>> = Mutex::new(None);

#[cfg(test)]
thread_local! {
    // Each test keeps its own, out of the way of the tests beside it.
    static HELD: Mutex<Option<Held>> = const { Mutex::new(None) };
}

fn held<T>(with: impl FnOnce(&mut Held) -> T) -> T {
    let run = |cell: &Mutex<Option<Held>>| {
        let mut guard = cell.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        with(guard.get_or_insert_with(Held::new))
    };
    #[cfg(not(test))]
    return run(&HELD);
    #[cfg(test)]
    return HELD.with(run);
}

/// Sets what the page says.
pub fn set(settings: Settings) {
    held(|held| held.settings = settings);
}

/// Whether a deleted message is kept in the chat right now: the plugin and
/// its Keep deleted messages switch are both on.
pub fn keeps_deletions() -> bool {
    held(|held| held.settings.on && held.settings.keep_deleted)
}

/// What the page says now.
pub fn settings() -> Settings {
    held(|held| held.settings.clone())
}

/// Whether the page says to leave the channel `channel` of server `guild`
/// alone: nothing deleted or edited there is kept.
pub fn ignores_place(guild: &str, channel: &str) -> bool {
    held(|held| held.settings.skips_place(guild, channel))
}

/// Tells the logger which account is this one, for Ignore your own messages.
pub fn note_me(me: &str) {
    held(|held| {
        if held.me != me {
            me.clone_into(&mut held.me);
        }
    });
}

/// The account this one is, as the last open chat told it; empty before.
pub fn me() -> String {
    held(|held| held.me.clone())
}

/// A message was deleted: whether the chat keeps it, marked as deleted.
pub fn keeps(author: &str, bot: bool, id: &str) -> bool {
    held(|held| {
        let kept = held.log.delete(&held.settings, &held.me, author, bot, id);
        if kept {
            held.changes += 1;
        }
        kept
    })
}

/// A message was edited from `old` to `new`.
pub fn edited(author: &str, bot: bool, id: &str, old: &str, new: &str) {
    held(|held| {
        let before = held.log.history(id).len();
        held.log
            .edit(&held.settings, &held.me, author, bot, id, old, new);
        if held.log.history(id).len() != before {
            held.changes += 1;
        }
    });
}

/// Marks a row the channel drew: a kept deleted message reads as deleted
/// and takes no replies, reactions or edits.
pub fn mark(row: &mut ChatMessage, id: &str) {
    if !held(|held| held.log.is_deleted(id)) {
        return;
    }
    row.deleted = true;
    row.can_reply = false;
    row.can_edit = false;
    row.can_delete = false;
    row.can_react = false;
    row.can_forward = false;
    row.can_pin = false;
}

/// Whether the timeline row with this item (`d:` and the id) is a message
/// the logger kept after it was deleted.
pub fn is_deleted(item: &str) -> bool {
    held(|held| held.log.is_deleted(item.strip_prefix("d:").unwrap_or(item)))
}

/// The earlier wordings of the row with this item, oldest first.
pub fn history(item: &str) -> Vec<String> {
    held(|held| held.log.history(item.strip_prefix("d:").unwrap_or(item)))
}

/// Whether kept deleted messages show as one folded line.
pub fn collapse() -> bool {
    held(|held| held.settings.collapse)
}

/// Forgets everything kept. A deleted message still in a chat reads as a
/// live one until the chat loads again.
pub fn clear() {
    held(|held| {
        held.log.clear();
        held.changes += 1;
    });
}

/// How many messages the log holds.
pub fn len() -> usize {
    held(|held| held.log.len())
}

/// How often the log changed, for a window to tell when to draw again.
pub fn changes() -> u64 {
    held(|held| held.changes)
}

/// The plugin's one menu row, on a message the log holds something of.
pub struct MessageLogger;

impl Hooks for MessageLogger {
    fn id(&self) -> &'static str {
        "message-logger"
    }

    fn menu(&self, message: &ChatMessage) -> Vec<(&'static str, &'static str)> {
        if is_deleted(&message.item) || !history(&message.item).is_empty() {
            vec![("clear-log", "Clear the log")]
        } else {
            Vec::new()
        }
    }

    fn pick(&self, action: &str, _message: &ChatMessage) -> Picked {
        if action == "clear-log" {
            clear();
            return Picked {
                actions: vec![Action::Republish],
                copy: None,
            };
        }
        Picked {
            actions: Vec::new(),
            copy: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> Settings {
        Settings {
            on: true,
            ..Settings::OFF
        }
    }

    #[test]
    fn a_deleted_message_is_kept_and_marked() {
        let mut log = Log::new(10);
        assert!(log.delete(&on(), "me", "ann", false, "1"));
        assert!(log.is_deleted("1"));
        assert!(!log.is_deleted("2"));
    }

    #[test]
    fn edits_keep_their_order_and_do_not_repeat() {
        let mut log = Log::new(10);
        let set = on();
        log.edit(&set, "me", "ann", false, "1", "a", "b");
        log.edit(&set, "me", "ann", false, "1", "b", "c");
        // The same edit heard in a second place.
        log.edit(&set, "me", "ann", false, "1", "b", "c");
        // An edit that changed nothing.
        log.edit(&set, "me", "ann", false, "1", "c", "c");
        assert_eq!(log.history("1"), ["a", "b"]);
    }

    #[test]
    fn the_ignore_settings_apply() {
        let mut log = Log::new(10);
        let set = Settings {
            ignore_bots: true,
            ignore_own: true,
            ..on()
        };
        assert!(!log.delete(&set, "me", "robot", true, "1"));
        assert!(!log.delete(&set, "me", "me", false, "2"));
        assert!(log.delete(&set, "me", "ann", false, "3"));
        log.edit(&set, "me", "robot", true, "4", "a", "b");
        log.edit(&set, "me", "me", false, "5", "a", "b");
        assert!(log.history("4").is_empty() && log.history("5").is_empty());
        // Without the settings, bots and your own are kept.
        assert!(log.delete(&on(), "me", "robot", true, "6"));
        assert!(log.delete(&on(), "me", "me", false, "7"));
    }

    #[test]
    fn listed_people_channels_and_servers_are_left_alone() {
        let mut log = Log::new(10);
        let set = Settings {
            ignore_people: Settings::ids("40, 41"),
            ignore_channels: Settings::ids("50"),
            ignore_servers: Settings::ids("60
61"),
            ..on()
        };
        assert!(!log.delete(&set, "me", "41", false, "1"));
        log.edit(&set, "me", "40", false, "2", "a", "b");
        assert!(log.history("2").is_empty());
        assert!(log.delete(&set, "me", "42", false, "3"));
        assert!(set.skips_place("1", "50") && set.skips_place("61", "9"));
        assert!(!set.skips_place("1", "9"));
        // A DM has no server, so an empty server id never matches.
        assert!(!set.skips_place("", "9"));
        assert_eq!(Settings::ids(" 1,2 

3 "), ["1", "2", "3"]);
    }

    #[test]
    fn the_switches_keep_deletes_and_edits_apart() {
        let mut log = Log::new(10);
        let no_deletes = Settings {
            keep_deleted: false,
            ..on()
        };
        assert!(!log.delete(&no_deletes, "me", "ann", false, "1"));
        let no_edits = Settings {
            keep_edits: false,
            ..on()
        };
        log.edit(&no_edits, "me", "ann", false, "2", "a", "b");
        assert!(log.history("2").is_empty());
        assert!(log.delete(&no_edits, "me", "ann", false, "3"));
    }

    #[test]
    fn off_keeps_nothing() {
        let mut log = Log::new(10);
        assert!(!log.delete(&Settings::OFF, "me", "ann", false, "1"));
        log.edit(&Settings::OFF, "me", "ann", false, "1", "a", "b");
        assert!(log.is_empty());
    }

    #[test]
    fn the_cap_drops_the_oldest_and_clear_empties() {
        let mut log = Log::new(3);
        for id in ["1", "2", "3", "4"] {
            assert!(log.delete(&on(), "me", "ann", false, id));
        }
        assert_eq!(log.len(), 3);
        assert!(!log.is_deleted("1") && log.is_deleted("4"));
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn the_global_log_marks_rows_and_clears() {
        set(on());
        note_me("me");
        assert!(keeps("ann", false, "10"));
        edited("ann", false, "11", "old", "new");
        let mut row = ChatMessage {
            item: "d:10".into(),
            can_reply: true,
            can_react: true,
            ..Default::default()
        };
        mark(&mut row, "10");
        assert!(row.deleted && !row.can_reply && !row.can_react);
        assert_eq!(history("d:11"), ["old"]);
        assert_eq!(len(), 2);
        clear();
        assert_eq!(len(), 0);
        set(Settings::OFF);
        assert!(!keeps("ann", false, "12"));
    }
}
