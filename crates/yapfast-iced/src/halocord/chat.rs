//! Chat plugins: the staff and new account tags by names, and the
//! character counter by the message box, and what a double-click on a
//! message does.

use iced::{
    Alignment, Element,
    widget::{container, text},
};
use yapfast_core::halocord::{self as engine, Tag};

use super::{Kept, Kind, Plugin, Setting, id, off};
use crate::{
    app::Message,
    ctx::Ctx,
    fonts::BOLD,
    pictures::Icon,
    style::{faded, hex, rounded},
    widgets::Pic,
};

pub(super) const STAFF_TAG: Plugin = Plugin {
    id: id::STAFF_TAG,
    name: "Staff tag",
    about: "Tags the owner, the admins and the mods of a server by their names in chat.",
    on_at_first: false,
    settings: &[
        off("crown", "Crown", "Draw a crown in place of the words."),
        Setting {
            key: "role_color",
            heading: "Role colour",
            note: "Draw the tag in the member's role colour, where they have one.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
    ],
    apply: |kept, plugin| {
        engine::staff::set_crown(kept.flag(plugin, &plugin.settings[0]));
        engine::staff::set_role_color(kept.flag(plugin, &plugin.settings[1]));
    },
    hooks: Some(&engine::staff::Staff),
};

pub(super) const COUNTER: Plugin = Plugin {
    id: id::COUNTER,
    name: "Character counter",
    about: "Shows how many letters you have typed beside the message box, like 1,234 / 2,000. It turns amber near the limit and red over it. The limit is 4,000 with Nitro.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const ALWAYS_TRUST: Plugin = Plugin {
    id: id::ALWAYS_TRUST,
    name: "Always trust links",
    about: "Opens a link whose words hide where it goes straight in the browser, without the \"Leaving Yapfast\" question first.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const NEW_USER: Plugin = Plugin {
    id: id::NEW_USER,
    name: "New account tag",
    about: "A small amber NEW tag by the names of people whose Discord account is only a few days old, to help you spot throwaway accounts.",
    on_at_first: false,
    settings: &[Setting {
        key: "days",
        heading: "NEW MEANS UNDER",
        note: "How old an account may be and still get the tag.",
        kind: Kind::Choice {
            names: &["7 days", "14 days", "30 days"],
            values: &engine::new_user::DAYS,
            unless_changed: engine::new_user::DEFAULT_DAYS,
        },
    }],
    apply: |kept, plugin| {
        engine::new_user::set_days(kept.value(plugin, &plugin.settings[0]));
    },
    hooks: Some(&engine::new_user::NewUsers),
};

/// Hook 4: A plugin's tag after a message's author: its words in a chip, or a
/// crown, in the colour it sets.
pub(super) const CLICK_ACTIONS: Plugin = Plugin {
    id: id::CLICK_ACTIONS,
    name: "Message click actions",
    about: "Double-click a message to reply to it, or your own to edit it.",
    on_at_first: false,
    settings: &[Setting {
        key: "edit_own",
        heading: "Edit your own",
        note: "A double-click on your own message edits it. Off, it replies, as on anyone else's.",
        kind: Kind::Switch {
            unless_changed: true,
        },
    }],
    apply: |_, _| {},
    hooks: None,
};

/// What a double-click on row `index`'s message sends while Message click
/// actions is on: Edit on one's own (unless its setting is off), else
/// Reply, as the hover bar's buttons do, where the message box writes.
pub fn double_click(cx: Ctx<'_>, index: usize) -> Option<Message> {
    use crate::menus::MenuAct;
    if !cx.layout.halocord.is(id::CLICK_ACTIONS) {
        return None;
    }
    let message = cx.row_message(index)?;
    if message.deleted || message.delivery != yapfast_core::chat::Delivery::default() {
        return None;
    }
    let writes = if index >= crate::timeline::ASIDE {
        crate::thread::aside_writes(cx)
    } else {
        crate::composer::composer_look(cx).enabled
    };
    if !writes {
        return None;
    }
    let edits = cx
        .layout
        .halocord
        .flag(&CLICK_ACTIONS, &CLICK_ACTIONS.settings[0]);
    let act = if message.own && message.can_edit && edits {
        MenuAct::Edit
    } else if message.can_reply {
        MenuAct::Reply
    } else {
        return None;
    };
    Some(Message::Menu(crate::menus::Event::Act(
        act,
        message.item.clone(),
        iced::Point::ORIGIN,
    )))
}

pub fn tag<'a>(ctx: Ctx<'a>, tag: Tag) -> Element<'a, Message> {
    let Some(color) = tag.color else {
        return ctx.blurple_tag(tag.text, 1.0);
    };
    if tag.crown {
        return Pic::square(ctx.look.icon(Icon::Crown, 14, color), 14.0).into();
    }
    let ink = hex(color);
    container(text(tag.text).size(10).font(BOLD).color(ink))
        .padding([0, 4])
        .height(16)
        .align_y(Alignment::Center)
        .style(move |_| rounded(faded(ink, 0.18), 4))
        .into()
}

/// How full the message box is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    Fine,
    Near,
    Over,
}

/// `1234` as `1,234`.
pub(super) fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The counter's words and how full the box is, for a draft of `used`
/// letters under `limit`. Amber from nine tenths of the limit.
pub fn counter_words(used: usize, limit: usize) -> (String, Fill) {
    let fill = if used > limit {
        Fill::Over
    } else if used * 10 >= limit * 9 {
        Fill::Near
    } else {
        Fill::Fine
    };
    (format!("{} / {}", thousands(used), thousands(limit)), fill)
}

/// What the counter shows for `draft`: nothing when it's off or the box is
/// empty.
pub fn count_words(kept: &Kept, draft: &str, nitro: bool) -> Option<(String, Fill)> {
    if !kept.is(id::COUNTER) || draft.is_empty() {
        return None;
    }
    Some(counter_words(
        draft.chars().count(),
        if nitro { 4000 } else { 2000 },
    ))
}

/// The counter's ink: `quiet` until near the limit.
pub fn count_color(fill: Fill, quiet: iced::Color) -> iced::Color {
    match fill {
        Fill::Fine => quiet,
        Fill::Near => hex(0xf0b232),
        Fill::Over => hex(0xf23f42),
    }
}

pub(super) const REVEAL_SPOILERS: Plugin = Plugin {
    id: id::REVEAL_SPOILERS,
    name: "Reveal all spoilers",
    about: "Clicking one spoiler in a message shows every spoiler in that message at once.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const PLAIN_FOLDER: Plugin = Plugin {
    id: id::PLAIN_FOLDER,
    name: "Plain folder icon",
    about: "A closed server folder shows the folder icon in its colour, instead of its first four servers' icons.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const SEND_TIMESTAMPS: Plugin = Plugin {
    id: id::SEND_TIMESTAMPS,
    name: "Send timestamps",
    about: "Write a time as <t:13:30>, <t:6pm>, <t:2026-12-24> or <t:2026-12-24 18:00> in a message, and it goes up as a timestamp each reader sees in their own time zone.",
    on_at_first: false,
    settings: &[],
    apply: |kept, plugin| engine::send_timestamps::set(kept.on(plugin)),
    hooks: None,
};

pub(super) const BETTER_FORWARDS: Plugin = Plugin {
    id: id::BETTER_FORWARDS,
    name: "Better forwards",
    about: "Forwarding a message to one conversation leaves you where you are, rather than opening the conversation it went to.",
    on_at_first: false,
    settings: &[],
    apply: |kept, plugin| engine::better_forwards::set(kept.on(plugin)),
    hooks: None,
};

pub(super) const FIX_FILE_EXTENSIONS: Plugin = Plugin {
    id: id::FIX_FILE_EXTENSIONS,
    name: "Fix file extensions",
    about: "A photo named like cat.jfif goes up as cat.jpg, so Discord shows it in chat instead of as a file to download.",
    on_at_first: false,
    settings: &[],
    apply: |kept, plugin| engine::fix_file_extensions::set(kept.on(plugin)),
    hooks: None,
};
