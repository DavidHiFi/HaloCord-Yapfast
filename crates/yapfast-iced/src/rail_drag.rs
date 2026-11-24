//! Dragging a Discord server or folder on the rail to move it, as in
//! Discord's server list. A tile pressed and moved 4 px is lifted; a faint
//! copy follows the pointer, a line marks where it would land, a ring the
//! server it would make a folder with or the folder it would go into, and
//! the rail scrolls near its ends. Let go on the rail, the list changes at
//! once and everywhere this account is signed in
//! (`Action::ArrangeServers`); Escape, or let go away from the rail, and
//! nothing moves.
//!
//! Over a server, its top and bottom thirds are before and after it, its
//! middle a new folder of the two; over a closed folder, its middle puts
//! the server in at its end; in an open folder, between its servers; past
//! the last one's bottom quarter, out of it. A folder goes only between
//! entries. A folder left with one server stops being one, as Discord's.

use std::time::Duration;

use iced::{
    Point, Rectangle, Subscription, Task,
    widget::{operation, scrollable::AbsoluteOffset},
};
use yapfast_core::{
    application::Action,
    chat::{ChatServer, ServerFolder, ServerGroup},
    discord,
};

use crate::{
    app::{App, Message},
    rail::{Event, FOLDER, GAP, Kind, RailItem, TILE, WIDTH},
};

/// The rail's scroll area, which a drag near its ends scrolls.
pub const LIST: &str = "rail-list";
/// How near the list's ends the pointer scrolls it, how far each step, and
/// how often.
const EDGE: f32 = 40.0;
const STEP: f32 = 8.0;
const EVERY: Duration = Duration::from_millis(16);
/// Let go this far right of the rail's edge, nothing moves.
const AWAY: f32 = 48.0;
/// The start of a Discord folder's id.
const FOLDER_ID: &str = "discord-folder:";

/// A server or folder being dragged.
#[derive(Clone, Debug)]
pub struct RailDrag {
    /// The dragged tile's id, and whether it's a folder.
    pub id: String,
    pub folder: bool,
    /// How far down the tile it was pressed.
    pub grab: f32,
    /// The rail's scroll area's top in the window.
    pub list_top: f32,
    /// The pointer, in the window.
    pub at: Point,
}

/// Where a drop would land, in the server list's entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    /// Between entries, before this one (the count for after the last).
    Between(usize),
    /// In this open folder, before this server of it.
    InFolder(usize, usize),
    /// Onto this server outside any folder: a new folder of the two.
    Join(usize),
    /// Into this closed folder, at its end.
    Into(usize),
}

/// What shows where it would land, in the rail's content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Marker {
    /// A line, its middle this far down.
    Line(f32),
    /// A ring around this tile.
    Ring(Rectangle),
}

/// What a rail row is in the server list.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    /// An unread DM or the line under them: before the first entry.
    Top,
    /// A server outside any folder, the list's entry so numbered.
    Lone(usize),
    Folder {
        group: usize,
        open: bool,
    },
    /// An open folder's server, its place in it.
    Nested {
        group: usize,
        at: usize,
        last: bool,
    },
    /// The add-a-server tile: after the last entry.
    End,
}

/// Each row's place, top and height in the rail's content, and how many
/// entries the list has.
fn places(items: &[RailItem]) -> (Vec<(Place, f32, f32)>, usize) {
    let (mut rows, mut groups, mut nested, mut top) = (Vec::new(), 0, 0, 0.0);
    for (index, item) in items.iter().enumerate() {
        let place = match &item.kind {
            Kind::Dm | Kind::Divider => Place::Top,
            Kind::Server => {
                groups += 1;
                Place::Lone(groups - 1)
            }
            Kind::Folder { open, .. } => {
                groups += 1;
                nested = 0;
                Place::Folder {
                    group: groups - 1,
                    open: *open,
                }
            }
            Kind::Nested { last, .. } => {
                nested += 1;
                Place::Nested {
                    group: groups.saturating_sub(1),
                    at: nested - 1,
                    last: *last,
                }
            }
            Kind::Add => Place::End,
        };
        let height = item.height(index);
        rows.push((place, top, height));
        top += height;
    }
    (rows, groups)
}

/// Where the drag would land with the pointer `y` px down the rail's
/// content, and what marks it; none while the rail has no entries.
pub fn spot(items: &[RailItem], drag: &RailDrag, y: f32) -> Option<(Spot, Marker)> {
    let (rows, groups) = places(items);
    if groups == 0 {
        return None;
    }
    let tile = |top: f32, side: f32| Rectangle {
        x: (WIDTH - side) / 2.0,
        y: top,
        width: side,
        height: side,
    };
    // The middle of the gap after an entry's last row.
    let after = |group: usize| {
        rows.iter()
            .rev()
            .find(|(place, ..)| match place {
                Place::Lone(at)
                | Place::Folder { group: at, .. }
                | Place::Nested { group: at, .. } => *at == group,
                _ => false,
            })
            .map_or(0.0, |(_, top, height)| top + height - GAP / 2.0)
    };
    // The first entry's row.
    let first = rows
        .iter()
        .find(|(place, ..)| !matches!(place, Place::Top))
        .map_or(0.0, |(_, top, _)| *top);
    let start = (Spot::Between(0), Marker::Line(first - GAP / 2.0));
    let end = (Spot::Between(groups), Marker::Line(after(groups - 1)));
    let Some(index) = rows
        .iter()
        .position(|(_, top, height)| y >= *top && y < top + height)
    else {
        return Some(if y < first { start } else { end });
    };
    let (place, top, height) = rows[index];
    let item = &items[index];
    let share = ((y - top) / height).clamp(0.0, 1.0);
    let server = !drag.folder;
    let middle = server && (0.3..=0.7).contains(&share);
    let beside = |group: usize| {
        if share < 0.5 {
            (Spot::Between(group), Marker::Line(top - GAP / 2.0))
        } else {
            (Spot::Between(group + 1), Marker::Line(after(group)))
        }
    };
    Some(match place {
        Place::Top => start,
        Place::End => end,
        Place::Lone(group) => {
            if middle && item.id != drag.id && discord::parse_server(&item.id).is_some() {
                (Spot::Join(group), Marker::Ring(tile(top, TILE)))
            } else {
                beside(group)
            }
        }
        Place::Folder { group, open: false } => {
            if middle {
                (Spot::Into(group), Marker::Ring(tile(top, FOLDER)))
            } else {
                beside(group)
            }
        }
        Place::Folder { group, open: true } => {
            if server && share >= 0.3 {
                (Spot::InFolder(group, 0), Marker::Line(top + height - 2.0))
            } else {
                beside(group)
            }
        }
        Place::Nested { group, .. } if drag.folder => {
            (Spot::Between(group + 1), Marker::Line(after(group)))
        }
        Place::Nested { group, last, .. } if last && share > 0.75 => {
            (Spot::Between(group + 1), Marker::Line(after(group)))
        }
        Place::Nested { group, at, last } => {
            if share < 0.5 {
                let gap = if at == 0 { 4.0 } else { GAP };
                (Spot::InFolder(group, at), Marker::Line(top - gap / 2.0))
            } else {
                let gap = if last { 4.0 } else { GAP };
                (
                    Spot::InFolder(group, at + 1),
                    Marker::Line(top + TILE + gap / 2.0),
                )
            }
        }
    })
}

/// The server list's entries from the core's servers, as the rail groups
/// them: a run of servers in the same folder, or one outside any.
pub fn groups<S: std::borrow::Borrow<ChatServer>>(servers: &[S]) -> Vec<ServerGroup> {
    let mut groups: Vec<ServerGroup> = Vec::new();
    for server in servers.iter().map(std::borrow::Borrow::borrow) {
        match groups.last_mut() {
            Some(last) if server.folder.is_some() && last.folder == server.folder => {
                last.servers.push(server.id.clone());
            }
            _ => groups.push(ServerGroup {
                folder: server.folder.clone(),
                servers: vec![server.id.clone()],
            }),
        }
    }
    groups
}

/// The list with the server or folder `id` moved to `spot`; none when it
/// would stay as it is.
pub fn moved(
    groups: &[ServerGroup],
    id: &str,
    folder: bool,
    spot: Spot,
) -> Option<Vec<ServerGroup>> {
    let mut next = groups.to_vec();
    // Where it was is left empty, so the spot's numbers still hold, and
    // cleared once it's in its new place.
    let origin;
    if folder {
        let from = next
            .iter()
            .position(|group| group.folder.as_ref().is_some_and(|folder| folder.id == id))?;
        let Spot::Between(to) = spot else {
            return None;
        };
        let group = std::mem::replace(
            &mut next[from],
            ServerGroup {
                folder: None,
                servers: Vec::new(),
            },
        );
        next.insert(to.min(next.len()), group);
        origin = None;
    } else {
        let (from, at) = next.iter().enumerate().find_map(|(index, group)| {
            group
                .servers
                .iter()
                .position(|server| server == id)
                .map(|at| (index, at))
        })?;
        origin = next[from].folder.clone();
        next[from].servers[at].clear();
        let id = id.to_owned();
        match spot {
            Spot::Between(to) => next.insert(
                to.min(next.len()),
                ServerGroup {
                    folder: None,
                    servers: vec![id],
                },
            ),
            Spot::InFolder(group, before) => {
                let servers = &mut next.get_mut(group)?.servers;
                servers.insert(before.min(servers.len()), id);
            }
            Spot::Join(group) => {
                let target = next.get_mut(group)?;
                if target.folder.is_some() || target.servers.iter().all(String::is_empty) {
                    return None;
                }
                target.folder = Some(ServerFolder {
                    id: String::new(),
                    name: None,
                    color: None,
                });
                target.servers.push(id);
            }
            Spot::Into(group) => next.get_mut(group)?.servers.push(id),
        }
    }
    for group in &mut next {
        group.servers.retain(|server| !server.is_empty());
    }
    next.retain(|group| !group.servers.is_empty());
    // The folder it came out of, left with one server, is no folder.
    if let Some(origin) = origin
        && let Some(left) = next
            .iter_mut()
            .find(|group| group.folder.as_ref() == Some(&origin))
        && left.servers.len() == 1
    {
        left.folder = None;
    }
    (next != groups).then_some(next)
}

/// Only Discord's servers, which its server list holds.
fn discord_only(groups: Vec<ServerGroup>) -> Vec<ServerGroup> {
    groups
        .into_iter()
        .filter_map(|mut group| {
            group
                .servers
                .retain(|id| discord::parse_server(id).is_some());
            (!group.servers.is_empty()).then_some(group)
        })
        .collect()
}

/// Whether the rail's tile `item` can be dragged: a Discord server or
/// folder.
pub fn liftable(item: &RailItem) -> bool {
    match item.kind {
        Kind::Server | Kind::Nested { .. } => discord::parse_server(&item.id).is_some(),
        Kind::Folder { .. } => item.id.starts_with(FOLDER_ID),
        _ => false,
    }
}

/// The pointer's height in the rail's content.
pub fn content_y(app: &App, drag: &RailDrag) -> f32 {
    drag.at.y - drag.list_top + app.rail.scroll.offset
}

/// The tile `id` was pressed and moved, `grab` px down it: lifts it.
pub fn lift(app: &mut App, id: String, grab: f32) -> Task<Message> {
    if app.rail.dragging.is_some() {
        return Task::none();
    }
    let Some(index) = app.rail.items.iter().position(|item| item.id == id) else {
        return Task::none();
    };
    let item = &app.rail.items[index];
    if !liftable(item) {
        return Task::none();
    }
    let top: f32 = app.rail.items[..index]
        .iter()
        .enumerate()
        .map(|(index, item)| item.height(index))
        .sum();
    let pressed = app.pressed.get();
    let folder = matches!(item.kind, Kind::Folder { .. });
    app.rail.dragging = Some(RailDrag {
        id,
        folder,
        grab,
        list_top: pressed.y - grab - (top - app.rail.scroll.offset),
        at: pressed,
    });
    Task::none()
}

/// The pointer moved, in the window.
pub fn carry(app: &mut App, at: Point) -> Task<Message> {
    if let Some(drag) = &mut app.rail.dragging {
        drag.at = at;
    }
    Task::none()
}

/// A step of scrolling, while the pointer is near an end of the rail.
pub fn creep(app: &App) -> Task<Message> {
    let Some(drag) = &app.rail.dragging else {
        return Task::none();
    };
    let by = if drag.at.y < drag.list_top + EDGE {
        -STEP
    } else if drag.at.y > drag.list_top + app.rail.scroll.height - EDGE {
        STEP
    } else {
        return Task::none();
    };
    operation::scroll_by(LIST, AbsoluteOffset { x: 0.0, y: by })
}

/// Let go: moves what's dragged where it would land, if that's on the
/// rail and changes the list.
pub fn let_go(app: &mut App) -> Task<Message> {
    let Some(drag) = app.rail.dragging.take() else {
        return Task::none();
    };
    if drag.at.x > WIDTH + AWAY {
        return Task::none();
    }
    let y = content_y(app, &drag);
    let Some((spot, _)) = spot(&app.rail.items, &drag, y) else {
        return Task::none();
    };
    let state = app.client.state();
    let Some(next) = moved(&groups(&state.servers), &drag.id, drag.folder, spot) else {
        return Task::none();
    };
    app.client
        .dispatch(Action::ArrangeServers(discord_only(next)));
    app.sync()
}

/// The pointer's moves, the button's release and Escape while a tile is
/// dragged, and the rail's scrolling while it's near an end.
pub fn subscription(app: &App) -> Option<Subscription<Message>> {
    let drag = app.rail.dragging.as_ref()?;
    let follow = iced::event::listen_with(|event, _, _| match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::Rail(Event::Carry(position)))
        }
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(Message::Rail(Event::Drop))
        }
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            ..
        }) => Some(Message::Rail(Event::Cancel)),
        _ => None,
    });
    if drag.at.y < drag.list_top + EDGE || drag.at.y > drag.list_top + app.rail.scroll.height - EDGE
    {
        let scroll = iced::time::every(EVERY).map(|_| Message::Rail(Event::Creep));
        return Some(Subscription::batch([follow, scroll]));
    }
    Some(follow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lone(id: &str) -> ServerGroup {
        ServerGroup {
            folder: None,
            servers: vec![id.into()],
        }
    }

    fn folder(id: &str, servers: &[&str]) -> ServerGroup {
        ServerGroup {
            folder: Some(ServerFolder {
                id: id.into(),
                name: None,
                color: None,
            }),
            servers: servers.iter().map(|&id| id.into()).collect(),
        }
    }

    fn list() -> Vec<ServerGroup> {
        vec![
            lone("discord:1"),
            folder("discord-folder:9", &["discord:2", "discord:3"]),
            lone("discord:4"),
        ]
    }

    #[test]
    fn a_server_moves_between_entries() {
        let next = moved(&list(), "discord:4", false, Spot::Between(0)).unwrap();
        assert_eq!(next[0], lone("discord:4"));
        assert_eq!(next[1], lone("discord:1"));
        assert_eq!(next.len(), 3);
    }

    #[test]
    fn dropping_where_it_was_changes_nothing() {
        assert_eq!(moved(&list(), "discord:1", false, Spot::Between(0)), None);
        assert_eq!(moved(&list(), "discord:1", false, Spot::Between(1)), None);
        assert_eq!(
            moved(&list(), "discord:2", false, Spot::InFolder(1, 0)),
            None
        );
        assert_eq!(
            moved(&list(), "discord-folder:9", true, Spot::Between(2)),
            None
        );
    }

    #[test]
    fn a_server_joins_another_as_a_new_folder() {
        let next = moved(&list(), "discord:4", false, Spot::Join(0)).unwrap();
        assert_eq!(next.len(), 2);
        let made = next[0].folder.as_ref().unwrap();
        assert!(made.id.is_empty());
        assert_eq!(next[0].servers, ["discord:1", "discord:4"]);
    }

    #[test]
    fn a_folder_left_with_one_server_stops_being_one() {
        let next = moved(&list(), "discord:3", false, Spot::Between(3)).unwrap();
        assert_eq!(
            next,
            [
                lone("discord:1"),
                lone("discord:2"),
                lone("discord:4"),
                lone("discord:3"),
            ]
        );
    }

    #[test]
    fn a_server_goes_into_a_folder_and_moves_in_it() {
        let next = moved(&list(), "discord:1", false, Spot::Into(1)).unwrap();
        assert_eq!(next[0].servers, ["discord:2", "discord:3", "discord:1"]);
        let next = moved(&list(), "discord:3", false, Spot::InFolder(1, 0)).unwrap();
        assert_eq!(next[1].servers, ["discord:3", "discord:2"]);
    }

    #[test]
    fn a_folder_moves_whole_and_only_between_entries() {
        let next = moved(&list(), "discord-folder:9", true, Spot::Between(0)).unwrap();
        assert_eq!(
            next[0],
            folder("discord-folder:9", &["discord:2", "discord:3"])
        );
        assert_eq!(
            moved(&list(), "discord-folder:9", true, Spot::Join(0)),
            None
        );
    }

    #[test]
    fn only_discord_servers_reach_the_list() {
        let groups = vec![lone("matrix:!a"), lone("discord:1")];
        assert_eq!(discord_only(groups), [lone("discord:1")]);
    }

    fn item(id: &str, kind: Kind) -> RailItem {
        RailItem {
            id: id.into(),
            name: id.into(),
            icon: None,
            kind,
            selected: false,
            unread: false,
            badge: String::new(),
            call: false,
            media: None,
            busy: Default::default(),
            unavailable: false,
        }
    }

    fn drag(id: &str, folder: bool) -> RailDrag {
        RailDrag {
            id: id.into(),
            folder,
            grab: 0.0,
            list_top: 0.0,
            at: Point::ORIGIN,
        }
    }

    #[test]
    fn the_pointer_picks_before_after_or_a_new_folder() {
        // The line has no row with no DM unread; each server's is 48 px.
        let items = vec![
            item("divider", Kind::Divider),
            item("discord:1", Kind::Server),
            item("discord:4", Kind::Server),
            item("+", Kind::Add),
        ];
        let dragged = drag("discord:4", false);
        assert_eq!(spot(&items, &dragged, 2.0).unwrap().0, Spot::Between(0));
        assert_eq!(spot(&items, &dragged, 24.0).unwrap().0, Spot::Join(0));
        assert_eq!(spot(&items, &dragged, 45.0).unwrap().0, Spot::Between(1));
        // On itself, its middle is no folder.
        assert_eq!(spot(&items, &dragged, 66.0).unwrap().0, Spot::Between(1));
        assert_eq!(spot(&items, &dragged, 500.0).unwrap().0, Spot::Between(2));
        // A folder dragged goes only between.
        assert_eq!(
            spot(&items, &drag("discord-folder:9", true), 20.0)
                .unwrap()
                .0,
            Spot::Between(0)
        );
    }

    #[test]
    fn in_an_open_folder_between_its_servers_and_out_past_the_last() {
        let tint = 0;
        let items = vec![
            item("divider", Kind::Divider),
            item(
                "discord-folder:9",
                Kind::Folder {
                    open: true,
                    tint,
                    preview: vec![],
                },
            ),
            item("discord:2", Kind::Nested { tint, last: false }),
            item("discord:3", Kind::Nested { tint, last: true }),
        ];
        let dragged = drag("discord:1", false);
        // The folder's row is 0 to 52, its servers' 52 to 100 and 100 to 152.
        assert_eq!(
            spot(&items, &dragged, 30.0).unwrap().0,
            Spot::InFolder(0, 0)
        );
        assert_eq!(
            spot(&items, &dragged, 90.0).unwrap().0,
            Spot::InFolder(0, 1)
        );
        assert_eq!(
            spot(&items, &dragged, 130.0).unwrap().0,
            Spot::InFolder(0, 2)
        );
        assert_eq!(spot(&items, &dragged, 150.0).unwrap().0, Spot::Between(1));
    }

    #[test]
    fn only_discord_tiles_lift() {
        assert!(liftable(&item("discord:1", Kind::Server)));
        assert!(!liftable(&item("matrix:!a", Kind::Server)));
        assert!(!liftable(&item("discord-dm:5", Kind::Dm)));
        assert!(!liftable(&item("+", Kind::Add)));
    }
}
