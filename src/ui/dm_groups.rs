//! Fork: the DM list's groups, named and coloured, which this device keeps
//! in `layout.json`. A DM's menu moves it into one, a group's heading folds
//! it, and the heading's menu edits or deletes it. While the list is
//! searched, the servers whose names match show under the DMs, so a server
//! is found there too.

use super::*;

/// What a group's heading folds by, as a [`RoomRow`]'s category: this,
/// then the group's id.
pub(super) const GROUP: &str = "fork-group:";
/// The heading of the servers a search finds.
pub(super) const SERVERS: &str = "fork-servers";
/// The DM menu's row that opens the groups, and its submenu's rows.
pub(super) const MOVE: &str = "fork-move-group";
pub(super) const MOVE_TO: &str = "fork-move-to:";
pub(super) const UNGROUP: &str = "fork-ungroup";
pub(super) const NEW: &str = "fork-new-group";
/// A group heading's menu.
pub(super) const EDIT: &str = "fork-edit-group";
pub(super) const DELETE: &str = "fork-delete-group";

/// The colours a group may take, as the dialog offers them.
pub(super) const PALETTE: [u32; 8] = [
    0xf23f43, 0xf0b232, 0x23a55a, 0x00a8fc, 0x5865f2, 0xeb459f, 0x9b59b6, 0x95a5a6,
];

/// A group of DMs.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(super) struct DmGroup {
    pub(super) id: String,
    pub(super) name: String,
    /// An index into [`PALETTE`].
    pub(super) color: usize,
    /// Its DMs, by room id, in the order they joined it.
    pub(super) rooms: Vec<String>,
    pub(super) collapsed: bool,
}

/// What the group dialog is open for.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Editing {
    /// A new group, which the DM by this room id joins, if any.
    New(Option<String>),
    /// The group by this id.
    Group(String),
}

pub(super) fn colour(index: usize) -> slint::Color {
    let [_, red, green, blue] = PALETTE[index % PALETTE.len()].to_be_bytes();
    slint::Color::from_rgb_u8(red, green, blue)
}

/// The DM rows, those in a group under its heading first, each group in
/// order, then the rest as they came. While searching, groups don't show.
pub(super) fn arrange(rows: Vec<RoomRow>, groups: &[DmGroup], searching: bool) -> Vec<RoomRow> {
    if searching || groups.is_empty() {
        return rows;
    }
    let mut rest = rows;
    let mut arranged = Vec::with_capacity(rest.len() + groups.len());
    for group in groups {
        let mut members: Vec<RoomRow> = Vec::new();
        for room in &group.rooms {
            if let Some(at) = rest.iter().position(|row| row.id.as_str() == room) {
                members.push(rest.remove(at));
            }
        }
        let heading = group.name.to_uppercase();
        let category: SharedString = format!("{GROUP}{}", group.id).into();
        let tint = colour(group.color);
        if members.is_empty() || group.collapsed {
            // The heading alone, on a row that shows nothing else.
            arranged.push(RoomRow {
                id: category.clone(),
                heading: heading.into(),
                category,
                heading_tint: tint,
                collapsed: group.collapsed,
                hidden: true,
                direct: true,
                ..Default::default()
            });
            continue;
        }
        for (index, mut row) in members.into_iter().enumerate() {
            row.category = category.clone();
            row.heading_tint = tint;
            if index == 0 {
                row.heading = heading.clone().into();
            }
            arranged.push(row);
        }
    }
    if let Some(first) = rest.first_mut() {
        first.heading = "DIRECT MESSAGES".into();
    }
    arranged.extend(rest);
    arranged
}

/// The servers whose names hold `query`, as rows under the DMs a search
/// found; clicking one opens it.
pub(super) fn servers(state: &AppState, query: &str) -> Vec<RoomRow> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    state
        .servers
        .iter()
        .filter(|server| server.name.to_lowercase().contains(query))
        .enumerate()
        .map(|(index, server)| RoomRow {
            id: server.id.as_str().into(),
            name: server.name.as_str().into(),
            heading: if index == 0 { "SERVERS" } else { "" }.into(),
            category: SERVERS.into(),
            direct: true,
            initial: initials(&server.name),
            tint: tint(&server.id),
            picture_url: server.icon.as_deref().unwrap_or_default().into(),
            ..Default::default()
        })
        .collect()
}

/// The group a DM is in, by room id.
pub(super) fn of<'a>(groups: &'a [DmGroup], room: &str) -> Option<&'a DmGroup> {
    groups
        .iter()
        .find(|group| group.rooms.iter().any(|id| id == room))
}

/// Moves a DM into a group, out of any other; none takes it out of all.
pub(super) fn put(groups: &mut [DmGroup], room: &str, group: Option<&str>) {
    for each in groups.iter_mut() {
        each.rooms.retain(|id| id != room);
        if Some(each.id.as_str()) == group {
            each.rooms.push(room.to_owned());
        }
    }
}

/// A new group's id, unused among `groups`.
pub(super) fn new_id(groups: &[DmGroup]) -> String {
    (1..)
        .map(|n: u32| n.to_string())
        .find(|id| groups.iter().all(|group| &group.id != id))
        .unwrap_or_default()
}

/// The DM menu's "Move to group" row and its submenu, for the DM `room`.
pub(super) fn menu_rows(groups: &[DmGroup], room: &str) -> (MenuRow, Vec<MenuRow>) {
    let current = of(groups, room).map(|group| group.id.as_str());
    let mut submenu: Vec<MenuRow> = groups
        .iter()
        .map(|group| MenuRow {
            id: format!("{MOVE_TO}{}", group.id).into(),
            text: group.name.as_str().into(),
            checkbox: true,
            checked: current == Some(group.id.as_str()),
            ..Default::default()
        })
        .collect();
    if current.is_some() {
        submenu.push(MenuRow {
            id: UNGROUP.into(),
            text: "Remove from group".into(),
            separated: true,
            ..Default::default()
        });
    }
    submenu.push(MenuRow {
        id: NEW.into(),
        text: "New group...".into(),
        separated: !groups.is_empty() && current.is_none(),
        ..Default::default()
    });
    let row = MenuRow {
        id: MOVE.into(),
        text: "Move to group".into(),
        separated: true,
        ..Default::default()
    };
    (row, submenu)
}

/// A group heading's menu.
pub(super) fn heading_menu() -> Vec<MenuRow> {
    vec![
        MenuRow {
            id: EDIT.into(),
            text: "Edit group".into(),
            ..Default::default()
        },
        MenuRow {
            id: DELETE.into(),
            text: "Delete group".into(),
            danger: true,
            ..Default::default()
        },
    ]
}

/// Opens the group dialog for `editing`.
pub(super) fn open(adapter: &mut Adapter, window: &YapfastWindow, editing: Editing) {
    let (name, color) = match &editing {
        Editing::Group(id) => adapter
            .layout
            .dm_groups
            .iter()
            .find(|group| &group.id == id)
            .map_or((String::new(), 0), |group| {
                (group.name.clone(), group.color)
            }),
        Editing::New(_) => (
            String::new(),
            adapter.layout.dm_groups.len() % PALETTE.len(),
        ),
    };
    window.set_group_dialog_new(matches!(editing, Editing::New(_)));
    window.set_group_dialog_name(name.into());
    window.set_group_dialog_color(color as i32);
    window.set_group_dialog_colors(ModelRc::new(VecModel::from(
        (0..PALETTE.len()).map(colour).collect::<Vec<_>>(),
    )));
    window.set_group_dialog_open(true);
    adapter.group_editing = Some(editing);
}

/// The dialog was saved with this name and colour.
pub(super) fn save(adapter: &mut Adapter, window: &YapfastWindow, name: &str, color: usize) {
    window.set_group_dialog_open(false);
    let name = name.trim();
    let Some(editing) = adapter.group_editing.take() else {
        return;
    };
    if name.is_empty() {
        return;
    }
    let color = color % PALETTE.len();
    let groups = &mut adapter.layout.dm_groups;
    match editing {
        Editing::New(room) => {
            let id = new_id(groups);
            groups.push(DmGroup {
                id: id.clone(),
                name: name.to_owned(),
                color,
                rooms: Vec::new(),
                collapsed: false,
            });
            if let Some(room) = room {
                put(groups, &room, Some(&id));
            }
        }
        Editing::Group(id) => {
            if let Some(group) = groups.iter_mut().find(|group| group.id == id) {
                group.name = name.to_owned();
                group.color = color;
            }
        }
    }
    changed(adapter, window);
}

/// Stores the groups and shows the list again.
pub(super) fn changed(adapter: &mut Adapter, window: &YapfastWindow) {
    adapter.layout.write(&adapter.root);
    // What the sidebar was built from leaves the layout out.
    adapter.sidebar = None;
    adapter.show_sidebar(window);
}

/// Carries out a choice from a DM's menu or a group heading's; whether it
/// was one of these.
pub(super) fn choose(
    adapter: &mut Adapter,
    window: &YapfastWindow,
    target: &str,
    id: &str,
) -> bool {
    if let Some(group) = target.strip_prefix(GROUP) {
        match id {
            EDIT => open(adapter, window, Editing::Group(group.to_owned())),
            DELETE => {
                adapter.layout.dm_groups.retain(|each| each.id != group);
                changed(adapter, window);
            }
            _ => return false,
        }
        return true;
    }
    if let Some(group) = id.strip_prefix(MOVE_TO) {
        put(&mut adapter.layout.dm_groups, target, Some(group));
    } else if id == UNGROUP {
        put(&mut adapter.layout.dm_groups, target, None);
    } else if id == NEW {
        open(adapter, window, Editing::New(Some(target.to_owned())));
        return true;
    } else {
        return id == MOVE;
    }
    changed(adapter, window);
    true
}

/// A group heading was clicked: fold it, or open it again.
pub(super) fn toggle(adapter: &mut Adapter, window: &YapfastWindow, category: &str) -> bool {
    let Some(id) = category.strip_prefix(GROUP) else {
        return category == SERVERS;
    };
    if let Some(group) = adapter
        .layout
        .dm_groups
        .iter_mut()
        .find(|group| group.id == id)
    {
        group.collapsed = !group.collapsed;
    }
    changed(adapter, window);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str) -> RoomRow {
        RoomRow {
            id: id.into(),
            direct: true,
            ..Default::default()
        }
    }

    #[test]
    fn groups_come_first_under_their_headings() {
        let groups = vec![
            DmGroup {
                id: "1".into(),
                name: "Pals".into(),
                rooms: vec!["c".into(), "a".into()],
                ..Default::default()
            },
            DmGroup {
                id: "2".into(),
                name: "Work".into(),
                collapsed: true,
                rooms: vec!["b".into()],
                ..Default::default()
            },
        ];
        let rows = arrange(vec![row("a"), row("b"), row("c"), row("d")], &groups, false);
        let shown: Vec<_> = rows
            .iter()
            .map(|row| (row.id.to_string(), row.heading.to_string(), row.hidden))
            .collect();
        assert_eq!(
            shown,
            [
                ("c".into(), "PALS".into(), false),
                ("a".into(), String::new(), false),
                ("fork-group:2".into(), "WORK".into(), true),
                ("d".into(), "DIRECT MESSAGES".into(), false),
            ]
        );
        // A search shows the DMs as they came.
        assert_eq!(arrange(vec![row("a")], &groups, true).len(), 1);
    }

    #[test]
    fn a_dm_sits_in_one_group_at_most() {
        let mut groups = vec![
            DmGroup {
                id: "1".into(),
                ..Default::default()
            },
            DmGroup {
                id: "2".into(),
                ..Default::default()
            },
        ];
        put(&mut groups, "a", Some("1"));
        put(&mut groups, "a", Some("2"));
        assert_eq!(of(&groups, "a").map(|group| group.id.as_str()), Some("2"));
        put(&mut groups, "a", None);
        assert!(of(&groups, "a").is_none());
        assert_eq!(new_id(&groups), "3");
    }
}
