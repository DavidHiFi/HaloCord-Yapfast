//! A short walk through the new plugins, drawn by the window as a person sees
//! it, in the MochaCord theme: the plugins page, switches turned on, a draft
//! counted as it is typed, the member count, and Copy mention on a member's
//! menu. Ignored: it writes numbered frames to `audit/release3/demo/` for a
//! demo video. Run it with `cargo test -p yapfast-iced --lib -- --ignored
//! halocord_demo_frames`.

use std::path::PathBuf;

use iced::{Point, Settings, Size};
use iced_test::Simulator;

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

struct Reel {
    folder: PathBuf,
    count: usize,
}

impl Reel {
    fn new() -> Self {
        Self::into("release3")
    }

    /// A reel writing to `audit/<release>/demo/`, emptied first.
    fn into(release: &str) -> Self {
        let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../../audit/{release}/demo"));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("the demo folder");
        Self {
            folder: folder.canonicalize().expect("the demo folder"),
            count: 0,
        }
    }

    /// The window as drawn now, as the next frame.
    fn frame(&mut self, app: &App, name: &str) {
        self.count += 1;
        let path = self.folder.join(format!("{:02}-{name}.png", self.count));
        let written = path.with_file_name(format!("{:02}-{name}-tiny-skia.png", self.count));
        let mut ui = Simulator::with_size(Settings::default(), app.shell.frame, app.view());
        let _ = ui
            .snapshot(&iced::Theme::Dark)
            .expect("drawn")
            .matches_image(&path);
        std::fs::rename(&written, &path).expect("the frame was written");
    }
}

fn turn(app: &mut App, id: &str, on: bool) {
    let plugin = place(id).expect("the plugin");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on },
    );
}

fn mocha() -> (App, tempfile::TempDir) {
    let (mut app, folder) = preview();
    app.shell.frame.height = 2400.0;
    let _ = app.update(crate::app::settings(Event::Open(Section::Appearance)));
    let _ = app.update(crate::settings::appearance_event(
        crate::settings::ThemeEvent::Choose("mochacord".into()),
    ));
    close_settings(&mut app);
    app.shell.frame = FRAME;
    app.shell.own_title_bar = true;
    crate::window_bar::fit_window(&mut app);
    let _ = app.sync();
    (app, folder)
}

fn close_settings(app: &mut App) {
    let _ = app.update(crate::app::settings(Event::Close));
    let _ = app.update(Message::PopupGone(Popup::Layer(Layer::Settings)));
}

#[test]
#[ignore = "writes demo frames for the owner to look at"]
fn halocord_demo_frames() {
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    let mut reel = Reel::new();
    reel.frame(&app, "window");

    let _ = app.update(crate::app::settings(Event::Open(Section::Plugins)));
    let _ = app.sync();
    reel.frame(&app, "plugins-page");
    for (query, ids) in [
        (
            "count",
            &[crate::halocord::COUNTER, crate::halocord::MEMBER_COUNT][..],
        ),
        ("mention", &["copy-user-mention"][..]),
    ] {
        let mut typed = String::new();
        for letter in query.chars() {
            typed.push(letter);
            let _ = app.update(crate::app::settings(Event::Search(typed.clone())));
        }
        let _ = app.sync();
        reel.frame(&app, &format!("search-{query}"));
        for id in ids {
            turn(&mut app, id, true);
            let _ = app.sync();
            reel.frame(&app, id);
        }
    }
    let _ = app.update(crate::app::settings(Event::Search(String::new())));
    close_settings(&mut app);
    let _ = app.sync();
    app.show_roster(false);
    reel.frame(&app, "member-count");

    let mut draft = String::new();
    for (step, words) in [
        (1, "Anyone up for co-op tonight? "),
        (2, "I can be on after nine. "),
    ] {
        draft.push_str(words);
        let caret = draft.len();
        crate::composer::replace_text(
            &mut app,
            crate::composer::Writing::Draft,
            draft.clone(),
            caret,
        );
        reel.frame(&app, &format!("typing-{step}"));
    }
    for (step, size) in [(3, 1_850), (4, 2_040)] {
        let words = "word ".repeat(size / 5);
        let caret = words.len();
        crate::composer::replace_text(&mut app, crate::composer::Writing::Draft, words, caret);
        reel.frame(&app, &format!("typing-{step}"));
    }
    assert!(shows(&app, "2,040 / 2,000"));

    // A member's menu, signed in, with Copy mention at its foot.
    app.client.app = yapfast_core::application::member_server(&["30"]);
    app.client.app.state.account = Some("1".into());
    app.client.app.state.selected = Some("discord:10:12".into());
    let _ = app.sync();
    app.shell.frame = FRAME;
    let _ = crate::person_menu::open_person_menu(
        &mut app,
        "1",
        Point::new(420.0, 200.0),
        crate::person_menu::CONTEXT_MENU,
    );
    assert!(
        app.win
            .menu
            .as_ref()
            .is_some_and(|menu| menu.rows.iter().any(|row| row.label == "Copy mention"))
    );
    reel.frame(&app, "copy-mention");
}

/// Build 4's two plugins: Platform indicators found and switched on, the
/// marks in the member list, then Copy file text on a file's menu. Writes to
/// `audit/release4/demo/`.
#[test]
#[ignore = "writes demo frames for the owner to look at"]
fn halocord_demo4_frames() {
    use serde_json::json;
    use yapfast_core::yapfast_discord::halocord::note;
    let (mut app, _folder) = mocha();
    let _alone = super::plugin_alone();
    let mut reel = Reel::into("release4");
    app.show_roster(false);
    let _ = app.sync();
    reel.frame(&app, "window");

    let _ = app.update(crate::app::settings(Event::Open(Section::Plugins)));
    let _ = app.sync();
    for (query, id) in [
        ("platform", crate::halocord::PLATFORMS),
        ("file text", crate::halocord::COPY_FILE_TEXT),
    ] {
        let mut typed = String::new();
        for letter in query.chars() {
            typed.push(letter);
            let _ = app.update(crate::app::settings(Event::Search(typed.clone())));
        }
        let _ = app.sync();
        reel.frame(&app, &format!("search-{}", query.replace(' ', "-")));
        turn(&mut app, id, true);
        let _ = app.sync();
        reel.frame(&app, id);
    }
    let _ = app.update(crate::app::settings(Event::Search(String::new())));
    close_settings(&mut app);

    for (user, apps) in [
        (
            "@riley:preview.local",
            json!({"desktop": "online", "mobile": "idle"}),
        ),
        ("@sam:preview.local", json!({"web": "dnd"})),
        ("@you:preview.local", json!({"mobile": "online"})),
    ] {
        note(user, &json!({ "client_status": apps }));
    }
    let _ = app.sync();
    reel.frame(&app, "platform-marks");

    super::halocord_menu_pictures::a_files_menu(&mut app);
    assert!(
        app.win
            .menu
            .as_ref()
            .is_some_and(|menu| menu.rows.iter().any(|row| row.label == "Copy file text"))
    );
    reel.frame(&app, "copy-file-text");
}

/// Build 7's themes: the Appearance tiles, three of the new themes on the
/// whole window, and the theme editor with its file buttons. Writes to
/// `audit/release7/demo/`.
#[test]
#[ignore = "writes theme pictures for the owner to look at"]
fn halocord_theme_frames() {
    let (mut app, _folder) = preview();
    let mut reel = Reel::into("release7");
    app.shell.frame = Size::new(1280.0, 1400.0);
    let _ = app.update(crate::app::settings(Event::Open(Section::Appearance)));
    let _ = app.sync();
    reel.frame(&app, "appearance-tiles");
    for id in ["tokyo-night", "rose-pine", "catppuccin-latte"] {
        let _ = app.update(crate::settings::appearance_event(
            crate::settings::ThemeEvent::Choose(id.into()),
        ));
        let _ = app.sync();
        reel.frame(&app, &format!("appearance-{id}"));
    }
    let _ = app.update(crate::settings::appearance_event(
        crate::settings::ThemeEvent::Choose("mochacord".into()),
    ));
    let _ = app.update(crate::settings::appearance_event(
        crate::settings::ThemeEvent::New,
    ));
    let _ = app.sync();
    assert!(
        shows(&app, "Save to file"),
        "the editor has the file buttons"
    );
    reel.frame(&app, "theme-editor");
    for id in ["kanagawa", "everforest"] {
        let _ = app.update(crate::settings::appearance_event(
            crate::settings::ThemeEvent::Cancel,
        ));
        let _ = app.update(crate::settings::appearance_event(
            crate::settings::ThemeEvent::Choose(id.into()),
        ));
        close_settings(&mut app);
        app.shell.frame = FRAME;
        let _ = app.sync();
        reel.frame(&app, &format!("window-{id}"));
        let _ = app.update(crate::app::settings(Event::Open(Section::Appearance)));
        app.shell.frame = Size::new(1280.0, 1400.0);
    }
}
