//! HaloCord's settings and plug-in points for Yapfast's core, kept here so
//! Yapfast's own files carry only the hooks that call in. `docs/halocord.md`
//! lists every hook.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub mod copy_file_contents;
pub mod copy_media_link;
pub mod copy_sticker_link;
pub mod copy_user_mention;
pub mod copy_user_url;
pub mod fake_dms;
pub mod fix_file_extensions;
pub mod message_logger;
pub mod new_user;
pub mod quick_mention;
pub mod send_timestamps;
pub mod staff;
pub mod themes;

use crate::{
    application::{Action, Application, Effect, INVITE_AGES, INVITE_USES},
    chat::{ChatMessage, Command, Event},
};

/// The bit rates HaloCord's page offers, in kbit/s.
pub const BITRATES: [u32; 5] = [64, 128, 256, 384, 510];
/// The bit rate HaloCord sends unless changed, in kbit/s: the most Opus sends.
pub const DEFAULT_BITRATE: u32 = 510;

static FLAT: AtomicBool = AtomicBool::new(false);

/// Sends every call's microphone flat, in stereo: it opens with both its
/// channels and skips echo cancellation, the rumble filter, noise
/// suppression, gain control and leveling, so only its volume and the voice
/// gate (or push to talk) apply. A call going on switches within a block.
/// For music, instruments, or a good microphone with headphones.
pub fn set_flat(on: bool) {
    FLAT.store(on, Ordering::Relaxed);
}

/// Whether the microphone goes flat.
pub fn flat() -> bool {
    FLAT.load(Ordering::Relaxed)
}

/// Sets the bit rate Discord calls send the microphone at, in kbit/s, or
/// Yapfast's own for `None`; a call going on takes it at its next frame.
pub fn set_bitrate(kbps: Option<u32>) {
    #[cfg(feature = "discord")]
    {
        use yapfast_discord::voice;
        voice::set_microphone_bitrate(kbps.map_or(voice::DEFAULT_MICROPHONE_BITRATE, |kbps| {
            kbps.saturating_mul(1000)
        }));
    }
    #[cfg(not(feature = "discord"))]
    let _ = kbps;
}

/// A person's and a stream's volume slider goes to this much, in percent,
/// as Discord's does.
pub const VOLUME_STOCK: u32 = 200;
/// The most the Volume booster's page offers, in percent.
pub const VOLUME_CAPS: [u32; 5] = [200, 400, 600, 800, 1000];

static VOLUME_CAP: AtomicU32 = AtomicU32::new(VOLUME_STOCK);

/// Sets how far a person's and a stream's volume slider goes, in percent, or
/// Discord's own 200 for `None`. Past 200 the sound keeps growing in step
/// with the slider: 1000 is five times as loud as 200 (see
/// [`crate::volume::from_slider`]).
pub fn set_volume_cap(percent: Option<u32>) {
    let percent = percent.map_or(VOLUME_STOCK, |percent| {
        percent.clamp(VOLUME_STOCK, VOLUME_CAPS[VOLUME_CAPS.len() - 1])
    });
    VOLUME_CAP.store(percent, Ordering::Relaxed);
}

/// How far a person's and a stream's volume slider goes, in percent.
pub fn volume_cap() -> u32 {
    VOLUME_CAP.load(Ordering::Relaxed)
}

/// No first invite picked: a new invite dialog starts as Discord's does.
const NO_INVITE: u64 = u64::MAX;
#[cfg(not(test))]
static FIRST_INVITE: AtomicU64 = AtomicU64::new(NO_INVITE);

#[cfg(test)]
thread_local! {
    // Each test picks its own, out of the way of the tests beside it.
    static FIRST_INVITE: AtomicU64 = const { AtomicU64::new(NO_INVITE) };
}

fn first_invite_cell<T>(with: impl FnOnce(&AtomicU64) -> T) -> T {
    #[cfg(not(test))]
    return with(&FIRST_INVITE);
    #[cfg(test)]
    return FIRST_INVITE.with(with);
}

/// Sets what a new invite dialog's link starts with (Invite defaults): how
/// long it lasts and how many times it works, in seconds and uses, or
/// Discord's 7 days and any number of times for `None`. A pair that isn't
/// one of the dialog's own choices counts as `None`.
pub fn set_first_invite(invite: Option<(u32, u32)>) {
    let packed = invite
        .filter(|(age, uses)| INVITE_AGES.contains(age) && INVITE_USES.contains(uses))
        .map_or(NO_INVITE, |(age, uses)| {
            u64::from(age) << 32 | u64::from(uses)
        });
    first_invite_cell(|cell| cell.store(packed, Ordering::Relaxed));
}

/// What a new invite dialog's link starts with, when Invite defaults set it.
pub fn first_invite() -> Option<(u32, u32)> {
    let packed = first_invite_cell(|cell| cell.load(Ordering::Relaxed));
    (packed != NO_INVITE).then_some(((packed >> 32) as u32, packed as u32))
}

static MONO: AtomicBool = AtomicBool::new(false);

/// Plays everyone's incoming voice as mono (Force mono).
pub fn set_mono(on: bool) {
    MONO.store(on, Ordering::Relaxed);
}

pub fn mono() -> bool {
    MONO.load(Ordering::Relaxed)
}

/// Adds one person's interleaved stereo frame to `sum` at `gain`, with both
/// channels turned into the mean of the two.
pub fn add_mono(sum: &mut [f32], samples: &[i16], gain: f32) {
    for (total, pair) in sum.chunks_mut(2).zip(samples.chunks(2)) {
        let mean = pair.iter().map(|sample| f32::from(*sample)).sum::<f32>() / 2.0 * gain;
        for channel in total {
            *channel += mean;
        }
    }
}

static FAKE_MUTE: AtomicBool = AtomicBool::new(false);
static FAKE_DEAF: AtomicBool = AtomicBool::new(false);

/// Makes others see this account muted, or muted and deafened, in calls,
/// whatever it really is: the voice state sent to Discord says so, while the
/// microphone and the sound stay as the account set them. See
/// [`voice_state`].
pub fn set_fake_voice(mute: bool, deaf: bool) {
    FAKE_MUTE.store(mute, Ordering::Relaxed);
    FAKE_DEAF.store(deaf, Ordering::Relaxed);
}

/// What the voice state sent to Discord says, from the real mute and
/// deafen: (self mute, self deaf). A fake deafen shows as Discord shows a
/// real one, deafened and muted.
pub fn voice_state(muted: bool, deafened: bool) -> (bool, bool) {
    shown_voice_state(
        muted,
        deafened,
        FAKE_MUTE.load(Ordering::Relaxed),
        FAKE_DEAF.load(Ordering::Relaxed),
    )
}

fn shown_voice_state(
    muted: bool,
    deafened: bool,
    fake_mute: bool,
    fake_deaf: bool,
) -> (bool, bool) {
    let deaf = deafened || fake_deaf;
    (muted || deaf || fake_mute, deaf)
}

/// `user` when it is a Discord user's id (digits), as a person's menu is
/// given it; Matrix ids and anything else give none.
pub fn discord_user(user: &str) -> Option<&str> {
    (!user.is_empty() && user.bytes().all(|byte| byte.is_ascii_digit())).then_some(user)
}

/// A tag a plugin draws after a message's author: its words, a crown in
/// place of them, and the colour it's drawn in (an RGB number), if the
/// plugin sets one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tag {
    pub text: &'static str,
    pub crown: bool,
    pub color: Option<u32>,
}

impl Tag {
    /// Plain words in the window's own tag colour.
    pub const fn words(text: &'static str) -> Self {
        Self {
            text,
            crown: false,
            color: None,
        }
    }
}

/// What a plugin hooks into. Each one does nothing unless the plugin says
/// otherwise, so a plugin writes only the ones it needs. The numbers are the
/// plug-in points in `docs/halocord.md`.
pub trait Hooks: Sync {
    /// The plugin's id, as on HaloCord's page.
    fn id(&self) -> &'static str;

    /// Hook 1: Before the app handles an action; `false` drops it.
    fn action(&self, _action: &Action) -> bool {
        true
    }

    /// Hook 1: Before the app takes an event from the networks; `false` drops it.
    fn event(&self, _event: &Event) -> bool {
        true
    }

    /// Hook 1: After the app handled an action or an event: what it now holds,
    /// and what it's about to do, to change or add to.
    fn after(&self, _app: &Application, _effects: &mut Vec<Effect>) {}

    /// Hook 2: A command on its way to the networks; `false` holds it back.
    fn command(&self, _command: &Command) -> bool {
        true
    }

    /// Hook 3: A live Discord event, before any channel sees it; `false` drops it.
    #[cfg(feature = "discord")]
    fn live(&self, _live: &yapfast_discord::service::Live) -> bool {
        true
    }

    /// Hook 4: Tags drawn after a message's author, with the app as it is now.
    fn extras(&self, _message: &ChatMessage, _app: &Application) -> Vec<Tag> {
        Vec::new()
    }

    /// Hook 5: Rows for a message's menu, as its action and its label.
    fn menu(&self, _message: &ChatMessage) -> Vec<(&'static str, &'static str)> {
        Vec::new()
    }

    /// Hook 5: One of this plugin's menu rows was chosen for `message`.
    fn pick(&self, _action: &str, _message: &ChatMessage) -> Picked {
        Picked::default()
    }

    /// Hook 5: Rows for a person's menu (`user` is their id), as action and label.
    fn person_menu(&self, _user: &str) -> Vec<(&'static str, &'static str)> {
        Vec::new()
    }

    /// Hook 5: One of this plugin's person menu rows was chosen for `user`.
    fn person_pick(&self, _action: &str, _user: &str) -> Picked {
        Picked::default()
    }

    /// Hook 5: Rows for the right-click menu of a file or sticker, attachment `at`
    /// of `message`, as action and label.
    fn attachment_menu(
        &self,
        _message: &ChatMessage,
        _at: usize,
    ) -> Vec<(&'static str, &'static str)> {
        Vec::new()
    }

    /// Hook 5: One of this plugin's attachment menu rows was chosen.
    fn attachment_pick(&self, _action: &str, _message: &ChatMessage, _at: usize) -> Picked {
        Picked::default()
    }
}

/// What a plugin's menu row does: actions for the app, text to copy, and a
/// web page to open in the browser.
#[derive(Default)]
pub struct Picked {
    pub actions: Vec<Action>,
    pub copy: Option<String>,
    pub open: Option<String>,
}

#[cfg(not(test))]
static PLUGGED: std::sync::RwLock<Vec<&'static dyn Hooks>> = std::sync::RwLock::new(Vec::new());

#[cfg(test)]
thread_local! {
    // Each test plugs its own, out of the way of the tests beside it.
    static PLUGGED: std::cell::RefCell<Vec<&'static dyn Hooks>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Plugs a plugin's hooks in, or takes them out.
pub fn plug(hooks: &'static dyn Hooks, on: bool) {
    let change = |plugged: &mut Vec<&'static dyn Hooks>| {
        plugged.retain(|held| held.id() != hooks.id());
        if on {
            plugged.push(hooks);
        }
    };
    #[cfg(not(test))]
    change(
        &mut PLUGGED
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    );
    #[cfg(test)]
    PLUGGED.with(|plugged| change(&mut plugged.borrow_mut()));
}

/// The plugins plugged in, copied so a hook may plug or unplug.
fn plugged() -> Vec<&'static dyn Hooks> {
    #[cfg(not(test))]
    return PLUGGED
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    #[cfg(test)]
    PLUGGED.with(|plugged| plugged.borrow().clone())
}

/// Hook 1: Whether the app handles `action`.
pub fn keeps_action(action: &Action) -> bool {
    plugged().iter().all(|hooks| hooks.action(action))
}

/// Hook 1: Whether the app takes `event`.
pub fn keeps_event(event: &Event) -> bool {
    plugged().iter().all(|hooks| hooks.event(event))
}

/// Hook 1: The plugins' say after an action or an event.
pub fn after(app: &Application, effects: &mut Vec<Effect>) {
    for hooks in plugged() {
        hooks.after(app, effects);
    }
}

/// Hook 2: Whether `command` goes on to the networks.
pub fn sends(command: &Command) -> bool {
    plugged().iter().all(|hooks| hooks.command(command))
}

/// Hook 3: A live Discord event, unless a plugin drops it.
#[cfg(feature = "discord")]
pub fn live(live: yapfast_discord::service::Live) -> Option<yapfast_discord::service::Live> {
    plugged()
        .iter()
        .all(|hooks| hooks.live(&live))
        .then_some(live)
}

/// Hook 4: The plugins' tags for a message's author line.
pub fn extras(message: &ChatMessage, app: &Application) -> Vec<Tag> {
    plugged()
        .iter()
        .flat_map(|hooks| hooks.extras(message, app))
        .collect()
}

/// Hook 5: The plugins' rows for a message's menu: each row's key (the plugin's
/// id and the action, as `id/action`) and its label.
pub fn menu(message: &ChatMessage) -> Vec<(String, &'static str)> {
    rows_of(|hooks| hooks.menu(message))
}

/// Hook 5: What the row with `key` does for `message`; nothing once its plugin
/// is out.
pub fn pick(key: &str, message: &ChatMessage) -> Picked {
    pick_with(key, |hooks, action| hooks.pick(action, message))
}

/// Hook 5: The plugins' rows for a person's menu, keyed as [`menu`]'s are.
pub fn person_menu(user: &str) -> Vec<(String, &'static str)> {
    rows_of(|hooks| hooks.person_menu(user))
}

/// Hook 5: What the person menu's row with `key` does for `user`.
pub fn person_pick(key: &str, user: &str) -> Picked {
    pick_with(key, |hooks, action| hooks.person_pick(action, user))
}

/// Hook 5: The plugins' rows for the menu of attachment `at` of `message`, keyed
/// as [`menu`]'s are.
pub fn attachment_menu(message: &ChatMessage, at: usize) -> Vec<(String, &'static str)> {
    rows_of(|hooks| hooks.attachment_menu(message, at))
}

/// Hook 5: What the attachment menu's row with `key` does.
pub fn attachment_pick(key: &str, message: &ChatMessage, at: usize) -> Picked {
    pick_with(key, |hooks, action| {
        hooks.attachment_pick(action, message, at)
    })
}

/// The rows each plugged plugin gives, keyed `id/action`.
fn rows_of(
    rows: impl Fn(&'static dyn Hooks) -> Vec<(&'static str, &'static str)>,
) -> Vec<(String, &'static str)> {
    plugged()
        .into_iter()
        .flat_map(|hooks| {
            rows(hooks)
                .into_iter()
                .map(|(action, label)| (format!("{}/{action}", hooks.id()), label))
        })
        .collect()
}

/// What the plugin a row's `key` names does for its action; nothing once
/// the plugin is out.
fn pick_with(key: &str, pick: impl FnOnce(&'static dyn Hooks, &str) -> Picked) -> Picked {
    let Some((id, action)) = key.split_once('/') else {
        return Picked::default();
    };
    plugged()
        .into_iter()
        .find(|hooks| hooks.id() == id)
        .map_or_else(Picked::default, |hooks| pick(hooks, action))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drops drafts and every event, and asks for a page after the rest.
    struct Tap;

    impl Hooks for Tap {
        fn id(&self) -> &'static str {
            "tap"
        }
        fn action(&self, action: &Action) -> bool {
            !matches!(action, Action::SetDraft(_))
        }
        fn event(&self, _event: &Event) -> bool {
            false
        }
        fn after(&self, _app: &Application, effects: &mut Vec<Effect>) {
            effects.push(Effect::OpenPage("after".into()));
        }
        fn command(&self, command: &Command) -> bool {
            !matches!(command, Command::Typing { .. })
        }
        fn extras(&self, message: &ChatMessage, _app: &Application) -> Vec<Tag> {
            if message.bot {
                vec![Tag::words("BOT")]
            } else {
                Vec::new()
            }
        }
        fn menu(&self, _message: &ChatMessage) -> Vec<(&'static str, &'static str)> {
            vec![("raw", "Copy raw")]
        }
        fn pick(&self, action: &str, message: &ChatMessage) -> Picked {
            Picked {
                copy: (action == "raw").then(|| message.body.clone()),
                ..Picked::default()
            }
        }
    }

    fn opened_after(effects: &[Effect]) -> bool {
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::OpenPage(page) if page == "after"))
    }

    #[test]
    fn a_plugin_taps_actions_and_events_before_and_after() {
        let mut app = Application::default();
        assert!(!opened_after(&app.dispatch(Action::AskAppCommands)));
        plug(&Tap, true);
        assert!(app.dispatch(Action::SetDraft("dropped".into())).is_empty());
        assert!(opened_after(&app.dispatch(Action::AskAppCommands)));
        assert!(app.apply(Event::InteractionSettled("1".into())).is_empty());
        plug(&Tap, false);
        assert!(!opened_after(&app.dispatch(Action::AskAppCommands)));
    }

    #[test]
    fn a_plugin_holds_back_commands() {
        let typing = Command::Typing {
            room: "room".into(),
            typing: true,
        };
        assert!(sends(&typing));
        plug(&Tap, true);
        assert!(!sends(&typing));
    }

    #[test]
    fn a_plugin_adds_tags_and_menu_rows() {
        let bot = ChatMessage {
            bot: true,
            body: "hello".into(),
            ..ChatMessage::default()
        };
        let app = Application::default();
        assert!(extras(&bot, &app).is_empty() && menu(&bot).is_empty());
        plug(&Tap, true);
        assert_eq!(extras(&bot, &app), [Tag::words("BOT")]);
        assert!(extras(&ChatMessage::default(), &app).is_empty());
        assert_eq!(menu(&bot), [("tap/raw".to_owned(), "Copy raw")]);
        assert_eq!(pick("tap/raw", &bot).copy.as_deref(), Some("hello"));
        assert!(pick("gone/raw", &bot).copy.is_none());
        plug(&Tap, false);
        assert!(pick("tap/raw", &bot).copy.is_none());
    }

    #[test]
    fn force_mono_plays_the_mean_of_both_sides_in_both() {
        let mut sum = [0f32; 4];
        // Left 1000, right -200: the mean is 400, doubled by a gain of 2.
        add_mono(&mut sum, &[1000, -200, 10, 30], 2.0);
        assert_eq!(sum, [800.0, 800.0, 40.0, 40.0]);
        // A second voice adds on, and a frame that stops short leaves the rest.
        add_mono(&mut sum, &[100, 100], 1.0);
        assert_eq!(sum, [900.0, 900.0, 40.0, 40.0]);
        set_mono(true);
        assert!(mono());
        set_mono(false);
        assert!(!mono());
    }

    #[test]
    fn a_fake_state_shows_to_others_and_leaves_the_real_one() {
        assert_eq!(
            shown_voice_state(false, false, false, false),
            (false, false)
        );
        assert_eq!(shown_voice_state(true, false, false, false), (true, false));
        // A fake mute shows muted while really unmuted, never deafened.
        assert_eq!(shown_voice_state(false, false, true, false), (true, false));
        // A fake deafen shows deafened, and muted as a real deafen does.
        assert_eq!(shown_voice_state(false, false, false, true), (true, true));
        // A real deafen stays, fake or not.
        assert_eq!(shown_voice_state(true, true, true, false), (true, true));
    }

    /// The voice state sent says deafened while the real deafen stays off,
    /// which is what keeps the sound playing: the call plays sound by the
    /// real `deafened`, and only `voice_state`'s answer goes to Discord.
    #[test]
    fn a_fake_deafen_sends_self_deaf_while_the_real_deafen_stays_off() {
        let real_deafened = false;
        let real_muted = false;
        set_fake_voice(false, true);
        let sent = voice_state(real_muted, real_deafened);
        set_fake_voice(false, false);
        assert_eq!(sent, (true, true), "Discord is told muted and deafened");
        assert!(
            !real_deafened,
            "the real state, which plays the sound, is off"
        );
        assert_eq!(voice_state(real_muted, real_deafened), (false, false));
    }

    #[cfg(feature = "discord")]
    #[test]
    fn a_plugin_drops_live_events() {
        struct KeepDeleted;
        impl Hooks for KeepDeleted {
            fn id(&self) -> &'static str {
                "keep-deleted"
            }
            fn live(&self, live: &yapfast_discord::service::Live) -> bool {
                !matches!(live, yapfast_discord::service::Live::Deleted { .. })
            }
        }
        let deleted = || yapfast_discord::service::Live::Deleted {
            channel: "1".into(),
            id: "2".into(),
        };
        assert!(live(deleted()).is_some());
        plug(&KeepDeleted, true);
        assert!(live(deleted()).is_none());
    }
}
