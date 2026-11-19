//! Fork: a Spotify player over the account panel. What plays comes from
//! the account's own "Listening to Spotify" activity, which Yapfast shares
//! itself (`yapfast_discord::spotify`); paused, Spotify shares nothing, so
//! the player keeps the last song, paused, for a while. Its buttons go back,
//! pause or play, and skip, through Spotify's Web API with the token Discord
//! lends for the link (`yapfast_discord::spotify_control`); Spotify allows
//! that with Premium only.

use std::time::{Duration, Instant};

use iced::{
    Alignment, Element, Length, Padding, Task,
    widget::{column, container, row, text},
};
use yapfast_core::chat::{Activity, ActivityKind, SpotifyControl};

use crate::{
    app::{App, Message},
    fonts::SEMIBOLD,
    pictures::Icon,
};

/// How long a paused song stays in the player.
const KEPT: Duration = Duration::from_secs(30 * 60);
const COVER: f32 = 40.0;

/// The song the player shows.
#[derive(Default)]
pub struct Held {
    song: Option<Activity>,
    /// When it was seen paused; none while it plays.
    paused: Option<Instant>,
}

/// The account's own Spotify activity, if it shares one.
fn playing(state: &yapfast_core::application::AppState) -> Option<&Activity> {
    state
        .discord_activities
        .iter()
        .find(|activity| activity.kind == ActivityKind::Listening && &*activity.name == "Spotify")
}

/// Takes in what the account plays now, after each batch of the core's
/// events.
pub fn sync(app: &mut App) {
    let now = playing(app.client.state()).cloned();
    if let Some(cover) = now.as_ref().and_then(|song| song.large_image.as_deref()) {
        app.client.want_pictures(vec![cover.to_owned()]);
    }
    let held = &mut app.fork.spotify;
    match now {
        Some(song) => {
            held.paused = None;
            if held.song.as_ref() != Some(&song) {
                held.song = Some(song);
            }
        }
        None if held.song.is_some() => {
            let since = *held.paused.get_or_insert_with(Instant::now);
            if since.elapsed() > KEPT {
                *held = Held::default();
            }
        }
        None => {}
    }
}

/// A button of the player was pressed.
pub fn control(app: &mut App, control: SpotifyControl) -> Task<Message> {
    app.client
        .dispatch(yapfast_core::application::Action::Spotify(control));
    // Paused or playing shows at once; Spotify's word follows.
    match control {
        SpotifyControl::Pause => app.fork.spotify.paused = Some(Instant::now()),
        SpotifyControl::Play => app.fork.spotify.paused = None,
        SpotifyControl::Next | SpotifyControl::Previous => {}
    }
    app.sync()
}

fn clock(ms: u64) -> String {
    let seconds = ms / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The player: the cover, the song and its artists, where it is in it, and
/// the buttons; none while nothing plays or was paused lately.
pub fn view<'a>(cx: crate::ctx::Ctx<'a>, held: &'a Held) -> Option<Element<'a, Message>> {
    let song = held.song.as_ref()?;
    let theme = cx.look.theme;
    let paused = held.paused.is_some();
    let cover: Element<'a, Message> = match song
        .large_image
        .as_deref()
        .and_then(|url| cx.picture(url, COVER as u32, COVER as u32, true))
    {
        Some(picture) => crate::widgets::Pic::square(picture, COVER)
            .radius(6.0)
            .into(),
        None => cx
            .look
            .glyph(Icon::Music, 20, crate::style::hex(0x1db954))
            .boxed(COVER)
            .into(),
    };
    let title = song.details.as_deref().unwrap_or(&song.name);
    let artists = song
        .state
        .as_deref()
        .unwrap_or_default()
        .replace("; ", ", ");
    let mut words = column![
        text(title)
            .size(13)
            .font(SEMIBOLD)
            .color(theme.text)
            .wrapping(text::Wrapping::None),
        text(artists)
            .size(12)
            .color(theme.muted)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(1)
    .width(Length::Fill);
    if let (Some(start), Some(end)) = (song.start, song.end) {
        let length = end.saturating_sub(start);
        if paused {
            words = words.push(
                text(format!("Paused · {}", clock(length)))
                    .size(11)
                    .color(theme.faint),
            );
        } else {
            // The clock moves by itself, each second: no message, no view.
            words = words.push(
                crate::widgets::live_text(Duration::from_secs(1), move || {
                    let at = yapfast_core::clock::now_ms()
                        .saturating_sub(start)
                        .min(length);
                    format!("{} / {}", clock(at), clock(length))
                })
                .size(11)
                .color(theme.faint),
            );
        }
    }
    let press = |icon: Icon, label: &'static str, control: SpotifyControl, side: u32| {
        cx.look
            .icon_button(icon, label, Message::Fork(super::Event::Spotify(control)))
            .size(28.0, side)
    };
    let buttons = row![
        press(Icon::SkipBack, "Previous", SpotifyControl::Previous, 14),
        if paused {
            press(Icon::Play, "Play", SpotifyControl::Play, 18)
        } else {
            press(Icon::Pause, "Pause", SpotifyControl::Pause, 18)
        },
        press(Icon::SkipForward, "Next", SpotifyControl::Next, 14),
    ]
    .align_y(Alignment::Center);
    let panel = theme.panel;
    let body = row![
        cover,
        container(words).clip(true).width(Length::Fill),
        buttons
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    Some(
        container(body)
            .padding(Padding {
                left: 12.0,
                right: 8.0,
                top: 8.0,
                bottom: 8.0,
            })
            .width(Length::Fill)
            .style(move |_| crate::style::fill(panel))
            .into(),
    )
}
