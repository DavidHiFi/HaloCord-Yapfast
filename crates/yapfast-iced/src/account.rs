//! The account's card at the foot of the rail and the sidebar: the avatar
//! with its presence, the name and what the account is up to (its username
//! under the pointer), then the mic, the headphones and Settings.

use iced::{
    Alignment, Element, Length, Padding, Point,
    widget::{Space, column, container, mouse_area, row, stack, text},
};
use yapfast_core::{
    application::AppState,
    chat::{CustomStatus, Presence},
    voice,
};

use crate::{
    app::{App, Message},
    fonts::BOLD,
    looks::{Status, one_line},
    people::StatusInk,
    pictures::{Face, Icon},
    style::{fill, rounded},
    theme::rgb,
    widgets::{Pic, PlateRow},
};

/// The card's height and its inset from the window's edges.
pub const HEIGHT: f32 = 57.0;
pub const INSET: f32 = 8.0;
/// The avatar on the card.
const AVATAR: f32 = 40.0;
/// The compact card's height, avatar and buttons (the User area plugin).
const COMPACT_HEIGHT: f32 = 44.0;
pub(crate) const COMPACT_AVATAR: f32 = 30.0;
pub(crate) const COMPACT_BUTTON: f32 = 28.0;

/// The card's height, compact or not, without the plugin buttons' row.
pub fn height(compact: bool) -> f32 {
    if compact { COMPACT_HEIGHT } else { HEIGHT }
}

/// The plugin buttons' row above the name: its buttons' height, the gap
/// above them and the inset from the card's sides.
const PLUGIN_BUTTON: f32 = 32.0;
const PLUGIN_TOP: f32 = 6.0;
const PLUGIN_INSET: f32 = 8.0;

/// The gap above the plugin buttons' row: compact tightens it too.
fn plugin_top(compact: bool) -> f32 {
    if compact { 2.0 } else { PLUGIN_TOP }
}

/// The window's own corner, which the dock's bottom-left follows (the
/// custom corner radius on this desktop). A maximized window has none.
pub const WINDOW_RADIUS: f32 = 16.0;

/// What the card takes in all: its height, and the plugin buttons' row when
/// one is on.
pub fn full_height(ctx: crate::ctx::Ctx<'_>) -> f32 {
    let compact = ctx.layout.halocord.dock().compact;
    height(compact)
        + if plugin_buttons(ctx).is_empty() {
            0.0
        } else {
            PLUGIN_BUTTON + plugin_top(compact)
        }
}

/// What the card shows of the account.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Account {
    pub name: String,
    pub username: String,
    pub avatar: Option<String>,
    pub presence: Option<Presence>,
    pub status: Status,
    /// In a call, and muted or deafened there; out of one, how calls are
    /// joined.
    pub in_call: bool,
    pub muted: bool,
    /// Discord won't let this account speak in the call's channel, so
    /// its mute shows on and does nothing.
    pub speaking_blocked: bool,
    pub deafened: bool,
    /// The call's voice is through, so its mute and deafen act; while it
    /// joins or leaves they wait.
    pub ready: bool,
    /// The microphone carries voice in the call.
    pub speaking: bool,
    /// Discord is linked, so the avatar and the name open the status's
    /// popover.
    pub linked: bool,
    /// The dot shown without a Discord presence: the preview's, or whether
    /// the account syncs.
    pub dot: Dot,
}

/// What the dot on an account with no Discord presence says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dot {
    #[default]
    Preview,
    Online,
    Offline,
}

impl Account {
    /// Shown muted: in a call, as it is; out of one, as calls are joined.
    pub(crate) fn shown_muted(&self, join: (bool, bool)) -> bool {
        if self.in_call {
            self.muted || self.speaking_blocked
        } else {
            !join.0 || join.1
        }
    }

    pub(crate) fn shown_deafened(&self, join: (bool, bool)) -> bool {
        if self.in_call { self.deafened } else { join.1 }
    }
}

/// What a custom status says, and its custom emoji's picture.
fn custom_words(status: &CustomStatus) -> (String, Option<String>) {
    let text = status.text.trim();
    match (&status.emoji, &status.emoji_id) {
        (_, Some(id)) => (one_line(text), yapfast_core::discord::emoji_picture(id)),
        (Some(emoji), None) if text.is_empty() => (emoji.clone(), None),
        (Some(emoji), None) => (one_line(&format!("{emoji} {text}")), None),
        (None, None) => (one_line(text), None),
    }
}

/// The card, from the core's state.
pub fn account(state: &AppState) -> Account {
    // The profile's name, else the account's id with a Matrix id's server
    // left off.
    let id = state.account.as_deref().unwrap_or("Preview");
    let name = state.account_name.as_deref().unwrap_or_else(|| {
        id.strip_prefix('@')
            .and_then(|rest| rest.split_once(':'))
            .map_or(id, |(local, _)| local)
    });
    let live = state.online && !state.preview;
    let custom = state
        .custom_status
        .as_ref()
        .filter(|_| live)
        .map(custom_words);
    // The game shown as playing: alone it names the game, before a custom
    // status it adds a bullet.
    let playing = state
        .discord_playing
        .as_ref()
        .filter(|_| live && state.discord_show_game);
    // What this account shares, as Discord's panel writes it: the first
    // activity's icon and words, or its icon, a bullet and the custom
    // status.
    let shared = (live && !state.discord_activities.is_empty()).then(|| {
        crate::looks::Status::activity(
            &state.discord_activities,
            custom.as_ref().map(|(words, _)| words.as_str()),
        )
    });
    let words = if state.preview {
        "Local only".to_owned()
    } else if let Some(shared) = shared.as_ref().filter(|shared| !shared.bullet) {
        shared.words.clone()
    } else if let Some((words, _)) = &custom {
        words.clone()
    } else if let Some(playing) = playing {
        playing.name.clone()
    } else if let Some(status) = &state.account_status {
        status.clone()
    } else if state.online {
        "Online".to_owned()
    } else {
        "Offline, sync paused".to_owned()
    };
    let presence = if cfg!(feature = "matrix") || state.preview {
        None
    } else if !state.discord_online {
        Some(Presence::Offline)
    } else {
        Some(match state.discord_presence.as_deref() {
            Some("idle") => Presence::Idle,
            Some("dnd") => Presence::DoNotDisturb,
            Some("invisible") => Presence::Offline,
            _ => Presence::Online,
        })
    };
    Account {
        name: name.to_owned(),
        username: state.account_username.clone().unwrap_or_default(),
        avatar: state.account_avatar.clone(),
        presence,
        status: match &shared {
            Some(shared) => Status {
                words,
                emoji: if shared.bullet {
                    custom.and_then(|(_, emoji)| emoji)
                } else {
                    None
                },
                icon: shared.icon,
                bullet: shared.bullet,
                more: shared.more,
            },
            None => Status {
                words,
                emoji: custom.and_then(|(_, emoji)| emoji),
                icon: u8::from(playing.is_some()),
                bullet: playing.is_some() && state.custom_status.is_some(),
                more: 0,
            },
        },
        in_call: state.voice.active(),
        muted: state.voice.muted,
        speaking_blocked: state.voice.speaking_blocked(),
        deafened: state.voice.deafened,
        ready: matches!(
            state.voice.phase,
            voice::Phase::Connected | voice::Phase::Reconnecting
        ),
        // As the account's own tile in the call rings.
        speaking: state.voice.active()
            && state
                .voice
                .participants
                .iter()
                .any(|person| person.local && crate::call::speaking(person)),
        linked: state.discord_linked || state.servers.iter().any(|server| server.discord),
        dot: if state.preview {
            Dot::Preview
        } else if state.online {
            Dot::Online
        } else {
            Dot::Offline
        },
    }
}

/// What's done on the card.
#[derive(Clone, Debug)]
pub enum Event {
    Point(bool),
    Mute,
    Deafen,
    /// Offline: connect again now, not when the backend next tries.
    Reconnect,
    /// The Game activity button: share the game played, or stop.
    ShowGame,
    /// The tray of buttons the card has no room for: shown over it, or gone.
    More(bool),
    /// One of the card's plugin buttons: a plugin switched.
    Plugin(crate::halocord::Change),
}

pub fn update(app: &mut App, event: Event) -> iced::Task<Message> {
    match event {
        Event::Point(pointed) => {
            app.account_pointed = pointed;
            return app.ask();
        }
        Event::More(open) => {
            if open {
                app.account_more = true;
                app.tray_leaving = false;
            } else {
                app.close_tray();
            }
            return iced::Task::none();
        }
        // In a call they toggle once its voice is through; out of one
        // they say how the next starts.
        Event::Mute | Event::Deafen if app.account.in_call && !app.account.ready => {
            return iced::Task::none();
        }
        Event::Mute if app.account.in_call => {
            app.client
                .dispatch(yapfast_core::application::Action::Voice(
                    voice::Command::Mute(!app.account.muted),
                ));
        }
        Event::Deafen if app.account.in_call => {
            app.client
                .dispatch(yapfast_core::application::Action::Voice(
                    voice::Command::Deafen(!app.account.deafened),
                ));
        }
        Event::Mute | Event::Deafen => {
            crate::call::toggle_on_join(app, matches!(event, Event::Deafen));
            return iced::Task::none();
        }
        Event::ShowGame => {
            let shown = app.client.state().discord_show_game;
            app.client
                .dispatch(yapfast_core::application::Action::ShowGame(!shown));
        }
        Event::Plugin(change) => {
            crate::halocord::changed(&mut app.layout, &mut app.client, change);
        }
        Event::Reconnect if app.client.state().busy => return iced::Task::none(),
        Event::Reconnect => app
            .client
            .dispatch(yapfast_core::application::Action::RetrySync),
    }
    app.sync()
}

/// The card, inset from the rail's and the sidebar's edges.
pub fn view<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let account = ctx.account;
    let pointed = ctx.account_pointed;
    let state = ctx.state;
    let decoration = state
        .cosmetics
        .decoration(state.account_decoration.as_deref());
    let dock = ctx.layout.halocord.dock();
    let compact = dock.compact;
    let extras = plugin_buttons(ctx);
    let offline = !state.preview && !state.online;
    let fit = crate::halocord::card_fit(ctx, compact, extras.len(), offline);
    // Too narrow for the full size: the buttons beside the name take the
    // compact one.
    let small = fit.small;
    let avatar = if compact { COMPACT_AVATAR } else { AVATAR };
    let face = ctx.face(
        Face::new(account.avatar.as_deref(), &account.name, avatar)
            .decoration(decoration, pointed)
            .dot(account.presence.map(|presence| {
                (
                    presence,
                    rgb(theme.panel),
                    Point::new(avatar - 14.0, avatar - 14.0),
                )
            })),
    );
    let face = ring_and_dot(ctx, account, face, avatar);
    let faking = ctx.layout.halocord.faking();
    // A theme may leave the account's own avatar out.
    let face: Element<'a, Message> =
        if crate::theme_chrome::hides(ctx, yapfast_core::themes::chrome::Hide::AccountAvatar) {
            iced::widget::Space::new().width(0).into()
        } else {
            face
        };
    let face = crate::theme_effects::slot(
        face,
        ctx,
        yapfast_core::themes::package::Slot::OwnAvatar,
        Some(theme.panel),
    );
    let second: Element<'a, Message> = if pointed && !account.username.is_empty() {
        text(crate::halocord::ellipsized(
            &account.username,
            13.0,
            crate::fonts::REGULAR,
            fit.name_room,
        ))
        .size(theme.type_size("small", 13.0))
        .font(theme.font("small", crate::fonts::REGULAR))
        .color(theme.muted)
        .wrapping(text::Wrapping::None)
        .into()
    } else if let Some(faking) = faking.filter(|_| account.in_call) {
        // Fake mute or fake deafen is on in a call: say so under the name.
        text(faking)
            .size(13)
            .color(theme.accent)
            .wrapping(text::Wrapping::None)
            .into()
    } else {
        ctx.status_line(&account.status, StatusInk::under_name(ctx.look.theme, 1.0))
    };
    let lit = theme.wash(0.08);
    let open = account
        .linked
        .then_some(Message::Status(crate::status::Event::Toggle));
    let name_size = if compact { 14.0 } else { 15.0 };
    let name_colour = if compact && faking.is_some() && account.in_call {
        theme.accent
    } else {
        theme.text
    };
    // As Slint's dock: a theme's name is bold in the window's own face (a
    // theme's body), not the theme's names.
    let name: Element<'a, Message> = if theme.packaged() {
        crate::elide::elide(account.name.as_str())
            .size(name_size)
            .font(theme.font("body", BOLD))
            .color(name_colour)
            .into()
    } else {
        text(crate::halocord::ellipsized(
            &account.name,
            name_size,
            BOLD,
            fit.name_room,
        ))
        .size(name_size)
        .font(BOLD)
        .color(name_colour)
        .wrapping(text::Wrapping::None)
        .into()
    };
    let names = mouse_area(
        container(if compact {
            column![name].width(Length::Fill)
        } else {
            column![name, second].width(Length::Fill)
        })
        .padding([0, 6])
        .height(Length::Fill)
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .clip(true)
        .style(move |_| {
            if pointed {
                rounded(lit, 6)
            } else {
                Default::default()
            }
        }),
    )
    .on_enter(Message::Account(Event::Point(true)))
    .on_exit(Message::Account(Event::Point(false)));
    let names = match open.clone() {
        Some(open) => names
            .on_press(open)
            .interaction(iced::mouse::Interaction::Pointer),
        None => names,
    };
    let join = (
        ctx.layout.unmute_on_join.unwrap_or(true),
        ctx.layout.deafen_on_join,
    );
    let (mic, mic_label) = if account.speaking_blocked {
        (Icon::MicOff, "Speaking is restricted in this channel")
    } else if account.shown_muted(join) {
        (
            Icon::MicOff,
            if account.in_call {
                "Unmute"
            } else {
                "Join calls unmuted"
            },
        )
    } else {
        (
            Icon::Mic,
            if account.in_call {
                "Mute"
            } else {
                "Join calls muted"
            },
        )
    };
    let (ear, ear_label) = if account.shown_deafened(join) {
        (
            Icon::HeadphoneOff,
            if account.in_call {
                "Undeafen"
            } else {
                "Join calls undeafened"
            },
        )
    } else {
        (
            Icon::Headphones,
            if account.in_call {
                "Deafen"
            } else {
                "Join calls deafened"
            },
        )
    };
    // Each device's toggle with the caret of its menu, at the toggle's
    // middle; the menu opens above it.
    let side = if small { COMPACT_BUTTON } else { 32.0 };
    // The card's glyphs, a little smaller when compact.
    let glyph = |size: u32| if small { size - 3 } else { size };
    let carets = !dock.arrows_hidden;
    let caret = |label, popup| {
        ctx.spot(
            ctx.look
                .icon_button(
                    Icon::ChevronDown,
                    label,
                    Message::Call(crate::call::Event::Open(popup)),
                )
                .size(if small { 16.0 } else { 20.0 }, 16),
        )
    };
    let alert = |on: bool| if on { theme.danger } else { theme.text };
    // A muted or deafened device keeps a dark ground as Discord's does:
    // measured over a nameplate, 20% of a near-black red, which on the
    // plain panel reads as the same step down, with the faint seam
    // Discord draws between the toggle and its caret.
    let dimmed = |on: bool, pair: iced::widget::Row<'a, Message>| -> Element<'a, Message> {
        if !on {
            return pair.into();
        }
        let dark = iced::Color::from_rgba8(0x1e, 0, 0, 0.2);
        let seam = iced::Color::from_rgba8(0, 0, 0, 0.15);
        let ground = container(pair)
            .height(side)
            .align_y(Alignment::Center)
            .style(move |_| rounded(dark, 4));
        if !carets {
            return ground.into();
        }
        stack![
            ground,
            row![
                Space::new().width(side),
                container(Space::new())
                    .width(1)
                    .height(side)
                    .style(move |_| fill(seam)),
            ],
        ]
        .into()
    };
    // A toggle with its caret, or the toggle alone when the arrows are hidden.
    let paired = |toggle: Element<'a, Message>, caret: Element<'a, Message>| {
        let pair = row![toggle].align_y(Alignment::Center);
        if carets { pair.push(caret) } else { pair }
    };
    let toggles = !account.in_call || account.ready;
    let toggle = |event| toggles.then_some(Message::Account(event));
    // In a call, a gel theme's mute and deafen are gel buttons (the
    // as asked): the danger gel while on.
    let device = |icon, label: &'static str, on: bool, size: u32, event| -> Element<'a, Message> {
        let message = if matches!(event, Event::Mute) && account.speaking_blocked {
            None
        } else {
            toggle(event)
        };
        let gel = if account.in_call {
            crate::theme_art::gel_call_button(
                ctx,
                icon,
                glyph(size) - 2,
                label,
                if on {
                    crate::theme_art::GelTone::Danger
                } else {
                    crate::theme_art::GelTone::Plain
                },
                (30.0, 30.0),
                message.clone(),
            )
        } else {
            None
        };
        gel.unwrap_or_else(|| {
            tinted_button(ctx, icon, label, alert(on), glyph(size), side, message)
        })
    };
    let controls = row![
        dimmed(
            account.shown_muted(join),
            paired(
                device(mic, mic_label, account.shown_muted(join), 20, Event::Mute),
                caret("Input options", crate::call::Popup::Input),
            ),
        ),
        dimmed(
            account.shown_deafened(join),
            paired(
                device(
                    ear,
                    ear_label,
                    account.shown_deafened(join),
                    21,
                    Event::Deafen
                ),
                caret("Output options", crate::call::Popup::Output),
            ),
        ),
    ]
    .spacing(5)
    .align_y(Alignment::Center);
    // The plugin buttons (the User area's, and the top bar's when Yapfast
    // draws none) go in a row of their own above the name, so the name keeps
    // its room. Those with no room on it go in the tray its last button,
    // "More buttons", opens over the card.
    let plugins: Option<Element<'a, Message>> = (!extras.is_empty()).then(|| {
        let mut line = row![].spacing(4).align_y(Alignment::Center);
        for (icon, label, ink, message) in extras.into_iter().take(fit.inline) {
            line = line.push(plugin_button(ctx, icon, label, ink, message));
        }
        if fit.more {
            line = line.push(plugin_button(
                ctx,
                Icon::Ellipsis,
                "More buttons",
                if ctx.account_more {
                    theme.accent
                } else {
                    theme.text
                },
                Some(Message::Account(Event::More(!ctx.account_more))),
            ));
        }
        container(line)
            .padding(Padding {
                left: PLUGIN_INSET,
                right: PLUGIN_INSET,
                top: plugin_top(compact),
                bottom: 0.0,
            })
            .width(Length::Fill)
            .into()
    });
    // Offline, Reconnect before Settings, which is always on the card.
    let controls = if offline {
        controls.push(tinted_button(
            ctx,
            Icon::Refresh,
            "Reconnect",
            theme.secondary,
            glyph(20),
            side,
            (!state.busy).then_some(Message::Account(Event::Reconnect)),
        ))
    } else {
        controls
    };
    let controls = controls.push(tinted_button(
        ctx,
        Icon::Settings,
        "Settings",
        theme.text,
        glyph(24),
        side,
        Some(crate::app::settings(crate::settings::Event::Reopen)),
    ));
    let ground = theme.token_colour("account", theme.panel);
    let face: Element<'a, Message> = match open {
        Some(open) => mouse_area(face)
            .on_press(open)
            .interaction(iced::mouse::Interaction::Pointer)
            .into(),
        None => face,
    };
    let (face_width, face_left, right) = if compact {
        (avatar + 8.0, 6.0, 8.0)
    } else {
        (50.0, 8.0, 14.0)
    };
    let face_width =
        if crate::theme_chrome::hides(ctx, yapfast_core::themes::chrome::Hide::AccountAvatar) {
            14.0
        } else {
            face_width
        };
    // With no room at all for the name (the sidebar at its least), it
    // gives its padding to the buttons; the avatar still opens the popover.
    let names: Element<'a, Message> = if fit.name_room >= 1.0 {
        container(names)
            .padding([if compact { 2 } else { 6 }, 0])
            .height(Length::Fill)
            .into()
    } else {
        Space::new().width(Length::Fill).into()
    };
    let card = row![
        container(face)
            .width(face_width)
            .padding(Padding {
                left: face_left,
                ..Padding::ZERO
            })
            .align_y(Alignment::Center),
        names,
        controls,
    ]
    .spacing(if compact { 3 } else { 5 })
    .padding(Padding {
        right,
        ..Padding::ZERO
    })
    .height(height(compact))
    .align_y(Alignment::Center);
    // The account's nameplate fills the card, brighter and playing
    // under the pointer.
    let card: Element<'a, Message> =
        match crate::members::account_plate(ctx, height(compact), pointed, pointed) {
            Some((plate, palette)) => PlateRow::new(card, Some(plate), palette, ground, 0.0, 1.0)
                .account()
                .into(),
            None => card.into(),
        };
    let mut stacked = column![];
    if let Some(plugins) = plugins {
        stacked = stacked.push(plugins);
    }
    // A theme's glass account card floats as a pane of its own (see
    // `app::view`): the glass is its ground and edge, as in Slint, so the
    // card is bare and fills it.
    if ctx
        .themes
        .glass(
            &theme.id,
            yapfast_core::themes::package::art::ArtSurface::Account,
        )
        .is_some()
    {
        return stacked.push(card).into();
    }
    // MochaCord's dock is one piece across the whole foot (`app::view`
    // draws its ground and edges), so the card is bare and fills it.
    if crate::style::mocha(theme) {
        return stacked.push(card).into();
    }
    container(
        container(stacked.push(card))
            .clip(true)
            .style(move |_| rounded(ground, 8)),
    )
    .padding(Padding {
        // Compact stays under the channel list, off the rail's foot.
        left: INSET
            + if compact {
                crate::rail::width(ctx.layout)
            } else {
                0.0
            },
        right: INSET,
        bottom: INSET,
        top: 0.0,
    })
    .into()
}

/// MochaCord's foot as one dock: `foot` (the voice panel over the card)
/// on one ground across the full width, the window's own corner at the
/// bottom-left and square elsewhere, a 2 px rest edge and the lit edge
/// while the pointer is on it.
pub fn dock<'a>(
    ctx: crate::ctx::Ctx<'a>,
    foot: Element<'a, Message>,
    maximized: bool,
) -> Element<'a, Message> {
    use iced::{Border, border::Radius};
    let theme = ctx.look.theme;
    let ground = theme.token_colour("account", theme.panel);
    let radius = if maximized { 0.0 } else { WINDOW_RADIUS };
    let corner = Radius {
        top_left: 0.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: radius,
    };
    let rice = crate::style::rice();
    let rest = rice.map_or(theme.edge, |rice| rice.rest);
    let lit = rice.map_or(theme.accent, |rice| rice.hover);
    let width = crate::style::edge_width();
    let base = container(foot)
        .width(Length::Fill)
        .padding(width)
        .style(move |_| iced::widget::container::Style {
            background: Some(ground.into()),
            border: Border {
                color: rest,
                width,
                radius: corner,
            },
            ..Default::default()
        });
    let over = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            border: Border {
                color: lit,
                width,
                radius: corner,
            },
            ..Default::default()
        });
    crate::widgets::Hover::new(base, over).into()
}

/// The avatar in the green ring while the microphone carries voice, and,
/// with no Discord presence to show, a dot saying whether the account
/// syncs: the accent in the preview, green online, grey offline.
fn ring_and_dot<'a>(
    ctx: crate::ctx::Ctx<'a>,
    account: &Account,
    face: Element<'a, Message>,
    avatar: f32,
) -> Element<'a, Message> {
    use iced::widget::{Space, pin, stack};
    let theme = ctx.look.theme;
    // The ring goes under the face, so the status badge and its cut-out
    // gap sit on top of it, whole.
    let mut layers = stack![];
    if account.speaking {
        let ring = theme.live;
        let side = avatar + 6.0;
        layers = layers.push(
            pin(container(Space::new())
                .width(side)
                .height(side)
                .style(move |_| iced::widget::container::Style {
                    border: iced::Border {
                        color: ring,
                        width: 2.0,
                        radius: (side / 2.0).into(),
                    },
                    ..Default::default()
                }))
            .x(-3.0)
            .y(-3.0),
        );
    }
    layers = layers.push(face);
    if account.presence.is_none() {
        let ink = match account.dot {
            Dot::Preview => theme.accent,
            Dot::Online => theme.live,
            Dot::Offline => theme.faint,
        };
        let ground = theme.token_colour("account", theme.panel);
        let dot = container(
            container(Space::new())
                .width(8)
                .height(8)
                .style(move |_| rounded(ink, 4)),
        )
        .width(14)
        .height(14)
        .center(14)
        .style(move |_| rounded(ground, 7));
        layers = layers.push(pin(dot).x(avatar - 13.0).y(avatar - 13.0));
    }
    layers.width(avatar).height(avatar).into()
}

/// An icon button of the card's, `side` px (32, or less when compact), its
/// glyph in `ink`; greyed without a message.
fn tinted_button<'a>(
    ctx: crate::ctx::Ctx<'a>,
    icon: Icon,
    label: &'static str,
    ink: iced::Color,
    glyph: u32,
    side: f32,
    message: Option<Message>,
) -> Element<'a, Message> {
    let face = Pic::square(ctx.look.icon(icon, glyph, rgb(ink)), glyph as f32);
    ctx.look.face_button(face, side, label, message).into()
}

/// One of the plugin buttons, on the row above the name or in the tray: a
/// dock button sharing the row's width, its glyph in `ink`.
pub(crate) fn plugin_button<'a>(
    ctx: crate::ctx::Ctx<'a>,
    icon: Icon,
    label: &'static str,
    ink: iced::Color,
    message: Option<Message>,
) -> Element<'a, Message> {
    crate::controls::dock_button(
        ctx.look.glyph(icon, 20, ink),
        label,
        false,
        ctx.look.theme,
        message,
    )
}

/// The plugin buttons the card carries, in order: icon, tip, ink and what a
/// press sends (none while it can't act). The User area's switches, the
/// call stats and game buttons, then the top bar's plugin buttons when
/// Yapfast draws no bar of its own.
pub(crate) fn plugin_buttons(
    ctx: crate::ctx::Ctx<'_>,
) -> Vec<(Icon, &'static str, iced::Color, Option<Message>)> {
    let theme = ctx.look.theme;
    let account = ctx.account;
    let kept = &ctx.layout.halocord;
    let dock = kept.dock();
    let mut buttons = Vec::new();
    // A plugin's switch, lit in the accent while it is on.
    let mut switch = |id: &str, icon: Icon, on_label: &'static str, off_label: &'static str| {
        let on = kept.is(id);
        let message = crate::halocord::place(id).map(|plugin| {
            Message::Account(Event::Plugin(crate::halocord::Change::Switch {
                plugin,
                on: !on,
            }))
        });
        buttons.push((
            icon,
            if on { on_label } else { off_label },
            if on { theme.accent } else { theme.text },
            message,
        ));
    };
    if dock.fake_mute {
        switch(
            crate::halocord::FAKE_MUTE,
            Icon::MicOff,
            "Stop faking mute",
            "Fake mute",
        );
    }
    if dock.fake_deafen {
        switch(
            crate::halocord::FAKE_DEAFEN,
            Icon::HeadphoneOff,
            "Stop faking deafen",
            "Fake deafen",
        );
    }
    if dock.fake_camera {
        switch(
            crate::halocord::FAKE_CAMERA,
            Icon::Video,
            "Stop faking camera",
            "Fake camera",
        );
    }
    if dock.fake_stream {
        switch(
            crate::halocord::FAKE_STREAM,
            Icon::ScreenShare,
            "Stop faking stream",
            "Fake stream",
        );
    }
    if dock.stereo {
        switch(
            crate::halocord::STEREO,
            Icon::Music,
            "Turn stereo off",
            "Turn stereo on",
        );
    }
    if dock.stats {
        buttons.push((
            Icon::ChartBar,
            "Call stats",
            theme.text,
            (account.in_call && account.ready).then_some(Message::Call(crate::call::Event::Open(
                crate::call::Popup::Link,
            ))),
        ));
    }
    if dock.game {
        // Lit while the game is shared; it needs the Discord account.
        let shown = ctx.state.discord_show_game;
        buttons.push((
            Icon::Gamepad,
            if shown {
                "Stop sharing your game"
            } else {
                "Share your game"
            },
            if shown { theme.accent } else { theme.text },
            ctx.state
                .discord_user
                .is_some()
                .then_some(Message::Account(Event::ShowGame)),
        ));
    }
    if !ctx.own_title_bar {
        for (icon, label, press) in crate::halocord::presses(kept) {
            buttons.push((
                icon,
                label,
                theme.text,
                Some(Message::Bar(crate::window_bar::Event::Halo(press))),
            ));
        }
    }
    buttons
}
