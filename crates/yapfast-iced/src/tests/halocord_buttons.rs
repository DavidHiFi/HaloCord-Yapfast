//! HaloCord's button plugins (restart, read all, close all DMs, the
//! character counter, the new user tag, the member count) driven through
//! the window.

use yapfast_core::{
    application::{Action, Effect},
    chat::Command,
};

use super::{change, preview};
use crate::{
    app::Message,
    halocord::{Change, Press, READ_ALL, RESTART, changed, place, presses},
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
        ["Restart Yapfast", "Mark everything read"],
        "both are the owner's own, on at first"
    );
    switch(&mut app, RESTART, false);
    assert_eq!(labels(&app), ["Mark everything read"]);
    switch(&mut app, READ_ALL, false);
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

#[test]
fn read_all_marks_each_server_and_unread_chat_and_only_those() {
    let (mut app, _folder) = preview();
    let (server, channel) = {
        let state = app.client.state();
        let server = state
            .servers
            .iter()
            .find(|server| !server.channels.is_empty())
            .expect("a server with channels");
        (server.id.clone(), server.channels[0].clone())
    };
    change(&mut app, |state| {
        state.preview = false;
        let room = state
            .rooms
            .iter_mut()
            .map(std::sync::Arc::make_mut)
            .find(|room| room.id == channel)
            .expect("the channel");
        room.unread = 3;
    });
    let actions = crate::halocord::read_all_actions(app.client.state());
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Action::MarkRead(id) if *id == server)),
        "the server with an unread channel"
    );
    let effects: Vec<Effect> = actions
        .into_iter()
        .flat_map(|action| app.client.app.dispatch(action))
        .collect();
    let marked: Vec<&str> = effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Backend(Command::MarkRead { room, .. }) => Some(room.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(marked, [channel.as_str()], "only the unread channel");
}

#[test]
fn the_counter_reads_with_commas_and_turns_amber_then_red() {
    use crate::halocord::{Fill, counter_words};
    assert_eq!(
        counter_words(1234, 2000),
        ("1,234 / 2,000".into(), Fill::Fine)
    );
    assert_eq!(counter_words(1800, 2000).1, Fill::Near, "nine tenths");
    assert_eq!(
        counter_words(2000, 2000).1,
        Fill::Near,
        "at the limit is still allowed"
    );
    assert_eq!(counter_words(2001, 2000).1, Fill::Over);
    assert_eq!(counter_words(5, 4000).0, "5 / 4,000");
}

#[test]
fn the_counter_is_off_at_first_and_shows_only_for_a_draft() {
    use crate::halocord::{COUNTER, count_words};
    let (mut app, _folder) = preview();
    assert!(
        count_words(&app.layout.halocord, "hello", false).is_none(),
        "off at first"
    );
    switch(&mut app, COUNTER, true);
    let kept = &app.layout.halocord;
    assert!(count_words(kept, "", false).is_none(), "nothing typed");
    assert_eq!(
        count_words(kept, "héllo", false).unwrap().0,
        "5 / 2,000",
        "letters, not bytes"
    );
    assert_eq!(
        count_words(kept, "hi", true).unwrap().0,
        "2 / 4,000",
        "Nitro"
    );
    let _ = app.view();
}

#[test]
fn the_new_account_tag_is_off_at_first_and_keeps_its_days() {
    use crate::halocord::{Change, NEW_USER, PLUGINS};
    let (mut app, _folder) = preview();
    assert!(!app.layout.halocord.is(NEW_USER), "off at first");
    switch(&mut app, NEW_USER, true);
    assert!(app.layout.halocord.is(NEW_USER));
    let plugin = place(NEW_USER).expect("the plugin");
    let days = &PLUGINS[plugin].settings[0];
    assert_eq!(
        app.layout.halocord.value(&PLUGINS[plugin], days),
        7,
        "7 unless changed"
    );
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Choose {
            plugin,
            setting: 0,
            choice: 2,
        },
    );
    assert_eq!(app.layout.halocord.value(&PLUGINS[plugin], days), 30);
    let _ = app.view();
}
