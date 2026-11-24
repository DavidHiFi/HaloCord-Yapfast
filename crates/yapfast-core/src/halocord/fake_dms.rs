//! The Fake DMs plugin: messages that exist only on this computer, added to
//! any chat. They sit in the timeline in time order like real ones, stay
//! across restarts in `halocord-fake-dms.json` in Yapfast's data folder, and
//! are never sent to Discord: no reply, reaction, edit or forward of one
//! leaves the app ([`FakeDms::command`] holds them back).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};

use super::{Hooks, Picked, Tag};
use crate::{
    application::{Action, Application, Effect},
    chat::{ChatMessage, Command, DiscordCommand},
};

/// The file in the data folder.
pub const FILE: &str = "halocord-fake-dms.json";
/// The most fake messages kept in all.
pub const MOST: usize = 500;
/// What a fake message's timeline item starts with.
pub const PREFIX: &str = "fake:";

/// One fake message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fake {
    pub id: u64,
    /// The chat's room id, such as `discord:@me:123`.
    pub room: String,
    /// The Discord user it says it is from.
    pub author: String,
    /// The name shown when Yapfast knows none for `author`.
    pub name: String,
    pub body: String,
    /// When it says it was sent, in Unix seconds.
    pub at: u64,
}

impl Fake {
    /// The timeline item of this message.
    pub fn item(&self) -> String {
        format!("{PREFIX}{}", self.id)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Store {
    next: u64,
    items: Vec<Fake>,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Store {
    fn open(root: &Path) -> Self {
        let path = root.join(FILE);
        let mut store: Store = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        store.next = store
            .next
            .max(store.items.iter().map(|fake| fake.id).max().unwrap_or(0) + 1);
        store.path = Some(path);
        store
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        match serde_json::to_vec_pretty(self) {
            Ok(bytes) => {
                if let Err(error) = std::fs::write(path, bytes) {
                    tracing::warn!(%error, "could not keep the fake messages");
                }
            }
            Err(error) => tracing::warn!(%error, "could not write the fake messages"),
        }
    }
}

struct Held {
    store: Store,
    local_mark: bool,
    /// The plugin is on: its messages show.
    on: bool,
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
        with(guard.get_or_insert_with(|| Held {
            store: Store {
                next: 1,
                ..Store::default()
            },
            local_mark: true,
            on: false,
        }))
    };
    #[cfg(not(test))]
    return run(&HELD);
    #[cfg(test)]
    return HELD.with(run);
}

/// Reads the fake messages kept in the data folder, and keeps them there
/// from now on.
pub fn open(root: &Path) {
    let store = Store::open(root);
    held(|held| held.store = store);
}

/// Whether the plugin is on; off, no fake message shows.
pub fn set_on(on: bool) {
    held(|held| held.on = on);
}

/// Whether fake messages wear a small "Local" mark.
pub fn set_local_mark(on: bool) {
    held(|held| held.local_mark = on);
}

/// Whether this timeline item is a fake message.
pub fn is_fake(item: &str) -> bool {
    item.starts_with(PREFIX)
}

fn id_of(item: &str) -> Option<u64> {
    item.strip_prefix(PREFIX)?.parse().ok()
}

/// Adds a fake message and keeps it; its timeline item. The oldest goes
/// when [`MOST`] is passed.
pub fn add(room: &str, author: &str, name: &str, body: &str, at: u64) -> String {
    held(|held| {
        let id = held.store.next.max(1);
        held.store.next = id + 1;
        held.store.items.push(Fake {
            id,
            room: room.to_owned(),
            author: author.to_owned(),
            name: name.to_owned(),
            body: body.to_owned(),
            at,
        });
        while held.store.items.len() > MOST {
            held.store.items.remove(0);
        }
        held.store.save();
        format!("{PREFIX}{id}")
    })
}

/// Removes the fake message with this item; whether there was one.
pub fn remove(item: &str) -> bool {
    let Some(id) = id_of(item) else {
        return false;
    };
    held(|held| {
        let before = held.store.items.len();
        held.store.items.retain(|fake| fake.id != id);
        held.store.save();
        held.store.items.len() != before
    })
}

/// Removes every fake message of a room; how many went.
pub fn clear_room(room: &str) -> usize {
    held(|held| {
        let before = held.store.items.len();
        held.store.items.retain(|fake| fake.room != room);
        held.store.save();
        before - held.store.items.len()
    })
}

/// The room a fake message is in.
pub fn room_of(item: &str) -> Option<String> {
    let id = id_of(item)?;
    held(|held| {
        held.store
            .items
            .iter()
            .find(|fake| fake.id == id)
            .map(|fake| fake.room.clone())
    })
}

/// A room's fake messages, by time.
pub fn in_room(room: &str) -> Vec<Fake> {
    let mut found: Vec<Fake> = held(|held| {
        held.store
            .items
            .iter()
            .filter(|fake| fake.room == room)
            .cloned()
            .collect()
    });
    found.sort_by_key(|fake| (fake.at, fake.id));
    found
}

/// How many fake messages a room has.
pub fn count(room: &str) -> usize {
    held(|held| {
        held.store
            .items
            .iter()
            .filter(|fake| fake.room == room)
            .count()
    })
}

/// Reads when a fake message says it was sent, as Unix seconds, from what
/// was typed, with `now` in Unix seconds: `now`, `5m ago`, `2h ago`, `1d ago`,
/// `-30s`, `+10m`, `14:30` (today), `2026-11-20`, `2026-11-20 14:30`. A bare
/// `5m` counts forward, as the TestCord plugin's does; with "ago" or a minus
/// it counts back.
pub fn parse_when(input: &str, now: u64) -> Option<u64> {
    let text = input.trim().to_lowercase();
    if text.is_empty() {
        return None;
    }
    if text == "now" || text == "0" {
        return Some(now);
    }
    if let Some(at) = relative(&text, now) {
        return Some(at);
    }
    let local = |naive: NaiveDateTime| {
        Local
            .from_local_datetime(&naive)
            .earliest()
            .map(|at| at.timestamp().max(0) as u64)
    };
    // 14:30 or 14:30:15, today.
    for format in ["%H:%M:%S", "%H:%M"] {
        if let Ok(time) = NaiveTime::parse_from_str(&text, format) {
            let today = Local
                .timestamp_opt(i64::try_from(now).ok()?, 0)
                .earliest()?
                .date_naive();
            return local(today.and_time(time));
        }
    }
    for format in [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dt%H:%M:%S",
        "%Y-%m-%dt%H:%M",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&text, format) {
            return local(naive);
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(&text, "%Y-%m-%d") {
        return local(date.and_time(NaiveTime::from_hms_opt(12, 0, 0)?));
    }
    chrono::DateTime::parse_from_rfc3339(&text)
        .ok()
        .map(|at| at.timestamp().max(0) as u64)
}

fn relative(text: &str, now: u64) -> Option<u64> {
    let (text, ago) = match text.strip_suffix(" ago") {
        Some(rest) => (rest.trim_end(), true),
        None => (text, false),
    };
    let unit_at = text.find(|c: char| !c.is_ascii_digit() && c != '+' && c != '-')?;
    let (number, unit) = text.split_at(unit_at);
    let amount: i64 = number.trim().parse().ok()?;
    let seconds = match unit.trim() {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86_400,
        _ => return None,
    };
    let mut change = amount.checked_mul(seconds)?;
    if ago {
        change = -change.abs();
    }
    let now = i64::try_from(now).ok()?;
    Some(now.saturating_add(change).max(0) as u64)
}

/// How the chat shows the person a fake message is from.
pub struct Who {
    pub name: String,
    pub avatar: Option<String>,
    pub color: Option<u32>,
}

/// Adds a room's fake messages to the real rows `rows`, in time order. A
/// fake older than the oldest row loaded is left out until the chat loads
/// back that far, and in a window of old messages those after its last
/// row are too. `who` gives a person's name, avatar and colour from their id
/// and the name kept with the message; `body` gives a text's body.
pub fn merge(
    rows: &mut Vec<Arc<ChatMessage>>,
    room: &str,
    me: &str,
    detached: bool,
    who: &dyn Fn(&str, &str) -> Who,
    body: &dyn Fn(&str) -> (String, Vec<crate::mention::Mentioned>),
) {
    let fakes = if held(|held| held.on) {
        in_room(room)
    } else {
        Vec::new()
    };
    if fakes.is_empty() {
        return;
    }
    let first = rows.first().map(|row| row.timestamp);
    let last = rows.last().map(|row| row.timestamp);
    for fake in fakes {
        if first.is_some_and(|first| fake.at < first) {
            continue;
        }
        if detached && last.is_some_and(|last| fake.at > last) {
            continue;
        }
        let who = who(&fake.author, &fake.name);
        let (text, mentions) = body(&fake.body);
        let row = Arc::new(ChatMessage {
            id: fake.item(),
            item: fake.item(),
            author: who.name,
            sender: crate::discord::sender_id(&fake.author),
            spoilers: true,
            avatar: who.avatar,
            color: who.color,
            body: text,
            mentions,
            formatted: true,
            time: crate::clock::time(fake.at),
            timestamp: fake.at,
            own: fake.author == me,
            delivery: crate::chat::Delivery::Sent,
            ..Default::default()
        });
        let at = rows.partition_point(|row| row.timestamp <= fake.at);
        rows.insert(at, row);
    }
}

/// [`Action::Republish`]: the open chat is drawn again, to show a change to
/// its fake messages.
pub fn republish(app: &Application) -> Vec<Effect> {
    app.state
        .selected
        .clone()
        .map(|room| Effect::Backend(Command::Discord(DiscordCommand::Republish { room })))
        .into_iter()
        .collect()
}

/// The timeline items a command names, for the ones that name a message.
fn items(command: &Command) -> Vec<&str> {
    match command {
        Command::Send { reply, .. } => reply.iter().map(String::as_str).collect(),
        Command::Edit { item, .. }
        | Command::Forward { item, .. }
        | Command::React { item, .. }
        | Command::Reactors { item, .. }
        | Command::Vote { item, .. }
        | Command::EndPoll { item, .. }
        | Command::Voters { item, .. }
        | Command::RemoveReactor { item, .. }
        | Command::Delete { item, .. }
        | Command::Pin { item, .. }
        | Command::Retry { item, .. }
        | Command::Cancel { item, .. }
        | Command::SaveAttachment { item, .. }
        | Command::CopyAttachment { item, .. }
        | Command::ViewImage { item, .. }
        | Command::PlayVideo { item, .. }
        | Command::PlayVoiceMessage { item, .. }
        | Command::MoveVoiceMessage { item, .. }
        | Command::Around { item, .. } => vec![item.as_str()],
        Command::Discord(DiscordCommand::StartThread {
            message: Some(message),
            ..
        }) => vec![message.as_str()],
        Command::Discord(DiscordCommand::DismissEphemeral { item, .. }) => vec![item.as_str()],
        _ => Vec::new(),
    }
}

/// Whether a command names a fake message, and so mustn't go to Discord.
pub fn names_fake(command: &Command) -> bool {
    items(command).into_iter().any(is_fake)
}

/// The plugin's hooks: nothing about a fake message leaves the app, a "Local"
/// mark by its author, and its own menu rows.
pub struct FakeDms;

impl Hooks for FakeDms {
    fn id(&self) -> &'static str {
        "fake-dms"
    }

    fn command(&self, command: &Command) -> bool {
        !names_fake(command)
    }

    fn extras(&self, message: &ChatMessage, _app: &Application) -> Vec<Tag> {
        if is_fake(&message.item) && held(|held| held.local_mark) {
            vec![Tag::words("Local")]
        } else {
            Vec::new()
        }
    }

    fn menu(&self, message: &ChatMessage) -> Vec<(&'static str, &'static str)> {
        if is_fake(&message.item) {
            vec![
                ("remove", "Remove fake message"),
                ("clear", "Clear fake messages here"),
            ]
        } else {
            Vec::new()
        }
    }

    fn pick(&self, action: &str, message: &ChatMessage) -> Picked {
        let done = match action {
            "remove" => remove(&message.item),
            "clear" => room_of(&message.item).is_some_and(|room| clear_room(&room) > 0),
            _ => false,
        };
        Picked {
            actions: if done {
                vec![Action::Republish]
            } else {
                Vec::new()
            },
            copy: None,
            open: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() {
        held(|held| {
            held.store = Store {
                next: 1,
                ..Store::default()
            };
            held.on = true;
        });
    }

    #[test]
    fn fakes_come_back_by_time() {
        fresh();
        add("r", "1", "Ann", "late", 300);
        add("r", "1", "Ann", "early", 100);
        add("other", "1", "Ann", "elsewhere", 200);
        let bodies: Vec<_> = in_room("r").into_iter().map(|fake| fake.body).collect();
        assert_eq!(bodies, ["early", "late"]);
    }

    #[test]
    fn fakes_persist_and_load() {
        fresh();
        let dir = std::env::temp_dir().join(format!("halocord-fakes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        open(&dir);
        let item = add("r", "1", "Ann", "hello", 100);
        add("r", "2", "Bo", "again", 200);
        assert!(remove(&item));
        fresh();
        open(&dir);
        let kept = in_room("r");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].body, "again");
        // A new one does not reuse an id.
        let next = add("r", "1", "Ann", "third", 300);
        assert_ne!(next, item);
        std::fs::remove_dir_all(&dir).ok();
        fresh();
    }

    #[test]
    fn clearing_a_room_leaves_the_others() {
        fresh();
        add("a", "1", "Ann", "x", 1);
        add("a", "1", "Ann", "y", 2);
        add("b", "1", "Ann", "z", 3);
        assert_eq!(clear_room("a"), 2);
        assert_eq!((count("a"), count("b")), (0, 1));
    }

    #[test]
    fn the_oldest_goes_past_the_most() {
        fresh();
        for at in 0..(MOST as u64 + 3) {
            add("r", "1", "Ann", "x", at);
        }
        let all = in_room("r");
        assert_eq!(all.len(), MOST);
        assert_eq!(all[0].at, 3);
    }

    #[test]
    fn nothing_about_a_fake_message_is_sent() {
        let fake = "fake:1".to_owned();
        let room = "discord:@me:5".to_owned();
        let named = [
            Command::Send {
                room: room.clone(),
                body: "hi".into(),
                reply: Some(fake.clone()),
                options: Default::default(),
                files: Vec::new(),
            },
            Command::Edit {
                room: room.clone(),
                item: fake.clone(),
                body: "x".into(),
            },
            Command::React {
                room: room.clone(),
                item: fake.clone(),
                key: "x".into(),
            },
            Command::Delete {
                room: room.clone(),
                item: fake.clone(),
            },
            Command::Forward {
                room: room.clone(),
                item: fake.clone(),
                target: room.clone(),
                note: None,
                silent: false,
            },
            Command::Pin {
                room: room.clone(),
                item: fake.clone(),
                pinned: true,
            },
        ];
        for command in &named {
            assert!(!FakeDms.command(command));
        }
        // Real messages, and reading what is on screen, still go.
        let real = Command::React {
            room: room.clone(),
            item: "d:7".into(),
            key: "x".into(),
        };
        assert!(FakeDms.command(&real));
        let visible = Command::VisibleMessages {
            room: room.clone(),
            items: vec![fake, "d:7".into()],
            at_end: true,
        };
        assert!(FakeDms.command(&visible));
        let plain = Command::Send {
            room,
            body: "hi".into(),
            reply: None,
            options: Default::default(),
            files: Vec::new(),
        };
        assert!(FakeDms.command(&plain));
    }

    #[test]
    fn times_are_read_as_the_original_reads_them() {
        let now = 1_800_000_000;
        assert_eq!(parse_when("now", now), Some(now));
        assert_eq!(parse_when(" Now ", now), Some(now));
        assert_eq!(parse_when("5m ago", now), Some(now - 300));
        assert_eq!(parse_when("2h ago", now), Some(now - 7200));
        assert_eq!(parse_when("1d ago", now), Some(now - 86_400));
        assert_eq!(parse_when("-30s", now), Some(now - 30));
        assert_eq!(parse_when("+10m", now), Some(now + 600));
        assert_eq!(parse_when("5m", now), Some(now + 300));
        assert_eq!(parse_when("", now), None);
        assert_eq!(parse_when("whenever", now), None);
        assert_eq!(parse_when("5x", now), None);
    }

    #[test]
    fn a_clock_time_and_a_date_are_local() {
        let now = 1_800_000_000;
        let today = Local.timestamp_opt(now as i64, 0).unwrap().date_naive();
        let expected = Local
            .from_local_datetime(&today.and_hms_opt(14, 30, 0).unwrap())
            .earliest()
            .unwrap()
            .timestamp() as u64;
        assert_eq!(parse_when("14:30", now), Some(expected));
        let date = Local
            .from_local_datetime(
                &NaiveDate::from_ymd_opt(2026, 11, 20)
                    .unwrap()
                    .and_hms_opt(9, 5, 0)
                    .unwrap(),
            )
            .earliest()
            .unwrap()
            .timestamp() as u64;
        assert_eq!(parse_when("2026-11-20 09:05", now), Some(date));
        assert!(parse_when("2026-11-20", now).is_some());
        assert_eq!(parse_when("25:99", now), None);
    }

    fn real(at: u64) -> Arc<ChatMessage> {
        Arc::new(ChatMessage {
            timestamp: at,
            item: format!("d:{at}"),
            ..Default::default()
        })
    }

    fn merged(rows: Vec<Arc<ChatMessage>>, detached: bool) -> Vec<String> {
        let mut rows = rows;
        merge(
            &mut rows,
            "r",
            "1",
            detached,
            &|_, name| Who {
                name: name.to_owned(),
                avatar: None,
                color: None,
            },
            &|text| (text.to_owned(), Vec::new()),
        );
        rows.iter().map(|row| row.item.clone()).collect()
    }

    #[test]
    fn fakes_sit_among_real_rows_in_time_order() {
        fresh();
        let a = add("r", "1", "Me", "between", 150);
        let b = add("r", "2", "Bo", "after", 500);
        let c = add("r", "2", "Bo", "before everything", 5);
        let items = merged(vec![real(100), real(200)], false);
        assert_eq!(items, ["d:100", a.as_str(), "d:200", b.as_str()]);
        // Older than the oldest loaded row waits for the chat to load back.
        assert!(!items.contains(&c));
        // A window of old messages takes none after its last row.
        let items = merged(vec![real(100), real(200)], true);
        assert_eq!(items, ["d:100", a.as_str(), "d:200"]);
        // An empty chat shows them all.
        assert_eq!(merged(Vec::new(), false).len(), 3);
    }

    #[test]
    fn a_fake_row_is_local_and_cannot_be_acted_on() {
        fresh();
        add("r", "1", "Me", "hi", 100);
        let mut rows = Vec::new();
        merge(
            &mut rows,
            "r",
            "1",
            false,
            &|_, name| Who {
                name: name.to_owned(),
                avatar: None,
                color: None,
            },
            &|text| (text.to_owned(), Vec::new()),
        );
        let row = &rows[0];
        assert!(row.own && !row.can_reply && !row.can_edit && !row.can_delete && !row.can_react);
        assert!(!row.can_forward && !row.can_pin);
        assert_eq!(row.body, "hi");
    }

    #[test]
    fn the_menu_shows_only_on_fakes() {
        fresh();
        let item = add("r", "1", "Me", "hi", 100);
        let fake = ChatMessage {
            item: item.clone(),
            ..Default::default()
        };
        let real = ChatMessage {
            item: "d:5".into(),
            ..Default::default()
        };
        assert_eq!(FakeDms.menu(&fake).len(), 2);
        assert!(FakeDms.menu(&real).is_empty());
        let picked = FakeDms.pick("remove", &fake);
        assert_eq!(picked.actions.len(), 1);
        assert_eq!(count("r"), 0);
    }
}
