//! Discord's servers dragged on the rail, as in Discord's server list: a
//! tile pressed and moved lifts, let go above another it moves there and
//! Discord is told the new order; Escape, or let go away from the rail,
//! and nothing moves; pressed and let go without moving, it opens.

use std::sync::Arc;

use iced::{Point, mouse, widget::Id};
use iced_test::{Simulator, selector::Candidate};
use yapfast_core::{
    chat::{Command, DiscordCommand, Event, ServerGroup},
    yapfast_discord::{
        marked::{MarkedServer, ServerMarks},
        ready::{self, permission},
        service::{Snapshot, Status},
    },
};

use super::{interact, local, start};
use crate::{
    app::{App, Message},
    rail::{self, TILE},
};

/// The window signed in to Discord with three servers on the rail, in
/// this order: Alpha ("10"), Bravo ("20") and Charlie ("30").
fn three_servers() -> (App, tempfile::TempDir) {
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    let mut app = start(&folder, false);
    let server = |id: &str, name: &str| MarkedServer {
        server: Arc::new(
            ready::server(&serde_json::json!({
                "id": id, "owner_id": "99",
                "properties": {"name": name},
                "roles": [{"id": id, "name": "@everyone", "position": 0,
                           "permissions": permission::VIEW_CHANNEL.to_string()}],
                "channels": [{"id": format!("{id}1"), "type": 0, "name": "general",
                              "position": 0}],
            }))
            .expect("the server"),
        ),
        marks: Arc::new(ServerMarks::default()),
    };
    app.client.apply(Event::Discord(Arc::new(Snapshot {
        status: Status::Online,
        user_id: Some("1".into()),
        servers: vec![
            server("10", "Alpha"),
            server("20", "Bravo"),
            server("30", "Charlie"),
        ],
        ..Default::default()
    })));
    app.client.app.state.account = Some("1".into());
    let _ = app.sync();
    (app, folder)
}

/// The middle of the rail tile `id`, in the window.
fn tile(ui: &mut Simulator<'_, Message>, app: &App, id: &str) -> Point {
    let list = ui
        .find(|candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                id: Some(list),
                visible_bounds: Some(bounds),
                ..
            } if *list == Id::new(crate::rail_drag::LIST) => Some(bounds),
            _ => None,
        })
        .expect("the rail's list");
    let index = app
        .rail
        .items
        .iter()
        .position(|item| item.id == id)
        .unwrap_or_else(|| panic!("no {id} on the rail"));
    let top: f32 = app.rail.items[..index]
        .iter()
        .enumerate()
        .map(|(index, item)| item.height(index))
        .sum();
    Point::new(rail::WIDTH / 2.0, list.y + top - app.rail.scroll.offset + TILE / 2.0)
}

/// Presses on the tile `id` and moves the pointer `by` px down, letting go
/// if `release`.
fn press(app: &mut App, id: &str, by: f32, release: bool) {
    let at = {
        let ui = &mut Simulator::with_size(
            iced::Settings::default(),
            app.shell.frame,
            app.view(),
        );
        tile(ui, app, id)
    };
    interact(app, |ui| {
        ui.point_at(at);
        ui.simulate([iced::Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))]);
        let to = Point::new(at.x, at.y + by);
        ui.point_at(to);
        ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved { position: to })]);
        if release {
            ui.simulate([iced::Event::Mouse(mouse::Event::ButtonReleased(
                mouse::Button::Left,
            ))]);
        }
    });
}

/// The orders the window sent Discord since this was last asked.
fn arranged(app: &mut App) -> Vec<Vec<ServerGroup>> {
    std::mem::take(&mut app.client.sent)
        .into_iter()
        .filter_map(|command| match command {
            Command::Discord(DiscordCommand::ArrangeServers(groups)) => Some(groups),
            _ => None,
        })
        .collect()
}

fn lone(id: &str) -> ServerGroup {
    ServerGroup {
        folder: None,
        servers: vec![id.to_owned()],
    }
}

#[test]
fn a_server_dragged_above_the_first_moves_there_and_discord_is_told() {
    let (mut app, _folder) = three_servers();
    let ids: Vec<&str> = app
        .rail
        .items
        .iter()
        .filter(|item| item.kind == rail::Kind::Server)
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(ids, ["discord:10", "discord:20", "discord:30"]);
    let _ = arranged(&mut app);

    // Moved 6 px, Charlie lifts.
    press(&mut app, "discord:30", 6.0, false);
    let drag = app.rail.dragging.as_ref().expect("Charlie lifted");
    assert_eq!(drag.id, "discord:30");
    assert!(!drag.folder);

    // Carried to Alpha's top third and let go: first, alone.
    let above = {
        let ui = &mut Simulator::with_size(
            iced::Settings::default(),
            app.shell.frame,
            app.view(),
        );
        tile(ui, &app, "discord:10")
    };
    let to = Point::new(above.x, above.y - TILE / 2.0 + 2.0);
    let _ = app.update(Message::Rail(rail::Event::Carry(to)));
    let _ = app.update(Message::Rail(rail::Event::Drop));
    assert!(app.rail.dragging.is_none());
    assert_eq!(
        arranged(&mut app),
        [vec![lone("discord:30"), lone("discord:10"), lone("discord:20")]]
    );
}

#[test]
fn escape_or_letting_go_off_the_rail_moves_nothing() {
    let (mut app, _folder) = three_servers();
    let _ = arranged(&mut app);
    press(&mut app, "discord:20", 6.0, false);
    assert!(app.rail.dragging.is_some());
    let _ = app.update(Message::Rail(rail::Event::Cancel));
    assert!(app.rail.dragging.is_none());

    press(&mut app, "discord:20", 6.0, false);
    let away = Point::new(rail::WIDTH + 200.0, 100.0);
    let _ = app.update(Message::Rail(rail::Event::Carry(away)));
    let _ = app.update(Message::Rail(rail::Event::Drop));
    assert!(app.rail.dragging.is_none());
    assert!(arranged(&mut app).is_empty(), "nothing moved");
}

#[test]
fn a_server_pressed_and_let_go_without_moving_opens() {
    let (mut app, _folder) = three_servers();
    let _ = arranged(&mut app);
    press(&mut app, "discord:20", 2.0, true);
    assert!(app.rail.dragging.is_none());
    assert!(arranged(&mut app).is_empty());
    let selected = app.client.state().selected.clone().unwrap_or_default();
    assert!(
        selected.starts_with("discord:20"),
        "Bravo opened: {selected:?}"
    );
}
