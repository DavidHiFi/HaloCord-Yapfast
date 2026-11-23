//! Message click actions' triple click and Backspace click. While either is
//! set to do something, a message's row says each press (the kit's
//! `DoublePress::presses`, on the same terms as a double press: nothing the
//! row shows took it), and the presses are counted here: two act as the
//! plugin's double-click does, three as the triple click is set, and one
//! while Backspace is held as the Backspace click is set. Each action goes
//! through the hover bar's own acts, so it does only what the message
//! allows.

use iced::{Element, Task};

use super::{Kind, Setting, chat::CLICK_ACTIONS, id};
use crate::{
    app::{App, Message},
    ctx::Ctx,
    menus::MenuAct,
    widgets::DoublePress,
};

/// What a click does, as the settings name it.
pub(super) const NOTHING: u32 = 0;
pub(super) const DELETE: u32 = 1;
const EDIT: u32 = 2;
const REPLY: u32 = 3;
const REACT: u32 = 4;
const NAMES: &[&str] = &["Nothing", "Delete", "Edit", "Reply", "React"];
const VALUES: &[u32] = &[NOTHING, DELETE, EDIT, REPLY, REACT];

/// A setting choosing what a click does.
pub(super) const fn choice(
    key: &'static str,
    heading: &'static str,
    unless_changed: u32,
) -> Setting {
    Setting {
        key,
        heading,
        note: "",
        kind: Kind::Choice {
            names: NAMES,
            values: VALUES,
            unless_changed,
        },
    }
}

/// The plugin's settings, by their place (0 is "Edit your own").
const TRIPLE: usize = 1;
const EMOJI: usize = 2;
const BACKSPACE_OWN: usize = 3;
const BACKSPACE_OTHERS: usize = 4;
const WITHIN: usize = 5;
const WAIT_FOR_TRIPLE: usize = 6;
pub(super) const NOT_IN_DMS: usize = 7;

/// The presses counted so far, kept on the window (`App::halocord_clicks`).
#[derive(Default)]
pub struct Clicks {
    /// The message last pressed.
    item: String,
    /// Its presses in a row so far.
    count: u32,
    last_ms: u64,
    /// Counts each press, so a double click's wait knows it's the latest.
    press: u64,
    /// When Backspace was last pressed or repeated, while it's held.
    backspace: Option<u64>,
}

/// Backspace counts as held this long after its last press or repeat, so a
/// release the window never heard (it lost the keys) can't leave it held.
const HELD_FOR: u64 = 1000;

/// Whether a triple click or a Backspace click is set to do something.
fn counts_presses(cx: Ctx<'_>) -> bool {
    let kept = &cx.layout.halocord;
    kept.is(id::CLICK_ACTIONS)
        && [TRIPLE, BACKSPACE_OWN, BACKSPACE_OTHERS]
            .iter()
            .any(|at| kept.value(&CLICK_ACTIONS, &CLICK_ACTIONS.settings[*at]) != NOTHING)
}

/// Hook: message row `index`, as the plugin watches it: the kit's
/// `DoublePress` with the double-click's message, or, while a triple click
/// or a Backspace click is set, saying each press.
pub fn click_row<'a>(
    cx: Ctx<'a>,
    index: usize,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    if !counts_presses(cx) {
        return match super::double_click(cx, index) {
            Some(double) => DoublePress::new(content, double).into(),
            None => content.into(),
        };
    }
    // Not the message being edited: presses in its field are the field's.
    let Some(message) = cx.row_message(index).filter(|message| {
        cx.composer.editing.as_deref() != Some(message.item.as_str())
            && !message.deleted
            && message.delivery == yapfast_core::chat::Delivery::default()
    }) else {
        return content.into();
    };
    let press = super::Event::Press(message.item.clone(), super::chat::double_act(cx, index));
    DoublePress::presses(content, Message::HaloCord(press)).into()
}

/// Backspace pressed (or repeated), or let go; the window losing the keys
/// lets go of it too.
pub(super) fn backspace(app: &mut App, held: bool) {
    let now = yapfast_core::clock::now_ms();
    app.halocord_clicks.backspace = held.then_some(now);
}

/// The keys the plugin listens to while a Backspace click is set: Backspace
/// held and let go, and the window losing the keys.
pub fn click_keys(app: &App) -> Option<iced::Subscription<Message>> {
    let kept = &app.layout.halocord;
    let set = kept.is(id::CLICK_ACTIONS)
        && [BACKSPACE_OWN, BACKSPACE_OTHERS]
            .iter()
            .any(|at| kept.value(&CLICK_ACTIONS, &CLICK_ACTIONS.settings[*at]) != NOTHING);
    set.then(|| {
        iced::event::listen_with(|event, _, _| {
            use iced::keyboard::{Event as Key, Key::Named, key::Named::Backspace};
            let held = match event {
                iced::Event::Keyboard(Key::KeyPressed {
                    key: Named(Backspace),
                    ..
                }) => true,
                iced::Event::Keyboard(Key::KeyReleased {
                    key: Named(Backspace),
                    ..
                })
                | iced::Event::Window(iced::window::Event::Unfocused) => false,
                _ => return None,
            };
            Some(Message::HaloCord(super::Event::Backspace(held)))
        })
    })
}

/// What a press makes of the presses so far.
#[derive(Debug, PartialEq)]
enum Counted {
    Nothing,
    /// Act as set for this many clicks (1: a click with Backspace held).
    Act(u32),
    /// A double click, unless a third click follows within the wait.
    Wait(u64),
}

fn count(clicks: &mut Clicks, item: &str, now: u64, within: u64, wait_for_triple: bool) -> Counted {
    if clicks.item == item && now.saturating_sub(clicks.last_ms) <= within {
        clicks.count += 1;
    } else {
        item.clone_into(&mut clicks.item);
        clicks.count = 1;
    }
    clicks.last_ms = now;
    clicks.press += 1;
    let backspace = clicks
        .backspace
        .is_some_and(|pressed| now.saturating_sub(pressed) <= HELD_FOR);
    match clicks.count {
        1 if backspace => {
            clicks.count = 0;
            Counted::Act(1)
        }
        2 if wait_for_triple => Counted::Wait(clicks.press),
        2 => Counted::Act(2),
        3 => {
            clicks.count = 0;
            Counted::Act(3)
        }
        _ => Counted::Nothing,
    }
}

/// A press on the row of message `item`, whose double-click does `double`.
pub(super) fn press(app: &mut App, item: String, double: Option<MenuAct>) -> Task<Message> {
    let kept = &app.layout.halocord;
    if !kept.is(id::CLICK_ACTIONS) {
        return Task::none();
    }
    let settings = CLICK_ACTIONS.settings;
    let within = u64::from(kept.value(&CLICK_ACTIONS, &settings[WITHIN]));
    // Only worth waiting for a third click that does something.
    let wait = kept.flag(&CLICK_ACTIONS, &settings[WAIT_FOR_TRIPLE])
        && kept.value(&CLICK_ACTIONS, &settings[TRIPLE]) != NOTHING;
    let now = yapfast_core::clock::now_ms();
    match count(&mut app.halocord_clicks, &item, now, within, wait) {
        Counted::Nothing => Task::none(),
        Counted::Act(clicks) => act(app, &item, clicks, double),
        Counted::Wait(press) => Task::perform(
            async move { tokio::time::sleep(std::time::Duration::from_millis(within)).await },
            move |()| Message::HaloCord(super::Event::DoubleClickDue(item.clone(), press, double)),
        ),
    }
}

/// A double click's wait ran out: unless another press came, it acts.
pub(super) fn double_click_due(
    app: &mut App,
    item: &str,
    press: u64,
    double: Option<MenuAct>,
) -> Task<Message> {
    let clicks = &mut app.halocord_clicks;
    let due = clicks.press == press && clicks.item == item && clicks.count == 2;
    if due {
        clicks.count = 0;
    }
    if due {
        act(app, item, 2, double)
    } else {
        Task::none()
    }
}

/// How many presses the plugin has counted, which a double click's wait
/// names.
#[cfg(test)]
pub fn presses_so_far(app: &App) -> u64 {
    app.halocord_clicks.press
}

/// Carries out `clicks` clicks (1 with Backspace held) on message `item`:
/// two as the double-click does, the rest as set, as the hover bar would.
fn act(app: &mut App, item: &str, clicks: u32, double: Option<MenuAct>) -> Task<Message> {
    let state = app.client.state();
    let Some(message) = state.message(item).cloned() else {
        return Task::none();
    };
    let kept = &app.layout.halocord;
    let settings = CLICK_ACTIONS.settings;
    let in_dm = state
        .rooms
        .iter()
        .any(|room| room.direct && state.selected.as_ref() == Some(&room.id));
    if in_dm && kept.flag(&CLICK_ACTIONS, &settings[NOT_IN_DMS]) {
        return Task::none();
    }
    let setting = match (clicks, message.own) {
        (2, _) => {
            return double.map_or_else(Task::none, |double| {
                crate::menus::update(
                    app,
                    crate::menus::Event::Act(double, item.to_owned(), iced::Point::ORIGIN),
                )
            });
        }
        (1, true) => BACKSPACE_OWN,
        (1, false) => BACKSPACE_OTHERS,
        _ => TRIPLE,
    };
    let emoji = kept
        .text(&CLICK_ACTIONS, &settings[EMOJI])
        .trim()
        .to_owned();
    // Deleting someone else's message takes a click set to Delete for
    // others' messages, which none is unless changed: the triple click, set
    // to Delete, deletes only your own.
    let act = match kept.value(&CLICK_ACTIONS, &settings[setting]) {
        DELETE if message.can_delete && (message.own || setting != TRIPLE) => MenuAct::DeleteNow,
        EDIT if message.own && message.can_edit => MenuAct::Edit,
        REPLY if message.can_reply => MenuAct::Reply,
        REACT if message.can_react && !emoji.is_empty() => {
            return crate::menus::update(app, crate::menus::Event::React(item.to_owned(), emoji));
        }
        _ => return Task::none(),
    };
    crate::menus::update(
        app,
        crate::menus::Event::Act(act, item.to_owned(), iced::Point::ORIGIN),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presses_count_into_double_triple_and_backspace_clicks() {
        let mut clicks = Clicks::default();
        assert_eq!(count(&mut clicks, "d:1", 1000, 300, true), Counted::Nothing);
        assert_eq!(count(&mut clicks, "d:1", 1200, 300, true), Counted::Wait(2));
        assert_eq!(count(&mut clicks, "d:1", 1400, 300, true), Counted::Act(3));
        // Too slow, or on another message, starts again.
        assert_eq!(count(&mut clicks, "d:1", 2000, 300, true), Counted::Nothing);
        assert_eq!(count(&mut clicks, "d:2", 2100, 300, true), Counted::Nothing);
        // Not waiting for a third, a double click acts at once.
        assert_eq!(count(&mut clicks, "d:2", 2200, 300, false), Counted::Act(2));
        clicks.backspace = Some(4500);
        assert_eq!(count(&mut clicks, "d:3", 5000, 300, true), Counted::Act(1));
        // A Backspace not pressed again for a while no longer counts.
        assert_eq!(count(&mut clicks, "d:4", 9000, 300, true), Counted::Nothing);
    }
}
