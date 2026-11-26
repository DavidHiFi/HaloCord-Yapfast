//! Regression checks for the compact MochaCord user dock.

use iced::{Rectangle, Settings, Size};
use iced_test::{Simulator, selector::Candidate};

use super::preview;
use crate::{
    app::App,
    halocord::{Change, USER_AREA, changed, place},
};

/// The widest box touching the window's left and bottom edges.
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
fn mochacord_user_dock_fills_the_foot_and_compact_tightens_it() {
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
    let tall = crate::account::full_height(app.ctx());
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Flag {
            plugin: place(USER_AREA).expect("user area"),
            setting: 0,
            on: true,
        },
    );
    let _ = app.sync();
    let compact = foot_dock(&app);
    assert!(compact.width >= navigation - 1.0, "{compact:?}");
    assert!(crate::account::full_height(app.ctx()) <= tall);
    assert!(compact.height <= roomy.height, "{compact:?} vs {roomy:?}");
}
