//! The top of the server list: Read all's button there, and Server list
//! indicators' counts of the friends online and the servers you're in.

use iced::{
    Element, Length,
    widget::{button, column, container, text, tooltip},
};
use yapfast_core::chat::Presence;

use super::{Kind, Plugin, Press, Setting, id, off};
use crate::{app::Message, controls, ctx::Ctx, fonts::MEDIUM, rail, style::rounded};

/// What Server list indicators counts.
const FRIENDS: u32 = 0;
const SERVERS: u32 = 1;
const BOTH: u32 = 2;

pub(super) const SERVER_COUNTS: Plugin = Plugin {
    id: id::SERVER_COUNTS,
    name: "Server list indicators",
    about: "How many of your friends are online and how many servers you're in, at the top of the server list.",
    on_at_first: true,
    settings: &[
        Setting {
            key: "mode",
            heading: "What it counts",
            note: "Your friends online, your servers, or both.",
            kind: Kind::Choice {
                names: &["Friends online", "Servers", "Both"],
                values: &[FRIENDS, SERVERS, BOTH],
                unless_changed: FRIENDS,
            },
        },
        off(
            "only_online",
            "Only friends who are online",
            "Leaves out friends who are idle or on do not disturb.",
        ),
    ],
    apply: |_, _| {},
    hooks: None,
};

/// Hook: what plugins put above the home tile, top first.
pub fn rail_top<'a>(cx: Ctx<'a>) -> Vec<Element<'a, Message>> {
    let mut items = Vec::new();
    let kept = &cx.layout.halocord;
    if let Some(small) = kept.read_all_on_rail() {
        items.push(read_all(cx, small));
    }
    if kept.on(&SERVER_COUNTS) {
        items.push(counts(cx));
    }
    items
}

/// Read all's button: a tile with the tick, or the words in a small pill.
fn read_all<'a>(cx: Ctx<'a>, small: bool) -> Element<'a, Message> {
    let theme = cx.look.theme;
    let (ground, lit, ink) = (theme.raised, theme.active, theme.secondary);
    let face: Element<'a, Message> = if small {
        container(text("Read all").size(11).font(MEDIUM).color(ink))
            .padding([3, 8])
            .into()
    } else {
        cx.look
            .glyph(crate::pictures::Icon::MarkRead, 20, ink)
            .boxed(rail::TILE)
            .into()
    };
    let radius = if small { 10.0 } else { 12.0 };
    let press = button(face)
        .padding(0)
        .on_press(Message::Bar(crate::window_bar::Event::Halo(Press::ReadAll)))
        .style(move |_, status| {
            let fill = rounded(
                if controls::lights(status) {
                    lit
                } else {
                    ground
                },
                radius,
            );
            button::Style {
                background: fill.background,
                border: fill.border,
                text_color: ink,
                ..Default::default()
            }
        });
    let tip = tooltip(
        press,
        controls::tip(theme, "Mark everything read"),
        tooltip::Position::Right,
    )
    .gap(8)
    .snap_within_viewport(true);
    container(tip).center_x(rail::WIDTH).into()
}

/// The counts, one short line each, with what they mean when pointed at.
fn counts<'a>(cx: Ctx<'a>) -> Element<'a, Message> {
    let theme = cx.look.theme;
    let kept = &cx.layout.halocord;
    let mode = kept.value(&SERVER_COUNTS, &SERVER_COUNTS.settings[0]);
    let only_online = kept.flag(&SERVER_COUNTS, &SERVER_COUNTS.settings[1]);
    let friends = cx
        .state
        .friends
        .iter()
        .filter(|friend| match friend.presence {
            Some(Presence::Online | Presence::Mobile) => true,
            Some(Presence::Idle | Presence::DoNotDisturb) => !only_online,
            Some(Presence::Offline) | None => false,
        })
        .count();
    let servers = cx.state.servers.len();
    let plural = |n: usize, one: &str, many: &str| {
        if n == 1 {
            format!("1 {one}")
        } else {
            format!("{n} {many}")
        }
    };
    let mut lines = column![].spacing(2).width(Length::Fill);
    let mut said = Vec::new();
    if mode != SERVERS {
        lines = lines.push(line(theme, format!("{friends} online")));
        said.push(format!("{} online", plural(friends, "friend", "friends")));
    }
    if mode != FRIENDS {
        lines = lines.push(line(theme, plural(servers, "server", "servers")));
        said.push(plural(servers, "server", "servers"));
    }
    let tip = tooltip(
        container(lines).center_x(rail::WIDTH),
        controls::tip(theme, said.join(", ")),
        tooltip::Position::Right,
    )
    .gap(8)
    .snap_within_viewport(true);
    container(tip).width(rail::WIDTH).into()
}

fn line<'a>(theme: &yapfast_iced_kit::theme::Theme, words: String) -> Element<'a, Message> {
    container(text(words).size(11).font(MEDIUM).color(theme.muted))
        .center_x(Length::Fill)
        .into()
}
