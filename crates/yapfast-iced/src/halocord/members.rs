//! Member list plugins: the member count line, Platform indicators, Show
//! timeout duration, Better joined date, View icons, Show hidden things and Show song
//! name.

use iced::Element;

use chrono::NaiveDate;

use super::{Kept, Kind, Plugin, Setting, id, thousands};
use crate::{app::Message, ctx::Ctx, pictures::Icon, style::hex};

pub(super) const SHOW_SONG_NAME: Plugin = Plugin {
    id: id::SHOW_SONG_NAME,
    name: "Show song name",
    about: "Someone listening to Spotify shows the song's name under their name in the member list, not \"Spotify\".",
    on_at_first: false,
    settings: &[],
    apply: |kept, plugin| {
        yapfast_core::yapfast_discord::halocord::set_song_names(kept.on(plugin));
    },
    hooks: None,
};

/// Clickable roles in the member list: the role a heading names ("Mods — 3"),
/// by its id in the open server. Online, Offline and other groups aren't
/// roles, and neither is a name two of the server's roles share: the
/// heading can't say which, so it opens no card.
pub fn heading_role(cx: Ctx<'_>, heading: &str) -> Option<String> {
    let name = heading
        .rsplit_once(" — ")
        .filter(|(_, count)| !count.is_empty() && count.bytes().all(|byte| byte.is_ascii_digit()))
        .map_or(heading, |(name, _)| name);
    if matches!(name, "Online" | "Offline") {
        return None;
    }
    let place = cx.state.selected.as_deref()?;
    let mut named = cx
        .app
        .server_roles(place)
        .into_iter()
        .filter(|role| role.name == name);
    let role = named.next()?;
    named.next().is_none().then_some(role.id)
}

/// Hook: a member list heading, which opens its role's card when clicked
/// while Clickable roles is on and the heading is a role.
pub fn role_heading<'a>(
    cx: Ctx<'a>,
    heading: &str,
    line: Element<'a, Message>,
) -> Element<'a, Message> {
    let role = cx
        .layout
        .halocord
        .is(id::CLICKABLE_ROLES)
        .then(|| heading_role(cx, heading))
        .flatten();
    let Some(role) = role else {
        return line;
    };
    iced::widget::mouse_area(line)
        .on_press(Message::Info(crate::info_cards::Event::ProfileRole(role)))
        .interaction(iced::mouse::Interaction::Pointer)
        .into()
}

pub(super) const SHOW_CONNECTIONS: Plugin = Plugin {
    id: id::SHOW_CONNECTIONS,
    name: "Show connections",
    about: "Someone's profile card shows their connections (Steam, Twitch, GitHub and the like), as their full profile does.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// Hook: whether a profile card shows the connections, as the full profile
/// does, while Show connections is on.
pub fn connections_on_card(cx: Ctx<'_>) -> bool {
    cx.layout.halocord.is(id::SHOW_CONNECTIONS)
}

pub(super) const MEMBER_COUNT: Plugin = Plugin {
    id: id::MEMBER_COUNT,
    name: "Member count",
    about: "Shows how many people are online and how many have joined, in a line at the top of the member list.",
    on_at_first: false,
    settings: &[super::off(
        "tooltip",
        "In the server list's tooltips",
        "Adds how many members a server has after its name when you point at it in the server list.",
    )],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const SHOW_TIMEOUT: Plugin = Plugin {
    id: id::SHOW_TIMEOUT,
    name: "Show timeout duration",
    about: "Says how long a timeout has left: on the clock beside a timed out member's name when you point at it, and in the message box while you're timed out.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// How long is left until `until` (milliseconds) from `now`, in words:
/// "under a minute", "5 minutes", "3 hours", "2 days".
pub fn time_left(until: u64, now: u64) -> String {
    let minutes = until.saturating_sub(now) / 60_000;
    let (count, unit) = match minutes {
        0 => return "under a minute".into(),
        1..60 => (minutes, "minute"),
        60..1440 => (minutes / 60, "hour"),
        _ => (minutes / 1440, "day"),
    };
    format!("{count} {unit}{}", if count == 1 { "" } else { "s" })
}

/// The clock beside a timed out member's name, saying how long is left
/// when pointed at while Show timeout duration is on.
pub fn timeout_clock<'a>(cx: Ctx<'a>, until: u64) -> Element<'a, Message> {
    let theme = cx.look.theme;
    let clock = cx.look.glyph(Icon::Clock, 14, theme.danger);
    if !cx.layout.halocord.is(id::SHOW_TIMEOUT) {
        return clock.into();
    }
    let left = time_left(until, yapfast_core::clock::now_ms());
    iced::widget::tooltip(
        clock,
        crate::controls::tip(theme, format!("Timed out, {left} left")),
        iced::widget::tooltip::Position::Top,
    )
    .gap(6)
    .into()
}

pub(super) const SHOW_HIDDEN: Plugin = Plugin {
    id: id::SHOW_HIDDEN,
    name: "Show hidden things",
    about: "Shows what Discord's client shows only to moderators: the clock by a timed out member's name on their messages too, not only in the member list, and \"Invites paused\" in a server's name when you point at it in the server list.",
    on_at_first: false,
    settings: &[
        Setting {
            key: "timeouts",
            heading: "Timeouts on messages",
            note: "The clock by a timed out member's name on their messages.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
        Setting {
            key: "invites",
            heading: "Invites paused",
            note: "In a server's name when you point at it in the server list.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
    ],
    apply: |_, _| {},
    hooks: None,
};

/// Whether Show hidden things is on with its switch `setting` on.
fn shows_hidden(cx: Ctx<'_>, setting: usize) -> bool {
    let kept = &cx.layout.halocord;
    kept.on(&SHOW_HIDDEN) && kept.flag(&SHOW_HIDDEN, &SHOW_HIDDEN.settings[setting])
}

/// Hook: the clock by a timed out member's name on their message, as the
/// member list has it, while Show hidden things shows timeouts there.
pub fn message_timeout<'a>(cx: Ctx<'a>, person: &str) -> Option<Element<'a, Message>> {
    if !shows_hidden(cx, 0) {
        return None;
    }
    // A message's sender is `discord:` and the member's id.
    let person = yapfast_core::discord::sender_user(person).unwrap_or(person);
    let now = yapfast_core::clock::now_ms();
    let Some(Ok(members)) = &cx.state.members else {
        return None;
    };
    let until = members
        .iter()
        .find(|member| member.id == person)
        .and_then(|member| member.timed_out_until)
        .filter(|until| *until > now)?;
    Some(timeout_clock(cx, until))
}

/// Hook: a server's name as the server list names it when pointed at, with
/// "Invites paused" after it while they are and Show hidden things says so.
pub fn server_tip<'a>(
    cx: Ctx<'_>,
    server: &str,
    name: std::borrow::Cow<'a, str>,
) -> std::borrow::Cow<'a, str> {
    let paused = yapfast_core::discord::parse_server(server)
        .is_some_and(yapfast_core::yapfast_discord::halocord::invites_paused);
    if paused && shows_hidden(cx, 1) {
        std::borrow::Cow::Owned(format!("{name} · Invites paused"))
    } else {
        name
    }
}

/// What the message box adds after when a timeout ends, while Show timeout
/// duration is on: how long is left.
pub fn timeout_note(cx: Ctx<'_>, until: u64) -> String {
    if cx.layout.halocord.is(id::SHOW_TIMEOUT) {
        format!(
            " ({} left)",
            time_left(until, yapfast_core::clock::now_ms())
        )
    } else {
        String::new()
    }
}

pub(super) const PLATFORMS: Plugin = Plugin {
    id: id::PLATFORMS,
    name: "Platform indicators",
    about: "Shows which of Discord's apps a member is on, desktop, phone, web or a console, beside their name in the member list, coloured by their status there.",
    on_at_first: false,
    settings: &[
        super::off(
            "dms",
            "In the DM list",
            "The marks by each friend's name in your direct messages.",
        ),
        super::off(
            "messages",
            "On messages",
            "The marks by the author's name on each message.",
        ),
        super::off(
            "voice",
            "Under voice channels",
            "The marks by each person in a voice channel in the channel list.",
        ),
        super::off(
            "profiles",
            "On profiles",
            "The marks after the username on someone's profile.",
        ),
    ],
    apply: |_, _| {},
    hooks: None,
};

/// Where a person's name is drawn, besides the member list, for the marks
/// plugins add by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A direct message's row in the DM list.
    Dms,
    /// A message's author line.
    Message,
    /// A person under a voice channel in the channel list.
    Voice,
    /// The line under a profile's name.
    Profile,
    /// A member's row in the member list.
    MemberList,
}

/// What plugins add by a person's name in `place`, by their Discord user
/// id: User voice show's speaker on messages and profiles, then Platform
/// indicators' marks, each while its switch for the place is on. None when
/// there's nothing.
pub fn name_marks<'a>(cx: Ctx<'a>, user: &str, place: Place) -> Option<Element<'a, Message>> {
    let voice = match place {
        Place::Message => super::voice::message_voice_mark(cx, user),
        Place::Profile => super::voice::profile_voice_mark(cx, user),
        Place::Dms | Place::Voice | Place::MemberList => None,
    };
    let marks: Vec<Element<'a, Message>> = super::chat::staff_mark(cx, user, place)
        .into_iter()
        .chain(super::chat::new_account_mark(cx, user, place))
        .chain(voice)
        .chain(place_platforms(cx, user, place))
        .collect();
    (!marks.is_empty()).then(|| iced::widget::Row::with_children(marks).spacing(4).into())
}

/// Platform indicators' marks in `place`, while its switch for it is on.
fn place_platforms<'a>(cx: Ctx<'a>, user: &str, place: Place) -> Option<Element<'a, Message>> {
    let kept = &cx.layout.halocord;
    let setting = match place {
        Place::Dms => 0,
        Place::Message => 1,
        Place::Voice => 2,
        Place::Profile => 3,
        // The member list's own call draws them.
        Place::MemberList => return None,
    };
    if !kept.flag(&PLATFORMS, &PLATFORMS.settings[setting]) {
        return None;
    }
    platform_marks(cx, user)
}

/// What plugins add by the name of the person a direct message is with, by
/// the DM's room id ([`name_marks`] in the DM list).
pub fn dm_marks<'a>(cx: Ctx<'a>, room: &str) -> Option<Element<'a, Message>> {
    // Nothing to look up while no plugin draws by DM names.
    let kept = &cx.layout.halocord;
    if !kept.on(&PLATFORMS) || !kept.flag(&PLATFORMS, &PLATFORMS.settings[0]) {
        return None;
    }
    let person = cx
        .state
        .rooms
        .iter()
        .find(|listed| listed.id == room)?
        .person
        .as_deref()?;
    name_marks(cx, person, Place::Dms)
}

/// The marks after a member's name for each of Discord's apps they're on
/// (Platform indicators), each in its status's colour and named when it's
/// pointed at. None while the plugin is off or nothing is known.
pub fn platform_marks<'a>(cx: Ctx<'a>, user: &str) -> Option<Element<'a, Message>> {
    use yapfast_core::yapfast_discord::halocord::{Status, clients};
    if !cx.layout.halocord.is(id::PLATFORMS) {
        return None;
    }
    let seen = clients(user);
    let theme = cx.look.theme;
    let marks: Vec<Element<'a, Message>> = [
        (seen.desktop, Icon::HaloDesktop, "Desktop"),
        (seen.mobile, Icon::HaloPhone, "Phone"),
        (seen.web, Icon::HaloWeb, "Web"),
        (seen.console, Icon::Gamepad, "Console"),
    ]
    .into_iter()
    .filter_map(|(status, icon, app)| {
        let (colour, said) = match status? {
            Status::Online => (theme.live, "online"),
            Status::Idle => (hex(0xf0b232), "idle"),
            Status::DoNotDisturb => (hex(0xf23f43), "do not disturb"),
        };
        Some(
            iced::widget::tooltip(
                cx.look.glyph(icon, 14, colour),
                crate::controls::tip(theme, format!("{app}: {said}")),
                iced::widget::tooltip::Position::Top,
            )
            .gap(6)
            .into(),
        )
    })
    .collect();
    (!marks.is_empty()).then(|| iced::widget::Row::with_children(marks).spacing(2).into())
}

/// The people in a member list's group headings (`Online \u{2014} 12`), as
/// (online, all): everyone but the group called Offline is online.
pub fn member_totals(lines: &[crate::looks::MemberLine]) -> (usize, usize) {
    let (mut online, mut all) = (0, 0);
    for line in lines {
        let crate::looks::MemberLine::Heading { text, .. } = line else {
            continue;
        };
        let Some((name, count)) = text.rsplit_once(" \u{2014} ") else {
            continue;
        };
        let Ok(count) = count.trim().parse::<usize>() else {
            continue;
        };
        all += count;
        if !name.trim().eq_ignore_ascii_case("offline") {
            online += count;
        }
    }
    (online, all)
}

/// Member count's tooltip on a server's icon, while its switch for it is on:
/// the name, then "· 1,234 members".
pub fn tip_member_count<'a>(
    cx: crate::ctx::Ctx<'_>,
    place: &str,
    name: std::borrow::Cow<'a, str>,
) -> std::borrow::Cow<'a, str> {
    let kept = &cx.layout.halocord;
    if !kept.on(&MEMBER_COUNT) || !kept.flag(&MEMBER_COUNT, &MEMBER_COUNT.settings[0]) {
        return name;
    }
    // As of sign-in: Discord's count isn't updated until the next one.
    match cx.app.server_member_count(place) {
        Some(total) => std::borrow::Cow::Owned(member_tip(&name, total)),
        None => name,
    }
}

/// A server's name with its member count after it.
fn member_tip(name: &str, total: u64) -> String {
    format!(
        "{name} \u{b7} {} member{}",
        thousands(total as usize),
        if total == 1 { "" } else { "s" }
    )
}

/// Puts the count's line first in a member list, when its plugin is on and
/// there are people to count.
pub fn add_member_count(kept: &Kept, lines: &mut Vec<crate::looks::MemberLine>) {
    if !kept.is(id::MEMBER_COUNT) {
        return;
    }
    let (online, all) = member_totals(lines);
    if all == 0 {
        return;
    }
    lines.insert(
        0,
        crate::looks::MemberLine::Heading {
            text: format!(
                "{} online \u{b7} {} members",
                thousands(online),
                thousands(all)
            ),
            icon: None,
        },
    );
}

pub(super) const BETTER_JOINED_DATE: Plugin = Plugin {
    id: id::BETTER_JOINED_DATE,
    name: "Better joined date",
    about: "Point at the dates on a profile, when they made their account and joined the server, to see the day of the week and how many days ago it was.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// A profile's date, `shown` as `day` (such as `Nov 5, 2026`), saying the
/// whole date and how long ago when pointed at while Better joined date is
/// on.
pub fn exact_day<'a>(
    cx: Ctx<'a>,
    shown: impl Into<Element<'a, Message>>,
    day: &str,
) -> Element<'a, Message> {
    let today = chrono::Local::now().date_naive();
    match day_words(day, today).filter(|_| cx.layout.halocord.is(id::BETTER_JOINED_DATE)) {
        Some(words) => iced::widget::tooltip(
            shown,
            crate::controls::tip(cx.look.theme, words),
            iced::widget::tooltip::Position::Top,
        )
        .gap(6)
        .into(),
        None => shown.into(),
    }
}

/// `day`, as a profile writes it, in full with how long ago it was from
/// `today`: "Thursday, November 5, 2026, 383 days ago".
fn day_words(day: &str, today: NaiveDate) -> Option<String> {
    let date = NaiveDate::parse_from_str(day.trim(), "%b %d, %Y").ok()?;
    let ago = match (today - date).num_days() {
        ..0 => return None,
        0 => "today".to_owned(),
        1 => "yesterday".to_owned(),
        days => format!("{} days ago", thousands(days as usize)),
    };
    Some(format!("{}, {ago}", date.format("%A, %B %-d, %Y")))
}

pub(super) const VIEW_ICONS: Plugin = Plugin {
    id: id::VIEW_ICONS,
    name: "View icons",
    about: "Click the banner on a profile, or the picture on a full profile, to open it full size in your browser; View decoration in someone's menu opens the decoration around their picture.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// What a click on a profile's picture at `url` sends while View icons is
/// on: it opens the picture at its biggest.
pub(super) const CLICKABLE_ROLES: Plugin = Plugin {
    id: id::CLICKABLE_ROLES,
    name: "Clickable roles",
    about: "Click a role on someone's profile, or a role's heading in the member list, to see who else has it.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// What a click on the role `role` (its Discord id) on a profile sends
/// while Clickable roles is on: its card, in place of the profile.
pub fn role_press(cx: Ctx<'_>, role: &str) -> Option<Message> {
    (cx.layout.halocord.is(id::CLICKABLE_ROLES) && !role.is_empty())
        .then(|| Message::Info(crate::info_cards::Event::ProfileRole(role.to_owned())))
}

pub(super) const IMAGE_ZOOM: Plugin = Plugin {
    id: id::IMAGE_ZOOM,
    name: "Image zoom",
    about: "Click a picture you opened to see it bigger and scroll around it. Click again to fit it back.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// The size a picture opened large shows at while Image zoom has it
/// zoomed: its own size, or twice the fitted size if that's bigger.
pub fn zoom_size(
    cx: Ctx<'_>,
    zoomed: bool,
    natural: iced::Size,
    fitted: iced::Size,
) -> Option<iced::Size> {
    (zoomed && cx.layout.halocord.is(id::IMAGE_ZOOM)).then(|| {
        let by = (natural.width / fitted.width.max(1.0)).max(2.0);
        iced::Size::new((fitted.width * by).round(), (fitted.height * by).round())
    })
}

/// The picture opened large while Image zoom is on: a click zooms it in or
/// back, and zoomed it scrolls inside the fitted size.
pub fn zoomable<'a>(
    cx: Ctx<'a>,
    picture: Element<'a, Message>,
    zoom: Option<iced::Size>,
    fitted: iced::Size,
) -> Element<'a, Message> {
    use iced::{
        mouse::Interaction,
        widget::{mouse_area, scrollable},
    };
    if !cx.layout.halocord.is(id::IMAGE_ZOOM) {
        return picture;
    }
    let press = Message::Image(crate::image_viewer::Event::Zoom);
    match zoom {
        None => mouse_area(picture)
            .on_press(press)
            .interaction(Interaction::ZoomIn)
            .into(),
        Some(_) => mouse_area(
            scrollable(picture)
                .direction(scrollable::Direction::Both {
                    vertical: scrollable::Scrollbar::new(),
                    horizontal: scrollable::Scrollbar::new(),
                })
                .width(fitted.width)
                .height(fitted.height),
        )
        .on_press(press)
        .interaction(Interaction::ZoomOut)
        .into(),
    }
}

pub fn full_picture(cx: Ctx<'_>, url: Option<&str>) -> Option<Message> {
    let url = url.filter(|_| cx.layout.halocord.is(id::VIEW_ICONS))?;
    Some(Message::Profile(crate::profile::Event::ViewPicture(
        biggest(url),
    )))
}

/// `url` asking Discord's picture server for the largest size it gives.
fn biggest(url: &str) -> String {
    match url.split_once("size=") {
        Some((before, after)) => {
            let rest = after.trim_start_matches(|c: char| c.is_ascii_digit());
            format!("{before}size=4096{rest}")
        }
        None => url.to_owned(),
    }
}

#[cfg(test)]
mod picture_tests {
    #[test]
    fn a_picture_opens_at_its_biggest() {
        use super::biggest;
        assert_eq!(
            biggest("https://cdn.discordapp.com/avatars/1/a.png?size=128"),
            "https://cdn.discordapp.com/avatars/1/a.png?size=4096"
        );
        assert_eq!(
            biggest("https://cdn.discordapp.com/banners/1/b.gif?size=600&quality=lossless"),
            "https://cdn.discordapp.com/banners/1/b.gif?size=4096&quality=lossless"
        );
        assert_eq!(
            biggest("https://cdn.discordapp.com/embed/avatars/3.png"),
            "https://cdn.discordapp.com/embed/avatars/3.png"
        );
    }
}

#[cfg(test)]
mod day_tests {
    use super::*;

    #[test]
    fn a_profile_date_reads_in_full_with_how_long_ago() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        assert_eq!(
            day_words("Nov 5, 2025", today).as_deref(),
            Some("Wednesday, November 5, 2025, 383 days ago")
        );
        assert_eq!(
            day_words("Nov 23, 2026", today).as_deref(),
            Some("Monday, November 23, 2026, today")
        );
        assert_eq!(
            day_words("Nov 22, 2026", today).as_deref(),
            Some("Sunday, November 22, 2026, yesterday")
        );
        assert_eq!(
            day_words("May 4, 2015", today).as_deref(),
            Some("Monday, May 4, 2015, 4,221 days ago")
        );
        for unread in ["", "soon", "Dec 1, 2026"] {
            assert_eq!(day_words(unread, today), None, "{unread}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_servers_tooltip_says_its_member_count() {
        assert_eq!(member_tip("Test", 1234), "Test \u{b7} 1,234 members");
        assert_eq!(member_tip("Test", 1), "Test \u{b7} 1 member");
    }
}
