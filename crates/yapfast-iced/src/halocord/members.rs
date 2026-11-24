//! Member list plugins: the member count line, Platform indicators, Show
//! timeout duration, Better joined date and View icons.

use iced::Element;

use chrono::NaiveDate;

use super::{Kept, Plugin, id, thousands};
use crate::{app::Message, ctx::Ctx, pictures::Icon, style::hex};

pub(super) const MEMBER_COUNT: Plugin = Plugin {
    id: id::MEMBER_COUNT,
    name: "Member count",
    about: "Shows how many people are online and how many have joined, in a line at the top of the member list.",
    on_at_first: false,
    settings: &[],
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
    about: "Shows which of Discord's apps a member is on, desktop, phone or web, beside their name in the member list, coloured by their status there.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

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
    about: "Click the banner on a profile, or the picture on a full profile, to open it full size in your browser; View decoration in someone's menu opens the decoration around their picture, moving.",
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
    about: "Click a role on someone's profile to see who else has it.",
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
