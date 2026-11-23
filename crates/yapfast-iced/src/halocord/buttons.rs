//! Button plugins: the user area's look and buttons, and the top bar's
//! Restart and Read all, Close all DMs, and Jump to start.

use iced::Task;
use yapfast_core::application::Action;

use super::{Kept, PLUGINS, Plugin, id, off};
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

pub(super) const READ_ALL: Plugin = Plugin {
    id: id::READ_ALL,
    name: "Read all button",
    about: "A button in the top bar (beside Settings when the bar is the system's) that marks every server and message you have unread as read, in one press.",
    on_at_first: true,
    settings: &[],
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
            stats: flag("stats_button"),
            stereo: flag("stereo_button"),
            game: flag("game_button"),
        }
    }
}

/// A button a plugin adds to the top bar or the user area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Restart,
    ReadAll,
    CloseDms,
    JumpStart,
}

/// The buttons the plugins that are on add: icon, tip and what they do.
pub fn presses(kept: &Kept) -> Vec<(Icon, &'static str, Press)> {
    let mut buttons = Vec::new();
    if kept.is(id::RESTART) {
        buttons.push((Icon::Refresh, "Restart Yapfast", Press::Restart));
    }
    if kept.is(id::READ_ALL) {
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
        Press::ReadAll => {
            for action in read_all_actions(app.client.state()) {
                app.client.dispatch(action);
            }
        }
    }
    app.sync()
}

// The User area's card: which of its buttons fit beside the name, the tray
// for the rest, and the name cut to its room (`account::view` calls these).

/// The width an extra button takes on the card, the gap between the card's
/// buttons, and the least the name keeps beside them (its padding
/// included) before buttons go in the tray.
const EXTRA: f32 = 32.0;
const GAP: f32 = 5.0;
const NAME_LEAST: f32 = 64.0;

/// How the card's row is shared: how many extra buttons stay on it, whether
/// the rest sit in the tray, and the room left for the name's words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fit {
    pub inline: usize,
    pub more: bool,
    pub name_room: f32,
    /// Even with every extra in the tray the card is too narrow (the
    /// sidebar dragged to its least): its buttons take the compact size.
    pub small: bool,
}

/// The card's width: the rail's and the sidebar's, inset, or the
/// sidebar's alone when compact.
fn card_width(ctx: crate::ctx::Ctx<'_>, compact: bool) -> f32 {
    crate::conversation::navigation(ctx.layout)
        - 2.0 * account::INSET
        - if compact { crate::rail::WIDTH } else { 0.0 }
}

/// Shares the card's row: the avatar, the mic and the headphones with
/// their carets, Reconnect while offline, and Settings always; then as
/// many of the `extras` as leave the name [`NAME_LEAST`], the rest behind
/// the tray's button.
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
    let tried = shared_at(card, compact, compact, carets, extras, reconnect);
    if compact || tried.name_room + 12.0 >= NAME_LEAST {
        return tried;
    }
    shared_at(card, compact, true, carets, extras, reconnect)
}

fn shared_at(
    card: f32,
    compact: bool,
    small: bool,
    carets: bool,
    extras: usize,
    reconnect: bool,
) -> Fit {
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
    let controls = |inline: usize, more: bool| {
        let items = 3 + usize::from(reconnect) + inline + usize::from(more);
        2.0 * (side + caret)
            + side * (1.0 + f32::from(u8::from(reconnect)) + f32::from(u8::from(more)))
            + EXTRA * inline as f32
            + GAP * (items - 1) as f32
    };
    let (inline, more) = if room - controls(extras, false) >= NAME_LEAST {
        (extras, false)
    } else {
        let inline = (0..extras)
            .rev()
            .find(|&inline| room - controls(inline, true) >= NAME_LEAST)
            .unwrap_or(0);
        (inline, true)
    };
    Fit {
        inline,
        more,
        // The names' padding is 6 px a side.
        name_room: (room - controls(inline, more) - 12.0).max(0.0),
        small,
    }
}

/// The buttons the card adds after the mic and the headphones: Reconnect
/// while offline, and the extras the User area switches on, in the card's
/// order (its switches, then the top bar's buttons when there's no top bar).
pub(crate) fn card_buttons<'a>(
    ctx: crate::ctx::Ctx<'a>,
    small: bool,
) -> (
    Option<iced::Element<'a, Message>>,
    Vec<iced::Element<'a, Message>>,
) {
    let theme = ctx.look.theme;
    let account = ctx.account;
    let dock = ctx.layout.halocord.dock();
    let side = if small { account::COMPACT_BUTTON } else { 32.0 };
    let glyph = |size: u32| if small { size - 3 } else { size };
    let kept = &ctx.layout.halocord;
    // A plugin's switch, lit in the accent while it is on.
    let switch_button = |id: &str, icon: Icon, on_label: &'static str, off_label: &'static str| {
        let on = kept.is(id);
        let message = super::place(id).map(|plugin| {
            Message::Account(account::Event::Plugin(super::Change::Switch {
                plugin,
                on: !on,
            }))
        });
        account::tinted_button(
            ctx,
            icon,
            if on { on_label } else { off_label },
            if on { theme.accent } else { theme.text },
            glyph(20),
            side,
            message,
        )
    };
    let mut extras = Vec::new();
    if dock.fake_mute {
        extras.push(switch_button(
            id::FAKE_MUTE,
            Icon::MicOff,
            "Stop faking mute",
            "Fake mute",
        ));
    }
    if dock.fake_deafen {
        extras.push(switch_button(
            id::FAKE_DEAFEN,
            Icon::HeadphoneOff,
            "Stop faking deafen",
            "Fake deafen",
        ));
    }
    if dock.stereo {
        extras.push(switch_button(
            id::STEREO,
            Icon::Music,
            "Turn stereo off",
            "Turn stereo on",
        ));
    }
    if dock.stats {
        extras.push(account::tinted_button(
            ctx,
            Icon::ChartBar,
            "Call stats",
            theme.text,
            glyph(20),
            side,
            (account.in_call && account.ready).then_some(Message::Call(crate::call::Event::Open(
                crate::call::Popup::Link,
            ))),
        ));
    }
    if dock.game {
        // Lit while the game is shared; it needs the Discord account.
        let shown = ctx.state.discord_show_game;
        extras.push(account::tinted_button(
            ctx,
            Icon::Gamepad,
            if shown {
                "Stop sharing your game"
            } else {
                "Share your game"
            },
            if shown { theme.accent } else { theme.text },
            glyph(20),
            side,
            ctx.state
                .discord_user
                .is_some()
                .then_some(Message::Account(account::Event::ShowGame)),
        ));
    }
    // Plugin buttons that sit in the top bar when Yapfast draws one.
    if !ctx.own_title_bar {
        for (icon, label, press) in presses(kept) {
            extras.push(account::tinted_button(
                ctx,
                icon,
                label,
                theme.text,
                glyph(20),
                side,
                Some(Message::Bar(crate::window_bar::Event::Halo(press))),
            ));
        }
    }
    // Offline, Reconnect before Settings, always on the card.
    let state = ctx.state;
    let reconnect = (!state.preview && !state.online).then(|| {
        account::tinted_button(
            ctx,
            Icon::Refresh,
            "Reconnect",
            theme.secondary,
            glyph(20),
            side,
            (!state.busy).then_some(Message::Account(account::Event::Reconnect)),
        )
    });
    (reconnect, extras)
}

/// The tray, while it's open and the card has buttons in it: those buttons
/// in a row on a menu's panel, its foot 4 px above the card and its right
/// edge at the card's.
pub fn card_tray<'a>(ctx: crate::ctx::Ctx<'a>) -> Option<iced::Element<'a, Message>> {
    if !ctx.account_more {
        return None;
    }
    let compact = ctx.layout.halocord.dock().compact;
    let (reconnect, extras) = card_buttons(ctx, compact);
    let fit = card_fit(ctx, compact, extras.len(), reconnect.is_some());
    let hidden: Vec<_> = extras.into_iter().skip(fit.inline).collect();
    if !fit.more || hidden.is_empty() {
        return None;
    }
    let count = hidden.len();
    let tray = iced::widget::container(iced::widget::Row::from_vec(hidden).spacing(GAP))
        .padding(6)
        .style(crate::controls::menu_panel(ctx.look.theme));
    let width = count as f32 * EXTRA + (count - 1) as f32 * GAP + 12.0 + 2.0;
    let right =
        account::INSET + if compact { crate::rail::WIDTH } else { 0.0 } + card_width(ctx, compact);
    let bottom = ctx.look.window.height - account::height(compact) - account::INSET - 4.0;
    Some(
        crate::widgets::Place::new(
            iced::widget::opaque(tray),
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
