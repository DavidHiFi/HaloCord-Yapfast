//! Fake DMs and Message logger in the window: their menu rows show only on
//! the right messages, a kept deleted message still opens a menu, and (in
//! the ignored tests) pictures of the box and of the chat, written to
//! `audit/hc17/messages/<name>.png` for a look by eye.
//! Run them with `-- --ignored halocord_messages`.

use std::{path::PathBuf, sync::Arc};

use iced::Settings;
use iced_test::Simulator;
use yapfast_core::{
    chat::ChatMessage,
    halocord::{fake_dms, message_logger},
};

use super::preview;
use crate::{
    app::App,
    halocord::{Change, FAKE_DMS, MESSAGE_LOGGER, changed, message_rows, place},
};

fn switch(app: &mut App, id: &str, on: bool) {
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(id).expect("the plugin"),
            on,
        },
    );
}

fn labels(app: &App, message: &ChatMessage) -> Vec<String> {
    message_rows(app, message)
        .iter()
        .map(|row| row.label.to_string())
        .collect()
}

fn message(item: &str) -> ChatMessage {
    ChatMessage {
        item: item.into(),
        author: "Ann".into(),
        body: "hello".into(),
        ..Default::default()
    }
}

#[test]
fn the_message_plugins_rows_show_only_on_the_right_messages() {
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    switch(&mut app, FAKE_DMS, true);
    switch(&mut app, MESSAGE_LOGGER, true);

    let fake = message(&fake_dms::add("r", "1", "Ann", "hi", 100));
    let shown = labels(&app, &fake);
    assert!(shown.contains(&"Remove fake message".to_owned()));
    assert!(shown.contains(&"Clear fake messages here".to_owned()));
    assert!(!shown.contains(&"Clear the log".to_owned()));

    // A real message that nothing was kept of has neither plugin's rows.
    let plain = labels(&app, &message("i9"));
    assert!(
        !plain
            .iter()
            .any(|label| label.contains("fake") || label == "Clear the log")
    );

    // A kept deleted one and an edited one offer to clear the log.
    assert!(message_logger::keeps("Ann", false, "77"));
    let mut gone = message("d:77");
    gone.deleted = true;
    let shown = labels(&app, &gone);
    assert!(shown.contains(&"Clear the log".to_owned()));
    assert!(!shown.iter().any(|label| label.contains("fake")));
    message_logger::edited("Ann", false, "78", "before", "after");
    assert!(labels(&app, &message("d:78")).contains(&"Clear the log".to_owned()));

    // Off, neither plugin adds a row.
    switch(&mut app, FAKE_DMS, false);
    switch(&mut app, MESSAGE_LOGGER, false);
    assert!(labels(&app, &fake).is_empty());
    assert!(labels(&app, &gone).is_empty());
    message_logger::clear();
}

#[test]
fn a_kept_deleted_message_opens_a_menu_and_a_plain_deleted_one_does_not() {
    let (mut app, _folder) = preview();
    let _alone = super::plugin_alone();
    switch(&mut app, MESSAGE_LOGGER, true);
    assert!(message_logger::keeps("Ann", false, "90"));
    let mut kept = message("d:90");
    kept.deleted = true;
    let mut plain = message("d:91");
    plain.deleted = true;
    assert!(crate::halocord::kept_deleted(&kept));
    assert!(!crate::halocord::kept_deleted(&plain));
    assert!(!crate::halocord::kept_deleted(&message("d:90")));
    switch(&mut app, MESSAGE_LOGGER, false);
    message_logger::clear();
}

#[test]
fn a_fake_message_is_known_by_its_item() {
    let item = fake_dms::add("r", "1", "Ann", "hi", 100);
    assert!(crate::halocord::is_fake_row(&message(&item)));
    assert!(!crate::halocord::is_fake_row(&message("d:1")));
    fake_dms::remove(&item);
}

const SIZE: iced::Size = iced::Size::new(1120.0, 640.0);

/// Where a picture goes.
fn audit(name: &str) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../audit/hc17/messages");
    std::fs::create_dir_all(&folder).expect("the audit folder");
    folder
        .canonicalize()
        .expect("the audit folder")
        .join(format!("{name}.png"))
}

/// The window as drawn now, written to `name`'s picture.
fn draw(app: &App, name: &str) {
    let path = audit(name);
    // The snapshot's file carries the renderer's name.
    let written = path.with_file_name(format!("{name}-tiny-skia.png"));
    for old in [&path, &written] {
        let _ = std::fs::remove_file(old);
    }
    let mut ui = Simulator::with_size(Settings::default(), SIZE, app.view());
    let _ = ui
        .snapshot(&iced::Theme::Dark)
        .expect("drawn")
        .matches_image(&path);
    std::fs::rename(&written, &path).expect("the picture was written");
}

fn mocha() -> (App, tempfile::TempDir) {
    let (mut app, folder) = preview();
    assert!(app.choose_theme(crate::mochacord::THEME), "MochaCord");
    app.shell.frame = SIZE;
    (app, folder)
}

fn said(item: &str, author: &str, body: &str, time: &str, at: u64) -> Arc<ChatMessage> {
    Arc::new(ChatMessage {
        item: item.into(),
        id: item.into(),
        author: author.into(),
        body: body.into(),
        time: time.into(),
        timestamp: at,
        formatted: true,
        ..Default::default()
    })
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_the_fake_dms_box() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    switch(&mut app, FAKE_DMS, true);
    let _ = app.fake_dm.open();
    draw(&app, "fake-dms-box");
    switch(&mut app, FAKE_DMS, false);
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_a_fake_message_with_its_local_mark() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    switch(&mut app, FAKE_DMS, true);
    let item = fake_dms::add(
        "lounge",
        "2",
        "Riley",
        "I will be there in ten minutes.",
        200,
    );
    super::change(&mut app, |state| {
        state.messages = Arc::new(vec![
            said("i0", "Sam", "Are we still on for tonight?", "2:28 pm", 100),
            said(
                &item,
                "Riley",
                "I will be there in ten minutes.",
                "2:30 pm",
                200,
            ),
            said("i1", "Sam", "Great, see you then.", "2:31 pm", 300),
        ]);
    });
    draw(&app, "fake-message-local-mark");
    switch(&mut app, FAKE_DMS, false);
    fake_dms::remove(&item);
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_a_deleted_message() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    switch(&mut app, MESSAGE_LOGGER, true);
    assert!(message_logger::keeps("Sam", false, "41"));
    let mut gone = (*said(
        "d:41",
        "Sam",
        "Meet me at the north gate at nine.",
        "2:29 pm",
        150,
    ))
    .clone();
    gone.deleted = true;
    super::change(&mut app, |state| {
        state.messages = Arc::new(vec![
            said("d:40", "Riley", "Where are we meeting?", "2:28 pm", 100),
            Arc::new(gone),
            said("d:42", "Riley", "On my way.", "2:31 pm", 300),
        ]);
    });
    draw(&app, "deleted-message");
    switch(&mut app, MESSAGE_LOGGER, false);
    message_logger::clear();
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_an_edited_message_with_its_history() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    switch(&mut app, MESSAGE_LOGGER, true);
    message_logger::edited(
        "Sam",
        false,
        "50",
        "Dinner is at six.",
        "Dinner is at seven.",
    );
    message_logger::edited(
        "Sam",
        false,
        "50",
        "Dinner is at seven.",
        "Dinner is at seven, my place.",
    );
    let mut edited = (*said(
        "d:50",
        "Sam",
        "Dinner is at seven, my place.",
        "2:30 pm",
        200,
    ))
    .clone();
    edited.edited = true;
    super::change(&mut app, |state| {
        state.messages = Arc::new(vec![
            said("d:49", "Riley", "What time is dinner?", "2:28 pm", 100),
            Arc::new(edited),
            said("d:51", "Riley", "Works for me.", "2:32 pm", 300),
        ]);
    });
    draw(&app, "edited-message-history");
    switch(&mut app, MESSAGE_LOGGER, false);
    message_logger::clear();
}
