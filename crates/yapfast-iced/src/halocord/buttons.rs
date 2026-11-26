//! Button plugins: the user area's look and buttons, and the top bar's
//! Restart and Read all, Close all DMs, and Jump to start.

use iced::Task;
use yapfast_core::application::Action;

use super::{Kept, Kind, PLUGINS, Plugin, Setting, id, off};
use crate::{
    account,
    app::{App, Message},
    pictures::Icon,
};

pub(super) const USER_AREA: Plugin = Plugin {
    id: id::USER_AREA,
    name: "User area",
    about: "How the card at the bottom of the channel list looks, and the buttons on it.",
    on_at_first: true,
    settings: &[
        off(
            "compact",
            "Compact mode",
            "A smaller, tighter card that stays inside the channel list.",
        ),
        off(
            "hide_arrows",
            "Hide the dropdown arrows",
            "Takes the little arrows off the mute and deafen buttons.",
        ),
        off(
            "fake_mute_button",
            "Fake mute button",
            "A button that switches Fake mute on and off.",
        ),
        off(
            "fake_deafen_button",
            "Fake deafen button",
            "A button that switches Fake deafen on and off.",
        ),
        off(
            "fake_camera_button",
            "Fake camera button",
            "A button that switches Fake camera on and off.",
        ),
        off(
            "fake_stream_button",
            "Fake stream button",
            "A button that switches Fake stream on and off.",
        ),
        off(
            "stats_button",
            "Stats button",
            "A button that opens the call's connection details.",
        ),
        off(
            "stereo_button",
            "Stereo button",
            "A button that switches the Stereo loader on and off.",
        ),
        off(
            "game_button",
            "Game activity button",
            "A button that switches sharing the game you play on and off.",
        ),
    ],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const RESTART: Plugin = Plugin {
    id: id::RESTART,
    name: "Restart button",
    about: "A button in the top bar (beside Settings when the bar is the system's) that closes Yapfast and starts it again the way it was started.",
    on_at_first: true,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// Where Read all's button sits.
const RAIL: u32 = 0;
const BAR: u32 = 1;
const BOTH: u32 = 2;

pub(super) const READ_ALL: Plugin = Plugin {
    id: id::READ_ALL,
    name: "Read all button",
    about: "A button at the top of the server list, or in the top bar (beside Settings when the bar is the system's), that marks every server and message you have unread as read, in one press.",
    on_at_first: true,
    settings: &[
        Setting {
            key: "place",
            heading: "Where it sits",
            note: "At the top of the server list, in the top bar, or both.",
            kind: Kind::Choice {
                names: &["Top of the server list", "Top bar", "Both"],
                values: &[RAIL, BAR, BOTH],
                unless_changed: RAIL,
            },
        },
        off(
            "compact",
            "Small text button",
            "The words Read all in a small pill instead of the tile with the tick.",
        ),
    ],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const CLOSE_DMS: Plugin = Plugin {
    id: id::CLOSE_DMS,
    name: "Close all DMs",
    about: "A small X beside Direct Messages that closes every one-to-one conversation in the list at once. Group chats stay. The messages aren't deleted, and a new message from someone opens theirs again.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const JUMP_TO_START: Plugin = Plugin {
    id: id::JUMP_TO_START,
    name: "Jump to start",
    about: "A button in the channel's top bar that jumps to the first message of the channel or DM you're in.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// How the user area looks and which buttons it has, from its plugin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dock {
    pub compact: bool,
    pub arrows_hidden: bool,
    pub fake_mute: bool,
    pub fake_deafen: bool,
    pub fake_camera: bool,
    pub fake_stream: bool,
    pub stats: bool,
    pub stereo: bool,
    pub game: bool,
}

impl Kept {
    /// The user area's options; as it started while its plugin is off.
    pub fn dock(&self) -> Dock {
        let Some(area) = PLUGINS
            .iter()
            .find(|plugin| plugin.id == id::USER_AREA)
            .filter(|plugin| self.on(plugin))
        else {
            return Dock::default();
        };
        let flag = |key: &str| {
            area.settings
                .iter()
                .find(|setting| setting.key == key)
                .is_some_and(|setting| self.flag(area, setting))
        };
        Dock {
            compact: flag("compact"),
            arrows_hidden: flag("hide_arrows"),
            fake_mute: flag("fake_mute_button"),
            fake_deafen: flag("fake_deafen_button"),
            fake_camera: flag("fake_camera_button"),
            fake_stream: flag("fake_stream_button"),
            stats: flag("stats_button"),
            stereo: flag("stereo_button"),
            game: flag("game_button"),
        }
    }
}

impl Kept {
    /// Where Read all's button sits while it is on.
    fn read_all_place(&self) -> Option<u32> {
        self.on(&READ_ALL)
            .then(|| self.value(&READ_ALL, &READ_ALL.settings[0]))
    }

    /// Read all's button at the top of the server list, while it is on and
    /// sits there: `Some(true)` for the small text button.
    pub fn read_all_on_rail(&self) -> Option<bool> {
        self.read_all_place()
            .filter(|place| *place != BAR)
            .map(|_| self.flag(&READ_ALL, &READ_ALL.settings[1]))
    }
}

/// A button a plugin adds to the top bar or the user area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Restart,
    ReadAll,
    CloseDms,
    JumpStart,
    /// Hide chat buttons' arrow by the message box.
    ChatButtons,
    /// Better user tools' rows in the account's popover.
    MicTest,
    OffTheRadar,
}

/// The buttons the plugins that are on add: icon, tip and what they do.
pub fn presses(kept: &Kept) -> Vec<(Icon, &'static str, Press)> {
    let mut buttons = Vec::new();
    if kept.is(id::RESTART) {
        buttons.push((Icon::Refresh, "Restart Yapfast", Press::Restart));
    }
    if kept.read_all_place().is_some_and(|place| place != RAIL) {
        buttons.push((Icon::MarkRead, "Mark everything read", Press::ReadAll));
    }
    buttons
}

/// Jump to start's button in the channel's top bar: icon, tip and what it
/// does, over a Discord channel or DM's messages (and the preview's).
pub fn start_button(cx: crate::ctx::Ctx<'_>) -> Option<(Icon, &'static str, Message)> {
    let discord = cx
        .state
        .selected
        .as_deref()
        .and_then(yapfast_core::discord::parse_room)
        .is_some();
    let shows = cx.layout.halocord.is(id::JUMP_TO_START)
        && cx.state.shows_text()
        && (cx.state.preview || discord);
    shows.then(|| {
        (
            Icon::ChevronUp,
            "Jump to start",
            Message::Bar(crate::window_bar::Event::Halo(Press::JumpStart)),
        )
    })
}

/// What marks everything read: every server, and each conversation with
/// something unread. The core drops those with nothing to mark.
pub(crate) fn read_all_actions(state: &yapfast_core::application::AppState) -> Vec<Action> {
    let servers = state.servers.iter().map(|server| server.id.clone());
    let chats = state
        .rooms
        .iter()
        .filter(|room| room.direct && (room.unread > 0 || room.mentions > 0))
        .map(|room| room.id.clone());
    servers.chain(chats).map(Action::MarkRead).collect()
}

/// What closes every conversation in the list: a close for each direct
/// one. The core drops the groups and anything that can't be closed.
pub(crate) fn close_dm_actions(state: &yapfast_core::application::AppState) -> Vec<Action> {
    state
        .rooms
        .iter()
        .filter(|room| room.direct)
        .map(|room| Action::CloseDm(room.id.clone()))
        .collect()
}

/// A plugin's button was pressed.
pub fn press(app: &mut App, press: Press) -> Task<Message> {
    match press {
        Press::Restart => app.requests.push(crate::ctx::Request::Restart),
        Press::CloseDms => {
            for action in close_dm_actions(app.client.state()) {
                app.client.dispatch(action);
            }
        }
        Press::JumpStart => app.client.dispatch(Action::JumpToStart),
        Press::ChatButtons => super::chat::toggle_chat_buttons(),
        Press::ReadAll => {
            for action in read_all_actions(app.client.state()) {
                app.client.dispatch(action);
            }
        }
        Press::MicTest => super::user_tools::toggle_mic_test(app),
        Press::OffTheRadar => super::user_tools::toggle_radar(app),
    }
    app.sync()
}

// The User area's card: the room its name keeps beside the mic, the
// headphones and Settings, how many plugin buttons fit on the row above it,
// the tray for the rest, and the name cut to its room (`account::view`
// calls these).

/// The gap between the buttons beside the name, and the least the name
/// keeps beside them (its padding included) before they take the compact
/// size.
const GAP: f32 = 5.0;
const NAME_LEAST: f32 = 64.0;
/// The least a plugin button keeps on the row above the name, the gap
/// between those buttons and the row's inset from the card's sides.
const PLUGIN_LEAST: f32 = 32.0;
const PLUGIN_GAP: f32 = 4.0;
const PLUGIN_INSET: f32 = 8.0;

/// How the card is shared: how many plugin buttons stay on the row above
/// the name, whether the rest sit in the tray, and the room left for the
/// name's words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fit {
    pub inline: usize,
    pub more: bool,
    pub name_room: f32,
    /// The card is too narrow for the full size (the sidebar dragged to
    /// its least): the buttons beside the name take the compact size.
    pub small: bool,
}

/// The card's width: the rail's and the sidebar's, inset, or the
/// sidebar's alone when compact.
fn card_width(ctx: crate::ctx::Ctx<'_>, compact: bool) -> f32 {
    let wide = if compact {
        crate::sidebar::width(ctx.layout, ctx.look.theme)
    } else {
        crate::conversation::navigation_ctx(ctx)
    };
    wide - 2.0 * account::INSET
}

/// Shares the card: beside the name the avatar, the mic and the headphones
/// with their carets, Reconnect while offline, and Settings always; above
/// it as many of the `extras` (the plugin buttons) as keep
/// [`PLUGIN_LEAST`] each, the rest behind the tray's button.
pub(crate) fn card_fit(
    ctx: crate::ctx::Ctx<'_>,
    compact: bool,
    extras: usize,
    reconnect: bool,
) -> Fit {
    let dock = ctx.layout.halocord.dock();
    shared(
        card_width(ctx, compact),
        compact,
        !dock.arrows_hidden,
        extras,
        reconnect,
    )
}

pub(crate) fn shared(
    card: f32,
    compact: bool,
    carets: bool,
    extras: usize,
    reconnect: bool,
) -> Fit {
    let (inline, more) = plugin_row(card, extras);
    let tried = beside_name(card, compact, compact, carets, reconnect);
    let (name_room, small) = if compact || tried.0 + 12.0 >= NAME_LEAST {
        tried
    } else {
        beside_name(card, compact, true, carets, reconnect)
    };
    Fit {
        inline,
        more,
        name_room,
        small,
    }
}

/// How many of `extras` plugin buttons stay on the row above the name,
/// [`PLUGIN_LEAST`] each at the least, and whether the rest go in the tray:
/// then the row's last place is the tray's button.
fn plugin_row(card: f32, extras: usize) -> (usize, bool) {
    let room = card - 2.0 * PLUGIN_INSET;
    let places = (((room + PLUGIN_GAP) / (PLUGIN_LEAST + PLUGIN_GAP)).floor() as usize).max(1);
    if extras <= places {
        (extras, false)
    } else {
        (places - 1, true)
    }
}

/// The room the name's words keep beside the card's own buttons at their
/// full or `small` size, and that size.
fn beside_name(
    card: f32,
    compact: bool,
    small: bool,
    carets: bool,
    reconnect: bool,
) -> (f32, bool) {
    let side = if small { account::COMPACT_BUTTON } else { 32.0 };
    let caret = match (carets, small) {
        (false, _) => 0.0,
        (true, true) => 16.0,
        (true, false) => 20.0,
    };
    let (face, right, spacing) = if compact {
        (account::COMPACT_AVATAR + 8.0, 8.0, 3.0)
    } else {
        (50.0, 14.0, 5.0)
    };
    // The names' and the buttons' room, after the avatar and the gaps.
    let room = card - right - face - 2.0 * spacing;
    // The mic and the headphones with their carets, Reconnect, Settings.
    let items = 3 + usize::from(reconnect);
    let controls = 2.0 * (side + caret)
        + side * (1.0 + f32::from(u8::from(reconnect)))
        + GAP * (items - 1) as f32;
    // The names' padding is 6 px a side.
    ((room - controls - 12.0).max(0.0), small)
}

/// The tray, while it's open or drawn going and the card has buttons in
/// it: those buttons in a row on a menu's panel, its foot 4 px above the
/// card and its right edge at the card's, growing in as popups do.
pub fn card_tray<'a>(ctx: crate::ctx::Ctx<'a>) -> Option<iced::Element<'a, Message>> {
    if !ctx.account_more && !ctx.tray_leaving {
        return None;
    }
    let compact = ctx.layout.halocord.dock().compact;
    let buttons = account::plugin_buttons(ctx);
    let (inline, more) = plugin_row(card_width(ctx, compact), buttons.len());
    if !more {
        return None;
    }
    let hidden: Vec<_> = buttons.into_iter().skip(inline).collect();
    let count = hidden.len();
    let line = hidden.into_iter().fold(
        iced::widget::Row::new().spacing(PLUGIN_GAP),
        |line, (icon, label, ink, message)| {
            line.push(account::plugin_button(ctx, icon, label, ink, message))
        },
    );
    let width = count as f32 * PLUGIN_LEAST + (count - 1) as f32 * PLUGIN_GAP + 12.0;
    let tray = iced::widget::container(line)
        .width(width)
        .padding(6)
        .style(crate::controls::menu_panel(ctx.look.theme));
    let right = account::INSET
        + if compact {
            crate::rail::width(ctx.layout)
        } else {
            0.0
        }
        + card_width(ctx, compact);
    let bottom = ctx.look.window.height - account::full_height(ctx) - account::INSET - 4.0;
    Some(
        crate::widgets::Place::new(
            crate::widgets::motion::grow(iced::widget::opaque(tray)),
            iced::Point::new((right - width).max(account::INSET), bottom),
            8.0,
        )
        .anchor(crate::widgets::Anchor::Bottom)
        .into(),
    )
}

/// How wide `words` are on one line, at `size` in `font`.
pub fn words_width(words: &str, size: f32, font: iced::Font) -> f32 {
    use iced::advanced::{
        graphics::text::Paragraph,
        text::{self, LineHeight, Paragraph as _, Shaping, Wrapping},
    };
    Paragraph::with_text(text::Text {
        content: words,
        bounds: iced::Size::INFINITE,
        size: size.into(),
        line_height: LineHeight::default(),
        font,
        align_x: text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::None,
    })
    .min_width()
}

/// `words` as they fit in `room` px on one line, at `size` in `font`:
/// whole, or cut with an ellipsis where they'd run past it.
pub fn ellipsized(words: &str, size: f32, font: iced::Font, room: f32) -> String {
    type Key = (String, u32, u32, u32);
    thread_local! {
        // The card draws on every view: each name is measured once a room.
        static CUT: std::cell::RefCell<std::collections::HashMap<Key, String>> =
            std::cell::RefCell::default();
    }
    let weight = if font == crate::fonts::BOLD { 1 } else { 0 };
    let key = (
        words.to_owned(),
        size.to_bits(),
        weight,
        room.round() as u32,
    );
    if let Some(cut) = CUT.with(|kept| kept.borrow().get(&key).cloned()) {
        return cut;
    }
    let cut = cut_to(words, size, font, room.round());
    CUT.with(|kept| {
        let mut kept = kept.borrow_mut();
        if kept.len() > 256 {
            kept.clear();
        }
        kept.insert(key, cut.clone());
    });
    cut
}

fn cut_to(words: &str, size: f32, font: iced::Font, room: f32) -> String {
    if words.is_empty() || words_width(words, size, font) <= room {
        return words.to_owned();
    }
    let ends: Vec<usize> = words
        .char_indices()
        .map(|(at, _)| at)
        .skip(1)
        .chain([words.len()])
        .collect();
    let cut = |count: usize| format!("{}…", words[..ends[count - 1]].trim_end());
    // The most letters that fit with the ellipsis; a letter and it at least.
    let (mut low, mut high) = (1, ends.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        if words_width(&cut(middle), size, font) <= room {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    cut(low)
}
