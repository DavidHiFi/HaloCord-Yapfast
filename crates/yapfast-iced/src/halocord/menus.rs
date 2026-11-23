//! Right-click menu plugins: the copy rows and Quick mention, and the
//! window's side of plug-in point 5 (menu rows and what a pick does).

use iced::Task;
use yapfast_core::{application::Action, chat::ChatMessage, halocord as engine};

use super::{Plugin, id};
use crate::{
    app::{App, Message},
    menus::{MenuAct, MenuRow},
};

pub(super) const COPY_MENTION: Plugin = Plugin {
    id: id::COPY_MENTION,
    name: "Copy mention",
    about: "Adds Copy mention to someone's right-click menu. It copies the text that pings them when you send it.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: Some(&engine::copy_user_mention::CopyUserMention),
};

pub(super) const COPY_MEDIA_LINK: Plugin = Plugin {
    id: id::COPY_MEDIA_LINK,
    name: "Copy file link",
    about: "Adds Copy file link to a file's right-click menu. It copies the web address of the file.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: Some(&engine::copy_media_link::CopyMediaLink),
};

pub(super) const COPY_FILE_TEXT: Plugin = Plugin {
    id: id::COPY_FILE_TEXT,
    name: "Copy file text",
    about: "Adds Copy file text to a text file's right-click menu, for files up to 2 MB. It copies what the file says, so you don't have to download it.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: Some(&engine::copy_file_contents::CopyFileContents),
};

pub(super) const COPY_STICKER_LINK: Plugin = Plugin {
    id: id::COPY_STICKER_LINK,
    name: "Copy sticker link",
    about: "Adds Copy sticker link and Open sticker link to a sticker's right-click menu. Copy copies the web address of the sticker; Open shows it at full size in your browser.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: Some(&engine::copy_sticker_link::CopyStickerLink),
};

pub(super) const QUICK_MENTION: Plugin = Plugin {
    id: id::QUICK_MENTION,
    name: "Quick mention",
    about: "Adds Mention to a message's right-click menu. It puts the sender's name in your message box.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: Some(&engine::quick_mention::QuickMention),
};

pub(super) const COPY_PROFILE_LINK: Plugin = Plugin {
    id: id::COPY_PROFILE_LINK,
    name: "Copy profile link",
    about: "Adds Copy profile link to someone's right-click menu. It copies the web link to their profile.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: Some(&engine::copy_user_url::CopyUserUrl),
};

/// The copy Copy file text asked for came back: the file as text, for the
/// clipboard in place of a picture. None when it was a picture.
pub fn copied_text(bytes: &[u8]) -> Option<String> {
    engine::copy_file_contents::take_wanted().then(|| engine::copy_file_contents::as_text(bytes))
}

/// Hook 5: HaloCord's plugins' rows for a message's menu, before Delete.
pub fn menu_rows(message: &ChatMessage) -> Vec<MenuRow> {
    keyed(engine::menu(message))
}

/// Hook 5: HaloCord's plugins' rows for a person's menu, after its own.
pub fn person_rows(user: &str) -> Vec<MenuRow> {
    keyed(engine::person_menu(user))
}

/// Menu rows from the engine's keys and labels, the first set apart.
fn keyed(rows: Vec<(String, &'static str)>) -> Vec<MenuRow> {
    let mut rows: Vec<MenuRow> = rows
        .into_iter()
        .map(|(key, label)| MenuRow::plain(MenuAct::Plugin, label).key(key))
        .collect();
    if let Some(first) = rows.first_mut() {
        first.separated = true;
    }
    rows
}

/// Hook 5: A plugin's menu row was chosen: in a person's menu (`place` is the
/// person) it's theirs, else it's for the message `item`.
pub fn chosen(app: &mut App, key: &str, item: &str, place: Option<&str>) -> Task<Message> {
    if let Some((key, at)) = key.split_once('#') {
        let Some(message) = app.client.state().message(item).cloned() else {
            return Task::none();
        };
        let at = at.parse().unwrap_or(usize::MAX);
        return done(app, engine::attachment_pick(key, &message, at));
    }
    match place {
        Some(user) => done(app, engine::person_pick(key, user)),
        None => pick(app, key, item),
    }
}

/// Hook 5: HaloCord's plugins' rows for the menu of attachment `at` of `message`.
/// Each key carries the attachment's place after a `#`.
pub fn attachment_rows(message: &ChatMessage, at: usize) -> Vec<MenuRow> {
    let mut rows = keyed(engine::attachment_menu(message, at));
    for row in &mut rows {
        row.key = format!("{}#{at}", row.key);
    }
    rows
}

/// Hook 5: Opens the menu of attachment `at` of message `item` at the pointer,
/// when plugins give it rows.
pub fn attachment_menu(app: &mut App, item: &str, at: usize) -> Task<Message> {
    let Some(message) = app.client.state().message(item) else {
        return Task::none();
    };
    let mut rows = attachment_rows(message, at);
    let Some(first) = rows.first_mut() else {
        return Task::none();
    };
    first.separated = false;
    app.win.menu = Some(crate::menus::Menu {
        item: Some(item.to_owned()),
        ..crate::menus::Menu::new(app.pressed.get(), rows)
    });
    iced::widget::operation::focus(crate::app::NOBODY)
}

/// Hook 5: A plugin's menu row was chosen for the message `item`.
pub fn pick(app: &mut App, key: &str, item: &str) -> Task<Message> {
    let Some(message) = app.client.state().message(item).cloned() else {
        return Task::none();
    };
    done(app, engine::pick(key, &message))
}

/// What a chosen row gave: its actions, its text to copy, and a web page to
/// open in the browser.
fn done(app: &mut App, picked: engine::Picked) -> Task<Message> {
    let mentions = picked
        .actions
        .iter()
        .any(|action| matches!(action, Action::MentionPerson(_)));
    for action in picked.actions {
        app.client.dispatch(action);
    }
    let copied = picked.copy.map_or_else(Task::none, iced::clipboard::write);
    if let Some(page) = picked.open {
        crate::browser::open(&page);
    }
    // A mention changes the message box's text: it takes it in and has focus.
    let focus = if mentions {
        let draft = app.client.state().draft().to_owned();
        app.composer.draft.reset(&draft);
        app.composer.draft_known = draft;
        iced::widget::operation::focus(crate::composer::COMPOSER)
    } else {
        Task::none()
    };
    Task::batch([app.sync(), copied, focus])
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rows;

    impl engine::Hooks for Rows {
        fn id(&self) -> &'static str {
            "rows-test"
        }
        fn menu(&self, message: &ChatMessage) -> Vec<(&'static str, &'static str)> {
            // Only for this test's message: the window's tests share the core.
            if message.body == "halocord rows test" {
                vec![("raw", "Copy raw"), ("source", "View source")]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn plugin_rows_join_a_messages_menu() {
        let message = ChatMessage {
            body: "halocord rows test".into(),
            ..ChatMessage::default()
        };
        engine::plug(&Rows, true);
        let rows = menu_rows(&message);
        engine::plug(&Rows, false);
        let keys: Vec<_> = rows.iter().map(|row| row.key.as_str()).collect();
        assert_eq!(keys, ["rows-test/raw", "rows-test/source"]);
        assert!(rows.iter().all(|row| row.act == MenuAct::Plugin));
        assert!(rows[0].separated && !rows[1].separated);
        assert!(menu_rows(&message).is_empty());
    }
}
