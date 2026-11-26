//! Pictures of the HaloCord group in Settings, drawn by the window in the
//! MochaCord theme. Each test is ignored: it writes a picture under
//! `audit/hc17/settings/` for the owner to look at. Run them with `cargo test
//! -p yapfast-iced --lib -- --include-ignored halocord_settings_pictures`.

use std::path::PathBuf;

use iced::{Settings, Size};
use iced_test::Simulator;

use super::{preview, shows};
use crate::{
    app::{
        App, Message,
        layers::{Layer, Popup},
    },
    halocord::{id, place},
    settings::{Event, HaloEvent, Section},
};

const WIDE: Size = Size::new(1280.0, 800.0);
const NARROW: Size = Size::new(820.0, 800.0);

fn audit(name: &str) -> PathBuf {
    let folder = PathBuf::from(r"D:\Agent Work\yapfast\audit\hc17\settings");
    std::fs::create_dir_all(&folder).expect("the audit folder");
    folder.join(format!("{name}.png"))
}

/// A window in the MochaCord theme at `size`, the system's bar replaced by
/// Yapfast's.
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

fn open(app: &mut App, section: Section) {
    let _ = app.update(crate::app::settings(Event::Open(section)));
    let _ = app.sync();
}

fn halo(app: &mut App, event: HaloEvent) {
    let _ = app.update(crate::app::settings(Event::HaloCord(event)));
    let _ = app.sync();
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_overview() {
    let (mut app, _folder) = mocha(WIDE);
    open(&mut app, Section::HaloCord);
    assert!(shows(&app, "THIS BUILD"));
    draw(&app, "overview");
    let (mut narrow, _folder) = mocha(NARROW);
    open(&mut narrow, Section::HaloCord);
    draw(&narrow, "overview-narrow");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_plugins() {
    let (mut app, _folder) = mocha(WIDE);
    open(&mut app, Section::Plugins);
    assert!(shows(&app, "Stereo loader"));
    draw(&app, "plugins-wide");
    let (mut narrow, _folder) = mocha(NARROW);
    open(&mut narrow, Section::Plugins);
    draw(&narrow, "plugins-narrow");
    let (mut small, _folder) = mocha(Size::new(640.0, 800.0));
    open(&mut small, Section::Plugins);
    draw(&small, "plugins-one-column");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_plugins_filtered() {
    let (mut app, _folder) = mocha(WIDE);
    open(&mut app, Section::Plugins);
    halo(&mut app, HaloEvent::Area(crate::halocord::Area::Voice));
    halo(&mut app, HaloEvent::Show(1));
    draw(&app, "plugins-voice-on");
    halo(&mut app, HaloEvent::Show(0));
    halo(&mut app, HaloEvent::Filter("zzz".into()));
    draw(&app, "plugins-empty");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_plugin_dialog() {
    let (mut app, _folder) = mocha(WIDE);
    open(&mut app, Section::Plugins);
    let plugin = place(id::STEREO).expect("the plugin");
    halo(&mut app, HaloEvent::Plugin(Some(plugin)));
    draw(&app, "plugin-dialog");
    let (mut narrow, _folder) = mocha(NARROW);
    open(&mut narrow, Section::Plugins);
    halo(&mut narrow, HaloEvent::Plugin(Some(plugin)));
    draw(&narrow, "plugin-dialog-narrow");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_themes() {
    let (mut app, _folder) = mocha(WIDE);
    open(&mut app, Section::Themes);
    draw(&app, "themes");
    let (mut narrow, _folder) = mocha(NARROW);
    open(&mut narrow, Section::Themes);
    draw(&narrow, "themes-narrow");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_mochacord_dialog() {
    let (mut app, _folder) = mocha(WIDE);
    open(&mut app, Section::Plugins);
    halo(&mut app, HaloEvent::OpenMocha);
    assert!(shows(&app, "Glass window"));
    draw(&app, "mochacord-dialog");
    let (mut narrow, _folder) = mocha(NARROW);
    open(&mut narrow, Section::Plugins);
    halo(&mut narrow, HaloEvent::OpenMocha);
    draw(&narrow, "mochacord-dialog-narrow");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_appearance() {
    let (mut app, _folder) = mocha(WIDE);
    open(&mut app, Section::Appearance);
    assert!(shows(&app, "PROFILES"));
    draw(&app, "appearance");
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_backup() {
    let (mut app, _folder) = mocha(WIDE);
    open(&mut app, Section::Backup);
    assert!(shows(&app, "Save to file"));
    draw(&app, "backup");
    halo(&mut app, HaloEvent::BackupRead(Some(Some("nope".into()))));
    draw(&app, "backup-not-a-backup");
    let (mut narrow, _folder) = mocha(NARROW);
    open(&mut narrow, Section::Backup);
    draw(&narrow, "backup-narrow");
}

/// A long build line, the way a development build with a commit and a
/// release number would show it.
fn long_build() -> crate::settings::Build {
    crate::settings::Build {
        halocord: "HaloCord for Yapfast 16".into(),
        headline: "Yapfast v0.5.0-nightly.20261107.12-extra-long".into(),
        channel: "Development build".into(),
        details: "3f086a1c9e2b4d71 on 24 November 2026".into(),
        tone: 2,
        summary: "one line".into(),
        version: "0.5.0".into(),
    }
}

#[test]
#[ignore = "writes a picture for the owner to look at"]
fn picture_of_sidebar_build_line() {
    let (mut app, _folder) = mocha(NARROW);
    app.build = long_build();
    open(&mut app, Section::HaloCord);
    assert!(shows(&app, "HaloCord for Yapfast 16"));
    draw(&app, "sidebar-build");
    // Shortest window the page allows, and the line still stays inside.
    let (mut small, _folder) = mocha(Size::new(640.0, 520.0));
    small.build = long_build();
    open(&mut small, Section::HaloCord);
    draw(&small, "sidebar-build-small");
}
