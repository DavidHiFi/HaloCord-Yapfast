//! Pictures of build 8 to 12's features, drawn by the window in the MochaCord
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
    themed(&mut app);
    (app, folder)
}

/// `app` in the MochaCord theme, the system's bar replaced by Yapfast's.
fn themed(app: &mut App) {
    app.shell.frame.height = 2400.0;
    let _ = app.update(Message::Settings(Event::Open(Section::Appearance)));
    let _ = app.update(crate::settings::appearance_event(
        crate::settings::ThemeEvent::Choose("mochacord".into()),
    ));
    let _ = app.update(Message::Settings(Event::Close));
    let _ = app.update(Message::PopupGone(Popup::Layer(Layer::Settings)));
    app.shell.frame = FRAME;
    app.shell.own_title_bar = true;
    crate::window_bar::fit_window(app);
    let _ = app.sync();
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
    let _ = app.update(Message::Settings(Event::Open(Section::Plugins)));
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

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_build_9_plugins() {
    let (mut app, _folder) = mocha();
    turn_on(&mut app, id::INVITE_DEFAULTS);
    let _ = app.update(Message::Settings(Event::Open(Section::Plugins)));
    for (query, name) in [
        ("invite", "Invite defaults"),
        ("pending", "No pending count"),
        ("timeout", "Show timeout duration"),
    ] {
        let _ = app.update(Message::Settings(Event::Search(query.into())));
        let _ = app.sync();
        assert!(shows(&app, name), "{name}");
        draw(&app, &format!("plugin-{query}"));
    }
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_show_timeout_duration() {
    use yapfast_core::{
        application::{Action, Navigation},
        yapfast_discord::commands::USE_APPLICATION_COMMANDS,
    };
    let (mut app, _folder) = super::server::discord_server(USE_APPLICATION_COMMANDS, |_, _| {});
    app.client
        .dispatch(Action::Navigate(Navigation::Server("discord:10".into())));
    app.client
        .dispatch(Action::SelectRoom("discord:10:11".into()));
    themed(&mut app);
    turn_on(&mut app, id::SHOW_TIMEOUT);
    let until = yapfast_core::clock::now_ms() + 3 * 3_600_000 + 60_000;
    super::change(&mut app, |state| {
        let room = state
            .rooms
            .iter_mut()
            .find(|room| room.id == "discord:10:11")
            .expect("#general");
        Arc::make_mut(room).timed_out_until = Some(until);
    });
    let _ = app.sync();
    draw(&app, "show-timeout-duration");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_build_10_plugins() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    turn_on(&mut app, id::SEND_TIMESTAMPS);
    let _ = app.update(Message::Settings(Event::Open(Section::Plugins)));
    for (query, name) in [
        ("timestamps", "Send timestamps"),
        ("extensions", "Fix file extensions"),
    ] {
        let _ = app.update(Message::Settings(Event::Search(query.into())));
        let _ = app.sync();
        assert!(shows(&app, name), "{name}");
        draw(&app, &format!("plugin-{query}"));
    }
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_a_sent_timestamp() {
    use iced::{keyboard, keyboard::key, widget::Id};
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    turn_on(&mut app, id::SEND_TIMESTAMPS);
    interact(&mut app, |ui| {
        ui.click(Id::new(crate::composer::COMPOSER))
            .expect("the composer");
        ui.typewrite("Game night at <t:20:00>, bring snacks");
        ui.tap_key(keyboard::Key::Named(key::Named::Enter));
    });
    let _ = app.sync();
    let body = app
        .client
        .state()
        .messages
        .last()
        .expect("sent")
        .body
        .clone();
    assert!(body.ends_with(":t>, bring snacks"), "{body}");
    // As Discord sends it back: the preview isn't a Discord chat, so it
    // shows the time the way a Discord one does.
    newest_says(&mut app, &yapfast_core::clock::render_timestamps(&body));
    let _ = app.sync();
    draw(&app, "send-timestamps");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_plugin_menu_icons() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    turn_on(&mut app, id::COPY_MENTION);
    turn_on(&mut app, id::COPY_PROFILE_LINK);
    // A member's menu, signed in, with the Lounge's chat open.
    app.client.app = yapfast_core::application::member_server(&["30"]);
    app.client.app.state.account = Some("1".into());
    app.client.app.state.selected = Some("discord:10:12".into());
    let _ = app.sync();
    app.shell.frame = iced::Size::new(1120.0, 900.0);
    let _ = crate::person_menu::open_person_menu(
        &mut app,
        "1",
        iced::Point::new(420.0, 200.0),
        crate::person_menu::CONTEXT_MENU,
    );
    let shown = app.win.menu.as_ref().is_some_and(|menu| {
        ["Copy mention", "Copy profile link"].iter().all(|label| {
            menu.rows
                .iter()
                .any(|row| row.label == *label && row.icon.is_some())
        })
    });
    assert!(shown, "both plugin rows, with icons");
    draw(&app, "menu-icons");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_build_11_plugins() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    turn_on(&mut app, id::BETTER_JOINED_DATE);
    let _ = app.update(Message::Settings(Event::Open(Section::Plugins)));
    for (query, name) in [
        ("joined", "Better joined date"),
        ("game activity", "Game activity button"),
    ] {
        let _ = app.update(Message::Settings(Event::Search(query.into())));
        let _ = app.sync();
        assert!(shows(&app, name), "{name}");
        draw(&app, &format!("plugin-{}", query.replace(' ', "-")));
    }
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_the_game_button() {
    let (mut app, _folder) = mocha();
    let plugin = place(crate::halocord::USER_AREA).expect("the user area");
    for setting in [4, 6] {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Flag {
                plugin,
                setting,
                on: true,
            },
        );
    }
    super::change(&mut app, |state| {
        state.discord_user = Some("42".into());
        state.discord_show_game = true;
    });
    let _ = app.sync();
    assert!(app.layout.halocord.dock().game);
    draw(&app, "game-button");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_build_12_plugins() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    turn_on(&mut app, id::VIEW_ICONS);
    let _ = app.update(Message::Settings(Event::Open(Section::Plugins)));
    let _ = app.update(Message::Settings(Event::Search("icons".into())));
    let _ = app.sync();
    assert!(shows(&app, "View icons"));
    draw(&app, "plugin-icons");
}

#[test]
#[ignore = "writes pictures for a person to look at"]
fn picture_of_build_13_plugins() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    turn_on(&mut app, id::JUMP_TO_START);
    turn_on(&mut app, id::CLICK_ACTIONS);
    // The channel's top bar with the button.
    let _ = app.sync();
    assert!(crate::halocord::start_button(app.ctx()).is_some());
    draw(&app, "plugin-jump-to-start-bar");
    let _ = app.update(Message::Settings(Event::Open(Section::Plugins)));
    let _ = app.update(Message::Settings(Event::Search("jump".into())));
    let _ = app.sync();
    assert!(shows(&app, "Jump to start"));
    draw(&app, "plugin-jump-to-start");
    let _ = app.update(Message::Settings(Event::Search("click actions".into())));
    let _ = app.sync();
    assert!(shows(&app, "Message click actions"));
    draw(&app, "plugin-click-actions");
}

#[test]
#[ignore = "writes pictures for a person to look at"]
fn picture_of_build_14_plugins() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    for (plugin, search, name, picture) in [
        (
            id::CLICKABLE_ROLES,
            "clickable roles",
            "Clickable roles",
            "plugin-clickable-roles",
        ),
        (
            id::COPY_STATUS_URLS,
            "status link",
            "Copy status link",
            "plugin-copy-status-link",
        ),
        (
            id::COPY_PROFILE_COLORS,
            "profile colors",
            "Copy profile colors",
            "plugin-copy-profile-colors",
        ),
        (
            id::FIND_REPLY,
            "find reply",
            "Find reply",
            "plugin-find-reply",
        ),
    ] {
        turn_on(&mut app, plugin);
        let _ = app.update(Message::Settings(Event::Open(Section::Plugins)));
        let _ = app.update(Message::Settings(Event::Search(search.into())));
        let _ = app.sync();
        assert!(shows(&app, name), "{name}");
        draw(&app, picture);
    }
}

#[test]
#[ignore = "writes pictures for a person to look at"]
fn picture_of_build_15_plugins() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    turn_on(&mut app, id::IMAGE_ZOOM);
    let _ = app.update(Message::Settings(Event::Open(Section::Plugins)));
    let _ = app.update(Message::Settings(Event::Search("image zoom".into())));
    let _ = app.sync();
    assert!(shows(&app, "Image zoom"));
    draw(&app, "plugin-image-zoom");
}

#[test]
#[ignore = "writes pictures for a person to look at"]
fn picture_of_build_16_look() {
    let (mut app, _folder) = mocha();
    assert!(
        crate::style::rice().is_some(),
        "MochaCord draws the rice look"
    );
    app.client.app = yapfast_core::application::member_server(&["30"]);
    app.client.app.state.account = Some("1".into());
    app.client.app.state.selected = Some("discord:10:12".into());
    let _ = app.sync();
    app.shell.frame = iced::Size::new(1120.0, 900.0);
    let _ = crate::person_menu::open_person_menu(
        &mut app,
        "1",
        iced::Point::new(420.0, 200.0),
        crate::person_menu::CONTEXT_MENU,
    );
    assert!(app.win.menu.is_some(), "the member's menu");
    draw(&app, "look-menu");
}
