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
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id::SEARCH_MENU).expect("the plugin"),
            on: false,
        },
    );
}
