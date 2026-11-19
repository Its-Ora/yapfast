//! Fork: the Spotify player's buttons. What the linked account plays comes
//! from [`crate::spotify`]; this asks Spotify's Web API to play, pause or
//! skip on it, with the token Discord lends for the link
//! (`GET /users/@me/connections/spotify/{id}/access-token`). Spotify allows
//! it with Premium only, as Discord's Listen Along needs.

use crate::api::Api;
use reqwest::Method;
use std::sync::Arc;
use tokio::sync::mpsc;

/// What a button asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpotifyControl {
    Play,
    Pause,
    Next,
    Previous,
}

pub(crate) fn start(api: Arc<Api>) -> mpsc::UnboundedSender<SpotifyControl> {
    let (controls, mut requests) = mpsc::unbounded_channel();
    if crate::client::test_server().is_none() {
        tokio::spawn(async move {
            let mut link = Link::default();
            while let Some(control) = requests.recv().await {
                if let Err(error) = act(&api, &mut link, control).await {
                    tracing::debug!(%error, "Spotify control failed");
                    link = Link::default();
                }
            }
        });
    }
    controls
}

/// The linked account and its token, once found; neither is logged.
#[derive(Default)]
struct Link {
    id: Option<String>,
    token: Option<String>,
}

async fn token(api: &Api, link: &mut Link) -> anyhow::Result<Option<String>> {
    if link.id.is_none() {
        let connections = api.connections().await?;
        link.id = connections
            .as_array()
            .into_iter()
            .flatten()
            .find(|connection| connection["type"] == "spotify" && connection["revoked"] != true)
            .and_then(|found| found["id"].as_str().map(str::to_owned));
    }
    let Some(id) = link.id.clone() else {
        return Ok(None);
    };
    if link.token.is_none() {
        link.token = Some(api.spotify_access_token(&id).await?);
    }
    Ok(link.token.clone())
}

async fn act(api: &Api, link: &mut Link, control: SpotifyControl) -> anyhow::Result<()> {
    let (method, path) = match control {
        SpotifyControl::Play => (Method::PUT, "/me/player/play"),
        SpotifyControl::Pause => (Method::PUT, "/me/player/pause"),
        SpotifyControl::Next => (Method::POST, "/me/player/next"),
        SpotifyControl::Previous => (Method::POST, "/me/player/previous"),
    };
    // A token that ran out is asked for again, once.
    for _ in 0..2 {
        let Some(token) = token(api, link).await? else {
            return Ok(());
        };
        let (status, _) = api.spotify(method.clone(), path, &token).await?;
        match status {
            200..=299 => return Ok(()),
            401 => link.token = None,
            // Premium only, or no device to play on.
            _ => anyhow::bail!("Spotify answered {status}"),
        }
    }
    Ok(())
}
