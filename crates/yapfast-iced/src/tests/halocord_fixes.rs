//! HaloCord's fixes to Yapfast's own bugs, one test each, named for the
//! Yapfast issue it closes.

use std::sync::Arc;

use super::{change, long_history, numbered, preview, scroll_up, shows, unread};
use crate::{
    app::Message,
    rail::{Event, Kind},
};

/// yapfast#286: a message in the DM being read doesn't put it on the rail
/// for a moment; with the window behind others, it does.
#[test]
fn a_message_in_the_dm_being_read_leaves_the_rail_still() {
    let (mut app, _folder) = preview();
    let _ = app.update(Message::Rail(Event::Dm("riley".into())));
    assert_eq!(app.client.state().selected.as_deref(), Some("riley"));
    let on_rail =
        |app: &crate::app::App| app.rail.items.iter().any(|item| item.kind == Kind::Dm);
    unread(&mut app, "riley", 1);
    assert!(!on_rail(&app), "the DM being read stays off the rail");
    app.shell.focused = false;
    unread(&mut app, "riley", 2);
    assert!(on_rail(&app), "read from behind other windows, it shows");
}

/// yapfast#287: a view in the space under the last message still follows
/// a new one down.
#[test]
fn a_new_message_brings_a_view_just_past_the_last_one_down() {
    let (mut app, _folder) = preview();
    long_history(&mut app, 80);
    scroll_up(&mut app, 12.0);
    assert!(
        app.timeline.from_end > 0.5 && app.timeline.at_end,
        "the view is in the space under the last message: {}",
        app.timeline.from_end
    );
    let new = numbered(&app, 80, 81);
    change(&mut app, |state| {
        let mut messages = (*state.messages).clone();
        messages.extend(new);
        state.messages = Arc::new(messages);
    });
    assert_eq!(app.timeline.from_end, 0.0, "the view went to the end");
    assert!(shows(&app, "Message number 80"), "the new one is in view");
}

/// yapfast#263: the hover bar goes while a drag selects, so the text under
/// it can be selected and seen, and comes back when the button is let go.
#[test]
fn the_hover_bar_goes_while_a_drag_selects() {
    let (mut app, _folder) = preview();
    let index = app.timeline.messages.len() - 1;
    let item: Arc<str> = app.timeline.messages[index].item.as_str().into();
    let bar = |app: &crate::app::App| {
        crate::message::hover_bar::hover_bar(app.ctx(), index).is_some()
    };
    assert!(bar(&app), "the bar shows");
    let mark = crate::piece::Mark {
        row: index,
        part: crate::piece::FIRST_PART,
        line: 0,
        index: 0,
    };
    let _ = app.update(Message::Select(crate::piece::Select::Start(mark, item)));
    assert!(app.timeline.selecting);
    assert!(!bar(&app), "gone while the drag selects");
    let _ = app.update(Message::SelectEnd);
    assert!(bar(&app), "back once it's let go");
}

/// yapfast#257, for a look by eye: a code block of several lines and a
/// message after it, written to `audit/fixes/code-block.png`.
#[test]
#[ignore]
fn code_block_picture() {
    let (mut app, _folder) = preview();
    app.shell.frame = iced::Size::new(1100.0, 700.0);
    change(&mut app, |state| {
        let template = (*state.messages[0]).clone();
        let said = |item: &str, body: &str, timestamp| {
            Arc::new(yapfast_core::chat::ChatMessage {
                id: item.into(),
                item: item.into(),
                author: template.author.clone(),
                sender: template.sender.clone(),
                body: body.into(),
                formatted: true,
                timestamp,
                ..Default::default()
            })
        };
        state.messages = Arc::new(vec![
            said("c0", "```\none line\n```", 1_700_000_000),
            said("c1", "Plain text after one line", 1_700_000_060),
            said(
                "c2",
                "```\nfirst line\nsecond line\nthird line\nfourth line\n```",
                1_700_000_120,
            ),
            said("c3", "Plain text after four lines", 1_700_000_180),
            said("c4", "```rust\nfn main() {\n    let x = 1;\n}\n```", 1_700_000_240),
            said("c5", "Plain text after Rust", 1_700_000_300),
        ]);
    });
    let mut ui = iced_test::Simulator::with_size(
        iced::Settings::default(),
        app.shell.frame,
        app.view(),
    );
    let snapshot = ui.snapshot(&iced::Theme::Dark).expect("drawn");
    std::fs::create_dir_all(r"D:\Agent Work\yapfast\audit\fixes").expect("folder");
    snapshot
        .matches_image(r"D:\Agent Work\yapfast\audit\fixes\code-block.png")
        .expect("written");
}

/// Reading further up still keeps the view where it is.
#[test]
fn a_new_message_leaves_a_view_a_screen_up_alone() {
    let (mut app, _folder) = preview();
    long_history(&mut app, 80);
    scroll_up(&mut app, 600.0);
    assert!(!app.timeline.at_end);
    let before = app.timeline.from_end;
    let new = numbered(&app, 80, 81);
    change(&mut app, |state| {
        let mut messages = (*state.messages).clone();
        messages.extend(new);
        state.messages = Arc::new(messages);
    });
    assert!(app.timeline.from_end >= before, "the view stayed up");
}
