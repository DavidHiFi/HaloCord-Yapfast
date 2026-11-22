//! HaloCord's settings and plug-in points for Yapfast's core, kept here so
//! Yapfast's own files carry only the hooks that call in. `docs/halocord.md`
//! lists every hook.

use std::sync::atomic::{AtomicBool, Ordering};

pub mod copy_user_mention;
pub mod staff;

use crate::{
    application::{Action, Application, Effect},
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

    /// 1. Before the app handles an action; `false` drops it.
    fn action(&self, _action: &Action) -> bool {
        true
    }

    /// 1. Before the app takes an event from the networks; `false` drops it.
    fn event(&self, _event: &Event) -> bool {
        true
    }

    /// 1. After the app handled an action or an event: what it now holds,
    /// and what it's about to do, to change or add to.
    fn after(&self, _app: &Application, _effects: &mut Vec<Effect>) {}

    /// 2. A command on its way to the networks; `false` holds it back.
    fn command(&self, _command: &Command) -> bool {
        true
    }

    /// 3. A live Discord event, before any channel sees it; `false` drops it.
    #[cfg(feature = "discord")]
    fn live(&self, _live: &yapfast_discord::service::Live) -> bool {
        true
    }

    /// 4. Tags drawn after a message's author, with the app as it is now.
    fn extras(&self, _message: &ChatMessage, _app: &Application) -> Vec<Tag> {
        Vec::new()
    }

    /// 5. Rows for a message's menu, as its action and its label.
    fn menu(&self, _message: &ChatMessage) -> Vec<(&'static str, &'static str)> {
        Vec::new()
    }

    /// 5. One of this plugin's menu rows was chosen for `message`.
    fn pick(&self, _action: &str, _message: &ChatMessage) -> Picked {
        Picked::default()
    }

    /// 5. Rows for a person's menu (`user` is their id), as action and label.
    fn person_menu(&self, _user: &str) -> Vec<(&'static str, &'static str)> {
        Vec::new()
    }

    /// 5. One of this plugin's person menu rows was chosen for `user`.
    fn person_pick(&self, _action: &str, _user: &str) -> Picked {
        Picked::default()
    }
}

/// What a plugin's menu row does: actions for the app, and text to copy.
#[derive(Default)]
pub struct Picked {
    pub actions: Vec<Action>,
    pub copy: Option<String>,
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

/// 1. Whether the app handles `action`.
pub fn keeps_action(action: &Action) -> bool {
    plugged().iter().all(|hooks| hooks.action(action))
}

/// 1. Whether the app takes `event`.
pub fn keeps_event(event: &Event) -> bool {
    plugged().iter().all(|hooks| hooks.event(event))
}

/// 1. The plugins' say after an action or an event.
pub fn after(app: &Application, effects: &mut Vec<Effect>) {
    for hooks in plugged() {
        hooks.after(app, effects);
    }
}

/// 2. Whether `command` goes on to the networks.
pub fn sends(command: &Command) -> bool {
    plugged().iter().all(|hooks| hooks.command(command))
}

/// 3. A live Discord event, unless a plugin drops it.
#[cfg(feature = "discord")]
pub fn live(live: yapfast_discord::service::Live) -> Option<yapfast_discord::service::Live> {
    plugged()
        .iter()
        .all(|hooks| hooks.live(&live))
        .then_some(live)
}

/// 4. The plugins' tags for a message's author line.
pub fn extras(message: &ChatMessage, app: &Application) -> Vec<Tag> {
    plugged()
        .iter()
        .flat_map(|hooks| hooks.extras(message, app))
        .collect()
}

/// 5. The plugins' rows for a message's menu: each row's key (the plugin's
/// id and the action, as `id/action`) and its label.
pub fn menu(message: &ChatMessage) -> Vec<(String, &'static str)> {
    rows_of(|hooks| hooks.menu(message))
}

/// 5. What the row with `key` does for `message`; nothing once its plugin
/// is out.
pub fn pick(key: &str, message: &ChatMessage) -> Picked {
    pick_with(key, |hooks, action| hooks.pick(action, message))
}

/// 5. The plugins' rows for a person's menu, keyed as [`menu`]'s are.
pub fn person_menu(user: &str) -> Vec<(String, &'static str)> {
    rows_of(|hooks| hooks.person_menu(user))
}

/// 5. What the person menu's row with `key` does for `user`.
pub fn person_pick(key: &str, user: &str) -> Picked {
    pick_with(key, |hooks, action| hooks.person_pick(action, user))
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
