//! The staff tag plugin: owner, admin and mod tags by names in chat. The
//! window turns its settings on and off here; the hook reads who is who
//! from the app.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{Hooks, Tag};
use crate::{application::Application, chat::ChatMessage};

static CROWN: AtomicBool = AtomicBool::new(false);
static ROLE_COLOR: AtomicBool = AtomicBool::new(true);

/// Draws a crown in place of the words.
pub fn set_crown(on: bool) {
    CROWN.store(on, Ordering::Relaxed);
}

/// Draws the tag in the member's role colour, where they have one.
pub fn set_role_color(on: bool) {
    ROLE_COLOR.store(on, Ordering::Relaxed);
}

/// What a member is to a server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rank {
    Owner,
    Admin,
    Mod,
}

impl Rank {
    fn words(self) -> &'static str {
        match self {
            Rank::Owner => "Owner",
            Rank::Admin => "Admin",
            Rank::Mod => "Mod",
        }
    }

    /// The colour a tag has with no role colour: gold, silver, bronze.
    fn fallback(self) -> u32 {
        match self {
            Rank::Owner => 0xfaa61a,
            Rank::Admin => 0xaaa9ad,
            Rank::Mod => 0xb88954,
        }
    }
}

const ADMINISTRATOR: u64 = 1 << 3;

/// What makes a moderator: moderating or managing the server, its channels,
/// roles, messages, threads or events, or members in voice.
#[cfg(feature = "discord")]
const MODERATING: u64 = {
    use yapfast_discord::ready::permission::*;
    KICK_MEMBERS
        | BAN_MEMBERS
        | MANAGE_CHANNELS
        | MANAGE_GUILD
        | MANAGE_MESSAGES
        | MANAGE_ROLES
        | MANAGE_EVENTS
        | MANAGE_THREADS
        | MODERATE_MEMBERS
        | MUTE_MEMBERS
        | DEAFEN_MEMBERS
        | MOVE_MEMBERS
};
#[cfg(not(feature = "discord"))]
const MODERATING: u64 = 0;

/// A member's rank from whether they own the server and what it lets them
/// do; `None` for an ordinary member.
pub fn rank(owner: bool, permissions: u64) -> Option<Rank> {
    if owner {
        Some(Rank::Owner)
    } else if permissions & ADMINISTRATOR != 0 {
        Some(Rank::Admin)
    } else if permissions & MODERATING != 0 {
        Some(Rank::Mod)
    } else {
        None
    }
}

/// The tag a rank gets, for a member whose name shows in `color`.
pub fn tag(rank: Rank, color: Option<u32>) -> Tag {
    let color = if ROLE_COLOR.load(Ordering::Relaxed) {
        color.unwrap_or_else(|| rank.fallback())
    } else {
        rank.fallback()
    };
    Tag {
        text: rank.words(),
        crown: CROWN.load(Ordering::Relaxed),
        color: Some(color),
    }
}

/// The plugin's hook: its tags in a server's chat.
pub struct Staff;

impl Hooks for Staff {
    fn id(&self) -> &'static str {
        "staff-tag"
    }

    #[cfg_attr(not(feature = "discord"), allow(unused_variables))]
    fn extras(&self, message: &ChatMessage, app: &Application) -> Vec<Tag> {
        #[cfg(feature = "discord")]
        {
            let (Some(place), Some(user)) = (
                app.state.selected.as_deref(),
                crate::discord::sender_user(&message.sender),
            ) else {
                return Vec::new();
            };
            if let Some((owner, permissions)) = app.member_standing(place, user)
                && let Some(rank) = rank(owner, permissions)
            {
                return vec![tag(rank, message.color)];
            }
        }
        Vec::new()
    }
}

#[cfg(all(test, feature = "discord"))]
mod tests {
    use super::*;
    use crate::application::member_server;

    fn said_by(user: &str) -> ChatMessage {
        ChatMessage {
            sender: crate::discord::sender_id(user),
            ..ChatMessage::default()
        }
    }

    #[test]
    fn ranks_come_from_ownership_and_permissions() {
        assert_eq!(rank(true, 0), Some(Rank::Owner));
        assert_eq!(rank(false, ADMINISTRATOR), Some(Rank::Admin));
        assert_eq!(rank(false, 1 << 1), Some(Rank::Mod));
        assert_eq!(rank(false, 1 << 40), Some(Rank::Mod));
        assert_eq!(rank(false, 1 << 11), None);
    }

    #[test]
    fn owner_admin_and_mod_get_a_tag_and_others_none() {
        let app = member_server(&["30"]);
        // Owen owns the server, Gus holds Wardens (Administrator), Alice
        // holds Staff (kick, ban, time out); Dave holds Crew, which can't.
        let tagged = |user: &str| {
            Staff
                .extras(&said_by(user), &app)
                .first()
                .map(|tag| tag.text)
        };
        assert_eq!(tagged("99"), Some("Owner"));
        assert_eq!(tagged("6"), Some("Admin"));
        assert_eq!(tagged("1"), Some("Mod"));
        assert_eq!(tagged("4"), None);
        assert_eq!(tagged("nobody"), None);
    }

    #[test]
    fn the_tag_wears_the_role_colour_or_its_own_and_the_crown_on_ask() {
        let app = member_server(&["30"]);
        let mut message = said_by("99");
        message.color = Some(0x3498db);
        set_role_color(true);
        set_crown(false);
        let plain = Staff.extras(&message, &app);
        assert_eq!(plain[0].color, Some(0x3498db));
        assert!(!plain[0].crown);
        set_role_color(false);
        set_crown(true);
        let gold = Staff.extras(&message, &app);
        set_role_color(true);
        set_crown(false);
        assert_eq!(gold[0].color, Some(0xfaa61a));
        assert!(gold[0].crown);
    }
}
