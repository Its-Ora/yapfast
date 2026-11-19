//! Fork: what the Its-Ora fork adds to the window, apart from the lines it
//! changes in place (each marked "Fork:" where it is).
//!
//! - [`hidden`]: servers and folders hidden from the rail, and Show All.
//! - [`dm_groups`]: the DM list's named, coloured groups.
//! - [`spotify`]: the Spotify player over the account panel.

pub mod dm_groups;
pub mod hidden;
pub mod spotify;

use iced::Task;

use crate::app::{App, Message};

/// What the fork's parts hold while the window is open.
#[derive(Default)]
pub struct State {
    /// The DM group being renamed in its heading, by id, and the name typed.
    pub renaming: Option<(String, String)>,
    /// What Spotify last played, kept while it's paused.
    pub spotify: spotify::Held,
}

/// What's done in the fork's parts.
#[derive(Clone, Debug)]
pub enum Event {
    /// The name typed in a DM group's heading, kept, or let go.
    GroupName(String),
    GroupSave,
    GroupCancel,
    /// A server the DM search found opens.
    OpenServer(String),
    /// A button of the Spotify player.
    Spotify(yapfast_core::chat::SpotifyControl),
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::GroupName(name) => {
            if let Some((_, typed)) = &mut app.fork.renaming {
                *typed = name;
            }
            Task::none()
        }
        Event::GroupSave => dm_groups::save_name(app),
        Event::GroupCancel => {
            app.fork.renaming = None;
            app.lists_shown = None;
            app.sync()
        }
        Event::OpenServer(id) => {
            app.sidebar.filter.clear();
            app.lists_shown = None;
            app.client
                .dispatch(yapfast_core::application::Action::Navigate(
                    yapfast_core::application::Navigation::Server(id),
                ));
            app.sync()
        }
        Event::Spotify(control) => spotify::control(app, control),
    }
}

/// What one of the fork's menu rows does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    /// The menu's server or folder leaves the rail, or comes back.
    Hide,
    Unhide,
    /// The menu's DM goes into the group in the row's key, out of its
    /// group, or into a new one.
    MoveToGroup,
    Ungroup,
    NewGroup,
    /// The group heading's menu, the group by the menu's place: renamed in
    /// its heading, given the colour in the row's key, deleted.
    EditGroup,
    Recolor,
    DeleteGroup,
}

/// Carries out a fork row of the menu about `place`.
pub fn act(app: &mut App, place: &str, key: &str, act: Act) -> Task<Message> {
    match act {
        Act::Hide | Act::Unhide => hidden::act(app, place, act == Act::Hide),
        Act::MoveToGroup
        | Act::Ungroup
        | Act::NewGroup
        | Act::EditGroup
        | Act::Recolor
        | Act::DeleteGroup => dm_groups::act(app, place, key, act),
    }
}
