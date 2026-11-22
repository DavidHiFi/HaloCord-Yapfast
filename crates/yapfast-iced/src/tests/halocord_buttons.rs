//! HaloCord's button plugins (restart, read all, close all DMs, the
//! character counter, the new user tag, the member count) driven through
//! the window.

use super::preview;
use crate::{
    app::Message,
    halocord::{Change, Press, RESTART, changed, place, presses},
    window_bar::Event,
};

fn switch(app: &mut crate::app::App, id: &str, on: bool) {
    let plugin = place(id).expect("the plugin");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on },
    );
}

#[test]
fn the_buttons_are_on_from_the_start_and_go_when_switched_off() {
    let (mut app, _folder) = preview();
    let labels = |app: &crate::app::App| -> Vec<&'static str> {
        presses(&app.layout.halocord)
            .into_iter()
            .map(|(_, label, _)| label)
            .collect()
    };
    assert_eq!(
        labels(&app),
        ["Restart Yapfast"],
        "the owner's own, on at first"
    );
    switch(&mut app, RESTART, false);
    assert!(labels(&app).is_empty());
}

#[test]
fn the_bar_draws_the_buttons_and_the_user_area_does_when_the_bar_is_the_systems() {
    let (mut app, _folder) = preview();
    app.shell.own_title_bar = true;
    crate::window_bar::fit_window(&mut app);
    assert!(
        crate::window_bar::window_bar(&app).is_some(),
        "the bar draws with them"
    );
    app.shell.own_title_bar = false;
    crate::window_bar::fit_window(&mut app);
    let _ = app.view();
}

#[test]
fn restart_asks_the_window_to_relaunch() {
    let (mut app, _folder) = preview();
    let _ = app.update(Message::Bar(Event::Halo(Press::Restart)));
    let seen = crate::EXIT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    crate::exit_for(crate::Exit::Closed);
    assert_eq!(seen, crate::Exit::Restart, "it leaves to start again");
}
