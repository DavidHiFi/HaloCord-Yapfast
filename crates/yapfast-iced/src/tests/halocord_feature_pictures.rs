//! Pictures of build 8's features, drawn by the window in the MochaCord
//! theme. Each test is ignored: it writes `audit/plugins/features-<name>.png`
//! for the owner to look at. Run them with `cargo test -p yapfast-iced --lib
//! -- --include-ignored halocord_feature_pictures`.

use std::{path::PathBuf, sync::Arc};

use iced::{Settings, Size};
use iced_test::Simulator;

use super::{interact, preview, shows, unread};
use crate::{
    app::{
        App, Message,
        layers::{Layer, Popup},
    },
    halocord::{Change, changed, id, place},
    settings::{Event, Section},
};

const FRAME: Size = Size::new(1280.0, 800.0);

fn audit(name: &str) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../audit/plugins");
    std::fs::create_dir_all(&folder).expect("the audit folder");
    folder
        .canonicalize()
        .expect("the audit folder")
        .join(format!("features-{name}.png"))
}

fn turn_on(app: &mut App, id: &str) {
    let plugin = place(id).expect("the plugin");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
}

/// A window in the MochaCord theme, the system's bar replaced by Yapfast's.
fn mocha() -> (App, tempfile::TempDir) {
    let (mut app, folder) = preview();
    app.shell.frame.height = 2400.0;
    let _ = app.update(Message::Settings(Event::Open(Section::Appearance)));
    let _ = app.update(crate::settings::appearance_event(
        crate::settings::ThemeEvent::Choose("mochacord".into()),
    ));
    let _ = app.update(Message::Settings(Event::Close));
    let _ = app.update(Message::PopupGone(Popup::Layer(Layer::Settings)));
    app.shell.frame = FRAME;
    app.shell.own_title_bar = true;
    crate::window_bar::fit_window(&mut app);
    let _ = app.sync();
    (app, folder)
}

fn draw(app: &App, name: &str) {
    let path = audit(name);
    let written = path.with_file_name(format!("features-{name}-tiny-skia.png"));
    for old in [&path, &written] {
        let _ = std::fs::remove_file(old);
    }
    let mut ui = Simulator::with_size(Settings::default(), app.shell.frame, app.view());
    let _ = ui
        .snapshot(&iced::Theme::Dark)
        .expect("drawn")
        .matches_image(&path);
    std::fs::rename(&written, &path).expect("the picture was written");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_go_live_keybind() {
    use crate::settings::keybinds::Event as KeybindEvent;
    let (mut app, _folder) = mocha();
    let _ = app.update(Message::Settings(Event::Open(Section::Keybinds)));
    interact(&mut app, |ui| {
        ui.click("Add a Keybind").expect("the button");
    });
    let _ = app.update(Message::Settings(Event::Keybind(KeybindEvent::Action(
        0, 6,
    ))));
    assert!(shows(&app, "Toggle Go Live"));
    draw(&app, "go-live-keybind");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_new_plugins() {
    let (mut app, _folder) = mocha();
    let _ = app.update(Message::Settings(Event::Open(Section::HaloCord)));
    let _ = app.sync();
    for (query, name) in [
        ("spoilers", "Reveal all spoilers"),
        ("folder icon", "Plain folder icon"),
        ("trust", "Always trust links"),
    ] {
        let _ = app.update(Message::Settings(Event::Search(query.into())));
        let _ = app.sync();
        assert!(shows(&app, name), "{name}");
        draw(&app, &format!("plugin-{}", query.replace(' ', "-")));
    }
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_pings_first() {
    let (mut app, _folder) = mocha();
    let pinging: Vec<String> = app
        .client
        .state()
        .rooms
        .iter()
        .filter(|room| !room.voice && Some(&room.id) != app.client.state().selected.as_ref())
        .map(|room| room.id.clone())
        .rev()
        .take(2)
        .collect();
    for (index, room) in pinging.iter().enumerate() {
        unread(&mut app, room, 3 - index as u64);
    }
    let _ = app.update(Message::Switcher(crate::switcher::Event::Toggle));
    draw(&app, "pings-first");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_reveal_all_spoilers() {
    let (mut app, _folder) = mocha();
    turn_on(&mut app, id::REVEAL_SPOILERS);
    let item = newest_says(
        &mut app,
        "Ending: ||the ship sinks|| and ||everyone sings||.",
    );
    let _ = app.sync();
    draw(&app, "spoilers-hidden");
    let link = yapfast_core::spoiler::link(&item, 0);
    let _ = crate::links::follow_link(&mut app, &link, iced::Point::ORIGIN);
    assert!(app.timeline.revealed.contains(&format!("{item}/1")));
    let _ = app.sync();
    draw(&app, "reveal-all-spoilers");
}

/// The newest message's words changed to `body`; its item.
fn newest_says(app: &mut App, body: &str) -> String {
    let mut item = String::new();
    super::change(app, |state| {
        let mut messages = (*state.messages).clone();
        let last = messages.last_mut().expect("a message");
        let mut message = (**last).clone();
        message.body = body.into();
        message.formatted = true;
        item = message.item.clone();
        *last = Arc::new(message);
        state.messages = Arc::new(messages);
    });
    item
}
