//! Updates: the desktop's `Updates` looks at HaloCord's releases on the
//! chosen channel and installs one (`desktop.rs`); this asks for a look
//! when the check is turned on, on Check now, once at start with the check
//! on and every six hours after, shows what it found, and tells of a newer
//! release once in a notification that opens About. With "Install updates
//! automatically" on, a newer release is put in place for the next start
//! as soon as a look finds it, so nothing running stops; Update and restart
//! (or Restart now) ends the window so `main` can finish it. It runs with
//! Settings closed; About only shows it (`settings/updates.rs`).

use std::time::Duration;

use iced::Task;

use crate::{
    app::{App, Message},
    desktop::{self, ABOUT, UpdateChoices, UpdateStatus, UpdatesShown},
    layout::Layout,
    settings::Section,
};

/// How long between looks while Yapfast runs. GitHub allows an address 60
/// requests an hour without an account.
pub const EVERY: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Clone, Debug)]
pub enum Event {
    /// Six hours went by.
    Due,
    CheckNow,
    Install,
    Cancel,
    Skip,
    /// The offered release's page, in the browser.
    View,
}

/// What the person chose of updates: the channel, and the release skipped.
pub fn choices(layout: &Layout) -> UpdateChoices {
    let prefs = &layout.prefs;
    UpdateChoices {
        channel: prefs.update_channel,
        skipped: prefs.skipped_update.clone(),
    }
}

/// What About shows of the updates, where the desktop offers them.
pub fn shown(layout: &Layout) -> Option<UpdatesShown> {
    Some(desktop::desktop().updates()?.shown(&choices(layout)))
}

pub fn update_choices(app: &App) -> UpdateChoices {
    choices(&app.layout)
}

/// Looks at the releases; `announce` tells of a newer one in a
/// notification, as a look nobody asked for does.
pub fn look_for_updates(app: &mut App, announce: bool) {
    let Some(updates) = desktop::desktop().updates() else {
        return;
    };
    app.announce_updates |= announce;
    updates.look();
}

/// The look or the update went further: a release to tell of, or an
/// update ready to finish.
pub fn updates_heard(app: &mut App) -> Task<Message> {
    let Some(updates) = desktop::desktop().updates() else {
        return Task::none();
    };
    if updates.ready() {
        tracing::info!("the update is ready; ending to finish it");
        crate::exit_for(crate::Exit::Update);
        return app.quit();
    }
    let choices = update_choices(app);
    let shown = updates.shown(&choices);
    // Installing by itself: once a look finds a release this copy can put in
    // place, it goes in for the next start, and there's nothing to tell of.
    let automatic = app.layout.prefs.installs_updates()
        && shown.status == UpdateStatus::Available
        && shown.can_install;
    if automatic {
        updates.install_later(&choices);
    }
    if app.announce_updates && shown.status != UpdateStatus::Checking {
        app.announce_updates = false;
        if !automatic {
            announce_update(app, updates.offered(&choices));
        }
    }
    Task::none()
}

/// Tells of the newest release in a notification, once for each release,
/// unless notifications are off or the person skipped it.
fn announce_update(app: &mut App, offered: Option<desktop::Offered>) {
    let Some(offered) = offered.filter(|offered| !offered.skipped) else {
        return;
    };
    let prefs = &mut app.layout.prefs;
    if prefs.announced_update.as_deref() == Some(offered.tag.as_str()) {
        return;
    }
    prefs.announced_update = Some(offered.tag.clone());
    crate::layout::store(app.client.root(), "announced_update", &offered.tag);
    if !app.layout.prefs.notifications.unwrap_or(true) {
        return;
    }
    if let Some(notifier) = &app.shell.notifier {
        notifier.show(
            yapfast_core::chat::Alert {
                room: ABOUT.into(),
                title: format!("{} is available", offered.tag),
                body: "Click to see what's new.".into(),
                picture: None,
                call: false,
                mention: false,
            },
            true,
        );
    }
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let Some(updates) = desktop::desktop().updates() else {
        return Task::none();
    };
    let choices = update_choices(app);
    match event {
        Event::Due => {
            if app.layout.prefs.checks_for_updates() {
                look_for_updates(app, true);
            }
        }
        Event::CheckNow => look_for_updates(app, false),
        Event::Install => updates.install(&choices),
        Event::Cancel => updates.cancel(),
        Event::Skip => {
            if let Some(offered) = updates.offered(&choices).filter(|offered| !offered.skipped) {
                crate::layout::store(app.client.root(), "skipped_update", &offered.tag);
                app.layout.prefs.skipped_update = Some(offered.tag);
            }
        }
        Event::View => {
            if let Some(offered) = updates.offered(&choices) {
                crate::browser::open(&offered.page);
            }
        }
    }
    Task::none()
}

/// An update downloads: About shows how far, a few times a second.
pub fn updates_installing(app: &App) -> bool {
    app.win.settings.is_open()
        && desktop::desktop()
            .updates()
            .is_some_and(|updates| updates.installing())
}

/// Settings on About, as the release's notification opens it.
pub fn open_about(app: &mut App) -> Task<Message> {
    app.open_settings(Section::About)
}
