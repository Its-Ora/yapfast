//! Fork: the DM list's groups, named and coloured, which `layout.json` keeps
//! under `dm_groups` (the same key and shape the fork's Slint window used,
//! so groups made there stay). A DM's menu moves it into a group or makes a
//! new one; a group's heading folds it, and its menu renames it in place,
//! changes its colour or deletes it. Grouped DMs come first, each group in
//! order, then the rest under "Other messages". While the list is searched,
//! groups don't show, and the servers whose names match come after the DMs,
//! so a server is found there too.

use iced::{
    Alignment, Element, Length, Padding, Task,
    widget::{Space, container, mouse_area, row, text, text_input},
};

use crate::{
    app::{App, Message},
    fonts::SEMIBOLD,
    menus::{MenuAct, MenuRow},
    pictures::Icon,
    sidebar::{Event as Side, Room, SideRow},
    widgets::RightPress,
};

use super::Act;

/// What a group's heading folds by, as its row's key: this, then its id.
pub const GROUP: &str = "fork-group:";

/// The colours a group may take, and their names in its menu.
pub const PALETTE: [(u32, &str); 8] = [
    (0xf23f43, "Red"),
    (0xf0b232, "Yellow"),
    (0x23a55a, "Green"),
    (0x00a8fc, "Blue"),
    (0x5865f2, "Blurple"),
    (0xeb459f, "Pink"),
    (0x9b59b6, "Purple"),
    (0x95a5a6, "Grey"),
];

/// A group of DMs.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DmGroup {
    pub id: String,
    pub name: String,
    /// An index into [`PALETTE`].
    pub color: usize,
    /// Its DMs, by room id, in the order they joined it.
    pub rooms: Vec<String>,
    pub collapsed: bool,
}

pub fn tint(color: usize) -> u32 {
    PALETTE[color % PALETTE.len()].0
}

/// The DM list's rows with the groups: those in a group under its heading
/// first, then the rest. While `filter` holds text, the rows as they came
/// and the servers whose names hold it after them. `renaming` is the
/// group renamed in its heading and the name typed.
pub fn arrange(
    rows: Vec<SideRow>,
    state: &yapfast_core::application::AppState,
    groups: &[DmGroup],
    filter: &str,
    renaming: Option<&(String, String)>,
) -> Vec<SideRow> {
    if state.navigation != yapfast_core::application::Navigation::DirectMessages {
        return rows;
    }
    let query = filter.trim().to_lowercase();
    if !query.is_empty() {
        return with_servers(rows, state, &query);
    }
    if groups.is_empty() {
        return rows;
    }
    let (mut top, mut rest): (Vec<SideRow>, Vec<Box<Room>>) = (Vec::new(), Vec::new());
    for row in rows {
        match row {
            SideRow::Room(room) if room.direct => rest.push(room),
            SideRow::Empty(_) => {}
            other => top.push(other),
        }
    }
    for group in groups {
        let mut members = Vec::new();
        for id in &group.rooms {
            if let Some(at) = rest.iter().position(|room| &room.id == id) {
                members.push(rest.remove(at));
            }
        }
        top.push(SideRow::ForkGroup {
            id: group.id.clone(),
            name: group.name.clone(),
            tint: tint(group.color),
            collapsed: group.collapsed,
            count: members.len(),
            editing: renaming
                .filter(|(id, _)| *id == group.id)
                .map(|(_, typed)| typed.clone()),
        });
        if !group.collapsed {
            top.extend(members.into_iter().map(SideRow::Room));
        }
    }
    if !rest.is_empty() {
        top.push(SideRow::ForkOthers);
        top.extend(rest.into_iter().map(SideRow::Room));
    }
    top
}

/// The DMs a search found, then the servers whose names hold `query`.
fn with_servers(
    mut rows: Vec<SideRow>,
    state: &yapfast_core::application::AppState,
    query: &str,
) -> Vec<SideRow> {
    let servers: Vec<SideRow> = state
        .servers
        .iter()
        .filter(|server| server.name.to_lowercase().contains(query))
        .map(|server| SideRow::ForkServer {
            id: server.id.clone(),
            name: server.name.clone(),
            icon: server.icon.clone(),
        })
        .collect();
    if servers.is_empty() {
        return rows;
    }
    rows.retain(|row| !matches!(row, SideRow::Empty(_)));
    rows.push(SideRow::ForkServersHeading);
    rows.extend(servers);
    rows
}

/// The group a DM is in, by room id.
fn of<'a>(groups: &'a [DmGroup], room: &str) -> Option<&'a DmGroup> {
    groups
        .iter()
        .find(|group| group.rooms.iter().any(|id| id == room))
}

/// Moves a DM into a group, out of any other; none takes it out of all.
fn put(groups: &mut [DmGroup], room: &str, group: Option<&str>) {
    for each in groups.iter_mut() {
        each.rooms.retain(|id| id != room);
        if Some(each.id.as_str()) == group {
            each.rooms.push(room.to_owned());
        }
    }
}

/// A new group's id, unused among `groups`.
fn new_id(groups: &[DmGroup]) -> String {
    (1..)
        .map(|n: u32| n.to_string())
        .find(|id| groups.iter().all(|group| &group.id != id))
        .unwrap_or_default()
}

/// Adds the groups' rows to a menu: Move to group on a DM's, and a group
/// heading's own rows on its.
pub fn menu_rows(app: &App, id: &str, rows: &mut Vec<MenuRow>) {
    let groups = &app.layout.dm_groups;
    if let Some(group) = id.strip_prefix(GROUP) {
        let current = groups.iter().find(|each| each.id == group);
        let colours = PALETTE
            .iter()
            .enumerate()
            .map(|(index, (_, name))| MenuRow {
                check: if current.is_some_and(|group| group.color % PALETTE.len() == index) {
                    crate::controls::Check::Current
                } else {
                    crate::controls::Check::None
                },
                ..MenuRow::plain(MenuAct::Fork(Act::Recolor), *name).key(index.to_string())
            })
            .collect();
        rows.push(MenuRow::new(
            MenuAct::Fork(Act::EditGroup),
            "Rename group",
            Icon::Pencil,
        ));
        rows.push(MenuRow {
            submenu: colours,
            ..MenuRow::plain(MenuAct::Event, "Colour")
        });
        rows.push(MenuRow {
            danger: true,
            separated: true,
            ..MenuRow::new(MenuAct::Fork(Act::DeleteGroup), "Delete group", Icon::Trash)
        });
        return;
    }
    let state = app.client.state();
    if !state.rooms.iter().any(|room| room.id == id && room.direct) {
        return;
    }
    let current = of(groups, id).map(|group| group.id.as_str());
    let mut submenu: Vec<MenuRow> = groups
        .iter()
        .map(|group| MenuRow {
            check: if current == Some(group.id.as_str()) {
                crate::controls::Check::Current
            } else {
                crate::controls::Check::None
            },
            ..MenuRow::plain(MenuAct::Fork(Act::MoveToGroup), group.name.clone())
                .key(group.id.clone())
        })
        .collect();
    if current.is_some() {
        submenu.push(MenuRow {
            separated: true,
            ..MenuRow::plain(MenuAct::Fork(Act::Ungroup), "Remove from group")
        });
    }
    submenu.push(MenuRow {
        separated: !groups.is_empty() && current.is_none(),
        ..MenuRow::new(MenuAct::Fork(Act::NewGroup), "New group", Icon::Add)
    });
    rows.push(MenuRow {
        separated: !rows.is_empty(),
        submenu,
        ..MenuRow::new(MenuAct::Event, "Move to group", Icon::Folder)
    });
}

/// Carries out a group row of the menu about `place`, with the row's `key`.
pub fn act(app: &mut App, place: &str, key: &str, act: Act) -> Task<Message> {
    let groups = &mut app.layout.dm_groups;
    match act {
        Act::MoveToGroup => put(groups, place, Some(key)),
        Act::Ungroup => put(groups, place, None),
        Act::NewGroup => {
            let id = new_id(groups);
            groups.push(DmGroup {
                id: id.clone(),
                name: format!("Group {}", groups.len() + 1),
                color: groups.len() % PALETTE.len(),
                rooms: Vec::new(),
                collapsed: false,
            });
            put(groups, place, Some(&id));
            // Its name is typed in its heading right away.
            let name = groups.last().map(|group| group.name.clone());
            app.fork.renaming = name.map(|name| (id, name));
        }
        Act::EditGroup => {
            let Some(id) = place.strip_prefix(GROUP) else {
                return Task::none();
            };
            let name = groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone());
            app.fork.renaming = name.map(|name| (id.to_owned(), name));
        }
        Act::Recolor => {
            let (Some(id), Ok(color)) = (place.strip_prefix(GROUP), key.parse::<usize>()) else {
                return Task::none();
            };
            if let Some(group) = groups.iter_mut().find(|group| group.id == id) {
                group.color = color % PALETTE.len();
            }
        }
        Act::DeleteGroup => {
            let Some(id) = place.strip_prefix(GROUP) else {
                return Task::none();
            };
            groups.retain(|group| group.id != id);
        }
        Act::Hide | Act::Unhide => return Task::none(),
    }
    changed(app)
}

/// The name typed in a heading is the group's now.
pub fn save_name(app: &mut App) -> Task<Message> {
    let Some((id, name)) = app.fork.renaming.take() else {
        return Task::none();
    };
    let name = name.trim();
    if !name.is_empty()
        && let Some(group) = app.layout.dm_groups.iter_mut().find(|group| group.id == id)
    {
        name.clone_into(&mut group.name);
    }
    changed(app)
}

/// A group's heading was clicked: it folds, or opens again. Whether `fold`
/// was a group's.
pub fn toggle(app: &mut App, fold: &str) -> bool {
    let Some(id) = fold.strip_prefix(GROUP) else {
        return false;
    };
    if let Some(group) = app.layout.dm_groups.iter_mut().find(|group| group.id == id) {
        group.collapsed = !group.collapsed;
    }
    crate::layout::store(app.client.root(), "dm_groups", &app.layout.dm_groups);
    app.lists_shown = None;
    true
}

/// Keeps the groups and lays the list out again.
fn changed(app: &mut App) -> Task<Message> {
    crate::layout::store(app.client.root(), "dm_groups", &app.layout.dm_groups);
    app.lists_shown = None;
    let focus = if app.fork.renaming.is_some() {
        iced::widget::operation::focus(RENAME)
    } else {
        Task::none()
    };
    Task::batch([app.sync(), focus])
}

/// The heading's field while a group is renamed.
const RENAME: &str = "fork-group-rename";

/// A group's heading: a dot and its name in its colour, how many DMs it
/// holds, and its chevron; while renamed, a field instead of the name.
#[allow(clippy::too_many_arguments, reason = "a heading has that many parts")]
pub fn heading<'a>(
    cx: crate::ctx::Ctx<'a>,
    id: &'a str,
    name: &'a str,
    tint: u32,
    collapsed: bool,
    count: usize,
    editing: Option<&'a str>,
) -> Element<'a, Message> {
    let theme = cx.look.theme;
    let fold = format!("{GROUP}{id}");
    let pointed = cx.pointed.side.as_deref() == Some(fold.as_str());
    let ink = crate::style::hex(tint);
    let ink = if pointed {
        crate::theme::mix(ink, theme.text, 0.75)
    } else {
        ink
    };
    let dot = container(Space::new())
        .width(8)
        .height(8)
        .style(move |_| crate::style::rounded(ink, 4));
    let label: Element<'a, Message> = match editing {
        Some(typed) => text_input("Group name", typed)
            .id(RENAME)
            .size(13)
            .padding([2, 6])
            .on_input(|typed| Message::Fork(super::Event::GroupName(typed)))
            .on_submit(Message::Fork(super::Event::GroupSave))
            .width(Length::Fill)
            .into(),
        None => text(name.to_uppercase())
            .size(12)
            .font(SEMIBOLD)
            .color(ink)
            .wrapping(text::Wrapping::None)
            .into(),
    };
    let mut face = row![dot, label].spacing(6).align_y(Alignment::Center);
    if editing.is_none() {
        face = face.push(text(count.to_string()).size(12).color(theme.muted));
        face = face.push(Space::new().width(Length::Fill));
        face = face.push(cx.look.glyph(
            if collapsed {
                Icon::ChevronRight
            } else {
                Icon::ChevronDown
            },
            12,
            ink,
        ));
    }
    let area = container(face)
        .padding(Padding {
            left: 8.0,
            right: 8.0,
            top: 17.0,
            bottom: 0.0,
        })
        .height(crate::sidebar::HEADING_HEIGHT)
        .width(Length::Fill)
        .align_y(Alignment::Center);
    if editing.is_some() {
        return area.into();
    }
    let area = mouse_area(area)
        .interaction(iced::mouse::Interaction::Pointer)
        .on_enter(Message::Side(Side::Point(fold.clone(), true)))
        .on_exit(Message::Side(Side::Point(fold.clone(), false)))
        .on_press(Message::Side(Side::Toggle(fold.clone())));
    let spot = cx.pressed;
    RightPress::new(area, move |_| {
        Message::Side(Side::Menu(fold.clone(), spot.get()))
    })
    .into()
}

/// The heading over the DMs in no group, and over the servers a search
/// found.
pub fn label<'a>(cx: crate::ctx::Ctx<'a>, words: &'static str) -> Element<'a, Message> {
    container(
        text(words)
            .size(12)
            .font(SEMIBOLD)
            .color(cx.look.theme.muted),
    )
    .padding(Padding {
        left: 8.0,
        top: 17.0,
        ..Padding::ZERO
    })
    .height(crate::sidebar::HEADING_HEIGHT)
    .align_y(Alignment::Center)
    .into()
}

/// A server the DM search found: its icon and name; a click opens it, a
/// right click its menu.
pub fn server_row<'a>(
    cx: crate::ctx::Ctx<'a>,
    id: &'a str,
    name: &'a str,
    icon: Option<&'a str>,
) -> Element<'a, Message> {
    let theme = cx.look.theme;
    let key = format!("fork-server:{id}");
    let pointed = cx.pointed.side.as_deref() == Some(key.as_str());
    let ground = if pointed { theme.hover } else { theme.sidebar };
    let face: Element<'a, Message> = match icon.and_then(|url| cx.picture(url, 32, 32, true)) {
        Some(picture) => crate::widgets::Pic::square(picture, 32.0)
            .radius(10.0)
            .into(),
        None => container(
            text(crate::rail::server_initials(name))
                .size(12)
                .font(SEMIBOLD)
                .color(theme.text),
        )
        .center(32)
        .style({
            let raised = theme.raised;
            move |_| crate::style::rounded(raised, 10)
        })
        .into(),
    };
    let body = container(
        row![
            face,
            text(name)
                .size(15)
                .color(if pointed { theme.text } else { theme.secondary })
                .wrapping(text::Wrapping::None),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([0, 8])
    .height(crate::sidebar::DM_ROW_HEIGHT)
    .width(Length::Fill)
    .align_y(Alignment::Center)
    .style(move |_| crate::style::rounded(ground, 6));
    let area = mouse_area(body)
        .interaction(iced::mouse::Interaction::Pointer)
        .on_enter(Message::Side(Side::Point(key.clone(), true)))
        .on_exit(Message::Side(Side::Point(key, false)))
        .on_press(Message::Fork(super::Event::OpenServer(id.to_owned())));
    let (server, spot) = (id.to_owned(), cx.pressed);
    RightPress::new(area, move |_| {
        Message::Side(Side::Menu(server.clone(), spot.get()))
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn reads_the_groups_the_slint_window_kept() {
        let kept = r#"[{"id":"1","name":"Pals","color":2,"rooms":["discord:5"],"collapsed":true}]"#;
        let groups: Vec<DmGroup> = serde_json::from_str(kept).unwrap();
        assert_eq!(groups[0].name, "Pals");
        assert_eq!(tint(groups[0].color), 0x23a55a);
        assert!(groups[0].collapsed);
    }
}
