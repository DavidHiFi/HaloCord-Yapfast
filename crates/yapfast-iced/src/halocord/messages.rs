//! What the Message Logger and Fake DMs draw in the timeline: a deleted
//! message in the danger tone with a "Deleted" mark (or folded to one
//! line), an edited message's earlier wordings under it, and the "Local"
//! mark of a fake message.

use iced::{
    Alignment, Color, Element, Length, Padding,
    widget::{Column, button, column, container, row, text},
};
use yapfast_core::{
    chat::ChatMessage,
    halocord::{self as engine, message_logger},
};

use super::{FakeEvent, Kind, Plugin, Setting, id, off};
use crate::{
    app::Message,
    controls,
    ctx::Ctx,
    fonts::{BOLD, ITALIC},
    message::{blocked_key, text_size},
    pictures::Icon,
    timeline::{AVATAR, CONTENT_PAD, END_PAD, GUTTER, LINE_HEIGHT, RowLook},
};

const fn on_switch(key: &'static str, heading: &'static str, note: &'static str) -> Setting {
    Setting {
        key,
        heading,
        note,
        kind: Kind::Switch {
            unless_changed: true,
        },
    }
}

/// A Message logger list of Discord ids to leave alone.
const fn ids(key: &'static str, heading: &'static str) -> Setting {
    Setting {
        key,
        heading,
        note: "Discord ids, separated by commas.",
        kind: Kind::Text {
            placeholder: "",
            unless_changed: "",
        },
    }
}

pub(super) const FAKE_DMS: Plugin = Plugin {
    id: id::FAKE_DMS,
    name: "Fake DMs",
    about: "Adds messages that exist only on this computer, in any chat. A button in the chat's top bar opens a box to write one. Nothing is ever sent to Discord, and nobody else sees them.",
    on_at_first: false,
    settings: &[on_switch(
        "local_mark",
        "Show a Local mark",
        "Puts a small Local mark on these messages so you can tell them apart.",
    )],
    apply: |kept, plugin| {
        engine::fake_dms::set_on(kept.on(plugin));
        engine::fake_dms::set_local_mark(kept.flag(plugin, &plugin.settings[0]));
    },
    hooks: Some(&engine::fake_dms::FakeDms),
};

pub(super) const MESSAGE_LOGGER: Plugin = Plugin {
    id: id::MESSAGE_LOGGER,
    name: "Message logger",
    about: "Keeps a message in the chat after someone deletes it, marked as deleted, and keeps the earlier wordings of one they edit. It only keeps what Yapfast had already seen, and only until you close it.",
    on_at_first: false,
    settings: &[
        off(
            "ignore_bots",
            "Ignore bots",
            "Keep nothing of what bots delete or edit.",
        ),
        off(
            "ignore_own",
            "Ignore your own messages",
            "Keep nothing of what you delete or edit.",
        ),
        on_switch(
            "keep_deleted",
            "Keep deleted messages",
            "A deleted message stays in the chat, marked as deleted.",
        ),
        on_switch(
            "keep_edits",
            "Keep edit history",
            "An edited message shows what it said before.",
        ),
        off(
            "collapse",
            "Collapse deleted messages",
            "Fold a deleted message to one line until you press Show it.",
        ),
        ids("ignore_people", "IGNORE PEOPLE"),
        ids("ignore_channels", "IGNORE CHANNELS"),
        ids("ignore_servers", "IGNORE SERVERS"),
    ],
    apply: |kept, plugin| {
        let flag = |at: usize| kept.flag(plugin, &plugin.settings[at]);
        let ids =
            |at: usize| message_logger::Settings::ids(kept.text(plugin, &plugin.settings[at]));
        message_logger::set(message_logger::Settings {
            on: kept.on(plugin),
            ignore_bots: flag(0),
            ignore_own: flag(1),
            keep_deleted: flag(2),
            keep_edits: flag(3),
            collapse: flag(4),
            ignore_people: ids(5),
            ignore_channels: ids(6),
            ignore_servers: ids(7),
        });
    },
    hooks: Some(&message_logger::MessageLogger),
};

/// Fake DMs' button in the chat's top bar: icon, tip and what it does,
/// over a Discord channel or DM.
pub fn fake_button(cx: Ctx<'_>) -> Option<(Icon, &'static str, Message)> {
    let discord = cx
        .state
        .selected
        .as_deref()
        .and_then(yapfast_core::discord::parse_room)
        .is_some();
    let shows = cx.layout.halocord.is(id::FAKE_DMS) && cx.state.shows_text() && discord;
    shows.then(|| {
        (
            Icon::MessageCirclePlus,
            "Add a fake message",
            Message::FakeDm(FakeEvent::Open),
        )
    })
}

/// How many earlier wordings show under an edited message.
const SHOWN: usize = 3;
/// How much of one wording shows.
const WORDING: usize = 200;

fn kept(message: &ChatMessage) -> bool {
    message.deleted && message_logger::is_deleted(&message.item)
}

/// Whether the logger kept this deleted message, which so keeps a menu.
pub fn kept_deleted(message: &ChatMessage) -> bool {
    kept(message)
}

/// Whether this row is a fake message, which takes no app commands.
pub fn is_fake_row(message: &ChatMessage) -> bool {
    engine::fake_dms::is_fake(&message.item)
}

/// A message the logger kept after its deletion reads in the danger tone.
pub fn message_ink(theme: &crate::theme::Theme, message: &ChatMessage, ink: Color) -> Color {
    if kept(message) { theme.danger } else { ink }
}

fn clipped(words: &str) -> String {
    let one_line = words.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= WORDING {
        one_line
    } else {
        let head: String = one_line.chars().take(WORDING).collect();
        format!("{head}...")
    }
}

/// The marks under a message's text: "Deleted" on a kept one, and the
/// earlier wordings of an edited one, the newest last.
pub fn notes<'a>(
    ctx: Ctx<'a>,
    message: &ChatMessage,
    mut content: Column<'a, Message>,
) -> Column<'a, Message> {
    let theme = ctx.look.theme;
    if kept(message) {
        content = content.push(
            text("Deleted")
                .size(11)
                .font(BOLD)
                .color(theme.danger)
                .wrapping(text::Wrapping::None),
        );
    }
    let history = message_logger::history(&message.item);
    if history.is_empty() || message.deleted {
        return content;
    }
    let hidden = history.len().saturating_sub(SHOWN);
    let mut lines = column![].spacing(2);
    if hidden > 0 {
        lines = lines.push(
            text(format!("{hidden} earlier"))
                .size(11)
                .color(theme.faint)
                .wrapping(text::Wrapping::None),
        );
    }
    for wording in history.iter().skip(hidden) {
        lines = lines.push(
            row![
                text("Before")
                    .size(11)
                    .font(BOLD)
                    .color(theme.faint)
                    .wrapping(text::Wrapping::None),
                text(clipped(wording))
                    .size(12)
                    .color(theme.muted)
                    .width(Length::Fill),
            ]
            .spacing(6)
            .align_y(Alignment::Start),
        );
    }
    content.push(container(lines).padding(Padding::ZERO.top(2)))
}

/// A kept deleted message folded to one line, until Show it is pressed.
pub fn collapsed_row<'a>(
    ctx: Ctx<'a>,
    message: &ChatMessage,
    look: &RowLook,
) -> Option<Element<'a, Message>> {
    if !message_logger::collapse()
        || !kept(message)
        || ctx
            .lanes
            .timeline
            .revealed
            .contains(&blocked_key(&message.item))
    {
        return None;
    }
    let theme = ctx.look.theme;
    let size = text_size(ctx);
    let line = row![
        text(format!("{}: ", message.author))
            .size(size)
            .font(BOLD)
            .color(theme.secondary)
            .wrapping(text::Wrapping::None),
        text("Deleted message.")
            .size(size)
            .font(ITALIC)
            .color(theme.danger),
        button(text("Show it").size(size))
            .padding(0)
            .on_press(Message::Timeline(crate::timeline::Event::ShowBlocked(
                look.item.clone()
            )))
            .style(controls::ink_style(theme.link, theme.text)),
    ]
    .spacing(4)
    .height(LINE_HEIGHT)
    .align_y(Alignment::Center);
    Some(
        container(
            row![
                container(iced::widget::Space::new())
                    .width(AVATAR)
                    .height(0),
                line
            ]
            .spacing(16),
        )
        .width(Length::Fill)
        .padding(Padding {
            top: CONTENT_PAD,
            bottom: CONTENT_PAD,
            left: (GUTTER - AVATAR) / 2.0,
            right: END_PAD,
        })
        .into(),
    )
}
