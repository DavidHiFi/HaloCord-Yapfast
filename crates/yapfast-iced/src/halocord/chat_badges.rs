//! Show badges in chat: a message author's Discord badges by their name,
//! for those whose profile Yapfast already has (opened as a card, a full
//! profile or a DM's panel), as Discord's client keeps the profiles it
//! fetched. Nothing is asked of Discord for it: an author whose profile
//! hasn't been opened shows none. Kept in memory, the last 500.

use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

use iced::{
    Element,
    widget::{row, tooltip},
};
use yapfast_core::{application::AppState, chat::ProfileBadge};

use super::{Kept, Kind, Plugin, Setting, id};
use crate::{app::Message, ctx::Ctx, widgets::Pic};

/// The most profiles' badges kept.
const KEPT: usize = 500;

pub(super) const SHOW_BADGES: Plugin = Plugin {
    id: id::SHOW_BADGES,
    name: "Show badges in chat",
    about: "A message author's Discord badges by their name, once their profile has been opened in Yapfast; nothing more is asked of Discord for it.",
    on_at_first: false,
    settings: &[
        Setting {
            key: "profile",
            heading: "Profile badges",
            note: "HypeSquad, Early Supporter, Active Developer and the like.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
        Setting {
            key: "nitro",
            heading: "The Nitro badge",
            note: "",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
    ],
    apply: |_, _| {},
    hooks: None,
};

/// The badges by user id, and the ids, oldest noted first.
#[derive(Default)]
struct Known {
    badges: HashMap<String, Vec<ProfileBadge>>,
    order: VecDeque<String>,
}

/// As a static, like Hide chat buttons' state: the messages draw from it.
static KNOWN: Mutex<Option<Known>> = Mutex::new(None);

fn known<T>(with: impl FnOnce(&mut Known) -> T) -> T {
    let mut held = KNOWN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    with(held.get_or_insert_with(Known::default))
}

/// Notes the badges of `person`'s profile, the oldest going past [`KEPT`].
fn note(person: &str, badges: &[ProfileBadge]) {
    known(|known| {
        if known.badges.get(person).is_some_and(|had| had == badges) {
            return;
        }
        if known
            .badges
            .insert(person.to_owned(), badges.to_vec())
            .is_none()
        {
            known.order.push_back(person.to_owned());
        }
        while known.order.len() > KEPT {
            if let Some(oldest) = known.order.pop_front() {
                known.badges.remove(&oldest);
            }
        }
    });
}

/// The badges known for `person`.
fn badges(person: &str) -> Vec<ProfileBadge> {
    known(|known| known.badges.get(person).cloned().unwrap_or_default())
}

/// Nitro's badge says since when they've subscribed.
fn nitro(badge: &ProfileBadge) -> bool {
    badge.description.starts_with("Subscriber since")
}

/// Hook, as the window takes in a change: notes the badges of the
/// profiles open now, while the plugin is on.
pub fn note_badges(kept: &Kept, state: &AppState) {
    if !kept.is(id::SHOW_BADGES) {
        return;
    }
    for view in [state.profile.as_ref(), state.panel_profile.as_ref()]
        .into_iter()
        .flatten()
    {
        let profile = &view.profile;
        // Discord user ids are numbers; Matrix ids start with `@`.
        if !profile.id.is_empty() && profile.id.bytes().all(|byte| byte.is_ascii_digit()) {
            note(&profile.id, &profile.badges);
        }
    }
}

/// The badges `person` (a message's sender) shows, as the switches say.
fn shown(kept: &Kept, person: &str) -> Vec<ProfileBadge> {
    if !kept.is(id::SHOW_BADGES) {
        return Vec::new();
    }
    let (profile, nitro_on) = (
        kept.flag(&SHOW_BADGES, &SHOW_BADGES.settings[0]),
        kept.flag(&SHOW_BADGES, &SHOW_BADGES.settings[1]),
    );
    let person = yapfast_core::discord::sender_user(person).unwrap_or(person);
    badges(person)
        .into_iter()
        .filter(|badge| if nitro(badge) { nitro_on } else { profile })
        .collect()
}

/// Hook: the pictures of the badges `person` shows, to ask for with the
/// message's other pictures.
pub fn badge_pictures(kept: &Kept, person: &str) -> Vec<String> {
    shown(kept, person)
        .into_iter()
        .map(|badge| badge.icon)
        .collect()
}

/// Hook: the author's badges on a message, each named when pointed at.
pub fn chat_badges<'a>(cx: Ctx<'a>, person: &str) -> Option<Element<'a, Message>> {
    let each: Vec<Element<'a, Message>> = shown(&cx.layout.halocord, person)
        .into_iter()
        .filter_map(|badge| {
            let picture = cx.picture(&badge.icon, 16, 16, false)?;
            Some(
                tooltip(
                    Pic::square(picture, 16.0),
                    crate::controls::tip(cx.look.theme, badge.description),
                    tooltip::Position::Top,
                )
                .gap(6)
                .into(),
            )
        })
        .collect();
    (!each.is_empty()).then(|| row(each).spacing(2).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn badge(description: &str) -> ProfileBadge {
        ProfileBadge {
            description: description.into(),
            icon: "https://cdn.discordapp.com/badge-icons/x.png".into(),
            link: None,
            level: None,
        }
    }

    #[test]
    fn keeps_the_badges_of_profiles_seen_the_newest_500() {
        note(
            "chat-badges-1",
            &[
                badge("Subscriber since 1 Jan 2020"),
                badge("HypeSquad Bravery"),
            ],
        );
        assert_eq!(badges("chat-badges-1").len(), 2);
        assert!(nitro(&badges("chat-badges-1")[0]));
        assert!(!nitro(&badges("chat-badges-1")[1]));
        for index in 0..KEPT {
            note(
                &format!("chat-badges-other-{index}"),
                &[badge("Active Developer")],
            );
        }
        assert!(badges("chat-badges-1").is_empty(), "the oldest went");
        assert_eq!(badges(&format!("chat-badges-other-{}", KEPT - 1)).len(), 1);
    }
}
