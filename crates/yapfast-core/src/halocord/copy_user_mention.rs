//! Copy mention: a row in a Discord user's menu that copies their mention,
//! the text that pings them when it is sent.

use super::{Hooks, Picked, discord_user};

pub struct CopyUserMention;

impl Hooks for CopyUserMention {
    fn id(&self) -> &'static str {
        "copy-user-mention"
    }

    fn person_menu(&self, user: &str) -> Vec<(&'static str, &'static str)> {
        discord_user(user).map_or_else(Vec::new, |_| vec![("copy", "Copy mention")])
    }

    fn person_pick(&self, action: &str, user: &str) -> Picked {
        Picked {
            copy: discord_user(user)
                .filter(|_| action == "copy")
                .map(|id| format!("<@{id}>")),
            ..Picked::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halocord::{person_menu, person_pick, plug};

    #[test]
    fn the_row_copies_a_discord_users_mention() {
        assert!(person_menu("123").is_empty());
        plug(&CopyUserMention, true);
        assert_eq!(
            person_menu("123"),
            [("copy-user-mention/copy".to_owned(), "Copy mention")]
        );
        assert_eq!(
            person_pick("copy-user-mention/copy", "123").copy.as_deref(),
            Some("<@123>")
        );
        plug(&CopyUserMention, false);
        assert!(person_pick("copy-user-mention/copy", "123").copy.is_none());
    }

    #[test]
    fn other_networks_people_get_no_row() {
        plug(&CopyUserMention, true);
        assert!(person_menu("@sam:matrix.org").is_empty());
        assert!(person_menu("").is_empty());
        assert!(
            person_pick("copy-user-mention/copy", "@sam:matrix.org")
                .copy
                .is_none()
        );
    }
}
