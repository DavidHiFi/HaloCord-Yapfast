//! HaloCord's button plugins (restart, read all, close all DMs, the
//! character counter, the new user tag, the member count) driven through
//! the window.

use yapfast_core::{
    application::{Action, Effect},
    chat::Command,
};

use super::{change, open_plugin, preview, shows};
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
        ["Restart Yapfast"],
        "Read all sits at the top of the server list at first"
    );
    let plugin = place(READ_ALL).expect("the plugin");
    for (choice, bar) in [(1, true), (2, true), (0, false)] {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Choose {
                plugin,
                setting: 0,
                choice,
            },
        );
        assert_eq!(
            labels(&app).contains(&"Mark everything read"),
            bar,
            "the top bar has it for choice {choice}"
        );
    }
    switch(&mut app, RESTART, false);
    assert!(labels(&app).is_empty());
}

#[test]
fn read_all_sits_at_the_top_of_the_server_list_and_goes_when_off() {
    let (mut app, _folder) = preview();
    let plugin = place(READ_ALL).expect("the plugin");
    // The small text button's words, so the window shows it as text.
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Flag {
            plugin,
            setting: 1,
            on: true,
        },
    );
    change(&mut app, |_| {});
    assert!(shows(&app, "Read all"), "the small button at the top");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Choose {
            plugin,
            setting: 0,
            choice: 1,
        },
    );
    change(&mut app, |_| {});
    assert!(!shows(&app, "Read all"), "only in the top bar now");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Choose {
            plugin,
            setting: 0,
            choice: 2,
        },
    );
    change(&mut app, |_| {});
    assert!(shows(&app, "Read all"), "both places");
    switch(&mut app, READ_ALL, false);
    change(&mut app, |_| {});
    assert!(!shows(&app, "Read all"), "gone while off");
}

#[test]
fn read_all_lists_its_settings() {
    let (mut app, _folder) = preview();
    open_plugin(&mut app, READ_ALL);
    for row in ["Where it sits", "Small text button"] {
        assert!(shows(&app, row), "{row} is offered");
    }
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
    let _alone = super::plugin_alone();
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

#[test]
fn close_all_dms_is_off_at_first_and_closes_each_direct_chat_only() {
    use crate::halocord::{CLOSE_DMS, close_dm_actions};
    let (mut app, _folder) = preview();
    assert!(!app.layout.halocord.is(CLOSE_DMS), "off at first");
    let direct: Vec<String> = app
        .client
        .state()
        .rooms
        .iter()
        .filter(|room| room.direct)
        .map(|room| room.id.clone())
        .collect();
    assert!(!direct.is_empty(), "the preview has direct chats");
    let closes: Vec<String> = close_dm_actions(app.client.state())
        .into_iter()
        .map(|action| {
            let Action::CloseDm(id) = action else {
                panic!("not a close");
            };
            id
        })
        .collect();
    assert_eq!(
        closes, direct,
        "one for each direct chat, no servers' channels"
    );
    switch(&mut app, CLOSE_DMS, true);
    let _ = app.view();
    let _ = app.update(Message::Bar(Event::Halo(Press::CloseDms)));
}

#[test]
fn the_member_count_adds_one_line_first_counting_everyone_but_offline_as_online() {
    use crate::{
        halocord::{MEMBER_COUNT, add_member_count, member_totals},
        looks::MemberLine,
    };
    let heading = |text: &str| MemberLine::Heading {
        text: text.into(),
        icon: None,
    };
    let mut lines = vec![
        heading("Admins \u{2014} 2"),
        heading("Online \u{2014} 1,200".replace(',', "").as_str()),
        heading("Offline \u{2014} 40"),
    ];
    assert_eq!(member_totals(&lines), (1202, 1242));
    let (mut app, _folder) = preview();
    add_member_count(&app.layout.halocord, &mut lines);
    assert_eq!(lines.len(), 3, "off at first: nothing added");
    switch(&mut app, MEMBER_COUNT, true);
    add_member_count(&app.layout.halocord, &mut lines);
    assert_eq!(
        lines[0],
        heading("1,202 online \u{b7} 1,242 members"),
        "first, with commas"
    );
    let mut none = Vec::new();
    add_member_count(&app.layout.halocord, &mut none);
    assert!(none.is_empty(), "nobody to count");
}

/// Member count's switch for the server list's tooltips: off at first; on,
/// a server's tooltip name says how many members it has, with thousands.
#[test]
fn member_count_adds_the_count_to_a_servers_tooltip() {
    use std::borrow::Cow;
    let (mut app, _folder) = preview();
    let plugin = place(crate::halocord::MEMBER_COUNT).expect("the plugin");
    let tip = |app: &crate::app::App| {
        crate::halocord::tip_member_count(app.ctx(), "discord:10", Cow::Borrowed("Test"))
            .into_owned()
    };
    app.client.app = yapfast_core::application::member_server(&[]);
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Switch { plugin, on: true },
    );
    assert_eq!(tip(&app), "Test", "the tooltip switch is off at first");
    changed(
        &mut app.layout,
        &mut app.client,
        Change::Flag {
            plugin,
            setting: 0,
            on: true,
        },
    );
    let count = app.client.app.server_member_count("discord:10");
    match count {
        Some(total) => assert!(
            tip(&app).starts_with("Test \u{b7} "),
            "{total}: {}",
            tip(&app)
        ),
        None => assert_eq!(tip(&app), "Test", "no count known, the name alone"),
    }
}

/// Signed in to Discord, as the card shows a real account.
fn signed_in(app: &mut crate::app::App) {
    change(app, |state| {
        state.discord_user = Some("42".into());
        state.preview = false;
        state.account = Some("sam".into());
        state.account_name = Some("Sam".into());
        state.discord_linked = true;
        state.discord_online = true;
        state.online = true;
    });
}

/// The middle of the card's Settings cog, its last button.
fn cog_at(app: &crate::app::App) -> iced::Point {
    let compact = app.layout.halocord.dock().compact;
    let right = crate::conversation::navigation_ctx(app.ctx()) - crate::account::INSET;
    // On the narrowest sidebar the card's buttons take the compact size.
    let small = compact || app.layout.sidebar_width.is_some_and(|width| width <= 180.0);
    let (pad, side) = match (compact, small) {
        (true, _) => (8.0, 28.0),
        (false, true) => (14.0, 28.0),
        (false, false) => (14.0, 32.0),
    };
    iced::Point::new(
        right - pad - side / 2.0,
        app.shell.frame.height - crate::account::INSET - crate::account::height(compact) / 2.0,
    )
}

/// A point inside the last button of the plugin row above the name: the
/// tray's, when some buttons go in it. The row is inset 8 px a side, its
/// buttons 32 px tall, 32 px wide at the least and 6 px under the card's top.
fn last_plugin_at(app: &crate::app::App) -> iced::Point {
    let right = crate::conversation::navigation_ctx(app.ctx()) - crate::account::INSET;
    let top =
        app.shell.frame.height - crate::account::INSET - crate::account::full_height(app.ctx());
    iced::Point::new(right - 8.0 - 16.0, top + 6.0 + 16.0)
}

/// What a click at `at` in the whole window sends.
fn clicked_at(app: &crate::app::App, at: iced::Point) -> Vec<Message> {
    let mut ui =
        iced_test::Simulator::with_size(iced::Settings::default(), app.shell.frame, app.view());
    ui.point_at(at);
    ui.simulate(iced_test::simulator::click());
    ui.into_messages().collect()
}

/// The card keeps its Settings cog whatever it carries: in a call with
/// every one of the User area's buttons and the top bar's buttons on the
/// row above the name (the bar is the system's), compact, and on the
/// narrowest sidebar. The buttons with no room on that row go in a tray
/// that its last button, "More buttons", opens over the card.
#[test]
fn the_settings_cog_is_always_on_the_card() {
    use crate::halocord::USER_AREA;
    let opens_settings = |app: &crate::app::App| {
        clicked_at(app, cog_at(app))
            .iter()
            .any(|message| matches!(message, Message::Settings(event) if matches!(event.as_ref(), crate::settings::Event::Reopen)))
    };
    let (mut app, _folder) = preview();
    app.shell.own_title_bar = false;
    signed_in(&mut app);
    let _ = app.sync();
    assert!(opens_settings(&app), "out of a call");
    change(&mut app, |state| {
        state.voice.phase = yapfast_core::voice::Phase::Connected;
        state.voice.room = Some("!lounge:test".into());
    });
    let _ = app.sync();
    assert!(opens_settings(&app), "in a call");
    let area = place(USER_AREA).unwrap();
    let flag = |app: &mut crate::app::App, setting, on| {
        changed(
            &mut app.layout,
            &mut app.client,
            Change::Flag {
                plugin: area,
                setting,
                on,
            },
        );
    };
    // Fake mute, fake deafen, fake camera, stats, stereo and game: with
    // the top bar's Restart (Read all sits on the server list), seven
    // plugin buttons.
    for setting in [2, 3, 4, 6, 7, 8] {
        flag(&mut app, setting, true);
    }
    let _ = app.sync();
    assert!(opens_settings(&app), "with every button");
    let opens_tray = |app: &crate::app::App| {
        clicked_at(app, last_plugin_at(app))
            .iter()
            .any(|message| matches!(message, Message::Account(crate::account::Event::More(true))))
    };
    assert!(
        !opens_tray(&app),
        "on the sidebar's own width all seven fit on their row"
    );
    // On the narrowest sidebar the row's last button is the tray's, and it
    // opens the rest over the card.
    app.layout.sidebar_width = Some(180.0);
    assert!(opens_settings(&app), "on the narrowest sidebar");
    assert!(opens_tray(&app), "the tray's button ends the plugin row");
    assert!(crate::halocord::card_tray(app.ctx()).is_none());
    assert!(app.popups_moving().is_empty());
    let _ = app.update(Message::Account(crate::account::Event::More(true)));
    assert!(
        crate::halocord::card_tray(app.ctx()).is_some(),
        "the tray shows the card's other buttons"
    );
    // It grows in as popups do, and shrinks away drawn going.
    let opening = app.popups_moving();
    assert_eq!(opening.len(), 1, "the tray's motion");
    assert!(!opening[0].1, "opening");
    let _ = app.update(Message::Account(crate::account::Event::More(false)));
    if !crate::widgets::motion::still() {
        assert!(
            crate::halocord::card_tray(app.ctx()).is_some(),
            "drawn going"
        );
        assert_eq!(app.popups_moving(), [(opening[0].0, true)], "closing");
        let _ = app.update(Message::PopupGone(crate::app::layers::Popup::Layer(
            crate::app::layers::Layer::CardTray,
        )));
    }
    assert!(crate::halocord::card_tray(app.ctx()).is_none(), "gone");
    assert!(app.popups_moving().is_empty());
    app.layout.sidebar_width = None;
    flag(&mut app, 0, true);
    assert!(opens_settings(&app), "compact");
}

/// The name keeps room beside the card's buttons on the sidebar's own
/// width, however many buttons there are: the rest go in the tray.
#[test]
fn the_card_keeps_room_for_the_name() {
    let card = (crate::rail::WIDTH + crate::sidebar::WIDTH) - 2.0 * crate::account::INSET;
    for extras in 0..12 {
        for compact in [false, true] {
            let fit = crate::halocord::shared(card, compact, true, extras, false);
            assert!(fit.name_room >= 52.0, "{extras} buttons: {fit:?}");
            assert!(fit.inline <= extras);
            assert_eq!(fit.more, fit.inline < extras);
        }
    }
    // On the narrowest sidebar each plugin button, the tray's included,
    // keeps 32 px of the row (inset 8 px a side, 4 px between them).
    let least = crate::rail::WIDTH + 180.0 - 2.0 * crate::account::INSET;
    let fit = crate::halocord::shared(least, false, true, 7, false);
    let places = fit.inline + usize::from(fit.more);
    assert!(fit.more, "{fit:?}");
    assert!((least - 16.0 - 4.0 * (places - 1) as f32) / places as f32 >= 32.0);
    // A long name is cut with an ellipsis to its room, not dropped.
    super::install_fonts();
    let cut = crate::halocord::ellipsized(
        "A rather long display name indeed",
        15.0,
        crate::fonts::BOLD,
        90.0,
    );
    assert!(cut.ends_with('…') && cut.len() > 3, "{cut}");
    assert!(crate::halocord::words_width(&cut, 15.0, crate::fonts::BOLD) <= 90.0);
    assert_eq!(
        crate::halocord::ellipsized("Sam", 15.0, crate::fonts::BOLD, 90.0),
        "Sam"
    );
}
