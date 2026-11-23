//! Song link's buttons: under a message with a song's link, "Song links"
//! asks song.link's API where the song is on each service, only when it's
//! pressed; then each service has a button, opening its page or, with
//! "Open in the apps" on, its desktop app. The answers are kept by link
//! until Yapfast ends, the last 64 songs asked about, so a song asked for
//! once shows its buttons wherever it's sent again.

use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

use iced::{
    Element, Task,
    widget::{row, text},
};
use yapfast_core::{
    chat::ChatMessage,
    yapfast_discord::halocord::{SongPlatform, song_platforms},
};

use super::{Event, id, menus};
use crate::{
    app::{App, Message},
    controls::{Kind, action},
    ctx::Ctx,
};

/// What song.link said of a link, or that it's being asked.
#[derive(Clone)]
enum Asked {
    Waiting,
    Found(Vec<SongPlatform>),
    Failed(String),
}

/// How many songs' answers are kept; the one asked about longest ago goes
/// first.
const KEPT: usize = 64;

/// song.link's answers by link, at most [`KEPT`] of them.
#[derive(Default)]
struct Answers {
    by_link: HashMap<String, Asked>,
    /// The links, asked about longest ago first.
    order: VecDeque<String>,
}

impl Answers {
    fn get(&self, link: &str) -> Option<&Asked> {
        self.by_link.get(link)
    }

    /// Keeps `answer` for `link`, letting the oldest go past [`KEPT`].
    fn put(&mut self, link: String, answer: Asked) {
        if self.by_link.insert(link.clone(), answer).is_none() {
            self.order.push_back(link);
        }
        while self.order.len() > KEPT {
            if let Some(oldest) = self.order.pop_front() {
                self.by_link.remove(&oldest);
            }
        }
    }
}

/// The answers, as a static like Hide chat buttons' state: the window
/// draws the buttons from it, and the answers come back to it.
static ASKED: Mutex<Option<Answers>> = Mutex::new(None);

fn asked<T>(with: impl FnOnce(&mut Answers) -> T) -> T {
    let mut held = ASKED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    with(held.get_or_insert_with(Answers::default))
}

/// A service's name, by song.link's key for it.
fn service(key: &str) -> &str {
    match key {
        "spotify" => "Spotify",
        "appleMusic" => "Apple Music",
        "itunes" => "iTunes",
        "youtube" => "YouTube",
        "youtubeMusic" => "YouTube Music",
        "tidal" => "TIDAL",
        "deezer" => "Deezer",
        "amazonMusic" => "Amazon Music",
        "amazonStore" => "Amazon",
        "soundcloud" => "SoundCloud",
        "pandora" => "Pandora",
        "napster" => "Napster",
        "yandex" => "Yandex Music",
        "anghami" => "Anghami",
        "boomplay" => "Boomplay",
        "audiomack" => "Audiomack",
        "audius" => "Audius",
        "bandcamp" => "Bandcamp",
        "google" => "Google",
        "googleStore" => "Google Play",
        other => other,
    }
}

/// The link the buttons are for in `message`: its first song link, without
/// Spotify's share id.
fn song(message: &ChatMessage) -> Option<String> {
    menus::song_links(&message.body)
        .first()
        .map(|link| menus::without_share_id(link))
}

/// Hook: the buttons under `message`, or the one that asks for them, while
/// Song link shows them and the message has a song.
pub fn buttons<'a>(cx: Ctx<'a>, message: &ChatMessage) -> Option<Element<'a, Message>> {
    let kept = &cx.layout.halocord;
    if message.deleted
        || !kept.is(id::SONG_LINK)
        || !kept.flag(&menus::SONG_LINK, &menus::SONG_LINK.settings[1])
    {
        return None;
    }
    let link = song(message)?;
    let theme = cx.look.theme;
    let ask = |label: String| -> Element<'a, Message> {
        action(
            theme,
            label,
            Kind::Ghost,
            Some(Message::HaloCord(Event::SongLinks(link.clone()))),
        )
        .small()
        .into()
    };
    let shown = match asked(|asked| asked.get(&link).cloned()) {
        None => ask("Song links".into()),
        Some(Asked::Waiting) => text("Asking song.link…").size(12).color(theme.muted).into(),
        Some(Asked::Failed(error)) => ask(format!("song.link: {error}. Ask again")),
        Some(Asked::Found(found)) if found.is_empty() => text("song.link knows no service for it.")
            .size(12)
            .color(theme.muted)
            .into(),
        Some(Asked::Found(found)) => {
            let apps = kept.flag(&menus::SONG_LINK, &menus::SONG_LINK.settings[2]);
            let each: Vec<Element<'a, Message>> = found
                .into_iter()
                .map(|found| {
                    let address = match found.native {
                        Some(app) if apps => app,
                        _ => found.url,
                    };
                    action(
                        theme,
                        service(&found.key).to_owned(),
                        Kind::Plain,
                        Some(Message::HaloCord(Event::OpenSong(address))),
                    )
                    .small()
                    .into()
                })
                .collect();
            row(each).spacing(6).wrap().into()
        }
    };
    Some(shown)
}

/// "Song links" pressed: song.link asked for `link`.
pub fn ask(app: &App, link: String) -> Task<Message> {
    let country = app
        .layout
        .halocord
        .text(&menus::SONG_LINK, &menus::SONG_LINK.settings[0])
        .to_owned();
    asked(|asked| asked.put(link.clone(), Asked::Waiting));
    let asking = link.clone();
    let task = app
        .client
        .runtime()
        .spawn(async move { song_platforms(&asking, &country).await });
    Task::perform(task, move |answer| {
        let found = answer.unwrap_or_else(|error| Err(error.to_string()));
        Message::HaloCord(Event::SongLinksFound(link, found))
    })
}

/// song.link's answer, kept for its link.
pub fn found(link: String, found: Result<Vec<SongPlatform>, String>) {
    let answer = match found {
        Ok(found) => Asked::Found(found),
        Err(error) => Asked::Failed(error),
    };
    asked(|asked| asked.put(link, answer));
}

/// Whether song.link's answer for `link` is kept, for the tests.
#[cfg(test)]
pub fn known(link: &str) -> Option<usize> {
    asked(|asked| match asked.get(link) {
        Some(Asked::Found(found)) => Some(found.len()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_latest_answers_only() {
        let mut answers = Answers::default();
        for at in 0..=KEPT {
            answers.put(format!("song-{at}"), Asked::Found(Vec::new()));
        }
        assert!(answers.get("song-0").is_none(), "the oldest went");
        assert!(answers.get("song-1").is_some() && answers.get(&format!("song-{KEPT}")).is_some());
        assert_eq!(answers.by_link.len(), KEPT);
        // An answer to a link kept already doesn't count twice.
        answers.put("song-1".into(), Asked::Waiting);
        assert_eq!(answers.order.len(), KEPT);
    }
}
