//! Pictures of what the voice and call plugins add, in MochaCord, for a
//! look by eye. They are ignored; run them with
//! `-- --include-ignored halocord_pictures`. Each writes
//! `audit/plugins/voice-<plugin>-tiny-skia.png`.

use iced_test::Simulator;
use yapfast_core::application::Action;

use super::preview;
use crate::{
    app::App,
    call::Event,
    halocord::{Change, changed, place},
};

const SIZE: iced::Size = iced::Size::new(1280.0, 800.0);

/// The preview in MochaCord, at the picture's size.
fn mocha() -> (App, tempfile::TempDir) {
    let (mut app, folder) = preview();
    assert!(app.choose_theme(crate::mochacord::THEME), "MochaCord");
    app.shell.frame = SIZE;
    (app, folder)
}

fn switch(app: &mut App, id: &str, on: bool) {
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id).unwrap(),
            on,
        },
    );
}

/// The window as drawn now, written to the plugin's picture.
fn take(app: &App, plugin: &str) {
    let mut ui = Simulator::with_size(iced::Settings::default(), SIZE, app.view());
    let snapshot = ui.snapshot(&iced::Theme::Dark).expect("drawn");
    let path = format!(r"D:\Agent Work\yapfast\audit\plugins\voice-{plugin}.png");
    snapshot.matches_image(path).expect("written");
}

fn people(names: &[(&str, &str)]) -> Vec<yapfast_core::chat::VoicePerson> {
    names
        .iter()
        .map(|(id, name)| yapfast_core::chat::VoicePerson {
            id: (*id).into(),
            name: (*name).into(),
            ..Default::default()
        })
        .collect()
}

fn fill(app: &mut App, room: &str, who: Vec<yapfast_core::chat::VoicePerson>) {
    super::change(app, |state| {
        let room = state
            .rooms
            .iter_mut()
            .find(|listed| listed.id == room)
            .expect("the room");
        std::sync::Arc::make_mut(room).people = who;
    });
}

#[test]
#[ignore]
fn volume_booster() {
    let (mut app, _folder) = mocha();
    let room = "lounge".to_owned();
    app.client.dispatch(Action::SelectRoom(room.clone()));
    let _ = app.sync();
    app.client.dispatch(Action::SelectRoom(room.clone()));
    let _ = app.sync();
    super::change(&mut app, |state| {
        let mut carol = state.voice.participants[0].clone();
        carol.id = "preview-carol".into();
        carol.person = "carol".into();
        carol.user = "Carol".into();
        carol.local = false;
        state.voice.participants.push(carol);
    });
    let _ = crate::call::update(
        &mut app,
        Event::Person(room, "carol".into(), iced::Point::new(400.0, 300.0)),
    );
    let _ = crate::call::update(&mut app, Event::Volume(false, 800, true));
    take(&app, "volume-booster");
}

#[test]
#[ignore]
fn click_to_unmute() {
    let (mut app, _folder) = mocha();
    fill(
        &mut app,
        "lounge",
        people(&[
            ("@sam:preview.local", "Sam"),
            ("@riley:preview.local", "Riley"),
            ("@morgan:preview.local", "Morgan"),
        ]),
    );
    yapfast_core::volume::mute("@riley:preview.local", true);
    app.layout
        .voice
        .muted_people
        .insert("@riley:preview.local".into());
    take(&app, "click-to-unmute");
    yapfast_core::volume::mute("@riley:preview.local", false);
}

#[test]
#[ignore]
fn voice_count() {
    let (mut app, _folder) = mocha();
    fill(
        &mut app,
        "lounge",
        people(&[
            ("@sam:preview.local", "Sam"),
            ("@riley:preview.local", "Riley"),
            ("@morgan:preview.local", "Morgan"),
        ]),
    );
    fill(
        &mut app,
        "party-voice",
        people(&[("@you:preview.local", "You"), ("other", "Dana")]),
    );
    switch(&mut app, crate::halocord::VOICE_COUNT, true);
    take(&app, "voice-count");
}

#[test]
#[ignore]
fn user_voice_show() {
    let (mut app, _folder) = mocha();
    app.client.dispatch(Action::SelectRoom("general".into()));
    let _ = app.sync();
    fill(
        &mut app,
        "lounge",
        people(&[
            ("@sam:preview.local", "Sam"),
            ("@morgan:preview.local", "Morgan"),
        ]),
    );
    switch(&mut app, crate::halocord::USER_VOICE_SHOW, true);
    take(&app, "user-voice-show");
}
