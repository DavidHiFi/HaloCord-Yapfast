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
