//! HaloCord's plugins on the server list: Server list indicators' counts at
//! its top and Server sidebar's column for an open folder.

use std::sync::Arc;

use yapfast_core::chat::{Friend, Presence, ServerFolder};

use super::{change, open_plugin, preview, shows};
use crate::{
    app::{App, Message},
    halocord::{Change, SERVER_COUNTS, SERVER_SIDEBAR, changed, place},
    rail::{Event, Kind},
};

fn friend(id: &str, presence: Presence) -> Friend {
    Friend {
        id: id.into(),
        name: id.into(),
        decoration: None,
        nickname: None,
        avatar: None,
        presence: Some(presence),
        activity: None,
        activities: Default::default(),
        since: None,
    }
}

fn set(app: &mut App, change: Change) {
    changed(&mut app.layout, &mut app.client, change);
    super::change(app, |_| {});
}

#[test]
fn the_counts_show_at_the_top_of_the_server_list_as_set() {
    let (mut app, _folder) = preview();
    change(&mut app, |state| {
        state.friends = vec![
            friend("a", Presence::Online),
            friend("b", Presence::Mobile),
            friend("c", Presence::Idle),
            friend("d", Presence::Offline),
        ];
    });
    let servers = app.client.state().servers.len();
    let servers = format!("{servers} servers");
    assert!(shows(&app, "3 online"), "on at first, the friends around");
    assert!(!shows(&app, &servers), "only the friends at first");
    let plugin = place(SERVER_COUNTS).expect("the plugin");
    set(
        &mut app,
        Change::Flag {
            plugin,
            setting: 1,
            on: true,
        },
    );
    assert!(shows(&app, "2 online"), "idle friends left out");
    set(
        &mut app,
        Change::Choose {
            plugin,
            setting: 0,
            choice: 1,
        },
    );
    assert!(shows(&app, &servers) && !shows(&app, "2 online"));
    set(
        &mut app,
        Change::Choose {
            plugin,
            setting: 0,
            choice: 2,
        },
    );
    assert!(shows(&app, &servers) && shows(&app, "2 online"), "both");
    set(&mut app, Change::Switch { plugin, on: false });
    assert!(
        !shows(&app, &servers) && !shows(&app, "2 online"),
        "gone while off"
    );
}

#[test]
fn the_counts_list_their_settings() {
    let (mut app, _folder) = preview();
    open_plugin(&mut app, SERVER_COUNTS);
    for row in ["What it counts", "Only friends who are online"] {
        assert!(shows(&app, row), "{row} is offered");
    }
}

/// The preview's servers, all in one folder.
fn in_a_folder(app: &mut App) {
    let folder = ServerFolder {
        id: "discord-folder:7".into(),
        name: None,
        color: None,
    };
    change(app, |state| {
        for server in &mut state.servers {
            Arc::make_mut(server).folder = Some(folder.clone());
        }
    });
}

fn ids(items: &[crate::rail::RailItem]) -> Vec<&str> {
    items.iter().map(|item| item.id.as_str()).collect()
}

#[test]
fn an_open_folder_goes_to_the_sidebar_while_it_is_on() {
    let (mut app, _folder) = preview();
    in_a_folder(&mut app);
    let open = |app: &mut App| {
        let _ = app.update(Message::Rail(Event::Folder("discord-folder:7".into())));
    };
    open(&mut app);
    assert!(app.rail.side.is_empty(), "off at first: under the folder");
    let plain = crate::conversation::navigation_ctx(app.ctx());
    let plugin = place(SERVER_SIDEBAR).expect("the plugin");
    set(&mut app, Change::Switch { plugin, on: true });
    assert_eq!(
        ids(&app.rail.side),
        ["discord-folder:7", "friends", "coop"],
        "the folder and its servers beside the list"
    );
    assert_eq!(ids(&app.rail.items), ["divider", "discord-folder:7", "+"]);
    assert!(
        matches!(app.rail.items[1].kind, Kind::Folder { open: false, .. }),
        "closed in the list"
    );
    assert_eq!(
        crate::conversation::navigation_ctx(app.ctx()),
        plain + crate::rail::WIDTH,
        "the column takes the rail's width"
    );
    let _ = app.view();
    // Keep them in the list too; and no folder tile in the sidebar.
    set(
        &mut app,
        Change::Flag {
            plugin,
            setting: 0,
            on: true,
        },
    );
    set(
        &mut app,
        Change::Flag {
            plugin,
            setting: 4,
            on: false,
        },
    );
    assert_eq!(ids(&app.rail.side), ["friends", "coop"]);
    assert_eq!(
        ids(&app.rail.items),
        ["divider", "discord-folder:7", "friends", "coop", "+"]
    );
    // Closing it empties the sidebar and gives the room back.
    open(&mut app);
    assert!(app.rail.side.is_empty());
    assert_eq!(crate::conversation::navigation_ctx(app.ctx()), plain);
    // Off, an open folder is under its tile again.
    open(&mut app);
    set(&mut app, Change::Switch { plugin, on: false });
    assert!(app.rail.side.is_empty());
    assert_eq!(app.rail.items.len(), 5);
}

#[test]
fn the_sidebar_closes_folders_as_set() {
    let (mut app, _folder) = preview();
    in_a_folder(&mut app);
    let plugin = place(SERVER_SIDEBAR).expect("the plugin");
    set(&mut app, Change::Switch { plugin, on: true });
    let folder = "discord-folder:7".to_owned();
    app.layout.open_folders.insert("discord-folder:gone".into());
    let _ = app.update(Message::Rail(Event::Folder(folder.clone())));
    assert_eq!(
        app.layout.open_folders.iter().collect::<Vec<_>>(),
        [&folder],
        "one open at a time"
    );
    // Picking a server or Direct messages keeps it open until set so.
    let _ = app.update(Message::Rail(Event::Server("coop".into())));
    let _ = app.update(Message::Rail(Event::Home));
    assert!(app.layout.open_folders.contains(&folder));
    set(
        &mut app,
        Change::Flag {
            plugin,
            setting: 2,
            on: true,
        },
    );
    let _ = app.update(Message::Rail(Event::Server("coop".into())));
    assert!(
        !app.layout.open_folders.contains(&folder),
        "closed on a pick"
    );
    let _ = app.update(Message::Rail(Event::Folder(folder)));
    set(
        &mut app,
        Change::Flag {
            plugin,
            setting: 3,
            on: true,
        },
    );
    let _ = app.update(Message::Rail(Event::Home));
    assert!(
        app.layout.open_folders.is_empty(),
        "closed on Direct messages"
    );
}

#[test]
fn the_sidebar_lists_its_settings() {
    let (mut app, _folder) = preview();
    open_plugin(&mut app, SERVER_SIDEBAR);
    for row in [
        "Keep them in the server list too",
        "One folder open at a time",
        "Close the folder when you pick a server",
        "Close folders on Direct messages",
        "Show the folder above its servers",
    ] {
        assert!(shows(&app, row), "{row} is offered");
    }
}
