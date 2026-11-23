//! Mention: a row in someone's message menu that puts a mention of its author
//! in the message box. Your own messages and other networks' get no row. The
//! window can add the same as a button in the message's hover bar ([`author`]
//! says whose).

use super::{Hooks, Picked};
use crate::{application::Action, chat::ChatMessage, discord};

pub struct QuickMention;

/// The Discord user who sent `message`, when it is someone else's.
pub fn author(message: &ChatMessage) -> Option<&str> {
    if message.own {
        return None;
    }
    discord::sender_user(&message.sender).filter(|user| !user.is_empty())
}

impl Hooks for QuickMention {
    fn id(&self) -> &'static str {
        "quick-mention"
    }

    fn menu(&self, message: &ChatMessage) -> Vec<(&'static str, &'static str)> {
        author(message).map_or_else(Vec::new, |_| vec![("mention", "Mention")])
    }

    fn pick(&self, action: &str, message: &ChatMessage) -> Picked {
        match author(message).filter(|_| action == "mention") {
            Some(user) => Picked {
                actions: vec![Action::MentionPerson(user.to_owned())],
                ..Picked::default()
            },
            None => Picked::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halocord::{menu, pick, plug};

    fn from(sender: &str, own: bool) -> ChatMessage {
        ChatMessage {
            sender: sender.into(),
            own,
            ..Default::default()
        }
    }

    #[test]
    fn someone_elses_discord_message_gets_a_mention_row() {
        let message = from("discord:42", false);
        assert!(menu(&message).is_empty());
        plug(&QuickMention, true);
        assert_eq!(
            menu(&message),
            [("quick-mention/mention".to_owned(), "Mention")]
        );
        let picked = pick("quick-mention/mention", &message);
        assert!(matches!(
            picked.actions.as_slice(),
            [Action::MentionPerson(user)] if user == "42"
        ));
        plug(&QuickMention, false);
        assert!(pick("quick-mention/mention", &message).actions.is_empty());
    }

    #[test]
    fn your_own_and_other_networks_messages_get_no_row() {
        plug(&QuickMention, true);
        assert!(menu(&from("discord:42", true)).is_empty());
        assert!(menu(&from("@sam:matrix.org", false)).is_empty());
        assert!(menu(&from("", false)).is_empty());
        plug(&QuickMention, false);
    }
}
