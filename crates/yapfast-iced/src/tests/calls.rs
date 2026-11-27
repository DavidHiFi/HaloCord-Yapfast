//! The call: the dock's noise toggle, the device menus' volumes and a
//! device gone away, the share menu in a short window, its settings' rows
//! with their choices beside them, the application remembered for the
//! stream's audio, the microphone's
//! level only while its menu shows, the dock's channel line, a voice
//! channel's Join, someone's volumes and menu from their tile, a camera's
//! tile before its first frame, the corner picture's grip resizing it,
//! and the call's own order under its channel.

use iced::{Point, Rectangle, Size};
use yapfast_core::{
    application::{Action, Navigation},
    noise::Suppressor,
    stream_worker::{Shareable, Source},
    voice,
};

use super::{interact, last, preview, shows};
use crate::{
    app::App,
    call::{Event, Popup},
};

/// In the preview's call in Lounge, its channel open: a click on it joins,
/// as Discord's does, and a second shows it.
fn in_call() -> (App, tempfile::TempDir, String) {
    let (mut app, folder) = preview();
    let room = "lounge".to_owned();
    app.client.dispatch(Action::SelectRoom(room.clone()));
    let _ = app.sync();
    assert!(app.client.state().voice.active(), "the preview joins");
    assert_ne!(app.client.state().selected.as_deref(), Some("lounge"));
    app.client.dispatch(Action::SelectRoom(room.clone()));
    let _ = app.sync();
    (app, folder, room)
}

/// Discord's client keeps the user's own mute apart from deafening: a
/// call joined deafened undeafens to the microphone its other preference
/// asks for, and deafening in a call leaves the mute as it was.
#[test]
fn undeafening_leaves_the_mute_as_it_was() {
    let call = |app: &App| {
        let voice = &app.client.state().voice;
        (voice.muted, voice.deafened)
    };
    let deafen = |app: &mut App| {
        let _ = app.update(crate::app::Message::Account(crate::account::Event::Deafen));
        let _ = app.sync();
    };
    for unmute in [true, false] {
        let (mut app, _folder) = preview();
        app.layout.deafen_on_join = true;
        app.layout.unmute_on_join = Some(unmute);
        app.client
            .dispatch(Action::Voice(voice::Command::Join("lounge".into())));
        // Once for the join, which deafens, and once for the deafening.
        let _ = app.sync();
        let _ = app.sync();
        assert_eq!(call(&app), (true, true), "joined deafened, and so muted");
        deafen(&mut app);
        assert_eq!(call(&app), (!unmute, false));
        deafen(&mut app);
        assert_eq!(call(&app), (true, true));
        deafen(&mut app);
        assert_eq!(call(&app), (!unmute, false), "and so each time");
    }
}

#[test]
fn the_docks_noise_toggle_turns_back_on_the_kind_last_used() {
    let (mut app, folder) = preview();
    let stored = || {
        crate::layout::Layout::read(folder.path())
            .voice
            .noise_suppression
    };
    // DeepFilterNet, as it comes, goes off and comes back.
    let _ = crate::call::update(&mut app, Event::Noise);
    assert_eq!(app.layout.voice.noise_suppression, Suppressor::Off);
    assert_eq!(stored(), Suppressor::Off, "saved");
    let _ = crate::call::update(&mut app, Event::Noise);
    assert_eq!(
        app.layout.voice.noise_suppression,
        Suppressor::DeepFilterNet
    );
    // RNNoise, picked in the input menu, is the one it brings back.
    let _ = crate::call::update(&mut app, Event::Suppressor(Suppressor::RNNoise));
    let _ = crate::call::update(&mut app, Event::Noise);
    assert_eq!(app.layout.voice.noise_suppression, Suppressor::Off);
    let _ = crate::call::update(&mut app, Event::Noise);
    assert_eq!(app.layout.voice.noise_suppression, Suppressor::RNNoise);
    assert_eq!(stored(), Suppressor::RNNoise);
    // Off picked there after RNNoise brings RNNoise back too.
    let _ = crate::call::update(&mut app, Event::Suppressor(Suppressor::Off));
    let _ = crate::call::update(&mut app, Event::Noise);
    assert_eq!(app.layout.voice.noise_suppression, Suppressor::RNNoise);
    let _ = crate::call::update(&mut app, Event::Suppressor(Suppressor::DeepFilterNet));
}

#[test]
fn a_chosen_device_that_went_away_shows_as_not_connected() {
    use crate::call::device_rows;
    let device = |id: &str, name: &str| voice::Device {
        id: id.into(),
        name: name.into(),
    };
    let both = [
        device("{speakers}", "Speakers"),
        device("{headphones}", "Headphones"),
    ];
    let (rows, index) = device_rows(&both, Some("{headphones}"));
    assert_eq!(rows, ["Speakers", "Headphones"]);
    assert_eq!(index, Some(1));
    // A device known by its name, as on PulseAudio, has it in its id.
    let (rows, index) = device_rows(&both[..1], Some("name:USB headset"));
    assert_eq!(rows, ["Speakers", "USB headset (not connected)"]);
    assert_eq!(index, Some(1));
    // The default entry's id names the default it had; it stays the default.
    let pulse = [
        device("name:default: Headset", "default: Headset"),
        device("name:Headset", "Headset"),
    ];
    let (rows, index) = device_rows(&pulse, Some("name:default: Built-in"));
    assert_eq!(rows, ["Default (Headset)", "Headset"]);
    assert_eq!(index, Some(0));
    assert_eq!(device_rows(&[], None), (vec![], None), "nothing to choose");
    assert_eq!(device_rows(&both, None).1, Some(0), "the default, first");
}

#[test]
fn call_volumes_play_while_dragged_and_are_remembered() {
    let (mut app, folder) = preview();
    // Other tests' windows play their volumes as they open.
    let _volumes = crate::layout::tests::hold_volumes();
    let stored = || crate::layout::Layout::read(folder.path()).voice;
    let _ = crate::call::update(&mut app, Event::DeviceVolume(true, 200, false));
    let _ = crate::call::update(&mut app, Event::DeviceVolume(false, 50, false));
    assert!((yapfast_core::volume::microphone() - 1.995).abs() < 1e-3);
    assert!((yapfast_core::volume::speakers() - 0.143).abs() < 1e-3);
    assert_eq!(
        stored().microphone_volume,
        None,
        "a drag's steps aren't saved"
    );
    let _ = crate::call::update(&mut app, Event::DeviceVolume(true, 200, true));
    let _ = crate::call::update(&mut app, Event::DeviceVolume(false, 50, true));
    assert_eq!(stored().microphone_volume, Some(200));
    assert_eq!(stored().speaker_volume, Some(50));
    // A new window plays at the volumes kept, and the defaults at none.
    crate::layout::Voice::default().play_volumes();
    assert_eq!(yapfast_core::volume::microphone(), 1.0);
    stored().play_volumes();
    assert!((yapfast_core::volume::speakers() - 0.143).abs() < 1e-3);
    crate::layout::Voice::default().play_volumes();
}

/// What a listing of the screens and windows says, as it comes.
fn listed(count: u64) -> Event {
    use crate::call::{Listing, ShareEvent};
    let listed = vec![Shareable {
        source: Source::Display(1),
        title: "Entire screen".into(),
        program: None,
    }];
    Event::Share(ShareEvent::Listed(
        count,
        std::sync::Arc::new(Listing(listed)),
    ))
}

#[test]
fn the_share_menu_waits_for_its_own_listing_and_a_cancel_drops_it() {
    use iced::keyboard::{Key, key};
    let (mut app, _folder, _) = in_call();
    // Escape while it's listed cancels it: what's listed after opens nothing.
    let _ = crate::call::update(&mut app, Event::Open(Popup::Share));
    let first = app.call.share_listing.expect("listing");
    let _ = app.update(crate::app::Message::Key(
        Key::Named(key::Named::Escape),
        Default::default(),
    ));
    assert_eq!(app.call.share_listing, None, "Escape cancels it");
    let _ = crate::call::update(&mut app, listed(first));
    assert_eq!(app.call.popup, None, "nothing opens after the cancel");
    // A second click on Go Live cancels it too, and a third lists anew,
    // which the first listing's late answer doesn't open.
    let _ = crate::call::update(&mut app, Event::Open(Popup::Share));
    let _ = crate::call::update(&mut app, Event::Open(Popup::Share));
    assert_eq!(app.call.share_listing, None);
    let _ = crate::call::update(&mut app, Event::Open(Popup::Share));
    let third = app.call.share_listing.expect("listing again");
    let _ = crate::call::update(&mut app, listed(first));
    assert_eq!(app.call.popup, None, "an older listing's answer");
    // The input menu opened meanwhile stays.
    let _ = crate::call::update(&mut app, Event::Open(Popup::Input));
    let _ = crate::call::update(&mut app, listed(third));
    assert_eq!(app.call.popup, Some(Popup::Input));
    // Out of the call, its listing opens nothing.
    let _ = crate::call::update(&mut app, Event::Open(Popup::Share));
    let fourth = app.call.share_listing.expect("listing");
    app.client
        .dispatch(Action::Voice(yapfast_core::voice::Command::Leave));
    let _ = app.sync();
    let _ = crate::call::update(&mut app, listed(fourth));
    assert_eq!(app.call.popup, None, "left the call meanwhile");
    // Its own answer, in the call, opens it.
    let (mut app, _folder, _) = in_call();
    let _ = crate::call::update(&mut app, Event::Open(Popup::Share));
    let count = app.call.share_listing.expect("listing");
    let _ = crate::call::update(&mut app, listed(count));
    assert_eq!(app.call.popup, Some(Popup::Share));
}

#[test]
fn the_share_menu_fits_a_short_window_and_keeps_its_button() {
    use crate::call::{Listing, ShareEvent};
    let (mut app, _folder, _) = in_call();
    app.shell.frame = Size::new(800.0, 560.0);
    crate::window_bar::fit_window(&mut app);
    // More than the window is tall: a screen, twelve windows, four cameras.
    let mut listed = vec![Shareable {
        source: Source::Display(1),
        title: "Entire screen".into(),
        program: None,
    }];
    listed.extend((0..12).map(|index| Shareable {
        source: Source::Window(index),
        title: format!("Window {index}"),
        program: None,
    }));
    listed.extend((0..4).map(|index| Shareable {
        source: Source::Camera(format!("camera{index}")),
        title: format!("Camera {index}"),
        program: None,
    }));
    // Opened from the share button near the window's foot.
    app.call.spot.set(Rectangle::new(
        Point::new(500.0, 500.0),
        Size::new(52.0, 52.0),
    ));
    let _ = crate::call::update(&mut app, Event::Open(Popup::Share));
    let count = app.call.share_listing.expect("listing");
    let _ = crate::call::update(
        &mut app,
        Event::Share(ShareEvent::Listed(
            count,
            std::sync::Arc::new(Listing(listed)),
        )),
    );
    assert_eq!(app.call.popup, Some(Popup::Share));
    let mut go = None;
    interact(&mut app, |ui| {
        let at = last(ui, "Go Live");
        go = Some(at);
        assert!(at.y < 560.0 - 8.0, "Go Live is in view, at {at:?}");
        // The settings' rows stay in view with it, over the sources,
        // which scroll.
        assert!(last(ui, "Audio").y < at.y && last(ui, "Higher quality").y < at.y);
        let screen = ui.find("Entire screen").expect("the screen");
        ui.point_at(screen.visible_bounds().expect("in view").center());
        ui.simulate(crate::widgets::swipe(-120.0));
        ui.click("Window 1").expect("a window to pick");
    });
    assert_eq!(
        app.call.share.as_ref().and_then(|share| share.pick),
        Some(2)
    );

    // Window 1 and the screen have the same numeric ID. A picture is applied
    // only to its source kind and only to the listing that requested it.
    for preview_count in [count + 1, count] {
        let _ = crate::call::update(
            &mut app,
            Event::Share(ShareEvent::Picture(
                preview_count,
                Source::Window(1),
                iced::advanced::image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            )),
        );
        let share = app.call.share.as_ref().expect("share menu");
        assert!(share.screens[0].1.is_none());
        assert_eq!(share.windows[1].1.is_some(), preview_count == count);
    }
}

/// The share menu open over a window `size` large from the share button
/// at `spot`, with `listed` to share; `applications` as on Windows, where
/// the stream's audio can be one application's.
fn share_menu(
    size: Size,
    spot: Point,
    listed: Vec<Shareable>,
    applications: bool,
) -> (App, tempfile::TempDir) {
    let (mut app, folder, _) = in_call();
    open_share_menu(&mut app, size, spot, listed, applications);
    (app, folder)
}

fn open_share_menu(
    app: &mut App,
    size: Size,
    spot: Point,
    listed: Vec<Shareable>,
    applications: bool,
) {
    app.shell.frame = size;
    crate::window_bar::fit_window(app);
    app.call
        .spot
        .set(Rectangle::new(spot, Size::new(52.0, 52.0)));
    let _ = crate::call::update(app, Event::Close);
    let _ = crate::call::update(app, Event::Open(Popup::Share));
    let count = app.call.share_listing.expect("listing");
    let listing = crate::call::Listing(listed);
    let _ = crate::call::popups::share::list(app, count, &listing, applications);
    assert_eq!(app.call.popup, Some(Popup::Share));
}

/// A screen, `windows` windows, each of a program of its own
/// (`app<index>.exe`), and a camera.
fn sources(windows: u64) -> Vec<Shareable> {
    let mut listed = vec![Shareable {
        source: Source::Display(1),
        title: "Entire screen".into(),
        program: None,
    }];
    listed.extend((0..windows).map(|index| Shareable {
        source: Source::Window(index),
        title: format!("Window {index}"),
        program: Some(format!("app{index}.exe")),
    }));
    listed.push(Shareable {
        source: Source::Camera("camera".into()),
        title: "Camera".into(),
        program: None,
    });
    listed
}

/// The pointer moved onto the text reading `text`.
fn point_at(app: &mut App, text: &str) {
    interact(app, |ui| {
        let at = last(ui, text);
        ui.point_at(at);
        ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: at,
        })]);
    });
}

/// Where the texts reading each of `texts` are, in view.
fn places(app: &App, texts: &[&str]) -> Vec<Point> {
    use iced_test::Simulator;
    let mut ui = Simulator::with_size(iced::Settings::default(), app.shell.frame, app.view());
    texts.iter().map(|text| last(&mut ui, text)).collect()
}

/// A click on the last text reading `text`: a choice beside the menu,
/// which is drawn after it.
fn click_last(app: &mut App, text: &str) {
    interact(app, |ui| {
        let at = last(ui, text);
        ui.point_at(at);
        ui.simulate(iced_test::simulator::click());
    });
}

fn side(app: &App) -> Option<crate::call::popups::share::Setting> {
    app.call.share.as_ref().and_then(|share| share.side)
}

#[test]
fn the_share_menus_settings_are_rows_that_open_their_choices_beside_them() {
    use crate::call::{ShareEvent, popups::share::Setting};
    use iced::keyboard::{Key, key};
    let spot = Point::new(300.0, 640.0);
    let (mut app, _folder) = share_menu(Size::new(1200.0, 720.0), spot, sources(3), false);
    // Each setting is one row that names what's chosen, and none of the
    // choices show.
    for shown in [
        "Audio",
        "All applications",
        "Resolution",
        "1080p",
        "Frame rate",
        "30 fps",
        "Higher quality",
        "Go Live",
    ] {
        assert!(shows(&app, shown), "{shown}");
    }
    for hidden in ["No audio", "720p", "Source", "15 fps", "60 fps"] {
        assert!(!super::present(&app, hidden), "{hidden}");
    }
    // The pointer on a row opens its choices to the right of the menu,
    // the first level with the row, and nothing in the menu moves.
    let parts = [
        "Entire screen",
        "Window 1",
        "Audio",
        "Resolution",
        "Frame rate",
        "Higher quality",
        "Go Live",
    ];
    let closed = places(&app, &parts);
    let right = spot.x - 6.0 + 496.0;
    point_at(&mut app, "Resolution");
    assert_eq!(side(&app), Some(Setting::Resolution));
    assert_eq!(places(&app, &parts), closed, "the menu stays as it was");
    let choices = places(&app, &["720p", "Source", "Resolution"]);
    assert!(choices[0].x > right, "beside the menu: {choices:?}");
    assert!((choices[0].y - choices[2].y).abs() < 3.0, "{choices:?}");
    assert!(choices[1].y > choices[0].y);
    // Another row's take their place: one list at a time.
    point_at(&mut app, "Frame rate");
    assert_eq!(side(&app), Some(Setting::FrameRate));
    assert!(!super::present(&app, "720p"));
    assert_eq!(places(&app, &parts), closed);
    let choices = places(&app, &["15 fps", "Frame rate"]);
    assert!(choices[0].x > right && (choices[0].y - choices[1].y).abs() < 3.0);
    // A choice is taken and its list closes; the row names it.
    interact(&mut app, |ui| {
        ui.click("60 fps").expect("a frame rate");
    });
    let share = app.call.share.as_ref().expect("the menu stays");
    assert_eq!((share.quality.fps, share.side), (60, None));
    assert!(shows(&app, "60 fps") && !super::present(&app, "15 fps"));
    assert_eq!(places(&app, &parts), closed);
    // A click on a row opens its choices too.
    interact(&mut app, |ui| {
        ui.click("Resolution").expect("the row");
    });
    assert_eq!(side(&app), Some(Setting::Resolution));
    interact(&mut app, |ui| {
        ui.click("Source").expect("a resolution");
    });
    let share = app.call.share.as_ref().expect("the menu stays");
    assert_eq!(share.quality.resolution, voice::Resolution::Source);
    assert_eq!(share.side, None);
    // The audio's choices: here, without an application's.
    point_at(&mut app, "Audio");
    assert_eq!(side(&app), Some(Setting::Audio));
    assert!(shows(&app, "No audio") && shows(&app, "Excludes Yapfast's own audio"));
    assert!(!super::present(&app, "Changes apply immediately"));
    // The pointer on a row without choices closes them, as picking what
    // to share does.
    point_at(&mut app, "Higher quality");
    assert_eq!(side(&app), None);
    let _ = crate::call::update(
        &mut app,
        Event::Share(ShareEvent::Side(Some(Setting::Audio))),
    );
    interact(&mut app, |ui| {
        ui.click("Window 1").expect("a window to share");
    });
    let share = app.call.share.as_ref().expect("the menu stays");
    assert_eq!((share.pick, share.side), (Some(2), None));
    // Escape closes the choices, then the menu.
    let _ = crate::call::update(
        &mut app,
        Event::Share(ShareEvent::Side(Some(Setting::Audio))),
    );
    let escape = crate::app::Message::Key(Key::Named(key::Named::Escape), Default::default());
    let _ = app.update(escape.clone());
    assert_eq!(side(&app), None);
    assert_eq!(app.call.popup, Some(Popup::Share));
    let _ = app.update(escape);
    assert_eq!(app.call.popup, None);

    // Without room on the right the choices open on the left, the menu
    // where it was.
    let spot = Point::new(900.0, 640.0);
    open_share_menu(&mut app, Size::new(1200.0, 720.0), spot, sources(3), false);
    let closed = places(&app, &parts);
    point_at(&mut app, "Frame rate");
    assert_eq!(places(&app, &parts), closed);
    let left = 1200.0 - 8.0 - 496.0;
    let choices = places(&app, &["15 fps", "Frame rate"]);
    assert!(
        choices[0].x < left,
        "left of the menu: {choices:?} {closed:?}"
    );
    assert!((choices[0].y - choices[1].y).abs() < 3.0);
}

/// A theme that dashes the menus' outlines lays the outline over the
/// panel: a setting's choices still open level with its row.
#[test]
fn the_share_menus_choices_open_beside_their_row_under_a_themes_outline() {
    use crate::call::{ShareEvent, popups::share::Setting};
    let (mut app, _folder) = super::themed(
        serde_json::json!({ "shape": { "borders": { "menu": "dashed" } } }),
        &[],
    );
    assert!(app.ctx().look.theme.outline("menu").is_some());
    app.client.dispatch(Action::SelectRoom("lounge".into()));
    let _ = app.sync();
    assert!(app.client.state().voice.active(), "the preview joins");
    let spot = Point::new(300.0, 640.0);
    open_share_menu(&mut app, Size::new(1200.0, 720.0), spot, sources(3), false);
    let _ = crate::call::update(
        &mut app,
        Event::Share(ShareEvent::Side(Some(Setting::FrameRate))),
    );
    let choices = places(&app, &["15 fps", "Frame rate"]);
    assert!(choices[0].x > spot.x - 6.0 + 496.0, "{choices:?}");
    assert!((choices[0].y - choices[1].y).abs() < 3.0, "{choices:?}");
}

#[test]
fn the_share_menus_audio_is_an_application_picked_beside_its_row() {
    use crate::call::{ShareEvent, popups::share::Setting};
    let size = Size::new(1200.0, 720.0);
    let spot = Point::new(300.0, 640.0);
    // Where the audio can be one application's, its windows follow the
    // two choices there always are.
    let (mut app, _folder) = share_menu(size, spot, sources(3), true);
    point_at(&mut app, "Audio");
    click_last(&mut app, "Window 1");
    assert_eq!(
        app.call.share.as_ref().and_then(|share| share.pick),
        Some(0),
        "its row beside the menu, not its card"
    );
    assert_eq!(
        app.client.state().voice.stream_audio,
        voice::StreamAudio::Window(1)
    );
    assert_eq!(side(&app), None);
    let named = places(&app, &["Audio", "Window 1"]);
    assert!((named[0].y - named[1].y).abs() < 3.0, "its row names it");
    // More windows than fit: the list scrolls inside the window, and the
    // menu stays where it was.
    let parts = ["Entire screen", "Audio", "Go Live"];
    open_share_menu(&mut app, size, spot, sources(40), true);
    let closed = places(&app, &parts);
    let _ = crate::call::update(
        &mut app,
        Event::Share(ShareEvent::Side(Some(Setting::Audio))),
    );
    assert_eq!(places(&app, &parts), closed);
    assert!(shows(&app, "No audio"));
    assert!(!shows(&app, "Window 39"), "the last is scrolled to");
}

#[test]
fn an_application_chosen_for_the_audio_is_found_again_or_everything_is_shared() {
    let size = Size::new(1200.0, 720.0);
    let spot = Point::new(300.0, 640.0);
    let remembered = |app: &mut App, audio| {
        super::change(app, |state| {
            state.voice.stream_audio = audio;
            state.voice.stream_app = Some("app2.exe".into());
        });
    };
    let audio = |app: &App| app.client.state().voice.stream_audio;
    let named = |app: &App, title: &str| {
        let named = places(app, &["Audio", title]);
        (named[0].y - named[1].y).abs() < 3.0
    };
    // As a new start has it: the application by its program, everything
    // shared until a window of it is listed.
    let (mut app, _folder, _) = in_call();
    remembered(&mut app, voice::StreamAudio::All);
    open_share_menu(&mut app, size, spot, sources(4), true);
    assert_eq!(audio(&app), voice::StreamAudio::Window(2));
    assert!(named(&app, "Window 2"));
    // Not running: everything is shared, and it's still the one looked
    // for the next time.
    remembered(&mut app, voice::StreamAudio::All);
    open_share_menu(&mut app, size, spot, sources(2), true);
    assert_eq!(audio(&app), voice::StreamAudio::All);
    assert!(named(&app, "All applications"));
    assert_eq!(
        app.client.state().voice.stream_app.as_deref(),
        Some("app2.exe")
    );
    // Its window closed since it was chosen: another of its windows is
    // taken, and without one the menu says it's gone.
    remembered(&mut app, voice::StreamAudio::Window(77));
    let mut listed = sources(2);
    listed.push(Shareable {
        source: Source::Window(9),
        title: "Second window".into(),
        program: Some("app2.exe".into()),
    });
    open_share_menu(&mut app, size, spot, listed, true);
    assert_eq!(audio(&app), voice::StreamAudio::Window(9));
    remembered(&mut app, voice::StreamAudio::Window(77));
    open_share_menu(&mut app, size, spot, sources(2), true);
    assert_eq!(audio(&app), voice::StreamAudio::Window(77));
    assert!(named(&app, "Unavailable application"));
    // A window of it chosen and still open stays the choice.
    remembered(&mut app, voice::StreamAudio::Window(9));
    let mut listed = sources(3);
    listed.push(Shareable {
        source: Source::Window(9),
        title: "Second window".into(),
        program: Some("app2.exe".into()),
    });
    open_share_menu(&mut app, size, spot, listed, true);
    assert_eq!(audio(&app), voice::StreamAudio::Window(9));
    // A live stream's audio isn't changed by opening the menu.
    remembered(&mut app, voice::StreamAudio::All);
    super::change(&mut app, |state| {
        state.voice.sharing = Some(voice::Sharing {
            phase: voice::SharePhase::Live,
            frames: std::sync::Arc::new(yapfast_core::frames::Frames::create(16, 16).unwrap()),
            viewers: vec![],
        });
    });
    open_share_menu(&mut app, size, spot, sources(4), true);
    assert_eq!(audio(&app), voice::StreamAudio::All);
}

#[test]
fn the_input_menu_measures_the_microphone_only_while_it_shows() {
    let (mut app, _folder) = preview();
    assert!(!crate::call::metering(&app));
    let _ = crate::call::update(&mut app, Event::Open(Popup::Input));
    assert!(crate::call::metering(&app), "the level shows");
    assert!(shows(&app, "INPUT LEVEL"));
    assert!(shows(&app, "Input Device"));
    let _ = crate::call::update(&mut app, Event::Close);
    assert!(!crate::call::metering(&app));
    let _ = crate::call::update(&mut app, Event::Open(Popup::Output));
    assert!(
        !crate::call::metering(&app),
        "the speakers' menu has no level"
    );
    assert!(shows(&app, "Output Volume"));
}

#[test]
fn the_docks_status_right_clicked_hides_and_shows_the_calls_faces() {
    let (mut app, _folder, _room) = in_call();
    assert!(!app.layout.hide_voice_faces, "shown at first, as Discord's");
    interact(&mut app, |ui| super::right_click(ui, "Preview voice"));
    assert!(shows(&app, "Show people in the call"));
    for hidden in [true, false] {
        interact(&mut app, |ui| {
            let row = last(ui, "Show people in the call");
            ui.point_at(row);
            ui.simulate(iced_test::simulator::click());
        });
        assert_eq!(app.layout.hide_voice_faces, hidden);
        assert!(
            shows(&app, "Show people in the call"),
            "a tick keeps it open"
        );
    }
}

#[test]
fn dont_show_this_again_moves_calls_without_asking_from_then_on() {
    let (mut app, _folder, _room) = in_call();
    let other = app
        .client
        .state()
        .rooms
        .iter()
        .find(|room| room.voice && room.id != "lounge")
        .map(|room| room.id.clone())
        .expect("another voice channel");
    app.client.dispatch(Action::RequestJoinVoice(other.clone()));
    let _ = app.sync();
    assert!(shows(&app, "Don't show this again"));
    interact(&mut app, |ui| {
        let tick = last(ui, "Don't show this again");
        ui.point_at(tick);
        ui.simulate(iced_test::simulator::click());
    });
    assert!(app.call.switch_ticked);
    let _ = app.update(crate::app::Message::Call(Event::Switch(true)));
    let _ = app.sync();
    assert!(app.layout.skip_switch_prompt && app.client.state().skip_switch_prompt);
    assert_eq!(
        app.client.state().voice.room.as_deref(),
        Some(other.as_str())
    );
    // Back to Lounge: no prompt.
    app.client
        .dispatch(Action::RequestJoinVoice("lounge".into()));
    let _ = app.sync();
    assert!(app.client.state().switch_voice.is_none());
    assert_eq!(app.client.state().voice.room.as_deref(), Some("lounge"));
    assert!(!shows(&app, "Don't show this again"));
}

#[test]
fn clicking_the_docks_channel_line_opens_the_call() {
    let (mut app, _folder, room) = in_call();
    app.client
        .dispatch(Action::Navigate(Navigation::DirectMessages));
    let _ = app.sync();
    assert_ne!(app.client.state().selected.as_deref(), Some(room.as_str()));
    assert!(shows(&app, "Preview voice"));
    interact(&mut app, |ui| {
        let line = super::starting(ui, "Lounge").expect("the dock's line");
        ui.point_at(line);
        ui.simulate(iced_test::simulator::click());
    });
    assert_eq!(app.client.state().selected.as_deref(), Some(room.as_str()));
    assert!(
        shows(&app, &crate::call::stage::voice_description(&app)),
        "the call shows"
    );
}

#[test]
fn a_voice_channel_this_account_isnt_in_offers_to_join() {
    let (mut app, _folder, _) = in_call();
    let _ = crate::call::update(&mut app, Event::Leave);
    let _ = app.sync();
    assert!(!app.client.state().voice.active());
    assert!(shows(&app, "Join voice"));
    let description = crate::call::stage::voice_description(&app);
    assert!(description.contains("Join and hang out."), "{description}");
    assert!(shows(&app, &description));
}

#[test]
fn someones_tile_right_clicked_sets_their_volume_and_keeps_it() {
    let (mut app, folder, room) = in_call();
    super::change(&mut app, |state| {
        let mut bob = state.voice.participants[0].clone();
        bob.id = "preview-bob".into();
        bob.person = "bob".into();
        bob.user = "Bob".into();
        bob.local = false;
        state.voice.participants.push(bob);
    });
    // One's own tile has nothing to set in the preview.
    let you = app.client.state().voice.participants[0].person.clone();
    let _ = crate::call::update(
        &mut app,
        Event::Person(room.clone(), you, Point::new(400.0, 300.0)),
    );
    assert!(app.win.menu.is_none());
    let _ = crate::call::update(
        &mut app,
        Event::Person(room, "bob".into(), Point::new(400.0, 300.0)),
    );
    let volumes = app.win.menu.as_ref().and_then(|menu| menu.volumes.as_ref());
    assert_eq!(volumes.map(|volumes| volumes.name.as_str()), Some("Bob"));
    assert!(shows(&app, "User volume"));
    assert!(!shows(&app, "Stream volume"), "Bob isn't streaming");
    let gain = || yapfast_core::volume::get(yapfast_core::volume::Kind::Voice, "bob");
    let _ = crate::call::update(&mut app, Event::Volume(false, 50, false));
    let halved = gain();
    assert!(halved < 1.0, "plays quieter while dragged: {halved}");
    let stored = || crate::layout::Layout::read(folder.path()).voice.volumes;
    assert!(stored().is_empty(), "a drag's steps aren't saved");
    let _ = crate::call::update(&mut app, Event::Volume(false, 50, true));
    assert_eq!(stored().get("bob"), Some(&halved));
    // Back at its default it's forgotten.
    let default = yapfast_core::volume::to_slider(yapfast_core::volume::Kind::Voice.default_gain());
    let _ = crate::call::update(&mut app, Event::Volume(false, default.round() as u32, true));
    assert!(stored().is_empty());
}

/// HaloCord's Volume booster: on from the start, someone's slider goes to
/// 1000% and plays that loud, and it's kept; off, it stops at 200% again,
/// and the volume kept plays once it's back.
#[test]
fn the_volume_booster_takes_someones_slider_past_200() {
    use crate::halocord::{Change, VOLUME_BOOSTER, changed, place};
    use yapfast_core::volume::{self, Kind};
    let (mut app, folder, room) = in_call();
    super::change(&mut app, |state| {
        let mut carol = state.voice.participants[0].clone();
        carol.id = "preview-carol".into();
        carol.person = "carol".into();
        carol.user = "Carol".into();
        carol.local = false;
        state.voice.participants.push(carol);
    });
    assert!(app.layout.halocord.is(VOLUME_BOOSTER), "on at first");
    let _ = crate::call::update(
        &mut app,
        Event::Person(room, "carol".into(), Point::new(400.0, 300.0)),
    );
    let _ = crate::call::update(&mut app, Event::Volume(false, 600, true));
    assert!(shows(&app, "600%"), "the slider goes that far");
    assert!(
        volume::get(Kind::Voice, "carol") > 5.0,
        "and plays that loud"
    );
    let stored = || crate::layout::Layout::read(folder.path()).voice.volumes;
    let kept = *stored().get("carol").expect("kept");
    assert!((kept - volume::from_slider(600.0)).abs() < 1e-3);
    // The most it goes to is a choice of the plugin's.
    let _alone = super::plugin_alone();
    let plugin = place(VOLUME_BOOSTER).unwrap();
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Choose {
            plugin,
            setting: 0,
            choice: 1,
        },
    );
    assert_eq!(yapfast_core::halocord::volume_cap(), 400);
    assert!((volume::get(Kind::Voice, "carol") - volume::from_slider(400.0)).abs() < 1e-3);
    // Off, Yapfast's own 200% is the most, and what was kept stays kept.
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: false },
    );
    assert_eq!(yapfast_core::halocord::volume_cap(), 200);
    assert_eq!(volume::get(Kind::Voice, "carol"), volume::MAX);
    assert_eq!(stored().get("carol"), Some(&kept));
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Choose {
            plugin,
            setting: 0,
            choice: 4,
        },
    );
    assert!((volume::get(Kind::Voice, "carol") - kept).abs() < 1e-3);
    volume::set(Kind::Voice, "carol", 1.0);
}

#[test]
fn the_signal_opens_the_connections_details_with_their_tabs() {
    use crate::call::{LinkEvent, LinkTab};
    let (mut app, _folder, _) = in_call();
    assert!(
        !crate::call::details::link_measured(app.ctx()),
        "the preview doesn't measure"
    );
    let stats = voice::LinkStats {
        pings: vec![40, 80],
        loss: Some(1.5),
        privacy_code: Some("123456789012345678901234567890".into()),
        stream: Some(voice::StreamStats {
            engine: "h264_nvenc".into(),
            codec: "H264".into(),
            width: 1920,
            height: 1080,
            fps: 60,
            target_kbps: 4000,
            sent_kbps: 3812,
            keyframe_requests: 2,
            resent: 5,
        }),
    };
    app.client.app.state.voice.link = Some(voice::Link::new(move || stats.clone()));
    let _ = app.sync();
    assert!(crate::call::details::link_measured(app.ctx()));
    // The pointer on the signal reads the ping for its tooltip.
    let _ = crate::call::update(&mut app, Event::Link(LinkEvent::Pointed(true)));
    assert!(app.call.details.stats.is_some());
    let _ = crate::call::update(&mut app, Event::Open(Popup::Link));
    for words in [
        "Average ping:",
        "60 ms",
        "Last ping:",
        "80 ms",
        "1.5%",
        "100 ms",
        "50 ms",
        "Connection",
        "Secure connection",
    ] {
        assert!(shows(&app, words), "{words}");
    }
    assert!(!shows(&app, "Privacy"), "alone, no code to compare");
    // Someone else joins to compare it with.
    let voice = &mut app.client.app.state.voice;
    let mut other = voice.participants[0].clone();
    other.id = "other".into();
    voice.participants.push(other);
    let _ = app.sync();
    interact(&mut app, |ui| {
        ui.click("Privacy").expect("the privacy tab");
    });
    assert_eq!(app.call.details.tab, LinkTab::Privacy);
    assert!(shows(&app, "Voice Privacy Code"));
    assert!(shows(&app, "67890"));
    let _ = crate::call::update(&mut app, Event::Link(LinkEvent::Copy));
    assert!(shows(&app, "Copied"));
    let _ = crate::call::update(&mut app, Event::Link(LinkEvent::CopiedDone));
    assert!(shows(&app, "Copy"));
    let _ = crate::call::update(&mut app, Event::Link(LinkEvent::Tab(LinkTab::Stream)));
    for words in [
        "NVENC on the GPU",
        "H264",
        "1920×1080 at 60 fps",
        "3.8 Mbit/s",
        "4.0 Mbit/s",
    ] {
        assert!(shows(&app, words), "{words}");
    }
    // Closed with the pointer gone, nothing is copied out of the call.
    let _ = crate::call::update(&mut app, Event::Link(LinkEvent::Pointed(false)));
    let _ = crate::call::update(&mut app, Event::Close);
    assert!(app.call.details.stats.is_none());
    assert_eq!(app.call.details.tab, LinkTab::Connection);
}

#[test]
fn the_banners_show_over_a_voice_channels_call_too() {
    let (mut app, _folder, _) = in_call();
    super::change(&mut app, |state| {
        state.notice = Some("Discord asked Yapfast to slow down.".into());
    });
    assert!(crate::call::stage::voice_panel(&app), "the call's view");
    assert!(shows(&app, "Discord asked Yapfast to slow down."));
}

#[test]
fn cameras_are_made_at_the_windows_pixels_on_a_scaled_display() {
    let (mut app, _folder, _) = in_call();
    let feed = yapfast_core::picture::Feed::new();
    let camera = feed.clone();
    super::change(&mut app, move |state| {
        state.voice.participants[0].video = Some(camera);
    });
    let (width, height) = feed.shown_at().expect("made for its tile");
    let id = app.shell.window.expect("the window");
    let _ = app.update(crate::app::Message::Shell(crate::shell::Event::Scaled(
        id, 2.0,
    )));
    let (wide, tall) = feed.shown_at().expect("made for its tile");
    assert!(
        wide.abs_diff(width * 2) <= 1 && tall.abs_diff(height * 2) <= 1,
        "{width}x{height} at 100%, {wide}x{tall} at 200%"
    );
}

#[test]
fn a_camera_without_a_frame_yet_shows_their_picture_and_name() {
    let (mut app, _folder, _) = in_call();
    // Bob's camera is on, but no frame has come.
    super::change(&mut app, |state| {
        let mut bob = state.voice.participants[0].clone();
        bob.id = "preview-bob".into();
        bob.person = "bob".into();
        bob.user = "Bob".into();
        bob.local = false;
        bob.video = Some(yapfast_core::picture::Feed::new());
        state.voice.participants.push(bob);
    });
    assert!(crate::call::stage::voice_panel(&app), "the call's view");
    // His tile has his name at its foot, over his picture, until the
    // first frame.
    assert!(shows(&app, "Bob"), "his tile's name");
}

/// Discord's picture in picture: its grip resizes it horizontally only,
/// between 200 and 960 px, and settles on the nearest 40 px when let go.
#[test]
fn the_corner_pictures_grip_resizes_it_and_the_width_is_kept() {
    use crate::app::Message;
    let (mut app, folder, _) = in_call();
    // Bob's camera stays in the corner once the call is off screen.
    super::change(&mut app, |state| {
        let mut bob = state.voice.participants[0].clone();
        bob.id = "preview-bob".into();
        bob.person = "bob".into();
        bob.local = false;
        bob.video = Some(yapfast_core::picture::Feed::new());
        state.voice.participants.push(bob);
    });
    app.client.dispatch(Action::SelectRoom("general".into()));
    let _ = app.sync();
    assert!(
        crate::call::corner::corner_view(&app).is_some(),
        "in the corner"
    );
    let width = |app: &App| crate::call::corner::corner_size(app).width;
    let size = crate::call::corner::corner_size(&app);
    assert_eq!((size.width, size.height), (320.0, 180.0), "as it comes");
    // Its grip, 90 px towards the window's middle: 90 px wider while
    // dragged, whatever the pointer does up or down, then 400 px, the
    // nearest 40, once let go.
    let grip = crate::call::corner::grip(&app).center();
    app.pressed.set(grip);
    let _ = app.update(Message::Call(Event::Grab));
    assert!(app.call.resize.is_some(), "its grip resizes it");
    for down in [0.0, 120.0, -60.0] {
        let _ = app.update(Message::Call(Event::DragTo(Point::new(
            grip.x - 90.0,
            grip.y + down,
        ))));
        assert_eq!(width(&app), 410.0, "across only, {down} px down");
    }
    let _ = app.update(Message::Call(Event::Drop));
    assert!(app.call.resize.is_none() && app.call.grab.is_none());
    assert_eq!(width(&app), 400.0, "settled on 40 px");
    assert_eq!(
        crate::layout::Layout::read(folder.path()).corner_width,
        Some(400.0),
        "kept"
    );
    // Up and down alone change nothing.
    let grip = crate::call::corner::grip(&app).center();
    app.pressed.set(grip);
    let _ = app.update(Message::Call(Event::Grab));
    let _ = app.update(Message::Call(Event::DragTo(Point::new(
        grip.x,
        grip.y + 200.0,
    ))));
    let _ = app.update(Message::Call(Event::Drop));
    assert_eq!(width(&app), 400.0, "the same");
    // Towards the window's edge it shrinks, to 200 px at least.
    let grip = crate::call::corner::grip(&app).center();
    app.pressed.set(grip);
    let _ = app.update(Message::Call(Event::Grab));
    let _ = app.update(Message::Call(Event::DragTo(Point::new(
        grip.x + 500.0,
        grip.y,
    ))));
    assert_eq!(width(&app), 200.0, "no narrower");
    let _ = app.update(Message::Call(Event::Drop));
    assert_eq!(width(&app), 200.0);
    // And to 960 px at most, 20 past it while dragged, in a window with
    // the room.
    app.shell.frame = Size::new(3000.0, 2000.0);
    crate::window_bar::fit_window(&mut app);
    let grip = crate::call::corner::grip(&app).center();
    app.pressed.set(grip);
    let _ = app.update(Message::Call(Event::Grab));
    let _ = app.update(Message::Call(Event::DragTo(Point::new(
        grip.x - 2000.0,
        grip.y,
    ))));
    assert_eq!(width(&app), 980.0, "20 px past while dragged");
    let _ = app.update(Message::Call(Event::Drop));
    assert_eq!(width(&app), 960.0, "no wider");
    // A smaller window narrows it to the room it settles on.
    app.shell.frame = Size::new(1000.0, 700.0);
    crate::window_bar::fit_window(&mut app);
    assert!(width(&app) < 960.0 && width(&app) % 40.0 == 0.0);
    // A click on its grip opens nothing, where one on the picture returns
    // to the call.
    let selected = app.client.state().selected.clone();
    app.pressed.set(crate::call::corner::grip(&app).center());
    let _ = app.update(Message::Call(Event::Grab));
    let _ = app.update(Message::Call(Event::Drop));
    assert_eq!(app.client.state().selected, selected, "no call opened");
    // Pressed on the picture instead, it's moved, not resized.
    let grip = crate::call::corner::grip(&app).center();
    app.pressed.set(Point::new(grip.x + 100.0, grip.y - 100.0));
    let _ = app.update(Message::Call(Event::Grab));
    assert!(
        app.call.grab.is_some() && app.call.resize.is_none(),
        "moved"
    );
    let _ = app.update(Message::Call(Event::Drop));
    // Its edges fall on the window's own pixels, at any zoom, so its
    // picture and the shade over it end on the same row.
    app.shell.zoom = 1.15;
    let size = crate::call::corner::corner_size(&app);
    let pixels = app.device_scale();
    for length in [size.width, size.height] {
        assert!(
            ((length * pixels).round() - length * pixels).abs() < 0.01,
            "{length} is a whole window pixel: {}",
            length * pixels
        );
    }
}

/// Sat top left, the corner picture's grip is its bottom right, over the
/// stop button's corner: a press there resizes, as the pointer says, and
/// doesn't stop the share; the button's middle still stops it.
#[test]
fn the_corner_pictures_grip_over_its_stop_button_resizes() {
    use crate::app::Message;
    let (mut app, _folder, _) = in_call();
    super::change(&mut app, |state| {
        state.voice.sharing = Some(voice::Sharing {
            phase: voice::SharePhase::Live,
            frames: std::sync::Arc::new(yapfast_core::frames::Frames::create(16, 16).unwrap()),
            viewers: vec![],
        });
    });
    app.client.dispatch(Action::SelectRoom("general".into()));
    let _ = app.sync();
    assert!(
        crate::call::corner::corner_view(&app).is_some(),
        "your screen in the corner"
    );
    app.call.left = true;
    app.call.bottom = false;
    let grip = crate::call::corner::grip(&app);
    let size = crate::call::corner::corner_size(&app);
    // The button: 32 px, 8 px in from the picture's bottom right, where
    // the grip ends.
    let corner = Point::new(grip.x + grip.width, grip.y + grip.height);
    let overlap = Point::new(corner.x - 15.0, corner.y - 15.0);
    let button = Point::new(corner.x - 24.0, corner.y - 24.0);
    assert!(grip.contains(overlap) && !grip.contains(button));
    assert!(size.width > 64.0);
    let press = |app: &App, at: Point| -> Vec<Message> {
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), app.shell.frame, app.view());
        ui.point_at(at);
        let _ = ui.simulate(iced_test::simulator::click());
        ui.into_messages().collect()
    };
    let stops = |messages: &[Message]| {
        messages
            .iter()
            .any(|message| matches!(message, Message::Call(Event::StopSharing)))
    };
    let grabs = |messages: &[Message]| {
        messages
            .iter()
            .any(|message| matches!(message, Message::Call(Event::Grab)))
    };
    let messages = press(&app, button);
    assert!(stops(&messages), "the button's middle stops the share");
    let messages = press(&app, overlap);
    assert!(
        grabs(&messages) && !stops(&messages),
        "the grip takes it, {} messages",
        messages.len()
    );
    for message in messages {
        let _ = app.update(message);
    }
    assert!(app.call.resize.is_some(), "resized, not stopped");
    assert!(app.client.state().voice.sharing.is_some());
}

#[test]
fn the_call_this_account_is_in_lists_it_first_under_its_channel() {
    let (mut app, _folder, room) = in_call();
    let person = |id: &str, name: &str| yapfast_core::chat::VoicePerson {
        id: id.into(),
        name: name.into(),
        avatar: None,
        muted: false,
        deafened: false,
        server_muted: false,
        speaking_blocked: false,
        server_deafened: false,
        streaming: false,
        video: false,
        elsewhere: false,
        style: None,
        tag: None,
    };
    // Discord lists a channel's people by name; this account is among them.
    let you = app.client.state().voice.participants[0].person.clone();
    super::change(&mut app, |state| {
        let mut bob = state.voice.participants[0].clone();
        bob.id = "preview-bob".into();
        bob.person = "bob".into();
        bob.local = false;
        state.voice.participants.push(bob);
        let lounge = state
            .rooms
            .iter_mut()
            .find(|listed| listed.id == room)
            .expect("Lounge");
        std::sync::Arc::make_mut(lounge).people = vec![
            person("ann", "Ann Listed"),
            person("bob", "Bob Listed"),
            person(&you, "Zed Listed"),
        ];
    });
    let mut at = Vec::new();
    interact(&mut app, |ui| {
        for name in ["Zed Listed", "Ann Listed", "Bob Listed"] {
            at.push(last(ui, name).y);
        }
    });
    // This account first, then the call's others in its order, then
    // anyone the call hasn't got yet.
    assert!(at[0] < at[2] && at[2] < at[1], "{at:?}");
}

#[test]
fn the_large_tile_full_screen_puts_the_window_full_screen() {
    use crate::app::Message;
    use iced::keyboard::{Key, key};
    let (mut app, _folder, _) = in_call();
    let id = app.shell.window.expect("a window");
    let _ = app.update(Message::Call(Event::Focus(Some(0))));
    assert_eq!(app.call.screen, None, "large isn't full screen");
    let _ = app.update(Message::Call(Event::Full(true)));
    assert_eq!(
        app.call.screen,
        Some((id, false)),
        "the window goes full screen"
    );
    // Escape leaves it, and the window goes back.
    let _ = app.update(Message::Key(
        Key::Named(key::Named::Escape),
        Default::default(),
    ));
    assert!(!app.call.full);
    assert_eq!(app.call.screen, None);
    // A window the desktop made full screen is left so.
    app.shell.full_screen = true;
    let _ = app.update(Message::Call(Event::Full(true)));
    assert_eq!(app.call.screen, Some((id, true)));
    // Leaving the call leaves full screen too.
    let _ = app.update(Message::Call(Event::Leave));
    assert!(!crate::call::stage::full_shown(&app));
    assert_eq!(app.call.screen, None);
}

#[test]
fn a_window_closed_with_the_large_tile_full_screen_opens_again_as_a_window() {
    use crate::app::Message;
    let (mut app, _folder, _) = in_call();
    let id = app.shell.window.expect("a window");
    let _ = app.update(Message::Call(Event::Focus(Some(0))));
    let _ = app.update(Message::Call(Event::Full(true)));
    // As the window said, once it was.
    app.shell.full_screen = true;
    // Closed, as to the tray.
    let _ = app.update(Message::Shell(crate::shell::Event::Closed(id)));
    assert!(!app.call.full, "the tile is back in the call");
    assert_eq!(app.call.screen, None);
    assert!(!app.shell.full_screen, "the next window isn't full screen");
}

#[test]
fn stream_audio_can_change_while_sharing_without_picking_a_new_source() {
    let (mut app, _folder, _) = in_call();
    super::change(&mut app, |state| {
        state.voice.sharing = Some(voice::Sharing {
            phase: voice::SharePhase::Live,
            frames: std::sync::Arc::new(yapfast_core::frames::Frames::create(16, 16).unwrap()),
            viewers: vec![],
        });
    });
    let _ = crate::call::update(&mut app, Event::Open(Popup::Share));
    let count = app.call.share_listing.expect("listing");
    let _ = crate::call::update(
        &mut app,
        Event::Share(crate::call::ShareEvent::Listed(
            count,
            std::sync::Arc::new(crate::call::Listing(vec![])),
        )),
    );
    assert_eq!(app.call.popup, Some(Popup::Share));
    assert_eq!(app.call.share.as_ref().and_then(|share| share.pick), None);
    // A live stream keeps its quality: only the audio has a row, whose
    // choices say they apply at once.
    assert!(shows(&app, "Audio") && shows(&app, "Switch"));
    for hidden in ["Resolution", "Frame rate", "Higher quality"] {
        assert!(!super::present(&app, hidden), "{hidden}");
    }
    point_at(&mut app, "Audio");
    assert!(shows(&app, "Changes apply immediately"));
    interact(&mut app, |ui| {
        ui.click("No audio").expect("a choice");
    });
    assert_eq!(
        app.client.state().voice.stream_audio,
        voice::StreamAudio::Off
    );
    assert_eq!(app.call.popup, Some(Popup::Share));
    assert_eq!(side(&app), None);
    assert!(shows(&app, "No audio"), "the row names it");
}

#[test]
fn someones_voice_menu_keeps_the_cores_order_with_an_icon_on_each_row() {
    use yapfast_core::application::{MenuChoice, member_server};
    let (mut app, _folder) = preview();
    app.client.app = member_server(&["30"]);
    app.layout.developer_mode = true;
    let _ = app.sync();
    let lounge = "discord:10:12";
    let choices = app.client.app.voice_person_choices(lounge, "4");
    assert!(choices.contains(&MenuChoice::Profile) && choices.contains(&MenuChoice::CopyId));
    let _ = crate::call::update(
        &mut app,
        Event::Person(lounge.into(), "4".into(), Point::new(400.0, 300.0)),
    );
    let menu = app.win.menu.as_ref().expect("Dave's voice menu");
    let shown: Vec<_> = menu.rows.iter().map(|row| row.label.as_ref()).collect();
    let listed: Vec<_> = choices
        .iter()
        .map(|&choice| crate::place_menus::label(choice))
        .collect();
    assert_eq!(shown, listed, "the core's order");
    for row in &menu.rows {
        assert!(row.icon.is_some(), "{} has its icon", row.label);
    }
}

/// HaloCord's Force mono: off at first, switched on it tells the core to
/// play every voice the same in both ears, and off again it stops.
#[test]
fn force_mono_starts_off_and_switches_the_core() {
    use crate::halocord::{Change, FORCE_MONO, changed, place};
    let (mut app, _folder) = super::preview();
    let _alone = super::plugin_alone();
    assert!(!app.layout.halocord.is(FORCE_MONO), "off at first");
    let mut set = |on| {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Switch {
                plugin: place(FORCE_MONO).unwrap(),
                on,
            },
        );
        app.layout.halocord.is(FORCE_MONO)
    };
    assert!(set(true));
    assert!(yapfast_core::halocord::mono());
    assert!(!set(false));
    assert!(!yapfast_core::halocord::mono());
}

/// HaloCord's Click to unmute: someone muted for this account gets a mark in
/// the voice list while the plugin is on, and a click on it unmutes them
/// and keeps that; off, no mark.
#[test]
fn click_to_unmute_marks_the_muted_and_unmutes_on_a_click() {
    use crate::halocord::{CLICK_TO_UNMUTE, Change, changed, place, unmute_mark};
    let (mut app, folder) = super::preview();
    let dana = yapfast_core::chat::VoicePerson {
        id: "unmute-dana".into(),
        name: "Dana".into(),
        ..Default::default()
    };
    assert!(app.layout.halocord.is(CLICK_TO_UNMUTE), "on at first");
    assert!(unmute_mark(app.ctx(), &dana, None).is_none(), "not muted");
    // Muted from her menu's box, as it does.
    yapfast_core::volume::mute("unmute-dana", true);
    app.layout.voice.muted_people.insert("unmute-dana".into());
    assert!(unmute_mark(app.ctx(), &dana, None).is_some(), "the mark");
    let _ = app.update(crate::app::Message::HaloCord(
        crate::halocord::Event::Unmute("unmute-dana".into()),
    ));
    assert!(!yapfast_core::volume::muted("unmute-dana"));
    assert!(!app.layout.voice.muted_people.contains("unmute-dana"));
    crate::layout::flush(folder.path());
    let kept = std::fs::read_to_string(folder.path().join("layout.json")).expect("the layout");
    assert!(!kept.contains("unmute-dana"), "kept unmuted");
    // Switched off, the mark goes even for someone muted.
    yapfast_core::volume::mute("unmute-dana", true);
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(CLICK_TO_UNMUTE).unwrap(),
            on: false,
        },
    );
    assert!(unmute_mark(app.ctx(), &dana, None).is_none(), "plugin off");
    yapfast_core::volume::mute("unmute-dana", false);
}

/// HaloCord's Voice count: off at first; on, a voice channel's row ends in
/// how many people are in it, and an empty one stays bare unless its
/// setting is on.
#[test]
fn voice_count_writes_how_many_are_in_a_voice_channel() {
    use crate::halocord::{Change, SERVER_COUNTS, VOICE_COUNT, changed, place};
    let (mut app, _folder) = super::preview();
    // Server list indicators also draws a zero in the rail, so keep this
    // test focused on the voice-channel row.
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(SERVER_COUNTS).unwrap(),
            on: false,
        },
    );
    let crowd = |count: usize| {
        (0..count)
            .map(|at| yapfast_core::chat::VoicePerson {
                id: format!("crowd-{at}"),
                name: format!("Crowd {at}"),
                ..Default::default()
            })
            .collect::<Vec<_>>()
    };
    let put = |app: &mut App, count| {
        super::change(app, |state| {
            let lounge = state
                .rooms
                .iter_mut()
                .find(|room| room.id == "lounge")
                .expect("the preview's Lounge");
            std::sync::Arc::make_mut(lounge).people = crowd(count);
        });
    };
    let switch = |app: &mut App, on| {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Switch {
                plugin: place(VOICE_COUNT).unwrap(),
                on,
            },
        );
    };
    put(&mut app, 7);
    assert!(!app.layout.halocord.is(VOICE_COUNT), "off at first");
    assert!(!super::shows(&app, "7"), "no count while off");
    switch(&mut app, true);
    assert!(super::shows(&app, "7"), "seven in the Lounge");
    put(&mut app, 0);
    assert!(!super::shows(&app, "0"), "an empty channel stays bare");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Flag {
            plugin: place(VOICE_COUNT).unwrap(),
            setting: 0,
            on: true,
        },
    );
    assert!(super::shows(&app, "0"), "0 beside an empty one when asked");
}

/// HaloCord's User voice show: off at first; on, a member who is in a voice
/// channel gets a mark that names it, and one who isn't gets none.
#[test]
fn user_voice_show_marks_members_who_are_in_a_voice_channel() {
    use crate::halocord::{Change, USER_VOICE_SHOW, changed, in_voice_mark, place, voice_room_of};
    let (mut app, _folder) = super::preview();
    super::change(&mut app, |state| {
        let lounge = state
            .rooms
            .iter_mut()
            .find(|room| room.id == "lounge")
            .expect("the preview's Lounge");
        std::sync::Arc::make_mut(lounge).people = vec![yapfast_core::chat::VoicePerson {
            id: "voice-vera".into(),
            name: "Vera".into(),
            ..Default::default()
        }];
    });
    let lounge = app
        .client
        .state()
        .rooms
        .iter()
        .find(|room| room.id == "lounge")
        .expect("Lounge")
        .name
        .clone();
    assert_eq!(
        voice_room_of(&app.client.state().rooms, "voice-vera"),
        Some(lounge.as_str())
    );
    assert_eq!(voice_room_of(&app.client.state().rooms, "nobody"), None);
    assert!(!app.layout.halocord.is(USER_VOICE_SHOW), "off at first");
    assert!(in_voice_mark(app.ctx(), "voice-vera").is_none(), "off");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(USER_VOICE_SHOW).unwrap(),
            on: true,
        },
    );
    assert!(in_voice_mark(app.ctx(), "voice-vera").is_some(), "in voice");
    assert!(in_voice_mark(app.ctx(), "nobody").is_none(), "not in voice");
}

/// HaloCord's Platform indicators: off at first; on, someone seen on one of
/// Discord's apps gets marks, and someone never seen gets none.
#[test]
fn platform_indicators_mark_people_by_the_apps_they_are_on() {
    use crate::halocord::{Change, PLATFORMS, changed, place, platform_marks};
    use yapfast_core::yapfast_discord::halocord::note;
    let (mut app, _folder) = super::preview();
    note(
        "halo-platform-pat",
        &serde_json::json!({"client_status": {"desktop": "online", "web": "dnd"}}),
    );
    assert!(!app.layout.halocord.is(PLATFORMS), "off at first");
    assert!(
        platform_marks(app.ctx(), "halo-platform-pat").is_none(),
        "off"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch {
            plugin: place(PLATFORMS).unwrap(),
            on: true,
        },
    );
    assert!(
        platform_marks(app.ctx(), "halo-platform-pat").is_some(),
        "seen"
    );
    assert!(
        platform_marks(app.ctx(), "halo-platform-nobody").is_none(),
        "unseen"
    );
}

/// Platform indicators' other places: each one's switch, off at first, puts
/// the same marks by the name in the DM list, on messages, under voice
/// channels and on profiles.
#[test]
fn platform_indicators_show_in_each_place_switched_on() {
    use crate::halocord::{Change, PLATFORMS, Place, changed, name_marks, place};
    use yapfast_core::yapfast_discord::halocord::note;
    let (mut app, _folder) = super::preview();
    note(
        "platform-places-sam",
        &serde_json::json!({"client_status": {"mobile": "idle"}}),
    );
    let plugin = place(PLATFORMS).unwrap();
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
    let places = [Place::Dms, Place::Message, Place::Voice, Place::Profile];
    for at in places {
        assert!(
            name_marks(app.ctx(), "platform-places-sam", at).is_none(),
            "{at:?} is off at first"
        );
    }
    for (setting, at) in places.into_iter().enumerate() {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Flag {
                plugin,
                setting,
                on: true,
            },
        );
        assert!(
            name_marks(app.ctx(), "platform-places-sam", at).is_some(),
            "{at:?}"
        );
        assert!(
            name_marks(app.ctx(), "platform-places-nobody", at).is_none(),
            "{at:?}: someone never seen"
        );
    }
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: false },
    );
    assert!(
        name_marks(app.ctx(), "platform-places-sam", Place::Dms).is_none(),
        "the plugin off"
    );
}

/// User voice show's switches for messages and profiles, off at first: on,
/// someone in a voice channel gets the speaker by their name on a message,
/// and "In voice: channel" on their profile.
#[test]
fn user_voice_show_marks_messages_and_profiles() {
    use crate::halocord::{Change, Place, USER_VOICE_SHOW, changed, name_marks, place};
    let (mut app, _folder) = super::preview();
    let lounge = app
        .client
        .state()
        .rooms
        .iter()
        .find(|room| room.voice)
        .expect("a voice channel")
        .clone();
    let someone = "voice-show-sam";
    super::change(&mut app, |state| {
        for room in state.rooms.iter_mut() {
            if room.id == lounge.id {
                std::sync::Arc::make_mut(room)
                    .people
                    .push(yapfast_core::chat::VoicePerson {
                        id: someone.into(),
                        name: "Sam".into(),
                        ..Default::default()
                    });
            }
        }
    });
    let plugin = place(USER_VOICE_SHOW).unwrap();
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
    for at in [Place::Message, Place::Profile] {
        assert!(
            name_marks(app.ctx(), someone, at).is_none(),
            "{at:?} is off at first"
        );
    }
    for setting in [0, 1] {
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
    assert!(name_marks(app.ctx(), someone, Place::Message).is_some());
    assert!(name_marks(app.ctx(), someone, Place::Profile).is_some());
    assert!(
        name_marks(app.ctx(), "voice-show-nobody", Place::Message).is_none(),
        "someone not in voice"
    );
}

/// Staff tag outside chat: each place's switch, off at first, puts the tag
/// by an admin's name in the member list, under voice channels and on
/// profiles; the owner gets none in the member list, which crowns them, but
/// does under voice channels; an ordinary member gets none.
#[test]
fn staff_tag_shows_in_each_place_switched_on() {
    use crate::halocord::{Change, Place, STAFF_TAG, changed, name_marks, place};
    let (mut app, _folder) = super::preview();
    app.client.app = yapfast_core::application::member_server(&["30"]);
    app.client.app.state.account = Some("1".into());
    app.client.app.state.selected = Some("discord:10:12".into());
    let _ = app.sync();
    let plugin = place(STAFF_TAG).unwrap();
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
    let places = [Place::MemberList, Place::Voice, Place::Profile];
    for at in places {
        assert!(
            name_marks(app.ctx(), "6", at).is_none(),
            "{at:?} off at first"
        );
    }
    for setting in [2, 3, 4] {
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
    for at in places {
        assert!(
            name_marks(app.ctx(), "6", at).is_some(),
            "Gus, an admin: {at:?}"
        );
        assert!(
            name_marks(app.ctx(), "3", at).is_none(),
            "Cara, no rank: {at:?}"
        );
    }
    assert!(
        name_marks(app.ctx(), "99", Place::MemberList).is_none(),
        "the owner's crown"
    );
    assert!(
        name_marks(app.ctx(), "99", Place::Voice).is_some(),
        "the owner elsewhere"
    );
}

/// New account tag outside chat: its switches for the member list and
/// profiles, off at first, put the tag by an account younger than its days,
/// and not by an old one.
#[test]
fn new_account_tag_shows_in_the_member_list_and_on_profiles() {
    use crate::halocord::{Change, NEW_USER, Place, changed, name_marks, place};
    let (mut app, _folder) = super::preview();
    // A Discord id made two days ago, and one from 2015.
    let made = yapfast_core::clock::now_ms() - 2 * 86_400_000 - 1_420_070_400_000;
    let young = (made << 22).to_string();
    let old = "100000000000000000";
    let plugin = place(NEW_USER).unwrap();
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
    for at in [Place::MemberList, Place::Profile] {
        assert!(
            name_marks(app.ctx(), &young, at).is_none(),
            "{at:?} off at first"
        );
    }
    for setting in [1, 2] {
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
    for at in [Place::MemberList, Place::Profile] {
        assert!(name_marks(app.ctx(), &young, at).is_some(), "{at:?}");
        assert!(
            name_marks(app.ctx(), old, at).is_none(),
            "an old account: {at:?}"
        );
    }
}

#[test]
fn hiding_members_collapses_the_strip_stops_hidden_cameras_and_toggles_back() {
    use crate::call::stage::MEMBERS_TOGGLE;
    use iced::widget::Id;
    use iced_test::selector::Candidate;

    let (mut app, _folder, _) = in_call();
    let bob_feed = yapfast_core::picture::Feed::new();
    let bob_camera = bob_feed.clone();
    super::change(&mut app, move |state| {
        let mut bob = state.voice.participants[0].clone();
        bob.id = "preview-bob".into();
        bob.person = "bob".into();
        bob.user = "Bob".into();
        bob.local = false;
        bob.video = Some(bob_camera);
        state.voice.participants.push(bob);
    });
    app.shell.frame = Size::new(1400.0, 760.0);
    crate::window_bar::fit_window(&mut app);
    crate::call::stage::place_videos(&mut app);

    let on_stage = |app: &App, name: &str| {
        let left = crate::conversation::chat_left(app);
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), app.shell.frame, app.view());
        let mut found = false;
        let _ = ui.find(|candidate: Candidate<'_>| {
            if let Candidate::Text {
                content,
                visible_bounds: Some(bounds),
                ..
            } = candidate
                && content == name
                && bounds.x >= left
            {
                found = true;
            }
            None::<()>
        });
        found
    };

    // Grid mode has no strip to hide.
    interact(&mut app, |ui| {
        assert!(ui.find(Id::new(MEMBERS_TOGGLE)).is_err());
    });

    // Focus the local tile: Bob sits in the bottom strip and his camera decodes.
    let _ = crate::call::update(&mut app, Event::Focus(Some(0)));
    let (_, _, before_tiles, _) = crate::call::stage::stage(&app);
    assert!(before_tiles[1].width > 0.0 && before_tiles[1].height > 0.0);
    assert!(bob_feed.shown_at().is_some(), "strip camera decodes");
    assert!(on_stage(&app, "Bob"), "Bob shows in the strip");

    let mut shown_toggle = None;
    interact(&mut app, |ui| {
        let target = ui
            .find(Id::new(MEMBERS_TOGGLE))
            .expect("Hide Members button");
        let bounds = target.visible_bounds().expect("in view");
        assert_eq!(bounds.size(), Size::new(60.0, 32.0));
        let tip_area = Rectangle::new(
            Point::new(bounds.x - 40.0, bounds.y - 44.0),
            Size::new(140.0, 38.0),
        );
        assert!(
            super::hover_draws(ui, bounds.center(), tip_area),
            "Hide Members tooltip"
        );
        shown_toggle = Some(bounds);
    });
    interact(&mut app, |ui| {
        ui.click(Id::new(MEMBERS_TOGGLE))
            .expect("click Hide Members");
    });

    assert!(crate::call::members_hidden(&app));
    let (_, _, after_tiles, _) = crate::call::stage::stage(&app);
    assert_eq!(after_tiles[1].size(), Size::ZERO);
    assert_eq!(
        bob_feed.shown_at(),
        None,
        "hidden participant camera stops decoding"
    );
    assert!(after_tiles[0].height > before_tiles[0].height);
    assert!(after_tiles[0].width > before_tiles[0].width);
    assert!(!on_stage(&app, "Bob"), "Bob is hidden from the stage");

    interact(&mut app, |ui| {
        let target = ui
            .find(Id::new(MEMBERS_TOGGLE))
            .expect("Show Members button");
        let bounds = target.visible_bounds().expect("in view");
        assert_eq!(bounds.size(), Size::new(60.0, 32.0));
        assert!(
            bounds.y > shown_toggle.unwrap().y,
            "Show Members sits just above the controls"
        );
        let tip_area = Rectangle::new(
            Point::new(bounds.x - 40.0, bounds.y - 44.0),
            Size::new(140.0, 38.0),
        );
        assert!(
            super::hover_draws(ui, bounds.center(), tip_area),
            "Show Members tooltip"
        );
    });
    interact(&mut app, |ui| {
        ui.click(Id::new(MEMBERS_TOGGLE))
            .expect("click Show Members");
    });

    assert!(!crate::call::members_hidden(&app));
    let (_, _, restored_tiles, _) = crate::call::stage::stage(&app);
    assert_eq!(restored_tiles, before_tiles);
    assert!(
        bob_feed.shown_at().is_some(),
        "participant camera resumes decoding"
    );
    assert!(on_stage(&app, "Bob"), "Bob returns to the strip");

    // The channel keeps its choice after the call is left, as Discord's
    // client keeps it while it runs.
    let _ = crate::call::update(&mut app, Event::ToggleMembers);
    assert!(crate::call::members_hidden(&app));
    let room = app.client.state().voice.room.clone().expect("in a call");
    let _ = crate::call::update(&mut app, Event::Leave);
    assert!(app.call.members_hidden.contains(&room));
}

/// A call tile's name takes its person's display name style's font, as
/// the call's voice channel lists them by user id; anyone else's is bold.
#[test]
fn a_call_tiles_name_takes_its_persons_style_font() {
    use std::sync::Arc;
    use yapfast_core::chat::{NameStyle, VoicePerson};
    let (mut app, _folder) = preview();
    super::change(&mut app, |state| {
        state.voice.room = Some("party-voice".into());
        let party = state
            .rooms
            .iter_mut()
            .find(|room| room.id == "party-voice")
            .expect("the preview's Party");
        Arc::make_mut(party).people = vec![VoicePerson {
            id: "riley".into(),
            name: "Riley".into(),
            style: Some(Arc::new(NameStyle::new(12, 2, &[0xff00ff]))),
            ..Default::default()
        }];
    });
    let riley = crate::call::stage::call_name_font(&app, "riley");
    assert_eq!(riley.family, iced::font::Family::Name("Tempo"));
    let other = crate::call::stage::call_name_font(&app, "sam");
    assert_eq!(other, crate::fonts::BOLD);
}

/// Up to nine people fit the stage under the preview's line and over the
/// controls: every one's name shows whole, none cut off at the stage's foot
/// or behind the controls, in a small window and a large one.
#[test]
fn everyone_in_a_call_of_nine_shows_by_name_over_the_controls() {
    let names = ["Ada", "Bea", "Cy", "Dot", "Eli", "Fay", "Gus", "Hal", "Ivy"];
    for (width, height) in [(900.0, 640.0), (1120.0, 760.0), (1440.0, 900.0)] {
        let (mut app, _folder, _) = in_call();
        app.shell.frame = Size::new(width, height);
        crate::window_bar::fit_window(&mut app);
        super::change(&mut app, |state| {
            let voice = &mut state.voice;
            let mut template = voice.participants[0].clone();
            template.local = false;
            template.video = None;
            template.streaming = false;
            voice.participants.truncate(1);
            for (index, name) in names.iter().enumerate().skip(1) {
                let mut person = template.clone();
                person.id = format!("person-{index}");
                person.person = person.id.clone();
                person.user = (*name).to_owned();
                voice.participants.push(person);
            }
        });
        let local = app.client.state().voice.participants[0].user.clone();
        for name in names.iter().skip(1).chain([&local.as_str()]) {
            assert!(shows(&app, name), "{name} in {width}x{height}");
        }
    }
}
