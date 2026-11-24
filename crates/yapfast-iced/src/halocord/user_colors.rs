//! Custom user colours: "Set colour" in someone's menu gives their name a
//! colour of your choosing, for you alone, on their messages, in the member
//! list and in your DMs; "Reset colour" puts theirs back. The colours are
//! kept in `halocord-user-colors.json` in the app's folder, by user id;
//! nothing is sent to Discord.

use std::{collections::BTreeMap, path::Path, sync::Mutex};

use iced::Color;

use super::{Kept, Kind, Plugin, Setting, id};
use crate::{
    app::App,
    ctx::Ctx,
    menus::{MenuAct, MenuRow},
    pictures::Icon,
};

pub(super) const CUSTOM_USER_COLORS: Plugin = Plugin {
    id: id::USER_COLORS,
    name: "Custom user colours",
    about: "\"Set colour\" in someone's menu colours their name for you alone, on their messages, in the member list and in your DMs; \"Reset colour\" puts theirs back. Kept in halocord-user-colors.json in Yapfast's folder, on this computer only.",
    on_at_first: false,
    settings: &[
        Setting {
            key: "dm_list",
            heading: "In the DM list",
            note: "Their name in your DMs takes the colour too.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
        Setting {
            key: "servers",
            heading: "In servers too",
            note: "Off, the colours show only in DMs and group DMs.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
    ],
    apply: |_, _| {},
    hooks: None,
};

/// The colours offered: Discord's role colours, by name.
pub const SWATCHES: [(&str, u32); 12] = [
    ("Teal", 0x1a_bc_9c),
    ("Green", 0x2e_cc_71),
    ("Blue", 0x34_98_db),
    ("Purple", 0x9b_59_b6),
    ("Pink", 0xe9_1e_63),
    ("Yellow", 0xf1_c4_0f),
    ("Orange", 0xe6_7e_22),
    ("Red", 0xe7_4c_3c),
    ("Grey", 0x95_a5_a6),
    ("Slate", 0x60_7d_8b),
    ("White", 0xff_ff_ff),
    ("Black", 0x23_27_2a),
];

const FILE: &str = "halocord-user-colors.json";

/// The colours by user id, and the file they were read from.
static COLORS: Mutex<Option<(std::path::PathBuf, BTreeMap<String, u32>)>> = Mutex::new(None);

fn colors<T>(
    with: impl FnOnce(&mut Option<(std::path::PathBuf, BTreeMap<String, u32>)>) -> T,
) -> T {
    with(
        &mut COLORS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    )
}

/// Hook, as the window makes its lists again: reads the colours kept in
/// `root`, once for each folder, while the plugin is on.
pub fn load_user_colors(kept: &Kept, root: &Path) {
    if !kept.is(id::USER_COLORS) {
        return;
    }
    let path = root.join(FILE);
    colors(|held| {
        if held.as_ref().is_none_or(|(read, _)| *read != path) {
            let read = std::fs::read(&path)
                .ok()
                .and_then(|read| serde_json::from_slice(&read).ok())
                .unwrap_or_default();
            *held = Some((path, read));
        }
    });
}

/// Sets `person`'s colour, or with none takes it away, and keeps it in
/// `root`.
fn set(root: &Path, person: &str, color: Option<u32>) {
    let path = root.join(FILE);
    let json = colors(|held| {
        if held.as_ref().is_none_or(|(read, _)| *read != path) {
            let read = std::fs::read(&path)
                .ok()
                .and_then(|read| serde_json::from_slice(&read).ok())
                .unwrap_or_default();
            *held = Some((path.clone(), read));
        }
        let (_, kept) = held.as_mut()?;
        match color {
            Some(color) => kept.insert(person.to_owned(), color),
            None => kept.remove(person),
        };
        serde_json::to_vec(kept).ok()
    });
    if let Some(json) = json {
        let beside = path.with_extension("json.new");
        if let Err(error) =
            std::fs::write(&beside, json).and_then(|()| std::fs::rename(&beside, &path))
        {
            tracing::warn!(%error, "Custom user colours can't keep the colours");
        }
    }
}

/// The colour set for `person` (a user id, or a message's `discord:` and
/// id), where it shows: in a server only with "In servers too", in the DM
/// list only with "In the DM list".
pub fn user_color(kept: &Kept, person: &str, in_server: bool, dm_list: bool) -> Option<u32> {
    if !kept.is(id::USER_COLORS)
        || (in_server && !kept.flag(&CUSTOM_USER_COLORS, &CUSTOM_USER_COLORS.settings[1]))
        || (dm_list && !kept.flag(&CUSTOM_USER_COLORS, &CUSTOM_USER_COLORS.settings[0]))
    {
        return None;
    }
    let person = yapfast_core::discord::sender_user(person).unwrap_or(person);
    colors(|held| {
        held.as_ref()
            .and_then(|(_, kept)| kept.get(person).copied())
    })
}

/// Hook: the colour for `person`'s name where it's drawn, when one is set;
/// it takes the place of their own colour and name style.
pub fn name_color(cx: Ctx<'_>, person: &str, in_server: bool, dm_list: bool) -> Option<Color> {
    user_color(&cx.layout.halocord, person, in_server, dm_list).map(crate::style::hex)
}

/// Hook: the colour for the person of the DM `room` in the DM list.
pub fn dm_name_color(cx: Ctx<'_>, room: &str) -> Option<Color> {
    let person = cx
        .state
        .rooms
        .iter()
        .find(|known| known.id == room)?
        .person
        .clone()?;
    name_color(cx, &person, false, true)
}

/// The rows for `user`'s menu: "Set colour" with a swatch each, and
/// "Reset colour" when one is set.
pub fn rows(app: &App, user: &str) -> Vec<MenuRow> {
    let kept = &app.layout.halocord;
    if !kept.is(id::USER_COLORS) || user.parse::<u64>().is_err() {
        return Vec::new();
    }
    let mut set = MenuRow::new(MenuAct::Plugin, "Set colour", Icon::Palette)
        .key(format!("{}/menu", id::USER_COLORS));
    set.submenu = SWATCHES
        .iter()
        .map(|(name, rgb)| {
            MenuRow::new(MenuAct::Plugin, name, Icon::Palette)
                .key(format!("{}/set/{rgb:06x}", id::USER_COLORS))
        })
        .collect();
    let mut rows = vec![set];
    if user_color(kept, user, false, false).is_some() {
        rows.push(
            MenuRow::new(MenuAct::Plugin, "Reset colour", Icon::Palette)
                .key(format!("{}/reset", id::USER_COLORS)),
        );
    }
    rows
}

/// What one of its rows does for `user`, if `key` is one.
pub fn pick(app: &App, key: &str, user: &str) -> Option<()> {
    let action = key.strip_prefix(id::USER_COLORS)?.strip_prefix('/')?;
    let root = app.client.root();
    match action {
        "reset" => set(root, user, None),
        "menu" => {}
        _ => {
            let rgb = u32::from_str_radix(action.strip_prefix("set/")?, 16).ok()?;
            set(root, user, Some(rgb));
        }
    }
    Some(())
}
