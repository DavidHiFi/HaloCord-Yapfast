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
        off(
            "member_list",
            "In the member list",
            "Admins and mods by their names in the member list (the owner already has its crown there).",
        ),
        off(
            "voice",
            "Under voice channels",
            "By the names of people in a voice channel in the channel list.",
        ),
        off(
            "profiles",
            "On profiles",
            "After the username on someone's profile, in the server you're in.",
        ),
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
    settings: &[
        Setting {
            key: "days",
            heading: "NEW MEANS UNDER",
            note: "How old an account may be and still get the tag.",
            kind: Kind::Choice {
                names: &["7 days", "14 days", "30 days"],
                values: &engine::new_user::DAYS,
                unless_changed: engine::new_user::DEFAULT_DAYS,
            },
        },
        off(
            "member_list",
            "In the member list",
            "The tag by new accounts' names in the member list.",
        ),
        off(
            "profiles",
            "On profiles",
            "The tag after the username on a new account's profile.",
        ),
    ],
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
    about: "Double-click a message to reply to it, or your own to edit it. A triple click and a click while holding Backspace can act too, as set below, such as reacting or deleting a message without asking; others' messages only if you set it for them.",
    on_at_first: false,
    settings: &[
        Setting {
            key: "edit_own",
            heading: "Edit your own",
            note: "A double-click on your own message edits it. Off, it replies, as on anyone else's.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
        super::clicks::choice("triple", "TRIPLE CLICK", super::clicks::NOTHING),
        Setting {
            key: "emoji",
            heading: "REACT WITH",
            note: "The emoji a click set to React puts on the message.",
            kind: Kind::Text {
                placeholder: "💀",
                unless_changed: "💀",
            },
        },
        // Nothing unless changed, so the plugin works as before until a
        // Backspace or triple click is picked.
        super::clicks::choice(
            "backspace_own",
            "CLICK WITH BACKSPACE HELD, YOUR MESSAGES",
            super::clicks::NOTHING,
        ),
        // Deleting others' messages (with Manage Messages) is asked for,
        // never given.
        super::clicks::choice(
            "backspace_others",
            "CLICK WITH BACKSPACE HELD, OTHERS' MESSAGES",
            super::clicks::NOTHING,
        ),
        Setting {
            key: "within",
            heading: "CLICKS WITHIN",
            note: "How quickly the clicks of a double or triple click follow each other.",
            kind: Kind::Slider {
                most: 1000,
                step: 50,
                unit: " ms",
                unless_changed: 300,
            },
        },
        Setting {
            key: "wait_for_triple",
            heading: "Wait for a triple click",
            note: "While a triple click does something, a double click waits that long to see if a third click follows.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
        off(
            "not_in_dms",
            "Not in DMs",
            "Clicks do nothing in direct messages.",
        ),
    ],
    apply: |_, _| {},
    hooks: None,
};

/// What a double-click on row `index`'s message sends while Message click
/// actions is on: Edit on one's own (unless its setting is off), else
/// Reply, as the hover bar's buttons do, where the message box writes.
pub fn double_click(cx: Ctx<'_>, index: usize) -> Option<Message> {
    let act = double_act(cx, index)?;
    Some(Message::Menu(crate::menus::Event::Act(
        act,
        cx.row_message(index)?.item.clone(),
        iced::Point::ORIGIN,
    )))
}

/// What a double-click on row `index` does, as [`double_click`] says it.
pub(super) fn double_act(cx: Ctx<'_>, index: usize) -> Option<crate::menus::MenuAct> {
    use crate::menus::MenuAct;
    if !cx.layout.halocord.is(id::CLICK_ACTIONS) {
        return None;
    }
    let message = cx.row_message(index)?;
    // Not the message being edited: a double press in its field picks a
    // word, and editing it again would undo what was typed.
    let editing = cx.composer.editing.as_deref() == Some(message.item.as_str());
    if editing || message.deleted || message.delivery != yapfast_core::chat::Delivery::default() {
        return None;
    }
    // Not in DMs, when set: a DM, a group DM too, is `direct`.
    let state = cx.state;
    let in_dm = state
        .rooms
        .iter()
        .any(|room| room.direct && state.selected.as_ref() == Some(&room.id));
    if in_dm
        && cx.layout.halocord.flag(
            &CLICK_ACTIONS,
            &CLICK_ACTIONS.settings[super::clicks::NOT_IN_DMS],
        )
    {
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
    if message.own && message.can_edit && edits {
        Some(MenuAct::Edit)
    } else if message.can_reply {
        Some(MenuAct::Reply)
    } else {
        None
    }
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

/// Staff tag by `user`'s name outside chat, in the server on screen, while
/// its switch for `place` is on. `None` for the member list's owner, who
/// has the list's own crown.
pub(super) fn staff_mark<'a>(
    ctx: Ctx<'a>,
    user: &str,
    place: super::Place,
) -> Option<Element<'a, Message>> {
    let kept = &ctx.layout.halocord;
    let setting = match place {
        super::Place::MemberList => 2,
        super::Place::Voice => 3,
        super::Place::Profile => 4,
        super::Place::Dms | super::Place::Message => return None,
    };
    if !kept.on(&STAFF_TAG) || !kept.flag(&STAFF_TAG, &STAFF_TAG.settings[setting]) {
        return None;
    }
    let server = ctx.state.selected.as_deref()?;
    let found = engine::staff::tag_of(ctx.app, server, user, None)?;
    if place == super::Place::MemberList && found.text == "Owner" {
        return None;
    }
    Some(tag(ctx, found))
}

/// New account tag by `user`'s name outside chat, while its switch for
/// `place` is on, saying how old the account is on pointing.
pub(super) fn new_account_mark<'a>(
    ctx: Ctx<'a>,
    user: &str,
    place: super::Place,
) -> Option<Element<'a, Message>> {
    let kept = &ctx.layout.halocord;
    let setting = match place {
        super::Place::MemberList => 1,
        super::Place::Profile => 2,
        _ => return None,
    };
    if !kept.on(&NEW_USER) || !kept.flag(&NEW_USER, &NEW_USER.settings[setting]) {
        return None;
    }
    let days = engine::new_user::new_for_days(user, yapfast_core::clock::now_ms())?;
    let said = match days {
        0 => "Account created today".to_owned(),
        1 => "Account created 1 day ago".to_owned(),
        days => format!("Account created {days} days ago"),
    };
    Some(
        iced::widget::tooltip(
            tag(ctx, engine::new_user::TAG),
            crate::controls::tip(ctx.look.theme, said),
            iced::widget::tooltip::Position::Top,
        )
        .gap(6)
        .into(),
    )
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
    settings: &[off(
        "picker",
        "Clock by the message box",
        "A clock beside the message box's buttons puts a timestamp in from a list: in 15 minutes, tonight, tomorrow, as a time, a date or \"in 2 hours\".",
    )],
    apply: |kept, plugin| engine::send_timestamps::set(kept.on(plugin)),
    hooks: None,
};

/// One of Embedded URLs' hosts: a line of text, `host` unless changed.
const fn fixer(key: &'static str, heading: &'static str, host: &'static str) -> Setting {
    Setting {
        key,
        heading,
        note: "",
        kind: Kind::Text {
            placeholder: "Left as it is",
            unless_changed: host,
        },
    }
}

pub(super) const EMBEDDED_URLS: Plugin = Plugin {
    id: id::EMBEDDED_URLS,
    name: "Embedded URLs",
    about: "An Instagram, TikTok, X or Reddit post's link in a message you send or edit goes up with an embed fixer's site in its place, so its video or pictures show in chat. The service's tracking goes too; a link in <angle brackets> stays as it is. Empty a site to leave its links alone.",
    on_at_first: false,
    settings: &[
        fixer("instagram", "INSTAGRAM", "zzinstagram.com"),
        fixer("tiktok", "TIKTOK", "tnktok.com"),
        fixer("twitter", "X", "fxtwitter.com"),
        fixer("reddit", "REDDIT", "vxreddit.com"),
    ],
    apply: |kept, plugin| {
        let host = |at: usize| {
            engine::embedded_urls::Fixers::site(kept.text(plugin, &plugin.settings[at]))
        };
        engine::embedded_urls::set(kept.on(plugin).then(|| engine::embedded_urls::Fixers {
            instagram: host(0),
            tiktok: host(1),
            twitter: host(2),
            reddit: host(3),
        }));
    },
    hooks: None,
};

pub(super) const AUTO_ZIPPER: Plugin = Plugin {
    id: id::AUTO_ZIPPER,
    name: "Auto zipper",
    about: "A file with one of these extensions goes up as a ZIP holding it, named like scene.blend.zip, so Discord takes it whole as an archive. The ZIP is made from the file as it's sent, with no compression, so a file over Discord's upload limit is still over it.",
    on_at_first: false,
    settings: &[Setting {
        key: "extensions",
        heading: "EXTENSIONS",
        note: "Separated by commas.",
        kind: Kind::Text {
            placeholder: ".psd, .blend",
            unless_changed: ".psd,.blend,.exe,.apk",
        },
    }],
    apply: |kept, plugin| {
        engine::auto_zipper::set(
            kept.on(plugin)
                .then(|| kept.text(plugin, &plugin.settings[0])),
        );
    },
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

/// Whether GIF paste is on, which the GIF picker reads as it picks.
static GIF_PASTE_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(super) const GIF_PASTE: Plugin = Plugin {
    id: id::GIF_PASTE,
    name: "GIF paste",
    about: "A GIF you pick goes into the message you're writing, as its link, rather than out on its own at once. Shift and a click sends it straight away.",
    on_at_first: false,
    settings: &[],
    apply: |kept, plugin| {
        GIF_PASTE_ON.store(kept.on(plugin), std::sync::atomic::Ordering::Relaxed);
    },
    hooks: None,
};

/// Hook: whether a GIF picked with or without Shift goes into the message
/// box (Yapfast's Shift and a click) rather than out at once. GIF paste swaps
/// the two.
pub fn inserts_gif(shift: bool) -> bool {
    shift != GIF_PASTE_ON.load(std::sync::atomic::Ordering::Relaxed)
}

pub(super) const FIX_FILE_EXTENSIONS: Plugin = Plugin {
    id: id::FIX_FILE_EXTENSIONS,
    name: "Fix file extensions",
    about: "A photo named like cat.jfif goes up as cat.jpg, so Discord shows it in chat instead of as a file to download.",
    on_at_first: false,
    settings: &[],
    apply: |kept, plugin| engine::fix_file_extensions::set(kept.on(plugin)),
    hooks: None,
};

// Send timestamps' picker.

/// The start of the keys of the picker's rows.
const PICKER: &str = "send-timestamps/";

/// The picker's times from `now`: each one's label and Unix seconds, to
/// the minute.
fn picker_times(now: chrono::DateTime<chrono::Local>) -> Vec<(&'static str, i64)> {
    use chrono::{Duration, TimeZone, Timelike};
    let at = |hour: u32, days: i64| {
        (now + Duration::days(days))
            .date_naive()
            .and_hms_opt(hour, 0, 0)
            .and_then(|naive| chrono::Local.from_local_datetime(&naive).earliest())
    };
    let mut times = vec![
        ("In 15 minutes", now + Duration::minutes(15)),
        ("In 1 hour", now + Duration::hours(1)),
        ("In 3 hours", now + Duration::hours(3)),
    ];
    // The next 8 PM: tonight's, or tomorrow's once it's past.
    if let Some(evening) = at(20, 0).filter(|evening| *evening > now) {
        times.push(("Tonight, 8 PM", evening));
    } else if let Some(evening) = at(20, 1) {
        times.push(("Tomorrow, 8 PM", evening));
    }
    if let Some(noon) = at(12, 1) {
        times.push(("Tomorrow, noon", noon));
    }
    times.push(("Tomorrow, this time", now + Duration::days(1)));
    times.push(("In a week", now + Duration::weeks(1)));
    times
        .into_iter()
        .map(|(label, time)| {
            let time = time.with_second(0).and_then(|time| time.with_nanosecond(0));
            (label, time.map_or(now.timestamp(), |time| time.timestamp()))
        })
        .collect()
}

/// Discord's timestamp styles the picker offers: their names and letters.
const STYLES: [(&str, char); 4] = [
    ("As a time", 't'),
    ("As a date and time", 'f'),
    ("Relative", 'R'),
    ("As a long date and time", 'F'),
];

/// Roughly how Discord shows `unix` in `style` here, for the row's
/// subtitle: each reader sees it in their own time and language.
fn looks(unix: i64, style: char, now: chrono::DateTime<chrono::Local>) -> String {
    use chrono::TimeZone;
    let Some(time) = chrono::Local.timestamp_opt(unix, 0).single() else {
        return String::new();
    };
    match style {
        't' => time.format("%-H:%M").to_string(),
        'f' => time.format("%-d %B %Y %-H:%M").to_string(),
        'F' => time.format("%A, %-d %B %Y %-H:%M").to_string(),
        _ => {
            let minutes = (time - now).num_minutes().max(0);
            let (count, unit) = match minutes {
                ..60 => (minutes, "minute"),
                60..1440 => ((minutes + 30) / 60, "hour"),
                _ => ((minutes + 720) / 1440, "day"),
            };
            format!("in {count} {unit}{}", if count == 1 { "" } else { "s" })
        }
    }
}

/// Hook: the clock beside the message box's buttons, while Send timestamps
/// and its picker are on in a Discord conversation.
pub fn timestamp_button<'a>(
    cx: Ctx<'a>,
    buttons: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let kept = &cx.layout.halocord;
    let discord = cx
        .state
        .selected
        .as_deref()
        .and_then(yapfast_core::discord::parse_room)
        .is_some();
    if !discord
        || !kept.on(&SEND_TIMESTAMPS)
        || !kept.flag(&SEND_TIMESTAMPS, &SEND_TIMESTAMPS.settings[0])
    {
        return buttons.into();
    }
    let clock = container(crate::composer::composer_button(
        cx,
        Icon::Clock,
        "Insert a timestamp",
        20,
        cx.look.theme.secondary,
        Some(Message::HaloCord(super::Event::TimestampMenu)),
    ))
    .padding(iced::Padding::ZERO.bottom(crate::composer::BUTTON_INSET));
    iced::widget::row![clock, buttons.into()]
        .spacing(6)
        .align_y(Alignment::End)
        .into()
}

/// The picker: each time, with Discord's styles in its submenu.
pub(super) fn timestamp_menu(app: &mut crate::app::App) -> iced::Task<Message> {
    use crate::menus::{Menu, MenuAct, MenuRow};
    let now = chrono::Local::now();
    let rows = picker_times(now)
        .into_iter()
        .map(|(label, unix)| MenuRow {
            submenu: STYLES
                .iter()
                .map(|(name, style)| MenuRow {
                    subtitle: Some(looks(unix, *style, now).into()),
                    ..MenuRow::plain(MenuAct::Plugin, *name).key(format!("{PICKER}{unix}:{style}"))
                })
                .collect(),
            ..MenuRow::new(MenuAct::Plugin, label, Icon::Clock)
        })
        .collect();
    app.win.menu = Some(Menu::new(app.pressed.get(), rows));
    iced::Task::none()
}

/// A picked time goes in at the message box's caret, as Discord's own
/// timestamp, which Send timestamps leaves as it is.
pub(super) fn pick_timestamp(app: &mut crate::app::App, key: &str) -> Option<iced::Task<Message>> {
    let (unix, style) = key.strip_prefix(PICKER)?.split_once(':')?;
    let unix: i64 = unix.parse().ok()?;
    let style = style
        .chars()
        .next()
        .filter(|style| "tfRF".contains(*style))?;
    crate::composer::insert(
        app,
        crate::composer::Writing::Draft,
        &format!("<t:{unix}:{style}>"),
    );
    Some(app.sync())
}

#[cfg(test)]
mod picker_tests {
    use chrono::TimeZone;

    use super::*;

    fn local(day: u32, hour: u32, minute: u32) -> chrono::DateTime<chrono::Local> {
        chrono::Local
            .with_ymd_and_hms(2026, 11, day, hour, minute, 42)
            .earliest()
            .expect("a local time")
    }

    #[test]
    fn the_picker_offers_times_to_the_minute() {
        let now = local(23, 17, 5);
        let times = picker_times(now);
        let labels: Vec<_> = times.iter().map(|(label, _)| *label).collect();
        assert_eq!(
            labels,
            [
                "In 15 minutes",
                "In 1 hour",
                "In 3 hours",
                "Tonight, 8 PM",
                "Tomorrow, noon",
                "Tomorrow, this time",
                "In a week",
            ]
        );
        assert_eq!(times[0].1, local(23, 17, 20).timestamp() - 42);
        assert_eq!(times[3].1, local(23, 20, 0).timestamp() - 42);
        assert!(
            times.iter().all(|(_, unix)| unix % 60 == 0),
            "to the minute"
        );
        // Past 8 PM, the evening is tomorrow's.
        let late = picker_times(local(23, 21, 0));
        assert_eq!(late[3].0, "Tomorrow, 8 PM");
        assert_eq!(late[3].1, local(24, 20, 0).timestamp() - 42);
    }

    #[test]
    fn each_style_says_how_it_will_look() {
        let now = local(23, 17, 0);
        let unix = local(23, 20, 0).timestamp() - 42;
        assert_eq!(looks(unix, 't', now), "20:00");
        assert_eq!(looks(unix, 'f', now), "23 November 2026 20:00");
        assert_eq!(looks(unix, 'F', now), "Monday, 23 November 2026 20:00");
        assert_eq!(looks(unix, 'R', now), "in 3 hours");
        assert_eq!(looks(now.timestamp() + 15 * 60, 'R', now), "in 15 minutes");
        assert_eq!(looks(now.timestamp() + 7 * 86_400, 'R', now), "in 7 days");
    }
}
