//! Pictures of the button plugins, drawn by the window as a person sees it,
//! in the MochaCord theme. Each test is ignored: it writes
//! `audit/plugins/buttons-<plugin>.png` (replacing the one there) for the
//! owner to look at, and checks only that the plugin's part is on screen.
//! Run them with `cargo test -p yapfast-iced --lib -- --include-ignored
//! halocord_button_pictures`.

use std::path::PathBuf;

use iced::{Settings, Size};
use iced_test::Simulator;
use yapfast_core::application::{Action, Navigation};

use super::{preview, shows};
use crate::{
    app::{
        App, Message,
        layers::{Layer, Popup},
    },
    halocord::{Change, changed, place},
    settings::{Event, Section},
};

const FRAME: Size = Size::new(1280.0, 800.0);

/// Where the pictures go.
fn audit(plugin: &str) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../audit/plugins");
    std::fs::create_dir_all(&folder).expect("the audit folder");
    folder
        .canonicalize()
        .expect("the audit folder")
        .join(format!("buttons-{plugin}.png"))
}

/// Switches a plugin on or off, as its switch on the page does.
fn turn(app: &mut App, id: &str, on: bool) {
    let plugin = place(id).expect("the plugin");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on },
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
    // Settings close, and their going is over.
    let _ = app.update(Message::Settings(Event::Close));
    let _ = app.update(Message::PopupGone(Popup::Layer(Layer::Settings)));
    app.shell.frame = FRAME;
    app.shell.own_title_bar = true;
    crate::window_bar::fit_window(&mut app);
    let _ = app.sync();
    (app, folder)
}

/// The window as drawn now, written to the plugin's picture.
fn draw(app: &App, plugin: &str) {
    let path = audit(plugin);
    // The snapshot's file carries the renderer's name.
    let written = path.with_file_name(format!("buttons-{plugin}-tiny-skia.png"));
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
fn picture_of_restart_button() {
    let (mut app, _folder) = mocha();
    turn(&mut app, crate::halocord::READ_ALL, false);
    draw(&app, "restart-button");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_read_all_button() {
    let (mut app, _folder) = mocha();
    turn(&mut app, crate::halocord::RESTART, false);
    draw(&app, "read-all-button");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_character_counter() {
    let (mut app, _folder) = mocha();
    turn(&mut app, crate::halocord::COUNTER, true);
    let words = "word ".repeat(370);
    let caret = words.len();
    crate::composer::replace_text(&mut app, crate::composer::Writing::Draft, words, caret);
    assert!(shows(&app, "1,850 / 2,000"));
    draw(&app, "character-counter");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_new_user_tag() {
    use std::sync::Arc;
    use yapfast_core::chat::ChatMessage;
    let (mut app, _folder) = mocha();
    turn(&mut app, crate::halocord::NEW_USER, true);
    // An id made a day ago, and one made years ago.
    let day = 86_400_000_u64;
    let epoch = 1_420_070_400_000_u64;
    let now = yapfast_core::clock::now_ms();
    let id_at = |ms: u64| ((ms - epoch) << 22).to_string();
    let said = |item: &str, who: &str, ms: u64, body: &str| {
        Arc::new(ChatMessage {
            item: item.into(),
            author: who.into(),
            sender: yapfast_core::discord::sender_id(&id_at(ms)),
            body: body.into(),
            ..Default::default()
        })
    };
    super::change(&mut app, |state| {
        state.messages = Arc::new(vec![
            said("i0", "Sam", now - 400 * day, "Hi, welcome to the server."),
            said(
                "i1",
                "Jules",
                now - day,
                "Hey! Is this where I get the free gift?",
            ),
        ]);
    });
    assert!(shows(&app, "NEW"));
    draw(&app, "new-user-tag");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_close_all_dms() {
    let (mut app, _folder) = mocha();
    turn(&mut app, crate::halocord::CLOSE_DMS, true);
    app.client
        .dispatch(Action::Navigate(Navigation::DirectMessages));
    super::change(&mut app, |state| {
        state.preview = false;
        state.discord_linked = true;
        state.account = Some("1".into());
    });
    let _ = app.sync();
    draw(&app, "close-all-dms");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_member_count() {
    let (mut app, _folder) = mocha();
    turn(&mut app, crate::halocord::MEMBER_COUNT, true);
    let _ = app.sync();
    app.show_roster(false);
    draw(&app, "member-count");
}
