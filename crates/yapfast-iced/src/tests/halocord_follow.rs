//! HaloCord's Follow majority: you follow most of your voice channel's
//! people to another voice channel of the server.

use std::sync::Arc;

use yapfast_core::{application::Action, chat::VoicePerson};

use super::{change, open_plugin, preview, shows};
use crate::{
    app::App,
    halocord::{Change, FOLLOW_MAJORITY, changed, place},
};

/// Who is in the Friends server's voice channels: Lounge, and games made
/// one.
fn seat(app: &mut App, lounge: &[&str], games: &[&str]) {
    change(app, |state| {
        for room in &mut state.rooms {
            let who = match room.id.as_str() {
                "lounge" => lounge,
                "games" => games,
                _ => continue,
            };
            let room = Arc::make_mut(room);
            room.voice = true;
            room.people = who
                .iter()
                .map(|id| VoicePerson {
                    id: (*id).into(),
                    name: (*id).into(),
                    ..Default::default()
                })
                .collect();
        }
    });
}

fn room(app: &App) -> Option<&str> {
    app.client.state().voice.room.as_deref()
}

#[test]
fn you_follow_once_most_of_your_channel_has_moved() {
    let (mut app, _folder) = preview();
    seat(&mut app, &[], &[]);
    app.client
        .dispatch(Action::RequestJoinVoice("lounge".into()));
    let _ = app.sync();
    assert_eq!(room(&app), Some("lounge"), "the preview joins");
    let you = app
        .client
        .state()
        .voice
        .participants
        .iter()
        .find(|one| one.local)
        .map(|one| one.person.clone())
        .expect("this account in the call");
    let you = you.as_str();
    seat(&mut app, &[you, "ann", "bob", "cat", "dan"], &[]);
    seat(&mut app, &[you], &["ann", "bob", "cat", "dan"]);
    assert_eq!(room(&app), Some("lounge"), "off at first");
    let plugin = place(FOLLOW_MAJORITY).expect("the plugin");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
    seat(&mut app, &[you, "ann", "bob", "cat", "dan"], &[]);
    seat(&mut app, &[you, "bob", "cat", "dan"], &["ann"]);
    seat(&mut app, &[you, "cat", "dan"], &["ann", "bob"]);
    assert_eq!(room(&app), Some("lounge"), "half isn't most");
    seat(&mut app, &[you, "dan"], &["ann", "bob", "cat"]);
    assert_eq!(room(&app), Some("games"), "most have moved: followed");
}

#[test]
fn one_who_moves_alone_is_not_followed() {
    let (mut app, _folder) = preview();
    seat(&mut app, &[], &[]);
    app.client
        .dispatch(Action::RequestJoinVoice("lounge".into()));
    let _ = app.sync();
    let plugin = place(FOLLOW_MAJORITY).expect("the plugin");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
    seat(&mut app, &["ann"], &[]);
    seat(&mut app, &[], &["ann"]);
    assert_eq!(room(&app), Some("lounge"), "two at least");
}

#[test]
fn follow_majority_is_listed() {
    let (mut app, _folder) = preview();
    open_plugin(&mut app, FOLLOW_MAJORITY);
    assert!(shows(&app, "Follow majority"));
}
