//! Pictures of the right-click plugins' rows, drawn by the window as a person
//! sees them. Each test is ignored: it writes `audit/plugins/menus-<plugin>.png`
//! (replacing the one there) for the owner to look at, and checks nothing.
//! Run one with `cargo test -p yapfast-iced --lib -- --ignored <name>`.

use std::path::PathBuf;

use iced::{Point, Settings};
use iced_test::Simulator;

use super::preview;
use crate::{
    app::App,
    halocord::{Change, changed, place},
};

/// Where the pictures go.
fn audit(plugin: &str) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../audit/plugins");
    std::fs::create_dir_all(&folder).expect("the audit folder");
    folder
        .canonicalize()
        .expect("the audit folder")
        .join(format!("menus-{plugin}.png"))
}

/// Switches a plugin on, as its switch on the page does.
fn turn_on(app: &mut App, id: &str) {
    let plugin = place(id).expect("the plugin");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
}

/// The window as drawn now, written to the plugin's picture.
fn draw(app: &App, plugin: &str) {
    let path = audit(plugin);
    // The snapshot's file carries the renderer's name.
    let written = path.with_file_name(format!("menus-{plugin}-tiny-skia.png"));
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

/// The member list's menu on `user`, signed in, with the Lounge's chat open.
fn a_members_menu(app: &mut App, user: &str) {
    app.client.app = yapfast_core::application::member_server(&["30"]);
    app.client.app.state.account = Some("1".into());
    app.client.app.state.selected = Some("discord:10:12".into());
    let _ = app.sync();
    // Tall enough for the whole menu, the plugin's row at its foot.
    app.shell.frame = iced::Size::new(1120.0, 900.0);
    let _ = crate::person_menu::open_person_menu(
        app,
        user,
        Point::new(420.0, 200.0),
        crate::person_menu::CONTEXT_MENU,
    );
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_copy_user_mention() {
    let (mut app, _folder) = preview();
    turn_on(&mut app, "copy-user-mention");
    a_members_menu(&mut app, "1");
    assert!(
        app.win
            .menu
            .as_ref()
            .is_some_and(|menu| menu.rows.iter().any(|row| row.label == "Copy mention"))
    );
    draw(&app, "copy-user-mention");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_copy_user_urls() {
    let (mut app, _folder) = preview();
    turn_on(&mut app, "copy-user-urls");
    a_members_menu(&mut app, "1");
    assert!(
        app.win
            .menu
            .as_ref()
            .is_some_and(|menu| menu.rows.iter().any(|row| row.label == "Copy profile link"))
    );
    draw(&app, "copy-user-urls");
}

/// A message with a file, `notes.txt`, right-clicked in the preview's chat.
fn a_files_menu(app: &mut App) {
    use std::sync::Arc;
    use yapfast_core::chat::{Attachment, ChatMessage, Placement};
    super::change(app, |state| {
        state.messages = Arc::new(vec![Arc::new(ChatMessage {
            item: "i0".into(),
            author: "Sam".into(),
            body: "Here are the notes.".into(),
            attachments: vec![Attachment {
                name: "notes.txt".into(),
                size: Some(2048),
                image: false,
                dimensions: None,
                preview: None,
                preview_failed: false,
                placement: Placement::Message,
                spoiler: false,
                video: false,
                gif: None,
                url: Some("https://cdn.example/notes.txt".into()),
                description: None,
            }],
            ..Default::default()
        })]);
    });
    right_click_on(app, "notes.txt");
}

/// A right click on the text reading `text`, then what it sent.
fn right_click_on(app: &mut App, text: &str) {
    use iced::{Event, mouse};
    let messages: Vec<crate::app::Message> = {
        let mut ui = Simulator::with_size(Settings::default(), app.shell.frame, app.view());
        let found = ui.find(text).expect("the text is there");
        ui.point_at(found.visible_bounds().expect("in view").center());
        ui.simulate([
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
        ]);
        ui.into_messages().collect()
    };
    for message in messages {
        let _ = app.update(message);
    }
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_copy_media_link() {
    let (mut app, _folder) = preview();
    turn_on(&mut app, "copy-media-link");
    a_files_menu(&mut app);
    assert!(
        app.win
            .menu
            .as_ref()
            .is_some_and(|menu| menu.rows.iter().any(|row| row.label == "Copy file link"))
    );
    draw(&app, "copy-media-link");
}
