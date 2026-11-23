//! Pictures of the polish fixes, for a look by eye: the user area's plugin
//! row, an Active Now card's rounded head, and the choices that wrap. They
//! are ignored; run them with `-- --include-ignored halocord_polish_pictures`.
//! Each writes a PNG under `D:\Agent Work\yapfast\audit\hc17\polish\`.

use std::{path::PathBuf, sync::Arc};

use iced::{Point, Settings, Size, mouse};
use iced_test::Simulator;
use yapfast_core::{
    application::{Action, DmScreen, Navigation},
    chat::{Activity, ActivityKind, Friend, Presence},
};

use super::{change, preview};
use crate::{
    app::{App, Message},
    halocord::{Change, changed, place},
};

const FOLDER: &str = r"D:\Agent Work\yapfast\audit\hc17\polish";

fn path(name: &str) -> PathBuf {
    std::fs::create_dir_all(FOLDER).expect("the picture folder");
    PathBuf::from(FOLDER).join(name)
}

/// Writes `element`, drawn `size` big in the dark theme, to `name`, after
/// `touch` has worked on it, and gives the picture back to crop.
fn draw<'a>(
    element: iced::Element<'a, Message>,
    size: Size,
    name: &str,
    touch: impl FnOnce(&mut Simulator<'a, Message>),
) -> image::RgbaImage {
    let target = path(name);
    let written = target.with_file_name(name.replace(".png", "-tiny-skia.png"));
    for old in [&target, &written] {
        let _ = std::fs::remove_file(old);
    }
    let mut ui = Simulator::with_size(Settings::default(), size, element);
    touch(&mut ui);
    let _ = ui
        .snapshot(&iced::Theme::Dark)
        .expect("drawn")
        .matches_image(&target);
    std::fs::rename(&written, &target).expect("the picture was written");
    image::open(&target).expect("read back").to_rgba8()
}

/// The preview in MochaCord.
fn mocha() -> (App, tempfile::TempDir) {
    let (mut app, folder) = preview();
    assert!(app.choose_theme(crate::mochacord::THEME), "MochaCord");
    (app, folder)
}

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

/// `source`'s `area`, scaled up four times, to `name`.
fn zoom(source: &image::RgbaImage, area: (u32, u32, u32, u32), name: &str) -> image::RgbaImage {
    let (x, y, width, height) = area;
    let cropped = image::imageops::crop_imm(source, x, y, width, height).to_image();
    let big = image::imageops::resize(
        &cropped,
        width * 4,
        height * 4,
        image::imageops::FilterType::Nearest,
    );
    big.save(path(name)).expect("the zoom is written");
    big
}

#[test]
#[ignore]
fn user_panel_with_the_plugin_buttons() {
    let (mut app, _folder) = mocha();
    app.shell.own_title_bar = true;
    // Every plugin button on: fake mute, fake deafen, stats, stereo, game.
    let area = place(crate::halocord::id::USER_AREA).expect("user area");
    for setting in 2..=6 {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Flag {
                plugin: area,
                setting,
                on: true,
            },
        );
    }
    switch(&mut app, crate::halocord::id::STEREO, true);
    change(&mut app, |state| state.discord_user = Some("42".into()));
    for (wide, name) in [
        (300.0_f32, "user-panel-wide.png"),
        (220.0, "user-panel-narrow.png"),
    ] {
        let height = crate::account::full_height(app.ctx()) + crate::account::INSET;
        let _ = draw(
            crate::account::view(app.ctx()),
            Size::new(wide, height),
            name,
            |_| {},
        );
    }
}

fn playing(name: &str) -> Activity {
    Activity {
        kind: ActivityKind::Playing,
        name: name.into(),
        details: Some("Spring, day 12".into()),
        state: Some("Farming".into()),
        party: None,
        party_id: None,
        application_id: Some("stardew".into()),
        start: None,
        end: None,
        large_image: None,
        large_text: None,
        small_image: None,
        small_text: None,
        url: None,
        link: None,
        emoji: None,
    }
}

fn friend(id: &str, name: &str, activity: Activity) -> Friend {
    Friend {
        id: id.into(),
        name: name.into(),
        decoration: None,
        nickname: None,
        avatar: None,
        presence: Some(Presence::Online),
        activity: None,
        activities: Arc::from([activity]),
        since: None,
    }
}

#[test]
#[ignore]
fn active_now_head_follows_the_card_round() {
    let (mut app, _folder) = mocha();
    change(&mut app, |state| {
        state.friends = vec![friend("1", "Ann", playing("Stardew Valley"))];
    });
    let size = Size::new(1500.0, app.shell.frame.height);
    let _ = app.update(Message::Shell(crate::shell::Event::Resized(size)));
    app.client
        .dispatch(Action::Navigate(Navigation::DirectMessages));
    app.client.dispatch(Action::OpenDmScreen(DmScreen::Friends));
    let _ = app.sync();
    assert!(!app.active_now.is_empty(), "Ann's card is made");

    let panel = Size::new(crate::active_now::WIDTH, 260.0);
    // The head sits 13 + 20 + 10 px down and 17 px in.
    let head = Point::new(17.0 + 150.0, 13.0 + 24.0 + 10.0 + 34.0);
    for (name, state) in [
        ("active-now-rest.png", 0),
        ("active-now-hover.png", 1),
        ("active-now-selected.png", 2),
    ] {
        let picture = draw(crate::active_now::view(app.ctx()), panel, name, |ui| {
            if state > 0 {
                ui.point_at(head);
                let _ = ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved {
                    position: head,
                })]);
            }
            if state > 1 {
                let _ = ui.simulate([iced::Event::Mouse(mouse::Event::ButtonPressed(
                    mouse::Button::Left,
                ))]);
            }
        });
        // The card's two top corners, 40 px square each, four times big.
        // The picture is drawn at twice the panel's size.
        let left = 34 - 8;
        let top = 2 * (13 + 24 + 10) - 8;
        let right = picture.width() - 2 * 16 - 32;
        let _ = zoom(
            &picture,
            (left, top, 40, 40),
            &name.replace(".png", "-left.png"),
        );
        let _ = zoom(
            &picture,
            (right, top, 40, 40),
            &name.replace(".png", "-right.png"),
        );
    }
}

#[test]
#[ignore]
fn choices_wrap_and_fit() {
    let (app, _folder) = mocha();
    let theme = app.ctx().look.theme;
    let rates = ["64", "128", "256", "384", "510"];
    let long = [
        "30 minutes",
        "1 hour",
        "6 hours",
        "12 hours",
        "1 day",
        "7 days",
        "Never",
    ];
    for (width, name) in [
        (520.0_f32, "choices-wide.png"),
        (230.0, "choices-narrow.png"),
    ] {
        let view = iced::widget::column![
            crate::controls::choices(&theme, &rates, 1, |index| {
                let _ = index;
                Message::Minute
            }),
            crate::controls::choices(&theme, &long, 2, |index| {
                let _ = index;
                Message::Minute
            }),
        ]
        .spacing(12)
        .padding(12)
        .width(width);
        let _ = draw(view.into(), Size::new(width, 190.0), name, |_| {});
    }
}
