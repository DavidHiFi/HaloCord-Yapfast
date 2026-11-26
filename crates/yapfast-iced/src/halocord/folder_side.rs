//! Server sidebar: an open folder's servers in a column of their own beside
//! the server list, as TestCord's Better folders' sidebar.

use std::collections::BTreeSet;

use yapfast_core::application::AppState;

use super::{Kept, Kind, Plugin, Setting, id, off};
use crate::rail::{Kind as Tile, RailItem};

const KEEP_ICONS: usize = 0;
const CLOSE_OTHERS: usize = 1;
const CLOSE_ON_PICK: usize = 2;
const CLOSE_ON_HOME: usize = 3;
const FOLDER_ICON: usize = 4;

pub(super) const SERVER_SIDEBAR: Plugin = Plugin {
    id: id::SERVER_SIDEBAR,
    name: "Server sidebar",
    about: "An open folder's servers show in a column of their own beside the server list, instead of under the folder.",
    on_at_first: false,
    settings: &[
        off(
            "keep_icons",
            "Keep them in the server list too",
            "The open folder's servers also stay under it in the server list.",
        ),
        Setting {
            key: "close_others",
            heading: "One folder open at a time",
            note: "Opening a folder closes the others.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
        off(
            "close_on_pick",
            "Close the folder when you pick a server",
            "Picking a server in the sidebar closes its folder.",
        ),
        off(
            "close_on_home",
            "Close folders on Direct messages",
            "Going to Direct messages closes every open folder.",
        ),
        Setting {
            key: "folder_icon",
            heading: "Show the folder above its servers",
            note: "The folder's own tile tops the sidebar; pressing it closes the folder.",
            kind: Kind::Switch {
                unless_changed: true,
            },
        },
    ],
    apply: |_, _| {},
    hooks: None,
};

impl Kept {
    fn sidebar_flag(&self, setting: usize) -> bool {
        self.on(&SERVER_SIDEBAR) && self.flag(&SERVER_SIDEBAR, &SERVER_SIDEBAR.settings[setting])
    }

    /// The server sidebar's width beside the rail: none while no folder is
    /// open in it.
    pub fn folder_side_width(&self) -> f32 {
        if self.folder_side_open {
            crate::rail::WIDTH
        } else {
            0.0
        }
    }
}

/// Hook: the rail's items are made. While Server sidebar is on, takes the
/// open folders (their tile, if it shows, and their servers) out to the
/// sidebar's items, which it gives back; the folder stays in the list,
/// closed, unless its servers are kept there too.
pub fn server_sidebar(kept: &mut Kept, items: &mut Vec<RailItem>) -> Vec<RailItem> {
    let mut side = Vec::new();
    if kept.on(&SERVER_SIDEBAR) {
        let keep = kept.sidebar_flag(KEEP_ICONS);
        let icon = kept.sidebar_flag(FOLDER_ICON);
        let mut index = 0;
        while index < items.len() {
            match items[index].kind {
                Tile::Folder { open: true, .. } => {
                    if icon {
                        side.push(items[index].clone());
                    }
                    if let (false, Tile::Folder { open, .. }) = (keep, &mut items[index].kind) {
                        *open = false;
                    }
                    index += 1;
                }
                Tile::Nested { .. } => {
                    side.push(items[index].clone());
                    if keep {
                        index += 1;
                    } else {
                        items.remove(index);
                    }
                }
                _ => index += 1,
            }
        }
    }
    kept.folder_side_open = !side.is_empty();
    side
}

/// Hook: a folder on the rail was pressed. While Server sidebar keeps one
/// open at a time, the others close as it opens.
pub fn folder_pressed(kept: &Kept, open: &mut BTreeSet<String>, folder: &str) {
    if kept.sidebar_flag(CLOSE_OTHERS) && !open.contains(folder) {
        open.clear();
    }
}

/// Hook: a server on the rail was picked. Its folder closes while Server
/// sidebar says so.
pub fn server_picked(kept: &Kept, state: &AppState, open: &mut BTreeSet<String>, server: &str) {
    if !kept.sidebar_flag(CLOSE_ON_PICK) {
        return;
    }
    if let Some(folder) = state
        .servers
        .iter()
        .find(|one| one.id == server)
        .and_then(|one| one.folder.as_ref())
    {
        open.remove(&folder.id);
    }
}

/// Hook: Direct messages was picked on the rail. Every folder closes while
/// Server sidebar says so.
pub fn home_picked(kept: &Kept, open: &mut BTreeSet<String>) {
    if kept.sidebar_flag(CLOSE_ON_HOME) {
        open.clear();
    }
}
