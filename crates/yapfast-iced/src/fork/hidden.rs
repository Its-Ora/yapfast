//! Fork: servers and folders hidden from the rail. A server's or a folder's
//! menu hides it; while any is hidden, a Show All tile at the rail's bottom
//! shows them, faded, until it's pressed again. The open server shows all
//! the same. `layout.json` keeps them under `hidden_rail`.

use iced::Task;

use crate::{
    app::{App, Message},
    menus::{MenuAct, MenuRow},
    pictures::Icon,
};

use super::Act;

/// Adds Hide (or Show again) to the menu of a server or a folder.
pub fn menu_rows(app: &App, id: &str, rows: &mut Vec<MenuRow>) {
    let state = app.client.state();
    let server = state.servers.iter().any(|server| server.id == id);
    let folder = state
        .servers
        .iter()
        .any(|server| server.folder.as_ref().is_some_and(|folder| folder.id == id));
    if !server && !folder {
        return;
    }
    let hidden = app.layout.hidden_rail.contains(id);
    let (act, label, icon) = match (hidden, folder) {
        (false, false) => (Act::Hide, "Hide server", Icon::EyeOff),
        (false, true) => (Act::Hide, "Hide folder", Icon::EyeOff),
        (true, false) => (Act::Unhide, "Show server again", Icon::Eye),
        (true, true) => (Act::Unhide, "Show folder again", Icon::Eye),
    };
    rows.push(MenuRow {
        separated: !rows.is_empty(),
        ..MenuRow::new(MenuAct::Fork(act), label, icon)
    });
}

/// Hides `id` from the rail, or shows it again.
pub fn act(app: &mut App, id: &str, hide: bool) -> Task<Message> {
    let changed = if hide {
        app.layout.hidden_rail.insert(id.to_owned())
    } else {
        app.layout.hidden_rail.remove(id)
    };
    if !changed {
        return Task::none();
    }
    if app.layout.hidden_rail.is_empty() {
        app.rail.show_all = false;
    }
    crate::layout::store(app.client.root(), "hidden_rail", &app.layout.hidden_rail);
    app.rail.items = crate::rail::rail(
        app.client.state(),
        &app.layout.open_folders,
        &app.layout.hidden_rail,
        app.rail.show_all,
    );
    app.ask()
}
