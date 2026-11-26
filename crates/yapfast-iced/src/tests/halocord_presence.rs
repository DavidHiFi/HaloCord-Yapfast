//! The Custom RPC and Animated status plugins: they are on the Plugins page,
//! found by search, and tell the Discord service what they hold. The
//! ignored test draws their settings into `audit/hc17/presence/`; run it
//! with `-- --include-ignored halocord_presence`.

use std::path::PathBuf;

use iced::{Settings, Size};
use iced_test::Simulator;
use yapfast_core::yapfast_discord::halocord::presence::{self, Started};

use super::{plugin_alone, preview, shows};
use crate::{
    app::{
        App, Message,
        layers::{Layer, Popup},
    },
    halocord::{ANIMATED_STATUS, CUSTOM_RPC, Change, PLUGINS, changed, found, place},
    settings::{Event, Section},
};

fn open(app: &mut App, query: &str) {
    app.shell.frame.height = 2400.0;
    crate::window_bar::fit_window(app);
    let _ = app.update(crate::app::settings(Event::Open(Section::Plugins)));
    let _ = app.update(crate::app::settings(Event::Search(query.into())));
    let _ = app.sync();
}

fn plugin(id: &str) -> usize {
    place(id).expect("the plugin")
}

fn set(app: &mut App, id: &str, key: &str, change: impl Fn(usize, usize) -> Change) {
    let at = plugin(id);
    let setting = PLUGINS[at]
        .settings
        .iter()
        .position(|setting| setting.key == key)
        .expect("the setting");
    changed(&mut app.layout, &mut app.client, change(at, setting));
}

fn type_in(app: &mut App, id: &str, key: &str, text: &str) {
    set(app, id, key, |plugin, setting| Change::Type {
        plugin,
        setting,
        text: text.into(),
    });
}

fn switch(app: &mut App, id: &str, on: bool) {
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: plugin(id),
            on,
        },
    );
}

#[test]
fn the_presence_plugins_are_listed_with_their_settings() {
    let keys = |id: &str| -> Vec<&str> {
        PLUGINS[plugin(id)]
            .settings
            .iter()
            .map(|setting| setting.key)
            .collect()
    };
    assert_eq!(keys(CUSTOM_RPC).len(), 15);
    for key in [
        "application",
        "type",
        "name",
        "details",
        "state",
        "big-image",
        "big-text",
        "small-image",
        "small-text",
        "started",
        "button-1-text",
        "button-1-link",
        "button-2-text",
        "button-2-link",
        "stream-link",
    ] {
        assert!(keys(CUSTOM_RPC).contains(&key), "{key}");
    }
    assert_eq!(keys(ANIMATED_STATUS), ["lines", "every", "emoji"]);
}

#[test]
fn a_search_finds_the_presence_plugins_by_name_and_setting() {
    let find = |query: &str| -> Vec<&str> {
        PLUGINS
            .iter()
            .filter(|plugin| found(plugin, query))
            .map(|plugin| plugin.name)
            .collect()
    };
    assert_eq!(find("custom rpc"), ["Custom RPC"]);
    assert_eq!(find("application id"), ["Custom RPC"]);
    assert_eq!(find("stream link"), ["Custom RPC"]);
    assert_eq!(find("animated"), ["Animated status"]);
    assert_eq!(find("lines"), ["Animated status"]);
    let (mut app, _folder) = preview();
    for (query, name) in [
        ("custom rpc", "Custom RPC"),
        ("animated", "Animated status"),
    ] {
        open(&mut app, query);
        assert!(shows(&app, name), "{name}");
    }
}

#[test]
fn the_plugins_tell_the_service_what_they_hold_while_on() {
    let (mut app, _folder) = preview();
    let _alone = plugin_alone();
    assert_eq!(presence::config().rpc, None);
    assert_eq!(presence::config().animated, None);

    switch(&mut app, CUSTOM_RPC, true);
    type_in(&mut app, CUSTOM_RPC, "name", "Halo");
    type_in(&mut app, CUSTOM_RPC, "button-2-text", "Join");
    type_in(
        &mut app,
        CUSTOM_RPC,
        "button-2-link",
        "https://example.com/j",
    );
    set(&mut app, CUSTOM_RPC, "type", |plugin, setting| {
        Change::Choose {
            plugin,
            setting,
            choice: 4,
        }
    });
    set(&mut app, CUSTOM_RPC, "started", |plugin, setting| {
        Change::Choose {
            plugin,
            setting,
            choice: 2,
        }
    });
    let rpc = presence::config().rpc.expect("on");
    assert_eq!(rpc.name, "Halo");
    assert_eq!(rpc.kind, 1, "Streaming is Discord's 1");
    assert_eq!(rpc.started, Started::TurnedOn);
    assert_eq!(
        rpc.buttons[1],
        ("Join".into(), "https://example.com/j".into())
    );
    switch(&mut app, CUSTOM_RPC, false);
    assert_eq!(presence::config().rpc, None);

    switch(&mut app, ANIMATED_STATUS, true);
    type_in(&mut app, ANIMATED_STATUS, "lines", "one | two");
    let animated = presence::config().animated.expect("on");
    assert_eq!(animated.shown(), ["one", "two"]);
    assert_eq!(animated.every().as_secs(), 60);
    set(&mut app, ANIMATED_STATUS, "every", |plugin, setting| {
        Change::Slide {
            plugin,
            setting,
            value: 10,
        }
    });
    assert_eq!(
        presence::config().animated.expect("on").every().as_secs(),
        30,
        "never under 30 seconds"
    );
    switch(&mut app, ANIMATED_STATUS, false);
    assert_eq!(presence::config().animated, None);
}

fn audit(name: &str) -> PathBuf {
    let folder = PathBuf::from(r"D:\Agent Work\yapfast\audit\hc17\presence");
    std::fs::create_dir_all(&folder).expect("the audit folder");
    folder.join(format!("{name}.png"))
}

fn draw(app: &App, name: &str) {
    let path = audit(name);
    let written = path.with_file_name(format!("{name}-tiny-skia.png"));
    for old in [&path, &written] {
        let _ = std::fs::remove_file(old);
    }
    super::settle(app);
    let mut ui = Simulator::with_size(Settings::default(), app.shell.frame, app.view());
    let _ = ui
        .snapshot(&iced::Theme::Dark)
        .expect("drawn")
        .matches_image(&path);
    std::fs::rename(&written, &path).expect("the picture was written");
}

/// A window in the MochaCord theme at the picture's size.
fn mocha(size: Size) -> (App, tempfile::TempDir) {
    let (mut app, folder) = preview();
    app.shell.frame.height = 2400.0;
    let _ = app.update(crate::app::settings(Event::Open(Section::Appearance)));
    let _ = app.update(crate::settings::appearance_event(
        crate::settings::ThemeEvent::Choose("mochacord".into()),
    ));
    let _ = app.update(crate::app::settings(Event::Close));
    let _ = app.update(Message::PopupGone(Popup::Layer(Layer::Settings)));
    app.shell.frame = size;
    app.shell.own_title_bar = true;
    crate::window_bar::fit_window(&mut app);
    let _ = app.sync();
    (app, folder)
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_the_presence_plugins() {
    let (mut app, _folder) = mocha(Size::new(1280.0, 2300.0));
    let _alone = plugin_alone();
    switch(&mut app, CUSTOM_RPC, true);
    type_in(&mut app, CUSTOM_RPC, "name", "Halo Radio");
    type_in(&mut app, CUSTOM_RPC, "details", "Late night mix");
    type_in(&mut app, CUSTOM_RPC, "button-1-text", "Listen along");
    type_in(
        &mut app,
        CUSTOM_RPC,
        "button-1-link",
        "https://example.com/radio",
    );
    let _ = app.update(crate::app::settings(Event::Open(Section::Plugins)));
    let _ = app.update(crate::app::settings(Event::Search("custom rpc".into())));
    let _ = app.sync();
    assert!(shows(&app, "Custom RPC"));
    draw(&app, "custom-rpc");

    switch(&mut app, CUSTOM_RPC, false);
    switch(&mut app, ANIMATED_STATUS, true);
    type_in(
        &mut app,
        ANIMATED_STATUS,
        "lines",
        "Working | Playing | Away for lunch",
    );
    let _ = app.update(crate::app::settings(Event::Search("animated".into())));
    let _ = app.sync();
    assert!(shows(&app, "Animated status"));
    draw(&app, "animated-status");
    switch(&mut app, ANIMATED_STATUS, false);
}
