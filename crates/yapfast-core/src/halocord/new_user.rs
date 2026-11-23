//! The new user tag plugin: a small tag by the names of accounts made in the
//! last few days. An account's age is in its id: Discord's ids are snowflakes
//! whose top bits count the milliseconds since the start of 2015.

use std::sync::atomic::{AtomicU32, Ordering};

use super::{Hooks, Tag};
use crate::{application::Application, chat::ChatMessage};

/// How many days an account counts as new, unless changed.
pub const DEFAULT_DAYS: u32 = 7;
/// The days the plugin's page offers.
pub const DAYS: [u32; 3] = [7, 14, 30];

static NEWER_THAN: AtomicU32 = AtomicU32::new(DEFAULT_DAYS);

/// Sets how many days old an account may be and still be tagged.
pub fn set_days(days: u32) {
    NEWER_THAN.store(days, Ordering::Relaxed);
}

/// Milliseconds from the Unix epoch to the start of Discord's.
const DISCORD_EPOCH: u64 = 1_420_070_400_000;
const DAY: u64 = 86_400_000;

/// When the account with this Discord id was made, in milliseconds from the
/// Unix epoch.
pub fn made_at(user: &str) -> Option<u64> {
    let id: u64 = user.parse().ok()?;
    Some((id >> 22) + DISCORD_EPOCH)
}

/// Whether the account with this id is under `days` old at `now`
/// (milliseconds from the Unix epoch). A clock that's behind counts as new.
pub fn young(user: &str, now: u64, days: u32) -> bool {
    made_at(user).is_some_and(|made| now.saturating_sub(made) < u64::from(days) * DAY)
}

/// How many whole days old the account with this id is at `now`, when it's
/// new enough for the tag (younger than the days the plugin is set to).
pub fn new_for_days(user: &str, now: u64) -> Option<u64> {
    let made = made_at(user)?;
    young(user, now, NEWER_THAN.load(Ordering::Relaxed)).then(|| now.saturating_sub(made) / DAY)
}

/// The tag: amber, as a warning to take a second look.
pub const TAG: Tag = Tag {
    text: "NEW",
    crown: false,
    color: Some(0xf0b232),
};

/// The plugin's hook: its tag by new accounts' names in chat.
pub struct NewUsers;

impl Hooks for NewUsers {
    fn id(&self) -> &'static str {
        "new-user-tag"
    }

    fn extras(&self, message: &ChatMessage, _app: &Application) -> Vec<Tag> {
        if message.bot {
            return Vec::new();
        }
        let Some(user) = crate::discord::sender_user(&message.sender) else {
            return Vec::new();
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_millis() as u64);
        if young(user, now, NEWER_THAN.load(Ordering::Relaxed)) {
            vec![TAG]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The id of an account made at `ms` from the Unix epoch.
    fn id_at(ms: u64) -> String {
        ((ms - DISCORD_EPOCH) << 22).to_string()
    }

    #[test]
    fn an_id_says_when_the_account_was_made() {
        let at = 1_700_000_000_000;
        assert_eq!(made_at(&id_at(at)), Some(at));
        assert_eq!(made_at("not a number"), None);
    }

    #[test]
    fn only_accounts_under_the_days_are_young() {
        let now = 1_800_000_000_000;
        let aged = |days: u64| id_at(now - days * DAY);
        assert!(young(&aged(3), now, 7));
        assert!(!young(&aged(8), now, 7));
        assert!(young(&aged(8), now, 14));
        assert!(!young(&aged(31), now, 30));
        assert!(!young("junk", now, 30));
        // A clock that's behind the id's time doesn't hide the tag.
        assert!(young(&id_at(now + DAY), now, 7));
    }

    #[test]
    fn the_hook_tags_young_people_but_not_apps() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let said_by = |id: String, bot: bool| ChatMessage {
            sender: crate::discord::sender_id(&id),
            bot,
            ..ChatMessage::default()
        };
        let app = Application::default();
        set_days(7);
        let tags = |message| NewUsers.extras(&message, &app);
        assert_eq!(tags(said_by(id_at(now - DAY), false)), [TAG]);
        assert!(tags(said_by(id_at(now - DAY), true)).is_empty());
        assert!(tags(said_by(id_at(now - 400 * DAY), false)).is_empty());
        set_days(DEFAULT_DAYS);
    }
}
