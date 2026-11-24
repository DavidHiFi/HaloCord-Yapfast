//! Better user tools: the account's popover gets Mic test, to hear your
//! microphone outside a call as Voice's settings let you, and Off the radar,
//! to go idle and stop showing your game in one press, and back as you were
//! in another. Off the radar makes the same two changes as choosing Idle and
//! switching off "Share your detected activities" by hand.

use std::sync::Mutex;

use yapfast_core::{application::Action, discord_audio::GateMeter};

use super::{Kind, Plugin, Press, Setting, id};
use crate::{app::App, ctx::Ctx, pictures::Icon};

pub(super) const BETTER_USER_TOOLS: Plugin = Plugin {
    id: id::USER_TOOLS,
    name: "Better user tools",
    about: "Your account's popover gets Mic test, to hear your microphone outside a call, and Off the radar, to go idle and stop sharing your game in one press, and back as you were in another.",
    on_at_first: false,
    settings: &[
        Setting {
            key: "mic",
            heading: "Mic test",
            note: "Plays your microphone back to you, as Voice's settings do, until pressed again or a call starts.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
        Setting {
            key: "radar",
            heading: "Off the radar",
            note: "Sets you idle and stops sharing your game; pressed again, puts both back as they were.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
    ],
    apply: |_, _| {},
    hooks: None,
};

/// The mic test playing, while it does.
static MIC: Mutex<Option<GateMeter>> = Mutex::new(None);

/// What Off the radar put aside: the status and whether the game showed.
static RADAR: Mutex<Option<(Option<String>, bool)>> = Mutex::new(None);

fn held<T, R>(lock: &Mutex<T>, with: impl FnOnce(&mut T) -> R) -> R {
    with(&mut lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
}

/// Whether the mic test plays.
pub fn mic_testing() -> bool {
    held(&MIC, |mic| mic.is_some())
}

/// Whether Off the radar is on.
pub fn off_the_radar() -> bool {
    held(&RADAR, |radar| radar.is_some())
}

/// Hook: the popover's rows the plugin adds, before User Settings: label,
/// icon and press.
pub fn popover_rows(cx: Ctx<'_>) -> Vec<(&'static str, Icon, Press)> {
    let kept = &cx.layout.halocord;
    if !kept.is(id::USER_TOOLS) {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let flag = |setting: usize| kept.flag(&BETTER_USER_TOOLS, &BETTER_USER_TOOLS.settings[setting]);
    if flag(0) && cx.state.voice.room.is_none() {
        rows.push(if mic_testing() {
            ("Stop mic test", Icon::MicOff, Press::MicTest)
        } else {
            ("Mic test", Icon::Mic, Press::MicTest)
        });
    }
    if flag(1) && cx.state.discord_user.is_some() {
        rows.push(if off_the_radar() {
            ("Back on the radar", Icon::Eye, Press::OffTheRadar)
        } else {
            ("Off the radar", Icon::EyeOff, Press::OffTheRadar)
        });
    }
    rows
}

/// Mic test pressed: starts it outside a call, or stops it.
pub fn toggle_mic_test(app: &App) {
    held(&MIC, |mic| {
        if mic.take().is_none() {
            let voice = &app.client.state().voice;
            if voice.room.is_none() {
                *mic = Some(GateMeter::start(
                    voice.microphone.as_deref(),
                    voice.speaker.as_deref(),
                    true,
                ));
            }
        }
    });
}

/// Hook, as the window takes in a change: the mic test stops once a call
/// starts, which uses the microphone itself.
pub fn stop_mic_test_in_call(app: &App) {
    if app.client.state().voice.room.is_some() {
        held(&MIC, |mic| *mic = None);
    }
}

/// Off the radar pressed: idle and the game hidden, putting aside what
/// was; pressed again, both back.
pub fn toggle_radar(app: &mut App) {
    let state = app.client.state();
    let now = (state.discord_presence.clone(), state.discord_show_game);
    let actions = held(&RADAR, |radar| radar_actions(radar, now));
    for action in actions {
        app.client.dispatch(action);
    }
}

/// Off the radar's changes: with nothing put aside, `now` (the status and
/// whether the game shows) is put aside for idle and the game hidden; with
/// something put aside, it comes back.
fn radar_actions(
    radar: &mut Option<(Option<String>, bool)>,
    now: (Option<String>, bool),
) -> Vec<Action> {
    match radar.take() {
        Some((status, game)) => {
            let mut actions = vec![Action::ShowGame(game)];
            actions.extend(status.map(Action::SetStatus));
            actions
        }
        None => {
            *radar = Some(now);
            vec![Action::SetStatus("idle".into()), Action::ShowGame(false)]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_the_radar_goes_idle_hides_the_game_and_puts_both_back() {
        let mut radar = None;
        let off = radar_actions(&mut radar, (Some("dnd".into()), true));
        assert!(matches!(
            off.as_slice(),
            [Action::SetStatus(idle), Action::ShowGame(false)] if idle == "idle"
        ));
        let back = radar_actions(&mut radar, (Some("idle".into()), false));
        assert!(matches!(
            back.as_slice(),
            [Action::ShowGame(true), Action::SetStatus(status)] if status == "dnd"
        ));
        assert!(radar.is_none(), "nothing put aside now");
    }
}
