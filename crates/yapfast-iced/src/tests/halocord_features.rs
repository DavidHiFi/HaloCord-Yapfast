//! HaloCord's features beyond the plugins' own pages: one test each.

use super::{preview, unread};
use crate::{
    app::{App, Message},
    halocord::{Change, PLUGINS, changed, id, place},
};

/// Switches a plugin on, as its switch on the page does.
fn turn_on(app: &mut App, id: &str) {
    let plugin = place(id).expect("the plugin");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
}

/// yapfast#285: Alt+Shift+Down goes to the next channel with unread
/// messages, and does nothing when all are read.
#[test]
fn alt_shift_down_goes_to_the_next_unread_channel() {
    let (mut app, _folder) = preview();
    let open = app.client.state().selected.clone();
    let next = app
        .client
        .state()
        .scoped_rooms()
        .find(|room| !room.voice && Some(&room.id) != open.as_ref())
        .map(|room| room.id.clone())
        .expect("another channel");
    let _ = app.update(Message::Switcher(crate::switcher::Event::NextUnread(true)));
    assert_eq!(app.client.state().selected, open, "all read: it stays");
    unread(&mut app, &next, 0);
    let _ = app.update(Message::Switcher(crate::switcher::Event::NextUnread(true)));
    assert_eq!(app.client.state().selected.as_ref(), Some(&next));
}

/// Always trust links: a masked link opens without the question.
#[test]
fn always_trust_links_skips_the_leaving_question() {
    let (mut app, _folder) = preview();
    let masked = format!(
        "{}https://elsewhere.example/page",
        yapfast_core::links::LEAVE
    );
    let _ = crate::links::follow_link(&mut app, &masked, iced::Point::ORIGIN);
    assert!(app.leaving.is_some(), "it asks first");
    app.leaving = None;
    turn_on(&mut app, id::ALWAYS_TRUST);
    assert!(PLUGINS.iter().any(|plugin| plugin.id == id::ALWAYS_TRUST));
    let _ = crate::links::follow_link(&mut app, &masked, iced::Point::ORIGIN);
    assert!(app.leaving.is_none(), "it opens straight away");
}

/// A Go Live key, as the call's button: while live it stops the stream,
/// and out of a call it does nothing.
#[test]
fn the_go_live_key_stops_a_live_stream() {
    use crate::{call::Event, keybinds::go_live_key};
    use yapfast_core::{keybinds::Action, voice};
    assert!(Action::OFFERED.contains(&Action::ToggleGoLive));
    let (mut app, _folder) = preview();
    assert!(go_live_key(&app).is_none(), "out of a call, nothing");
    let _ = app.update(Message::Keybinds(
        app.keybinds.count,
        crate::keybinds::Told::Toggled(Action::ToggleGoLive),
    ));
    assert_eq!(app.call.popup, None);
    super::change(&mut app, |state| {
        state.voice.sharing = Some(voice::Sharing {
            phase: voice::SharePhase::Live,
            frames: std::sync::Arc::new(yapfast_core::frames::Frames::create(16, 16).unwrap()),
            viewers: vec![],
        });
    });
    assert!(matches!(go_live_key(&app), Some(Event::StopSharing)));
}

/// Reveal all spoilers: one click shows every spoiler in the message.
#[test]
fn reveal_all_spoilers_shows_the_others_too() {
    let (mut app, _folder) = preview();
    let mut message = (*app.timeline.messages[0]).clone();
    message.body = "||one|| and ||two|| and ||three||".into();
    let item = message.item.clone();
    let mut messages = (*app.timeline.messages).clone();
    messages[0] = std::sync::Arc::new(message);
    app.timeline.messages = std::sync::Arc::new(messages);
    let click = |app: &mut App| {
        let link = yapfast_core::spoiler::link(&item, 1);
        let _ = crate::links::follow_link(app, &link, iced::Point::ORIGIN);
    };
    let shown = |app: &App| {
        (0..3)
            .filter(|index| app.timeline.revealed.contains(&format!("{item}/{index}")))
            .count()
    };
    click(&mut app);
    assert_eq!(shown(&app), 1, "only the one clicked");
    app.timeline.revealed.clear();
    turn_on(&mut app, id::REVEAL_SPOILERS);
    click(&mut app);
    assert_eq!(shown(&app), 3, "all of them");
}

/// Plain folder icon is listed and starts off.
#[test]
fn plain_folder_icon_starts_off() {
    let plugin = PLUGINS
        .iter()
        .find(|plugin| plugin.id == id::PLAIN_FOLDER)
        .expect("listed");
    assert!(!plugin.on_at_first);
}

/// Invite defaults: the plugin sets what a new invite link starts with
/// (the core's tests open the dialog with it), and Discord's again once off.
#[test]
fn invite_defaults_pick_what_a_new_link_starts_with() {
    use yapfast_core::halocord::first_invite;
    let (mut app, _folder) = preview();
    turn_on(&mut app, id::INVITE_DEFAULTS);
    assert_eq!(first_invite(), Some((0, 0)), "never runs out, no limit");
    let plugin = place(id::INVITE_DEFAULTS).expect("the plugin");
    for (setting, choice) in [(0, 1), (1, 3)] {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Choose {
                plugin,
                setting,
                choice,
            },
        );
    }
    assert_eq!(first_invite(), Some((3600, 10)), "an hour, ten people");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: false },
    );
    assert_eq!(first_invite(), None);
}

/// No pending count: the DM home's count leaves out friend requests, and
/// so does the count beside Friends.
#[test]
fn no_pending_count_leaves_requests_out_of_the_counts() {
    use yapfast_core::chat::FriendRequest;
    let (mut app, _folder) = preview();
    super::change(&mut app, |state| {
        state.friend_requests = vec![FriendRequest {
            id: "10".into(),
            name: "Lena".into(),
            avatar: None,
            incoming: true,
            spam: false,
        }];
    });
    let _ = app.sync();
    let with = app.rail.home.badge.clone();
    turn_on(&mut app, id::NO_PENDING_COUNT);
    let _ = app.sync();
    let mut without = crate::rail::home(app.client.state());
    crate::halocord::home_mentions(app.client.state(), &mut without);
    assert_eq!(app.rail.home.badge, without.badge);
    assert_ne!(app.rail.home.badge, with, "one request fewer");
    let mut rows = vec![crate::sidebar::SideRow::Screens {
        friends: false,
        requests: false,
        badge: "1".into(),
    }];
    crate::halocord::no_request_badge(&mut rows);
    assert!(matches!(&rows[0], crate::sidebar::SideRow::Screens { badge, .. } if badge.is_empty()));
}

/// Message click actions' added clicks: nothing unless set, so the plugin
/// works as before; a Backspace click set to Delete deletes your own message,
/// without asking, and not someone else's; a triple click set to React
/// reacts; and with it set, two real presses wait for a third before
/// replying as the double-click does.
#[test]
fn message_click_actions_triple_and_backspace_clicks() {
    use std::sync::Arc;
    use yapfast_core::{application::Compose, chat::ChatMessage};

    use crate::{
        halocord::{Event, presses_so_far},
        menus::MenuAct,
    };
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    let press = |app: &mut App, item: &str| {
        let _ = app.update(Message::HaloCord(Event::Press(item.into(), None)));
    };
    let backspace_click = |app: &mut App, item: &str| {
        let _ = app.update(Message::HaloCord(Event::Backspace(true)));
        press(app, item);
        let _ = app.update(Message::HaloCord(Event::Backspace(false)));
    };
    // The preview's own messages first, which its reactions and deletions
    // change: someone else's to react to, and one of this account's.
    let (theirs, mine) = {
        let messages = &app.client.state().messages;
        let find = |own: bool| {
            messages
                .iter()
                .find(|message| {
                    message.own == own
                        && !message.deleted
                        && if own {
                            message.can_delete
                        } else {
                            message.can_react && message.reactions.is_empty()
                        }
                })
                .map(|message| message.item.clone())
                .expect("a sample message")
        };
        (find(false), find(true))
    };
    let deleted = |app: &App, item: &str| {
        app.client
            .state()
            .message(item)
            .is_some_and(|message| message.deleted)
    };
    backspace_click(&mut app, &mine);
    assert!(!deleted(&app, &mine), "off at first");

    turn_on(&mut app, id::CLICK_ACTIONS);
    let plugin = place(id::CLICK_ACTIONS).expect("the plugin");
    backspace_click(&mut app, &mine);
    assert!(!deleted(&app, &mine), "nothing unless set: as before");
    // A Backspace click on your own messages set to Delete.
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Choose {
            plugin,
            setting: 3,
            choice: 1,
        },
    );
    backspace_click(&mut app, &mine);
    assert!(deleted(&app, &mine), "your own, once set");
    assert!(
        app.client.state().confirm_delete.is_none(),
        "without asking"
    );
    // A triple click set to React.
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Choose {
            plugin,
            setting: 1,
            choice: 4,
        },
    );
    for _ in 0..3 {
        press(&mut app, &theirs);
    }
    assert!(
        app.client.state().message(&theirs).is_some_and(|message| {
            message.reactions.iter().any(|it| it.key == "💀" && it.mine)
        }),
        "💀 on their message"
    );

    // Then real presses, on messages of the test's own.
    let said = |item: &str, body: &str, own: bool| ChatMessage {
        id: item.into(),
        item: item.into(),
        author: if own { "You" } else { "Sam" }.into(),
        sender: if own {
            "@you:preview.local"
        } else {
            "@sam:preview.local"
        }
        .into(),
        body: body.into(),
        own,
        timestamp: 1_700_000_000,
        can_reply: true,
        can_react: true,
        can_edit: own,
        // As if this account had Manage Messages.
        can_delete: true,
        ..Default::default()
    };
    super::change(&mut app, |state| {
        state.messages = Arc::new(vec![
            Arc::new(said("c0", "A message from Sam", false)),
            Arc::new(said("c1", "One of mine", true)),
        ]);
    });
    super::interact(&mut app, |ui| {
        ui.click("A message from Sam").expect("the message");
        ui.click("A message from Sam").expect("the message");
    });
    assert!(app.client.state().compose().is_none(), "waits for a third");
    let due = |press| {
        Message::HaloCord(Event::DoubleClickDue(
            "c0".into(),
            press,
            Some(MenuAct::Reply),
        ))
    };
    let _ = app.update(due(0));
    assert!(app.client.state().compose().is_none(), "not that wait");
    let _ = app.update(due(presses_so_far(&app)));
    assert!(
        matches!(
            app.client.state().compose(),
            Some(Compose::Reply { item, .. }) if item == "c0"
        ),
        "replying to Sam"
    );
    // Clicks in the field of a message being edited are the field's: the
    // row's presses ignore what the field took.
    let _ = app.update(Message::Menu(crate::menus::Event::Act(
        MenuAct::Edit,
        "c1".into(),
        iced::Point::ORIGIN,
    )));
    assert_eq!(app.composer.editing.as_deref(), Some("c1"));
    let counted = presses_so_far(&app);
    super::interact(&mut app, |ui| {
        ui.click(iced::widget::Id::new(crate::composer::EDIT))
            .expect("the edit field");
        ui.click(iced::widget::Id::new(crate::composer::EDIT))
            .expect("the edit field");
    });
    assert_eq!(
        presses_so_far(&app),
        counted,
        "no presses counted while editing"
    );
    assert_eq!(app.composer.editing.as_deref(), Some("c1"), "still editing");
    // Someone else's message, even with leave to delete it: not unless set.
    backspace_click(&mut app, "c0");
    assert!(!deleted(&app, "c0"), "others' only if set");
    // Not in DMs: the double-click does nothing in a DM once set.
    let dm = app
        .client
        .state()
        .rooms
        .iter()
        .find(|room| room.direct)
        .map(|room| room.id.clone())
        .expect("a DM");
    app.client
        .dispatch(yapfast_core::application::Action::OpenChat(dm));
    let _ = app.sync();
    let rows = app.client.state().messages.len();
    let row = (0..rows)
        .find(|&row| {
            app.ctx()
                .row_message(row)
                .is_some_and(|message| !message.own && !message.deleted && message.can_reply)
        })
        .expect("a message in the DM");
    assert!(
        crate::halocord::double_click(app.ctx(), row).is_some(),
        "the double-click in a DM, as ever"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Flag {
            plugin,
            setting: 7,
            on: true,
        },
    );
    assert!(
        crate::halocord::double_click(app.ctx(), row).is_none(),
        "not in DMs, once set"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: false },
    );
}

/// Better forwards: off, a forward to one conversation opens it, as ever;
/// on, the conversation open stays open.
#[test]
fn better_forwards_stays_in_the_conversation_after_a_forward() {
    use yapfast_core::application::Action;
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    let here = app.client.state().selected.clone().expect("a conversation");
    let forward = |app: &mut App| {
        let item = app
            .client
            .state()
            .messages
            .iter()
            .rev()
            .find(|message| message.can_forward)
            .map(|message| message.item.clone())
            .expect("a message to forward");
        app.client.dispatch(Action::StartForward(item));
        let target = app
            .client
            .state()
            .forward_places()
            .into_iter()
            .map(|room| room.id.clone())
            .find(|room| *room != here)
            .expect("somewhere else");
        app.client.dispatch(Action::ChooseForward(target.clone()));
        app.client.dispatch(Action::Forward(String::new()));
        let _ = app.sync();
        target
    };
    let target = forward(&mut app);
    assert_eq!(
        app.client.state().selected.as_ref(),
        Some(&target),
        "off, it goes where the message went"
    );
    app.client.dispatch(Action::OpenChat(here.clone()));
    let _ = app.sync();
    turn_on(&mut app, id::BETTER_FORWARDS);
    forward(&mut app);
    assert_eq!(
        app.client.state().selected.as_ref(),
        Some(&here),
        "on, it stays"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::BETTER_FORWARDS).expect("the plugin"),
            on: false,
        },
    );
}

/// GIF paste: off, a picked GIF goes out at once and Shift puts it in the
/// message box, as Yapfast does; on, the other way round.
#[test]
fn gif_paste_puts_a_picked_gif_in_the_message_box() {
    use crate::halocord::inserts_gif;
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    assert!(!inserts_gif(false) && inserts_gif(true), "off at first");
    turn_on(&mut app, id::GIF_PASTE);
    assert!(inserts_gif(false), "a click puts it in");
    assert!(!inserts_gif(true), "Shift and a click sends it");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::GIF_PASTE).expect("the plugin"),
            on: false,
        },
    );
    assert!(!inserts_gif(false), "off again");
}
