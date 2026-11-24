//! Right-click menu plugins: the copy rows and Quick mention, and the
//! window's side of plug-in point 5 (menu rows and what a pick does).

use iced::Task;
use yapfast_core::{application::Action, chat::ChatMessage, halocord as engine};

use super::{Kind, Plugin, Setting, id, off};
use crate::{
    app::{App, Message},
    menus::{MenuAct, MenuRow},
    pictures::Icon,
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
    about: "Adds Copy sticker link and Open sticker link to a sticker's right-click menu, in a message and in the sticker picker. Copy copies the web address of the sticker; Open shows it at full size in your browser.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: Some(&engine::copy_sticker_link::CopyStickerLink),
};

/// Copy sticker link in the sticker picker: a tile's right click opens
/// the row, while the plugin is on.
pub fn sticker_tile<'a>(
    cx: crate::ctx::Ctx<'a>,
    picture: &str,
    tile: iced::widget::MouseArea<'a, Message>,
) -> iced::widget::MouseArea<'a, Message> {
    if !cx.layout.halocord.on(&COPY_STICKER_LINK) || picture.is_empty() {
        return tile;
    }
    tile.on_right_press(Message::HaloCord(super::Event::StickerMenu(
        picture.to_owned(),
    )))
}

/// A picker tile's menu: Copy sticker link, the sticker's address without
/// the picker's thumbnail size.
pub(super) fn sticker_menu(app: &mut App, picture: &str) -> Task<Message> {
    if !app.layout.halocord.on(&COPY_STICKER_LINK) {
        return Task::none();
    }
    let link = picture.split_once('?').map_or(picture, |(base, _)| base);
    // No icon, as the row on a sent sticker has none.
    let copy = MenuRow::plain(MenuAct::CopyTextLink, "Copy sticker link").key(link);
    app.win.menu = Some(crate::menus::Menu::new(app.pressed.get(), vec![copy]));
    iced::widget::operation::focus(crate::app::NOBODY)
}

pub(super) const QUICK_MENTION: Plugin = Plugin {
    id: id::QUICK_MENTION,
    name: "Quick mention",
    about: "Adds Mention to a message's right-click menu. It puts the sender's name in your message box.",
    on_at_first: false,
    settings: &[super::off(
        "hover_button",
        "Button in the hover bar",
        "An @ beside Reply when you point at someone's message, which does the same.",
    )],
    apply: |_, _| {},
    hooks: Some(&engine::quick_mention::QuickMention),
};

/// Quick mention's @ in a message's hover bar, while its switch for it is
/// on, the message is someone else's on Discord and the composer writes.
pub fn mention_button<'a>(
    ctx: crate::ctx::Ctx<'a>,
    message: &ChatMessage,
    writes: bool,
) -> Option<iced::Element<'a, Message>> {
    let kept = &ctx.layout.halocord;
    let shown =
        writes && kept.on(&QUICK_MENTION) && kept.flag(&QUICK_MENTION, &QUICK_MENTION.settings[0]);
    let user = engine::quick_mention::author(message).filter(|_| shown)?;
    Some(
        ctx.look
            .icon_button(
                crate::pictures::Icon::AtSign,
                "Mention",
                Message::HaloCord(super::Event::Mention(user.to_owned())),
            )
            // As the hover bar's own buttons.
            .size(32.0, 20)
            .into(),
    )
}

pub(super) const COPY_PROFILE_LINK: Plugin = Plugin {
    id: id::COPY_PROFILE_LINK,
    name: "Copy profile link",
    about: "Adds Copy profile link to someone's right-click menu. It copies the web link to their profile.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: Some(&engine::copy_user_url::CopyUserUrl),
};

pub(super) const COPY_STATUS_URLS: Plugin = Plugin {
    id: id::COPY_STATUS_URLS,
    name: "Copy status link",
    about: "Adds Copy status link to someone's right-click menu while they stream. It copies the web address of their stream.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const COPY_PROFILE_COLORS: Plugin = Plugin {
    id: id::COPY_PROFILE_COLORS,
    name: "Copy profile colors",
    about: "Adds Copy profile colors to someone's right-click menu when their profile has a theme. It copies its two colors as hex codes, such as #1E1E2E.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

pub(super) const FIND_REPLY: Plugin = Plugin {
    id: id::FIND_REPLY,
    name: "Find reply",
    about: "Adds Jump to reply to a message's right-click menu when someone replied to it. It takes you to the first reply. With a switch below on, it can also take you to a later reply to the author, or a mention of them.",
    on_at_first: false,
    settings: &[
        off(
            "author",
            "Also replies to the author",
            "Counts a later reply from someone else to any of the author's messages, not only this one.",
        ),
        off(
            "pings",
            "Also messages that mention the author",
            "Counts a later message from someone else that @mentions the author.",
        ),
    ],
    apply: |_, _| {},
    hooks: None,
};

/// The web address of the stream `user` is on, as their open profile, the
/// member list or the friend list knows it (Copy status link).
fn status_link(app: &App, user: &str) -> Option<String> {
    let state = app.client.state();
    let shown = [state.profile.as_ref(), state.panel_profile.as_ref()]
        .into_iter()
        .flatten()
        .filter(|view| view.profile.id == user)
        .map(|view| view.profile.activities.clone());
    let listed = state
        .members
        .as_ref()
        .and_then(|members| members.as_ref().ok())
        .and_then(|members| members.iter().find(|member| member.id == user))
        .map(|member| member.activities.clone());
    let friend = state
        .friends
        .iter()
        .find(|friend| friend.id == user)
        .map(|friend| friend.activities.clone());
    shown.chain(listed).chain(friend).find_map(|activities| {
        activities
            .iter()
            .find_map(|activity| activity.url.as_deref().map(str::to_owned))
    })
}

/// The two colours of `user`'s profile theme as hex codes, from their open
/// profile (Copy profile colors).
fn profile_colors(app: &App, user: &str) -> Option<String> {
    let state = app.client.state();
    let [primary, accent] = [state.profile.as_ref(), state.panel_profile.as_ref()]
        .into_iter()
        .flatten()
        .find(|view| view.profile.id == user)?
        .profile
        .theme?;
    Some(format!("#{primary:06X}, #{accent:06X}"))
}

/// The first loaded message that replies to `item` (Find reply), or, with
/// its switches on, a later one that answers or mentions its author.
fn first_reply(app: &App, item: &str) -> Option<String> {
    let messages = &app.client.state().messages;
    let answers = |later: &ChatMessage| {
        later.reply.as_ref().and_then(|reply| reply.item.as_deref()) == Some(item)
    };
    let kept = &app.layout.halocord;
    let (to_author, pings) = (
        kept.flag(&FIND_REPLY, &FIND_REPLY.settings[0]),
        kept.flag(&FIND_REPLY, &FIND_REPLY.settings[1]),
    );
    let at = messages.iter().position(|message| message.item == item);
    let (Some(at), true) = (at, to_author || pings) else {
        return messages
            .iter()
            .find(|message| answers(message))
            .map(|message| message.item.clone());
    };
    // The switches: later messages from someone else that answer the
    // author, or mention them.
    let message = &messages[at];
    let user = yapfast_core::discord::sender_user(&message.sender).filter(|user| !user.is_empty());
    messages[at + 1..]
        .iter()
        .find(|later| {
            let someone_else = later.sender != message.sender;
            answers(later)
                || (to_author
                    && someone_else
                    && later.reply.as_ref().is_some_and(|reply| {
                        // By the answered message's sender when it's loaded;
                        // else by the name the reply shows, as a reply
                        // carries no sender of its own.
                        match reply
                            .item
                            .as_deref()
                            .and_then(|item| messages.iter().find(|shown| shown.item == item))
                        {
                            Some(answered) => answered.sender == message.sender,
                            None => reply.author == message.author,
                        }
                    }))
                || (pings
                    && someone_else
                    && user.is_some_and(|user| {
                        later.mentions.iter().any(|mentioned| {
                            matches!(&mentioned.target, yapfast_core::mention::Target::User(id)
                                if id == user
                                    || yapfast_core::discord::sender_user(id) == Some(user))
                        })
                    }))
        })
        .map(|found| found.item.clone())
}

/// A row of the window's own plugins, which need more than the engine is
/// told: what the window knows of the person, or the other messages.
fn own_row(label: &'static str, icon: Icon, key: &str) -> MenuRow {
    MenuRow::new(MenuAct::Plugin, label, icon).key(key)
}

/// The first of a menu's plugin rows is set apart from the menu's own.
fn set_apart(mut rows: Vec<MenuRow>) -> Vec<MenuRow> {
    if let Some(first) = rows.first_mut() {
        first.separated = true;
    }
    rows
}

/// Hook 5: every plugin row for a message's menu, before Delete: the
/// engine's, then Find reply's when a loaded message replies to it.
pub fn message_rows(app: &App, message: &ChatMessage) -> Vec<MenuRow> {
    let mut rows = menu_rows(message);
    if app.layout.halocord.is(id::FIND_REPLY) && first_reply(app, &message.item).is_some() {
        rows.push(own_row("Jump to reply", Icon::Reply, id::FIND_REPLY));
    }
    if app.layout.halocord.is(id::SONG_LINK) && !song_links(&message.body).is_empty() {
        rows.push(own_row("Open on song.link", Icon::Music, id::SONG_LINK));
    }
    set_apart(rows)
}

pub(super) const SONG_LINK: Plugin = Plugin {
    id: id::SONG_LINK,
    name: "Song link",
    about: "\"Open on song.link\" in the menu of a message with a Spotify or TIDAL track, an Apple Music album or song, or a YouTube Music video opens song.link's page for it, which links the song on every service.",
    on_at_first: false,
    settings: &[
        Setting {
            key: "country",
            heading: "Country",
            note: "Two letters, for the services' links in that country. Empty, song.link goes by where you are.",
            kind: Kind::Text {
                placeholder: "US",
                unless_changed: "",
            },
        },
        Setting {
            key: "buttons",
            heading: "Buttons under a song",
            note: "\"Song links\" under a message with a song asks song.link where it is, then shows a button for each service.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
        off(
            "apps",
            "Open in the apps",
            "A service's button opens its desktop app, where song.link knows its address.",
        ),
    ],
    apply: |_, _| {},
    hooks: None,
};

/// Song link: the song links in `body` it knows, by their host and path:
/// a Spotify or TIDAL track, an Apple Music (or iTunes) album or song, and a
/// YouTube Music video.
pub fn song_links(body: &str) -> Vec<&str> {
    let mut found: Vec<&str> = body
        .split(|char: char| char.is_whitespace() || char == '<' || char == '>')
        .map(|word| word.trim_end_matches([')', '.', ',', '!', '?']))
        .filter(|word| song(word))
        .collect();
    found.dedup();
    found
}

/// Whether `link` is a song Song link knows; see [`song_links`].
fn song(link: &str) -> bool {
    let Some(rest) = link.strip_prefix("https://") else {
        return false;
    };
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let has = |segment: &str| path.split('/').any(|part| part == segment);
    match host {
        "open.spotify.com" | "play.spotify.com" | "tidal.com" | "listen.tidal.com" => has("track"),
        "music.apple.com" | "itunes.apple.com" => has("album") || has("song"),
        "music.youtube.com" => {
            path == "watch" && query.split('&').any(|pair| pair.starts_with("v="))
        }
        _ => false,
    }
}

/// Song link: song.link's page for `link`, for `country` when that's two
/// letters. Spotify's `si` (who shared it) is left off, and the link goes
/// into song.link's address encoded, so its own `?` and `&` stay its own.
pub fn song_page(link: &str, country: &str) -> String {
    let mut page = String::from("https://song.link/");
    percent_encode(&without_share_id(link), &mut page);
    let country = country.trim().to_uppercase();
    if country.len() == 2 && country.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        page.push_str("?userCountry=");
        page.push_str(&country);
    }
    page
}

/// `text` percent-encoded onto `into`: all but letters, digits and `-._~`.
fn percent_encode(text: &str, into: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            into.push(char::from(byte));
        } else {
            into.push('%');
            into.push(char::from(HEX[usize::from(byte >> 4)]));
            into.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
}

/// `link` without Spotify's `si`, which names who shared it.
pub(super) fn without_share_id(link: &str) -> String {
    let spotify = ["https://open.spotify.com/", "https://play.spotify.com/"]
        .iter()
        .any(|start| link.starts_with(start));
    let Some((base, query)) = link.split_once('?').filter(|_| spotify) else {
        return link.to_owned();
    };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|pair| !pair.starts_with("si=") && !pair.is_empty())
        .collect();
    if kept.is_empty() {
        base.to_owned()
    } else {
        format!("{base}?{}", kept.join("&"))
    }
}

/// Song link: the page for the first song link of the message `item`.
fn song_link_page(app: &App, item: &str) -> Option<String> {
    let message = app.client.state().message(item)?;
    let link = *song_links(&message.body).first()?;
    let country = app.layout.halocord.text(&SONG_LINK, &SONG_LINK.settings[0]);
    Some(song_page(link, country))
}

/// View icons: the decoration `user` wears around their avatar, in the
/// open server where they set one there, else their own.
fn decoration(app: &App, user: &str) -> Option<String> {
    let state = app.client.state();
    let guild = state
        .selected
        .as_deref()
        .and_then(yapfast_core::discord::parse_room)
        .map(|(guild, _)| guild)
        .filter(|guild| *guild != "@me");
    let names = state.discord_names.as_ref()?;
    let found = names.read().decoration(guild, user)?;
    Some(found.to_string())
}

/// View icons: a decoration's picture at 512 pixels, moving.
pub fn decoration_page(decoration: &str) -> String {
    let path = decoration.split('?').next().unwrap_or(decoration);
    format!("{path}?size=512&passthrough=true")
}

/// What a pick of one of the window's own plugin rows does, if it's one.
fn own_pick(app: &mut App, key: &str, item: &str, place: Option<&str>) -> Option<Task<Message>> {
    let copy = match (key, place) {
        (id::VIEW_ICONS, Some(user)) => {
            if let Some(decoration) = decoration(app, user) {
                crate::browser::open(&decoration_page(&decoration));
            }
            return Some(Task::none());
        }
        (id::COPY_STATUS_URLS, Some(user)) => status_link(app, user),
        (id::COPY_PROFILE_COLORS, Some(user)) => profile_colors(app, user),
        (id::SONG_LINK, None) => {
            if let Some(page) = song_link_page(app, item) {
                crate::browser::open(&page);
            }
            return Some(Task::none());
        }
        (id::FIND_REPLY, None) => {
            if let Some(reply) = first_reply(app, item) {
                app.client.dispatch(Action::Jump(reply));
            }
            return Some(app.sync());
        }
        _ => return None,
    };
    Some(copy.map_or_else(Task::none, iced::clipboard::write))
}

/// The copy Copy file text asked for came back: the file as text, for the
/// clipboard in place of a picture. None when it was a picture.
pub fn copied_text(bytes: &[u8]) -> Option<String> {
    engine::copy_file_contents::take_wanted().then(|| engine::copy_file_contents::as_text(bytes))
}

/// Hook 5: HaloCord's plugins' rows for a message's menu, before Delete.
pub fn menu_rows(message: &ChatMessage) -> Vec<MenuRow> {
    keyed(engine::menu(message))
}

/// Hook 5: HaloCord's plugins' rows for a person's menu, after its own:
/// the engine's, then Copy status link's and Copy profile colors' when
/// there's something to copy.
pub fn person_rows(app: &App, user: &str) -> Vec<MenuRow> {
    let mut rows = keyed(engine::person_menu(user));
    let on = |plugin| app.layout.halocord.is(plugin);
    if on(id::COPY_STATUS_URLS) && status_link(app, user).is_some() {
        rows.push(own_row(
            "Copy status link",
            Icon::Link,
            id::COPY_STATUS_URLS,
        ));
    }
    if on(id::VIEW_ICONS) && decoration(app, user).is_some() {
        rows.push(own_row("View decoration", Icon::Image, id::VIEW_ICONS));
    }
    if on(id::COPY_PROFILE_COLORS) && profile_colors(app, user).is_some() {
        rows.push(own_row(
            "Copy profile colors",
            Icon::Palette,
            id::COPY_PROFILE_COLORS,
        ));
    }
    set_apart(rows)
}

/// Menu rows from the engine's keys and labels, the first set apart.
fn keyed(rows: Vec<(String, &'static str)>) -> Vec<MenuRow> {
    let mut rows: Vec<MenuRow> = rows
        .into_iter()
        .map(|(key, label)| MenuRow::new(MenuAct::Plugin, label, icon_of(&key)).key(key))
        .collect();
    if let Some(first) = rows.first_mut() {
        first.separated = true;
    }
    rows
}

/// The icon by a plugin's row, from the plugin its `key` names, as every
/// other row in Yapfast's menus has one.
fn icon_of(key: &str) -> Icon {
    match key.split_once('/').map_or(key, |(plugin, _)| plugin) {
        id::COPY_MENTION | id::QUICK_MENTION => Icon::AtSign,
        id::COPY_PROFILE_LINK | id::COPY_MEDIA_LINK => Icon::Link,
        id::COPY_STICKER_LINK => Icon::Sticker,
        _ => Icon::Copy,
    }
}

/// Hook 5: A plugin's menu row was chosen: in a person's menu (`place` is the
/// person) it's theirs, else it's for the message `item`.
pub fn chosen(app: &mut App, key: &str, item: &str, place: Option<&str>) -> Task<Message> {
    if let Some(task) = super::chat::pick_timestamp(app, key) {
        return task;
    }
    if let Some((key, at)) = key.split_once('#') {
        let Some(message) = app.client.state().message(item).cloned() else {
            return Task::none();
        };
        let at = at.parse().unwrap_or(usize::MAX);
        return done(app, engine::attachment_pick(key, &message, at));
    }
    if let Some(task) = own_pick(app, key, item, place) {
        return task;
    }
    if let Some(task) = search_pick(app, key, item) {
        return task;
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
    // A search result's message too, for Search result menu's rows.
    let found = || search_hit(app, item).map(|hit| hit.message.clone());
    let Some(message) = app.client.state().message(item).cloned().or_else(found) else {
        return Task::none();
    };
    done(app, engine::pick(key, &message))
}

/// What a chosen row gave: its actions, its text to copy, and a web page to
/// open in the browser.
pub(super) fn done(app: &mut App, picked: engine::Picked) -> Task<Message> {
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

pub(super) const SEARCH_MENU: Plugin = Plugin {
    id: id::SEARCH_MENU,
    name: "Search result menu",
    about: "A right click on a search result opens a menu for its message: Jump to message, Copy text, Copy message link, Copy message ID, and the rows other plugins add to a message's menu.",
    on_at_first: false,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// Hook: a search result, with its menu on a right click while Search
/// result menu is on.
pub fn search_result<'a>(
    cx: crate::ctx::Ctx<'a>,
    found: iced::Element<'a, Message>,
    room: &str,
    item: &str,
) -> iced::Element<'a, Message> {
    if !cx.layout.halocord.is(id::SEARCH_MENU) {
        return found;
    }
    let (room, item) = (room.to_owned(), item.to_owned());
    crate::widgets::RightPress::new(found, move |_| {
        Message::HaloCord(super::Event::SearchMenu(room.clone(), item.clone()))
    })
    .into()
}

/// The search result with message `item`, as the search found it.
fn search_hit<'a>(app: &'a App, item: &str) -> Option<&'a yapfast_core::chat::SearchHit> {
    app.client
        .state()
        .search
        .as_ref()?
        .results
        .iter()
        .find(|hit| hit.message.item == item)
}

/// A Discord message's link and id, by its room and item, made as Copy
/// message link makes them.
fn search_link(room: &str, item: &str) -> Option<(String, String)> {
    let id = yapfast_core::discord::item_order(item)?;
    yapfast_core::discord::parse_room(room)?;
    Some((
        format!("{}/{id}", yapfast_core::application::network_link(room)),
        id.to_string(),
    ))
}

/// Opens the menu of the search result `item` of `room`, where the pointer
/// pressed.
pub(super) fn search_menu(app: &mut App, room: String, item: String) -> Task<Message> {
    let Some(hit) = search_hit(app, &item) else {
        return Task::none();
    };
    let message = hit.message.clone();
    let row = |label: &'static str, icon: Icon, action: &str| {
        MenuRow::new(MenuAct::Plugin, label, icon).key(format!("{}/{action}", id::SEARCH_MENU))
    };
    let mut rows = vec![row("Jump to message", Icon::ArrowRight, "jump")];
    if !message.body.is_empty() {
        rows.push(row("Copy text", Icon::Copy, "text"));
    }
    if search_link(&room, &item).is_some() {
        rows.push(row("Copy message link", Icon::Chain, "link"));
        rows.push(row("Copy message ID", Icon::Id, "id"));
    }
    rows.extend(menu_rows(&message));
    app.win.menu = Some(crate::menus::Menu {
        item: Some(item),
        ..crate::menus::Menu::new(app.pressed.get(), rows)
    });
    iced::widget::operation::focus(crate::app::NOBODY)
}

/// What one of Search result menu's own rows does.
fn search_pick(app: &mut App, key: &str, item: &str) -> Option<Task<Message>> {
    let action = key.strip_prefix(id::SEARCH_MENU)?.strip_prefix('/')?;
    let hit = search_hit(app, item)?;
    let (room, body) = (hit.room.clone(), hit.message.body.clone());
    Some(match action {
        "jump" => {
            app.client.dispatch(Action::JumpInRoom {
                room,
                item: item.to_owned(),
            });
            app.sync()
        }
        "text" => iced::clipboard::write(body),
        "link" | "id" => {
            let (link, id) = search_link(&room, item)?;
            iced::clipboard::write(if action == "link" { link } else { id })
        }
        _ => Task::none(),
    })
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
