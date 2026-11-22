//! Copy profile link: a row in a Discord user's menu that copies the link to
//! their profile.

use super::{Hooks, Picked, discord_user};

pub struct CopyUserUrl;

impl Hooks for CopyUserUrl {
    fn id(&self) -> &'static str {
        "copy-user-urls"
    }

    fn person_menu(&self, user: &str) -> Vec<(&'static str, &'static str)> {
        discord_user(user).map_or_else(Vec::new, |_| vec![("copy", "Copy profile link")])
    }

    fn person_pick(&self, action: &str, user: &str) -> Picked {
        Picked {
            copy: discord_user(user)
                .filter(|_| action == "copy")
                .map(|id| format!("https://discord.com/users/{id}")),
            ..Picked::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halocord::{person_menu, person_pick, plug};

    #[test]
    fn the_row_copies_a_discord_users_profile_link() {
        assert!(person_menu("123").is_empty());
        plug(&CopyUserUrl, true);
        assert_eq!(
            person_menu("123"),
            [("copy-user-urls/copy".to_owned(), "Copy profile link")]
        );
        assert_eq!(
            person_pick("copy-user-urls/copy", "123").copy.as_deref(),
            Some("https://discord.com/users/123")
        );
        plug(&CopyUserUrl, false);
        assert!(person_pick("copy-user-urls/copy", "123").copy.is_none());
    }

    #[test]
    fn other_networks_people_get_no_link() {
        plug(&CopyUserUrl, true);
        assert!(person_menu("@sam:matrix.org").is_empty());
        assert!(
            person_pick("copy-user-urls/copy", "@sam:matrix.org")
                .copy
                .is_none()
        );
    }
}
