//! Fake DMs' dialog: who says it, what it says and when, then Add. The
//! message exists only on this computer (`yapfast_core::halocord::fake_dms`).
//! The chat's own button opens it ([`super::fake_button`]).

use iced::{
    Element, Task,
    widget::{operation, text},
};
use yapfast_core::{
    application::{Action, AppState},
    halocord::{fake_dms, message_logger},
};

use crate::{
    app::Message,
    controls::{self, Kind, action},
    ctx::{Ctx, Cx},
    dialog::buttons,
    fonts::BOLD,
    form,
};

const WIDTH: f32 = 440.0;
const BODY: &str = "halocord-fake-body";
const WHEN: &str = "halocord-fake-when";
const MEMBER: &str = "halocord-fake-member";

/// What's typed in the dialog while it's open.
#[derive(Default)]
pub struct FakeDialog {
    open: bool,
    who: usize,
    member: String,
    body: String,
    when: String,
    error: String,
}

#[derive(Clone, Debug)]
pub enum FakeEvent {
    Open,
    Close,
    Who(usize),
    Member(String),
    Body(String),
    When(String),
    Add,
}

/// Someone who may be the author of a fake message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub username: Option<String>,
}

/// Who a fake message can be from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Speaker {
    You,
    Other,
    Named,
}

fn speakers(direct: bool) -> Vec<(Speaker, &'static str)> {
    let mut all = vec![(Speaker::You, "You")];
    if direct {
        all.push((Speaker::Other, "The other person"));
    }
    all.push((Speaker::Named, "Someone by name"));
    all
}

fn direct(state: &AppState) -> bool {
    state
        .selected
        .as_deref()
        .and_then(|id| state.rooms.iter().find(|room| room.id == id))
        .is_some_and(|room| room.direct)
}

/// The people the open chat knows: its members, then those who wrote in
/// what's loaded.
pub fn people(state: &AppState) -> Vec<Person> {
    let mut found: Vec<Person> = Vec::new();
    if let Some(Ok(members)) = &state.members {
        for member in members.iter() {
            if member.id.is_empty() || found.iter().any(|each| each.id == member.id) {
                continue;
            }
            found.push(Person {
                id: member.id.clone(),
                name: member.name.clone(),
                username: member.username.clone(),
            });
        }
    }
    for message in state.messages.iter() {
        let Some(id) = yapfast_core::discord::sender_user(&message.sender) else {
            continue;
        };
        if message.author.is_empty() || found.iter().any(|each| each.id == id) {
            continue;
        }
        found.push(Person {
            id: id.to_owned(),
            name: message.author.clone(),
            username: None,
        });
    }
    found
}

/// This account, as a person to write as.
fn you(state: &AppState) -> Option<Person> {
    let id = message_logger::me();
    if id.is_empty() {
        return None;
    }
    Some(Person {
        name: state
            .account_name
            .clone()
            .unwrap_or_else(|| "You".to_owned()),
        username: state.account_username.clone(),
        id,
    })
}

/// The person that `who` and the typed name pick, or why none.
fn resolve(state: &AppState, who: Speaker, typed: &str) -> Result<Person, String> {
    let me = message_logger::me();
    match who {
        Speaker::You => {
            you(state).ok_or_else(|| "Open the chat first so Yapfast knows who you are.".to_owned())
        }
        Speaker::Other => people(state)
            .into_iter()
            .find(|person| person.id != me)
            .ok_or_else(|| "Yapfast does not know who is in this chat yet.".to_owned()),
        Speaker::Named => {
            let typed = typed.trim().to_lowercase();
            if typed.is_empty() {
                return Err("Type the name of someone in this chat.".to_owned());
            }
            let all = people(state);
            let names = |person: &Person| {
                [
                    Some(person.name.to_lowercase()),
                    person.username.as_deref().map(str::to_lowercase),
                ]
            };
            let same = |person: &Person| names(person).into_iter().flatten().any(|n| n == typed);
            let starts = |person: &Person| {
                names(person)
                    .into_iter()
                    .flatten()
                    .any(|n| n.starts_with(&typed))
            };
            let has = |person: &Person| {
                names(person)
                    .into_iter()
                    .flatten()
                    .any(|n| n.contains(&typed))
            };
            all.iter()
                .find(|person| same(person))
                .or_else(|| all.iter().find(|person| starts(person)))
                .or_else(|| all.iter().find(|person| has(person)))
                .cloned()
                .ok_or_else(|| "No one here goes by that name.".to_owned())
        }
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

impl FakeDialog {
    /// Opens it over the chat that's open.
    pub fn open(&mut self) -> Task<Message> {
        *self = FakeDialog {
            open: true,
            ..FakeDialog::default()
        };
        operation::focus(BODY)
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        *self = FakeDialog::default();
    }

    pub fn update(&mut self, event: FakeEvent, cx: &mut Cx<'_>) -> Task<Message> {
        match event {
            FakeEvent::Open => return self.open(),
            FakeEvent::Close => self.close(),
            FakeEvent::Who(index) => {
                self.who = index;
                self.error.clear();
            }
            FakeEvent::Member(text) => {
                self.member = text;
                self.error.clear();
            }
            FakeEvent::Body(text) => {
                self.body = text;
                self.error.clear();
            }
            FakeEvent::When(text) => {
                self.when = text;
                self.error.clear();
            }
            FakeEvent::Add => return self.add(cx),
        }
        Task::none()
    }

    fn add(&mut self, cx: &mut Cx<'_>) -> Task<Message> {
        let state = cx.state();
        let Some(room) = state.selected.clone() else {
            return Task::none();
        };
        let list = speakers(direct(state));
        let who = list.get(self.who).map_or(Speaker::You, |(who, _)| *who);
        let body = self.body.trim().to_owned();
        let at = if self.when.trim().is_empty() {
            Some(now())
        } else {
            fake_dms::parse_when(&self.when, now())
        };
        let person = resolve(state, who, &self.member);
        self.error = if body.is_empty() {
            "Write what it says.".to_owned()
        } else if let Err(why) = &person {
            why.clone()
        } else if at.is_none() {
            "Try now, 5m ago, 2h ago, 14:30 or 2026-11-20 14:30.".to_owned()
        } else {
            String::new()
        };
        let (Ok(person), Some(at), false) = (person, at, body.is_empty()) else {
            return Task::none();
        };
        fake_dms::add(&room, &person.id, &person.name, &body, at);
        cx.dispatch(Action::Republish);
        self.close();
        Task::none()
    }

    /// Escape closes it.
    pub fn key(&mut self, named: Option<iced::keyboard::key::Named>) -> Task<Message> {
        if named == Some(iced::keyboard::key::Named::Escape) {
            self.close();
        }
        Task::none()
    }
}

fn message(event: FakeEvent) -> Message {
    Message::FakeDm(event)
}

/// The dialog, while it's open.
pub fn view<'a>(dialog: &'a FakeDialog, cx: Ctx<'a>) -> Option<Element<'a, Message>> {
    if !dialog.open {
        return None;
    }
    let theme = cx.look.theme;
    let caps = |words: &'static str| text(words).size(12).font(BOLD).color(theme.muted);
    let list = speakers(direct(cx.state));
    let chosen = dialog.who.min(list.len() - 1);
    let names: Vec<&str> = list.iter().map(|(_, name)| *name).collect();
    let mut body: Vec<Element<'a, Message>> = vec![
        cx.look
            .dialog_head("Add a fake message", message(FakeEvent::Close)),
        form::note(
            theme,
            "It stays on this computer. Nothing is sent to Discord.",
        ),
        caps("WHO SAYS IT").into(),
        controls::choices_within(theme, &names, chosen, 400.0, |index| {
            message(FakeEvent::Who(index))
        }),
    ];
    if list[chosen].0 == Speaker::Named {
        body.push(
            form::field(theme, MEMBER, "A name in this chat", &dialog.member)
                .padding(10)
                .on_input(|text| message(FakeEvent::Member(text)))
                .on_submit(message(FakeEvent::Add))
                .into(),
        );
        if !dialog.member.trim().is_empty() {
            body.push(match resolve(cx.state, Speaker::Named, &dialog.member) {
                Ok(person) => form::note(theme, format!("It will be from {}.", person.name)),
                Err(why) => form::note_in(why, theme.secondary),
            });
        }
    }
    body.push(caps("MESSAGE").into());
    body.push(
        form::field(theme, BODY, "What does it say?", &dialog.body)
            .padding(10)
            .on_input(|text| message(FakeEvent::Body(text)))
            .on_submit(message(FakeEvent::Add))
            .into(),
    );
    body.push(caps("WHEN").into());
    body.push(
        form::field(theme, WHEN, "now", &dialog.when)
            .padding(10)
            .on_input(|text| message(FakeEvent::When(text)))
            .on_submit(message(FakeEvent::Add))
            .into(),
    );
    body.push(form::note(
        theme,
        "Try 5m ago, 2h ago, 14:30 or 2026-11-20 14:30.",
    ));
    if !dialog.error.is_empty() {
        body.push(form::note_in(dialog.error.clone(), theme.danger));
    }
    body.push(buttons(vec![
        action(
            theme,
            "Cancel",
            Kind::Ghost,
            Some(message(FakeEvent::Close)),
        )
        .into(),
        action(theme, "Add", Kind::Primary, Some(message(FakeEvent::Add))).into(),
    ]));
    Some(cx.look.framed(body, WIDTH, message(FakeEvent::Close)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dm_offers_the_other_person_and_a_channel_does_not() {
        assert_eq!(speakers(true).len(), 3);
        assert_eq!(speakers(false).len(), 2);
        assert!(
            speakers(false)
                .iter()
                .all(|(who, _)| *who != Speaker::Other)
        );
    }
}
