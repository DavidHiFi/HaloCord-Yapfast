//! The look fixes: a plugin card shows its whole description, a theme card
//! keeps its long name inside, and the sidebar and search box take the
//! MochaCord look.

use iced::{Rectangle, Settings, Size};
use iced_test::{Simulator, selector::Candidate};

use super::{preview, shows};
use crate::{
    app::App,
    halocord::PLUGINS,
    settings::{Event, Section},
};

/// Settings on a section, in a window `size` big enough to show the whole page.
fn open(app: &mut App, section: Section, size: Size) {
    app.shell.frame = size;
    crate::window_bar::fit_window(app);
    let _ = app.update(crate::app::settings(Event::Open(section)));
    let _ = app.sync();
}

/// Each text in view that reads `wanted`, as its box and the part of it shown.
fn texts(app: &App, wanted: impl Fn(&str) -> bool + Sync) -> Vec<(Rectangle, Rectangle)> {
    let mut ui = Simulator::with_size(Settings::default(), app.shell.frame, app.view());
    let mut found = Vec::new();
    let _ = ui.find(|candidate: Candidate<'_>| {
        if let Candidate::Text {
            content,
            bounds,
            visible_bounds: Some(shown),
            ..
        } = candidate
            && wanted(content)
        {
            found.push((bounds, shown));
        }
        None::<()>
    });
    found
}

/// The widest box that touches the window's left edge and its bottom.
fn foot_dock(app: &App) -> Rectangle {
    let window = Rectangle::with_size(app.shell.frame);
    let mut ui = Simulator::with_size(Settings::default(), window.size(), app.view());
    let mut widest = Rectangle::new(iced::Point::ORIGIN, Size::ZERO);
    let _ = ui.find(|candidate: Candidate<'_>| {
        if let Candidate::Container {
            bounds,
            visible_bounds: Some(_),
            ..
        } = candidate
            && bounds.x.abs() < 0.5
            && (bounds.y + bounds.height - window.height).abs() < 0.5
            && bounds.width > widest.width
            && bounds.width < window.width - 1.0
        {
            widest = bounds;
        }
        None::<()>
    });
    widest
}

#[test]
fn mochacords_user_dock_fills_the_foot_and_compact_tightens_it() {
    let (mut app, _folder) = preview();
    assert!(app.choose_theme(crate::mochacord::THEME), "MochaCord");
    app.shell.frame = Size::new(1100.0, 700.0);
    crate::window_bar::fit_window(&mut app);
    let _ = app.sync();
    let navigation = crate::rail::WIDTH + crate::sidebar::WIDTH;
    let roomy = foot_dock(&app);
    assert!(
        roomy.width >= navigation - 1.0,
        "the dock runs the navigation's whole width: {roomy:?} of {navigation}"
    );
    // Compact: the same full width, and a shorter dock.
    let tall = crate::account::full_height(app.ctx());
    crate::halocord::changed(
        &mut app.layout,
        &mut app.client,
        crate::halocord::Change::Flag {
            plugin: crate::halocord::place(crate::halocord::USER_AREA).expect("user area"),
            setting: 0,
            on: true,
        },
    );
    super::change(&mut app, |_| {});
    let compact = foot_dock(&app);
    assert!(compact.width >= navigation - 1.0, "{compact:?}");
    assert!(
        crate::account::full_height(app.ctx()) <= tall,
        "compact never grows the card"
    );
    assert!(compact.height <= roomy.height, "{compact:?} vs {roomy:?}");
}

#[test]
fn a_plugin_card_shows_its_whole_description() {
    for width in [1280.0, 820.0, 640.0] {
        let (mut app, _folder) = preview();
        open(&mut app, Section::Plugins, Size::new(width, 9000.0));
        let abouts: Vec<&str> = PLUGINS.iter().map(|plugin| plugin.about).collect();
        let found = texts(&app, |content| abouts.contains(&content));
        assert!(!found.is_empty(), "{width}: plugin cards are listed");
        for (bounds, shown) in found {
            assert!(
                shown.height >= bounds.height - 0.5 && shown.width >= bounds.width - 0.5,
                "{width}: a description is cut off: {bounds:?} shows {shown:?}"
            );
        }
    }
}

#[test]
fn a_theme_card_keeps_its_long_name_inside() {
    let (mut app, _folder) = preview();
    open(&mut app, Section::Appearance, Size::new(1280.0, 2400.0));
    let _ = app.sync();
    // The card is 150 px wide with its name area 38 px tall.
    let found = texts(&app, |content| content == "Catppuccin Macchiato");
    assert!(!found.is_empty(), "the Macchiato card is listed");
    for (bounds, shown) in found {
        assert!(bounds.width <= 150.0, "the name fits the card: {bounds:?}");
        assert!(bounds.height <= 38.0, "the name fits its area: {bounds:?}");
        assert!(
            shown.width >= bounds.width - 0.5 && shown.height >= bounds.height - 0.5,
            "the name isn't cut off: {bounds:?} shows {shown:?}"
        );
    }
    assert!(shows(&app, "Catppuccin Macchiato"));
}
