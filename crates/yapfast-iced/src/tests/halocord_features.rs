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
    // The invite defaults live in the core for every window: held so no
    // other test's window, opening, sets them back while this one reads them.
    let _alone = super::plugin_alone();
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

/// A triple click set to Delete deletes your own message, without asking,
/// and never someone else's, even with leave to delete it.
#[test]
fn message_click_actions_triple_click_deletes_only_your_own() {
    use crate::halocord::Event;
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    let (theirs, mine) = {
        let messages = &app.client.state().messages;
        let find = |own: bool| {
            messages
                .iter()
                .find(|message| message.own == own && !message.deleted)
                .map(|message| message.item.clone())
                .expect("a sample message")
        };
        (find(false), find(true))
    };
    super::change(&mut app, |state| {
        for message in std::sync::Arc::make_mut(&mut state.messages) {
            std::sync::Arc::make_mut(message).can_delete = true;
        }
    });
    turn_on(&mut app, id::CLICK_ACTIONS);
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Choose {
            plugin: place(id::CLICK_ACTIONS).expect("the plugin"),
            setting: 1,
            choice: 1,
        },
    );
    let deleted = |app: &App, item: &str| {
        app.client
            .state()
            .message(item)
            .is_some_and(|message| message.deleted)
    };
    for _ in 0..3 {
        let _ = app.update(Message::HaloCord(Event::Press(theirs.clone(), None)));
    }
    assert!(!deleted(&app, &theirs), "not someone else's");
    for _ in 0..3 {
        let _ = app.update(Message::HaloCord(Event::Press(mine.clone(), None)));
    }
    assert!(deleted(&app, &mine), "your own");
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

/// Hide chat buttons: the arrow hides the message box's buttons and shows
/// them again, and "Shown at first" off starts them hidden; the arrow's
/// colour leaves them as they are.
#[test]
fn hide_chat_buttons_hides_the_message_boxs_buttons_and_shows_them() {
    use crate::halocord::{Press, chat_buttons_hidden};
    let arrow = || Message::Bar(crate::window_bar::Event::Halo(Press::ChatButtons));
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    turn_on(&mut app, id::HIDE_CHAT_BUTTONS);
    assert!(!chat_buttons_hidden(), "shown at first");
    let _ = app.update(arrow());
    assert!(chat_buttons_hidden());
    let _ = app.view();
    let _ = app.update(arrow());
    assert!(!chat_buttons_hidden());
    let plugin = place(id::HIDE_CHAT_BUTTONS).expect("the plugin");
    let _ = app.update(arrow());
    let red = |app: &mut App, on| {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Flag {
                plugin,
                setting: 1,
                on,
            },
        );
    };
    red(&mut app, true);
    assert!(chat_buttons_hidden(), "a red arrow keeps them hidden");
    red(&mut app, false);
    let _ = app.update(arrow());
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Flag {
            plugin,
            setting: 0,
            on: false,
        },
    );
    assert!(chat_buttons_hidden(), "hidden at first, as set");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: false },
    );
}

/// Home typing: the DM home shows typing dots only while the plugin is on
/// and someone types in a DM.
#[test]
fn home_typing_shows_dots_on_the_dm_home_while_someone_types() {
    use crate::halocord::home_face;
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    let dm = app
        .client
        .state()
        .rooms
        .iter()
        .find(|room| room.direct)
        .map(|room| room.id.clone())
        .expect("a DM");
    let server_room = app
        .client
        .state()
        .rooms
        .iter()
        .find(|room| !room.direct)
        .map(|room| room.id.clone())
        .expect("a server's channel");
    let types = |app: &mut App, room: &str, someone: bool| {
        let room = room.to_owned();
        super::change(app, move |state| {
            state.typing.clear();
            if someone {
                state.typing.insert(room, vec!["Sam".into()]);
            }
        });
    };
    types(&mut app, &dm, true);
    assert!(home_face(app.ctx(), false).is_none(), "off at first");
    turn_on(&mut app, id::HOME_TYPING);
    assert!(
        home_face(app.ctx(), false).is_some(),
        "dots while Sam types"
    );
    assert!(home_face(app.ctx(), true).is_some(), "on the lit tile too");
    let _ = app.view();
    types(&mut app, &server_room, true);
    assert!(
        home_face(app.ctx(), false).is_none(),
        "not for a server's channel"
    );
    types(&mut app, &dm, false);
    assert!(
        home_face(app.ctx(), false).is_none(),
        "the logo once nobody types"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::HOME_TYPING).expect("the plugin"),
            on: false,
        },
    );
}

/// Search result menu: with it on, a search result's right click opens a
/// menu whose Jump to message goes to it.
#[test]
fn search_result_menu_jumps_to_a_found_message() {
    use yapfast_core::{
        application::Search,
        chat::{ChatMessage, SearchHit},
    };

    use crate::halocord::Event;
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    let room = app.client.state().selected.clone().expect("a conversation");
    let message: ChatMessage = (**app.client.state().messages.first().expect("a message")).clone();
    let item = message.item.clone();
    super::change(&mut app, {
        let (room, message) = (room.clone(), message);
        move |state| {
            state.search = Some(Search {
                results: vec![SearchHit {
                    room,
                    message,
                    thread: None,
                }],
                ..Search::default()
            });
        }
    });
    let open = |app: &mut App| {
        app.win.menu = None;
        let _ = app.update(Message::HaloCord(Event::SearchMenu(
            room.clone(),
            item.clone(),
        )));
        app.win.menu.as_ref().map(|menu| {
            menu.rows
                .iter()
                .map(|row| row.label.to_string())
                .collect::<Vec<_>>()
        })
    };
    turn_on(&mut app, id::SEARCH_MENU);
    let labels = open(&mut app).expect("the menu");
    assert_eq!(labels[..2], ["Jump to message", "Copy text"], "{labels:?}");
    assert!(
        !labels.iter().any(|label| label == "Copy message link"),
        "not a Discord room"
    );
    let at = labels
        .iter()
        .position(|label| label == "Jump to message")
        .unwrap();
    let _ = app.update(Message::Menu(crate::menus::Event::Choose(at)));
    assert_eq!(
        app.client
            .state()
            .jump
            .as_ref()
            .map(|(jumped, _)| jumped.as_str()),
        Some(item.as_str()),
        "jumped to it"
    );
    // A hit with no text (a file alone): no Copy text, as in a message's
    // own menu.
    super::change(&mut app, |state| {
        if let Some(search) = &mut state.search {
            search.results[0].message.body = String::new();
        }
    });
    let labels = open(&mut app).expect("the menu");
    assert!(
        !labels.iter().any(|label| label == "Copy text"),
        "{labels:?}"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::SEARCH_MENU).expect("the plugin"),
            on: false,
        },
    );
}

/// Keep current channel: on as the window starts, the conversation kept
/// in the app's folder opens once it's listed, and the one open is kept
/// there. Switched on later, it opens nothing and keeps from then on.
#[test]
fn keep_current_channel_opens_the_kept_conversation_and_keeps_the_open_one() {
    let (mut app, _folder) = super::server::discord_server(0, |_, _| {});
    let _alone = super::plugin_alone();
    let file = app.client.root().join("halocord-channel");
    std::fs::write(&file, "discord:10:11\n").expect("a kept conversation");
    let general = |app: &App| app.client.state().selected.as_deref() == Some("discord:10:11");
    let _ = app.settle();
    assert!(!general(&app), "off at first");
    // Switched on in the middle of a session: nothing opens.
    turn_on(&mut app, id::KEEP_CHANNEL);
    let _ = app.settle();
    assert!(!general(&app), "not opened when switched on later");
    assert!(!app.halocord_kept.waiting());
    // As after a restart with it on: the kept one opens once listed.
    app.halocord_kept = Default::default();
    let _ = app.settle();
    assert!(general(&app), "opened once listed");
    // As after a restart with nothing kept: the one open is written.
    std::fs::remove_file(&file).expect("the file");
    app.halocord_kept = Default::default();
    let _ = app.settle();
    assert_eq!(
        std::fs::read_to_string(&file).expect("kept again").trim(),
        "discord:10:11"
    );
    switch_off(&mut app, id::KEEP_CHANNEL);
}

/// Keep current channel: while it waits for the kept conversation to be
/// listed, Yapfast's own picks keep it waiting, but someone going
/// somewhere else ends the wait, so the kept one doesn't take them back.
#[test]
fn keep_current_channel_stops_waiting_once_something_else_is_opened() {
    use yapfast_core::application::{Action, Navigation};
    let (mut app, _folder) = super::server::discord_server(0, |_, _| {});
    let _alone = super::plugin_alone();
    turn_on(&mut app, id::KEEP_CHANNEL);
    std::fs::write(app.client.root().join("halocord-channel"), "discord:10:11")
        .expect("a kept conversation");
    // #general isn't listed yet as the window starts.
    let mut later = None;
    super::change(&mut app, |state| {
        let at = state
            .rooms
            .iter()
            .position(|room| room.id == "discord:10:11")
            .expect("#general");
        later = Some(state.rooms.remove(at));
    });
    app.halocord_kept = Default::default();
    let _ = app.settle();
    assert!(app.halocord_kept.waiting(), "waits for it to be listed");
    // Yapfast closing what's open, as it does with nothing to show: still waiting.
    super::change(&mut app, |state| state.selected = None);
    let _ = app.settle();
    assert!(app.halocord_kept.waiting(), "Yapfast's own pick");
    // Someone goes to the server (the window starts on the DMs): the wait
    // ends.
    let server = Navigation::Server("discord:10".into());
    assert_ne!(app.client.state().navigation, server);
    app.client.dispatch(Action::Navigate(server.clone()));
    let _ = app.sync();
    let _ = app.settle();
    assert!(!app.halocord_kept.waiting(), "something else was opened");
    // #general listed now: nobody is taken back to it.
    super::change(&mut app, |state| {
        state.rooms.push(later.take().expect("kept"));
    });
    let _ = app.settle();
    assert_eq!(app.client.state().navigation, server);
    assert_ne!(
        app.client.state().selected.as_deref(),
        Some("discord:10:11")
    );
    switch_off(&mut app, id::KEEP_CHANNEL);
}

fn switch_off(app: &mut App, which: &str) {
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(which).expect("the plugin"),
            on: false,
        },
    );
}

/// Better upload button: with the plugin on, a click on the message box's
/// plus opens the file picker (no menu), and a right click opens its menu,
/// Upload a File first; off, a click opens the menu as Yapfast's does.
#[test]
fn better_upload_button_clicks_upload_and_right_clicks_open_the_menu() {
    use iced::{Point, mouse};
    let (mut app, _folder) = preview();
    // A Discord DM, where polls start, so the + has its menu, and where
    // files can go: the preview takes none, so out of preview, signed in.
    super::change(&mut app, |state| {
        state.preview = false;
        state.account = Some("1".into());
        state.discord_online = true;
        let open = state.selected.clone();
        for room in &mut state.rooms {
            if Some(&room.id) == open.as_ref() {
                let room = std::sync::Arc::make_mut(room);
                room.network = yapfast_core::network::Network::Discord;
                room.direct = true;
            }
        }
    });
    let plus = Point::new(
        crate::conversation::chat_left(&app)
            + crate::composer::GUTTER
            + crate::composer::BUTTON_INSET
            + 13.0,
        crate::composer::composer_top(&app) + crate::composer::HEIGHT / 2.0,
    );
    let press = |app: &mut App, button| {
        super::interact(app, |ui| {
            ui.point_at(plus);
            ui.simulate([
                iced::Event::Mouse(mouse::Event::ButtonPressed(button)),
                iced::Event::Mouse(mouse::Event::ButtonReleased(button)),
            ]);
        });
    };
    let first_row = |app: &App| app.win.menu.as_ref().map(|menu| menu.rows[0].label.clone());
    // Off: a click opens the menu; a right click does nothing.
    press(&mut app, mouse::Button::Left);
    assert_eq!(first_row(&app).as_deref(), Some("Upload a File"));
    app.win.menu = None;
    press(&mut app, mouse::Button::Right);
    assert!(app.win.menu.is_none(), "no menu on a right click while off");
    // On: a click goes to the file picker, a right click opens the menu.
    turn_on(&mut app, id::BETTER_UPLOAD_BUTTON);
    press(&mut app, mouse::Button::Left);
    assert!(app.win.menu.is_none(), "a click uploads, no menu");
    press(&mut app, mouse::Button::Right);
    assert_eq!(first_row(&app).as_deref(), Some("Upload a File"));
}

/// Notification title: its switch reaches the core, which then names whom a
/// reply's notification replies to, and stops again once it's off.
#[test]
fn notification_title_names_whom_a_reply_replies_to() {
    use yapfast_core::halocord::notification_title::{Replied, author};
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    let lena = || Some(Replied::Named("Lena".to_owned()));
    assert_eq!(author("Sam".into(), lena), "Sam", "off at first");
    turn_on(&mut app, id::NOTIFICATION_TITLE);
    assert_eq!(author("Sam".into(), lena), "Sam replied to Lena");
    assert_eq!(author("Sam".into(), || None), "Sam", "not a reply");

    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::NOTIFICATION_TITLE).expect("the plugin"),
            on: false,
        },
    );
    assert_eq!(author("Sam".into(), lena), "Sam");
}

/// Show hidden things: while on, a timed out member's messages get the
/// clock by their name, and a server whose invites are paused says so in
/// its name in the server list; each has its own switch, and neither shows
/// while the plugin is off.
#[test]
fn show_hidden_things_shows_timeouts_on_messages_and_paused_invites() {
    use std::borrow::Cow;

    use crate::halocord::{message_timeout, server_tip};
    use yapfast_core::yapfast_discord::halocord::note_invites;
    let (mut app, _folder) = preview();
    // The preview's first member, as a Discord message names its sender.
    let person = match &app.client.state().members {
        Some(Ok(members)) => members[0].id.clone(),
        _ => panic!("the preview's members"),
    };
    let sender = yapfast_core::discord::sender_id(&person);
    let until = yapfast_core::clock::now_ms() + 3_600_000;
    super::change(&mut app, |state| {
        if let Some(Ok(members)) = &mut state.members {
            for member in std::sync::Arc::make_mut(members) {
                if member.id == person {
                    member.timed_out_until = Some(until);
                }
            }
        }
    });
    note_invites("halo-test-paused", true);
    let tip = |app: &App| {
        server_tip(app.ctx(), "discord:halo-test-paused", Cow::Borrowed("Test")).into_owned()
    };
    let clock = |app: &App| message_timeout(app.ctx(), &sender).is_some();
    assert!(!clock(&app), "off: no clock on messages");
    assert_eq!(tip(&app), "Test");
    turn_on(&mut app, id::SHOW_HIDDEN);
    assert!(clock(&app), "on: the clock");
    assert_eq!(tip(&app), "Test · Invites paused");
    assert_eq!(
        server_tip(
            app.ctx(),
            "discord:halo-test-running",
            Cow::Borrowed("Other")
        ),
        "Other"
    );
    let plugin = place(id::SHOW_HIDDEN).expect("the plugin");
    for setting in [0, 1] {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Flag {
                plugin,
                setting,
                on: false,
            },
        );
    }
    assert!(!clock(&app), "its switch off");
    assert_eq!(tip(&app), "Test");

    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: false },
    );
    note_invites("halo-test-paused", false);
}

/// Show song name: Riley, listening to Spotify, shows the song under their
/// name in the member list while it's on, and "Spotify" again once off;
/// the list is made again on the switch, though nobody changed.
#[test]
fn show_song_name_names_the_song_in_the_member_list() {
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    let music = yapfast_core::yapfast_discord::activity::activities(&serde_json::json!([{
        "type": 2, "name": "Spotify", "details": "Ashes of the Damned", "state": "Kevin Sherwood",
    }]));
    let line = music.first().and_then(|first| first.line());
    super::change(&mut app, |state| {
        for members in state.members.iter_mut().flatten() {
            for member in std::sync::Arc::make_mut(members) {
                if member.name == "Riley" {
                    member.activities = music.clone();
                    member.activity = line.clone();
                }
            }
        }
    });
    app.show_roster(false);
    let song = "Ashes of the Damned";
    assert!(!super::shows(&app, song), "off: the app's name");
    turn_on(&mut app, id::SHOW_SONG_NAME);
    app.show_roster(false);
    assert!(super::shows(&app, song), "on: the song");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::SHOW_SONG_NAME).expect("the plugin"),
            on: false,
        },
    );
    app.show_roster(false);
    assert!(!super::shows(&app, song), "off again");
}

/// Song link: a message with a song's link gets "Open on song.link" in its
/// menu while the plugin is on, whose page is for the first song and the
/// country set; other links get no row.
#[test]
fn song_link_offers_song_link_for_a_song() {
    use crate::halocord::{message_rows, song_links, song_page};
    let (mut app, _folder) = preview();
    let item = app.client.state().messages[0].item.clone();
    let body =
        "hear https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC). and <https://example.com>";
    super::change(&mut app, |state| {
        std::sync::Arc::make_mut(&mut std::sync::Arc::make_mut(&mut state.messages)[0]).body =
            body.into();
    });
    let message = |app: &App| (*app.client.state().messages[0]).clone();
    let offered = |app: &App| {
        message_rows(app, &message(app))
            .iter()
            .any(|row| row.label == "Open on song.link")
    };
    assert!(!offered(&app), "off");
    turn_on(&mut app, id::SONG_LINK);
    assert!(offered(&app));
    assert_eq!(message(&app).item, item);
    let link = song_links(body)[0];
    assert_eq!(
        link,
        "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC"
    );
    // The link goes into song.link's address encoded; a country only when
    // one is set.
    assert_eq!(
        song_page(link, ""),
        "https://song.link/https%3A%2F%2Fopen.spotify.com%2Ftrack%2F4uLU6hMCjMI75M1A2tKUQC"
    );
    assert_eq!(
        song_page(link, "us"),
        "https://song.link/https%3A%2F%2Fopen.spotify.com%2Ftrack%2F4uLU6hMCjMI75M1A2tKUQC?userCountry=US"
    );
    assert_eq!(
        song_page("https://music.youtube.com/watch?v=abc&list=x", "nowhere"),
        "https://song.link/https%3A%2F%2Fmusic.youtube.com%2Fwatch%3Fv%3Dabc%26list%3Dx",
        "its & stays its own; not two letters, no country"
    );
    // Spotify's share id is left off.
    assert_eq!(
        song_page("https://open.spotify.com/track/1?si=abc", ""),
        "https://song.link/https%3A%2F%2Fopen.spotify.com%2Ftrack%2F1"
    );
    assert_eq!(
        song_links(
            "https://open.spotify.com/intl-de/track/2 https://tidal.com/browse/track/3 \
             https://listen.tidal.com/track/4 https://music.apple.com/au/song/x/5 \
             https://music.apple.com/au/album/x/123?i=4 https://music.youtube.com/watch?v=abc"
        ),
        [
            "https://open.spotify.com/intl-de/track/2",
            "https://tidal.com/browse/track/3",
            "https://listen.tidal.com/track/4",
            "https://music.apple.com/au/song/x/5",
            "https://music.apple.com/au/album/x/123?i=4",
            "https://music.youtube.com/watch?v=abc"
        ]
    );
    assert!(
        song_links(
            "https://open.spotify.com/album/1 https://open.spotify.com/artist/1 https://music.youtube.com/channel/1"
        )
        .is_empty(),
        "Spotify's albums and artists and YouTube Music's channels aren't songs; Apple shares a song as its album's link"
    );
    // An artist's page isn't a song.
    super::change(&mut app, |state| {
        std::sync::Arc::make_mut(&mut std::sync::Arc::make_mut(&mut state.messages)[0]).body =
            "https://open.spotify.com/artist/1".into();
    });
    assert!(!offered(&app));
    // Choosing it opens the page (no browser opens in tests).
    let _ = crate::halocord::chosen(&mut app, id::SONG_LINK, &item, None);
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::SONG_LINK).expect("the plugin"),
            on: false,
        },
    );
}

/// Favorite anything: its switch reaches the core, which then offers still
/// pictures and videos as favourites (the core's GIF tests show which).
#[test]
fn favorite_anything_switches_the_core() {
    use yapfast_core::halocord::favorite_anything::is_on;
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    assert!(!is_on(), "off at first");
    turn_on(&mut app, id::FAVORITE_ANYTHING);
    assert!(is_on());

    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::FAVORITE_ANYTHING).expect("the plugin"),
            on: false,
        },
    );
    assert!(!is_on());
}

/// Song link's buttons: under a message with a song, "Song links" while
/// the plugin is on; a press asks song.link (not run in tests) and its
/// answer is kept by the link; the "Buttons under a song" switch off takes
/// them away.
#[test]
fn song_link_buttons_ask_once_pressed_and_keep_the_answer() {
    use yapfast_core::yapfast_discord::halocord::SongPlatform;

    use crate::halocord::{Event, song_buttons};
    let (mut app, _folder) = preview();
    let link = "https://open.spotify.com/track/halo-test-buttons";
    super::change(&mut app, |state| {
        std::sync::Arc::make_mut(&mut std::sync::Arc::make_mut(&mut state.messages)[0]).body =
            format!("hear {link}?si=abc");
    });
    let message = |app: &App| (*app.client.state().messages[0]).clone();
    assert!(song_buttons(app.ctx(), &message(&app)).is_none(), "off");
    turn_on(&mut app, id::SONG_LINK);
    assert!(
        song_buttons(app.ctx(), &message(&app)).is_some(),
        "Song links"
    );
    // Pressed: asked for the link without Spotify's share id.
    let _ = app.update(Message::HaloCord(Event::SongLinks(link.into())));
    assert_eq!(crate::halocord::song_buttons_known(link), None, "waiting");
    let found = vec![
        SongPlatform {
            key: "spotify".into(),
            url: link.into(),
            native: Some("spotify:track:1".into()),
        },
        SongPlatform {
            key: "deezer".into(),
            url: "https://www.deezer.com/track/2".into(),
            native: None,
        },
    ];
    let _ = app.update(Message::HaloCord(Event::SongLinksFound(
        link.into(),
        Ok(found),
    )));
    assert_eq!(crate::halocord::song_buttons_known(link), Some(2));
    assert!(
        song_buttons(app.ctx(), &message(&app)).is_some(),
        "a button each"
    );
    let plugin = place(id::SONG_LINK).expect("the plugin");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Flag {
            plugin,
            setting: 1,
            on: false,
        },
    );
    assert!(
        song_buttons(app.ctx(), &message(&app)).is_none(),
        "its switch off"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: false },
    );
}

/// Clickable roles in the member list: a heading that names one of the
/// server's roles ("Mods — 3") finds it, Online and Offline don't, and its
/// card takes the role's name and colour from the server.
#[test]
fn clickable_roles_open_a_role_card_from_a_member_list_heading() {
    use std::{collections::HashMap, sync::Arc};

    use crate::halocord::heading_role;
    use crate::info_cards::Event as Card;
    let (mut app, _folder) = super::server::discord_server(0, |server, _| {
        server.role_names = Arc::new(HashMap::from([
            ("10".to_owned(), "@everyone".to_owned()),
            ("20".to_owned(), "Mods".to_owned()),
            ("30".to_owned(), "Helpers".to_owned()),
            ("31".to_owned(), "Helpers".to_owned()),
        ]));
        server.role_colors = Arc::new(HashMap::from([("20".to_owned(), (1, 0x89B4FA))]));
    });
    app.client.app.state.selected = Some("discord:10:11".into());
    let _ = app.sync();
    assert_eq!(heading_role(app.ctx(), "Mods — 3").as_deref(), Some("20"));
    assert_eq!(
        heading_role(app.ctx(), "Mods").as_deref(),
        Some("20"),
        "no count"
    );
    assert_eq!(heading_role(app.ctx(), "Online — 40"), None);
    assert_eq!(heading_role(app.ctx(), "Offline — 2"), None);
    assert_eq!(
        heading_role(app.ctx(), "Raiders — 1"),
        None,
        "not a role here"
    );
    assert_eq!(
        heading_role(app.ctx(), "Helpers — 2"),
        None,
        "two roles share the name: no card"
    );
    turn_on(&mut app, id::CLICKABLE_ROLES);
    let _ = app.update(Message::Info(Card::ProfileRole("20".into())));
    let shown = app.client.state().role.clone().expect("the role's card");
    assert_eq!(
        (shown.name.as_str(), shown.color),
        ("Mods", Some(0x89B4FA)),
        "named and coloured by the server"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::CLICKABLE_ROLES).expect("the plugin"),
            on: false,
        },
    );
}

/// View icons' View decoration: no row in someone's menu while Yapfast
/// knows no decoration for them (the preview knows none), and the page a
/// decoration opens is its picture at 512 pixels, moving.
#[test]
fn view_decoration_opens_the_decoration_moving() {
    use crate::halocord::{decoration_page, person_rows};
    let (mut app, _folder) = preview();
    turn_on(&mut app, id::VIEW_ICONS);
    assert!(
        !person_rows(&app, "4")
            .iter()
            .any(|row| row.label == "View decoration"),
        "none known"
    );
    assert_eq!(
        decoration_page(
            "https://cdn.discordapp.com/avatar-decoration-presets/a_1.png?size=96&passthrough=false"
        ),
        "https://cdn.discordapp.com/avatar-decoration-presets/a_1.png?size=512&passthrough=true"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::VIEW_ICONS).expect("the plugin"),
            on: false,
        },
    );
}

/// Show connections: someone's profile card shows their connections only
/// while it's on; the full profile shows them either way, as now.
#[test]
fn show_connections_puts_them_on_the_profile_card() {
    use yapfast_core::chat::ProfileConnection;
    let (mut app, _folder) = preview();
    let riley = app
        .client
        .state()
        .members
        .iter()
        .flatten()
        .flat_map(|members| members.iter())
        .find(|member| member.name == "Riley")
        .map(|member| member.id.clone())
        .expect("Riley in the preview");
    let _ = app.open_card(riley, iced::Point::new(500.0, 200.0));
    let view = app
        .client
        .app
        .state
        .profile
        .as_mut()
        .map(std::sync::Arc::make_mut)
        .expect("the card open");
    view.profile.connections = vec![ProfileConnection {
        kind: "github".into(),
        name: "halo-test-account".into(),
        verified: true,
        url: Some("https://github.com/halo-test-account".into()),
        since: None,
    }];
    let _ = app.sync();
    assert!(
        !super::shows(&app, "halo-test-account"),
        "off: not on the card"
    );
    turn_on(&mut app, id::SHOW_CONNECTIONS);
    assert!(super::shows(&app, "halo-test-account"), "on: on the card");
    assert!(super::shows(&app, "Connections"));
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::SHOW_CONNECTIONS).expect("the plugin"),
            on: false,
        },
    );
}

/// Last online: with it on, a DM whose person Yapfast saw go offline says
/// "Online just now" under the name, from the window's own list making;
/// back online, or with it off, nothing.
#[test]
fn last_online_says_when_a_dm_went_offline() {
    use yapfast_core::chat::Presence;
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    let dm = app.client.state().rooms[0].id.clone();
    let presence = |app: &mut App, presence| {
        super::change(app, |state| {
            let room = std::sync::Arc::make_mut(&mut state.rooms[0]);
            room.direct = true;
            room.presence = Some(presence);
        });
    };
    let said = |app: &App| crate::halocord::last_online(app.ctx(), &dm);
    turn_on(&mut app, id::LAST_ONLINE);
    let plugin = place(id::LAST_ONLINE).expect("the plugin");
    // Not kept across restarts in this test: nothing in its folder.
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Flag {
            plugin,
            setting: 0,
            on: false,
        },
    );
    presence(&mut app, Presence::Online);
    assert_eq!(said(&app), None, "online");
    presence(&mut app, Presence::Offline);
    assert_eq!(said(&app).as_deref(), Some("Online just now"));
    presence(&mut app, Presence::Idle);
    assert_eq!(said(&app), None, "back");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: false },
    );
    assert_eq!(said(&app), None, "off");
}
