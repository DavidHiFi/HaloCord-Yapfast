//! Member list plugins: the member count line and Platform indicators.

use iced::Element;

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
