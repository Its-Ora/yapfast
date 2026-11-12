//! Fork: what the Spotify account linked to this Discord account plays, as
//! Discord's client shows it: a "Listening to Spotify" activity, and a small
//! player that pauses, plays and skips. Spotify's Web API is asked with the
//! token Discord gives for the linked account
//! (`GET /users/@me/connections/spotify/{id}/access-token`), every few
//! seconds while something plays and less often otherwise. Controlling it
//! takes Spotify Premium, as Listen Along does in Discord.

use crate::api::Api;
use reqwest::Method;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, watch};

/// The track playing, or paused, on the linked Spotify account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpotifyNow {
    pub track_id: String,
    pub title: String,
    /// The artists, as Discord shows them: joined by "; ".
    pub artists: String,
    pub album: String,
    pub album_id: String,
    pub artist_ids: Vec<String>,
    /// The cover's address, and the id Discord names it by after
    /// `spotify:`.
    pub cover_url: Option<String>,
    pub cover_id: Option<String>,
    pub duration_ms: u64,
    /// How far in it was, at `at_ms` (Unix milliseconds).
    pub progress_ms: u64,
    pub at_ms: u64,
    pub playing: bool,
    /// Whether the linked account shows what it plays on Discord, as its
    /// connection's setting says.
    pub shown: bool,
}

impl SpotifyNow {
    /// How far in it is at `now_ms`.
    pub fn progress_at(&self, now_ms: u64) -> u64 {
        if self.playing {
            (self.progress_ms + now_ms.saturating_sub(self.at_ms)).min(self.duration_ms)
        } else {
            self.progress_ms
        }
    }

    /// The Listening activity Discord's client sends for it, for the account
    /// by `me`; none while paused or hidden.
    pub fn activity(&self, me: &str) -> Option<Value> {
        if !self.playing || !self.shown {
            return None;
        }
        let start = self.at_ms.saturating_sub(self.progress_ms);
        let mut assets = json!({ "large_text": self.album });
        if let Some(cover) = &self.cover_id {
            assets["large_image"] = json!(format!("spotify:{cover}"));
        }
        Some(json!({
            "type": 2,
            "name": "Spotify",
            "id": "spotify:1",
            "flags": 48,
            "details": self.title,
            "state": self.artists,
            "timestamps": { "start": start, "end": start + self.duration_ms },
            "assets": assets,
            "party": { "id": format!("spotify:{me}") },
            "sync_id": self.track_id,
            "metadata": {
                "album_id": self.album_id,
                "artist_ids": self.artist_ids,
                "type": "track",
            },
        }))
    }
}

/// What the player's buttons ask of Spotify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpotifyControl {
    PlayPause,
    Next,
    Previous,
}

pub(crate) fn start(
    api: Arc<Api>,
) -> (
    watch::Receiver<Option<Arc<SpotifyNow>>>,
    mpsc::UnboundedSender<SpotifyControl>,
) {
    let (now, receiver) = watch::channel(None);
    let (controls, requests) = mpsc::unbounded_channel();
    if crate::client::test_server().is_none()
        && std::env::var_os("YAPFAST_SPOTIFY").is_none_or(|value| value != "0")
    {
        tokio::spawn(run(api, now, requests));
    }
    (receiver, controls)
}

/// The linked account and its token, once found.
#[derive(Default)]
struct Link {
    id: Option<String>,
    shown: bool,
    token: Option<String>,
}

async fn run(
    api: Arc<Api>,
    now: watch::Sender<Option<Arc<SpotifyNow>>>,
    mut requests: mpsc::UnboundedReceiver<SpotifyControl>,
) {
    let mut link = Link::default();
    loop {
        let wait = match poll(&api, &mut link).await {
            Ok(found) => {
                let wait = match &found {
                    Some(track) if track.playing => {
                        // Ask again as the track ends, or every 5 s.
                        let left = track.duration_ms.saturating_sub(track.progress_ms) + 800;
                        Duration::from_millis(left.min(5_000))
                    }
                    Some(_) => Duration::from_secs(10),
                    None if link.id.is_some() => Duration::from_secs(15),
                    None => Duration::from_secs(120),
                };
                now.send_if_modified(|old| {
                    let changed = old.as_deref() != found.as_ref()
                        && !same_but_time(old.as_deref(), found.as_ref());
                    if changed {
                        *old = found.map(Arc::new);
                    }
                    changed
                });
                wait
            }
            Err(error) => {
                tracing::debug!(%error, "Spotify unavailable");
                link.token = None;
                Duration::from_secs(60)
            }
        };
        tokio::select! {
            () = now.closed() => return,
            () = tokio::time::sleep(wait) => {}
            control = requests.recv() => {
                let Some(control) = control else { return };
                let playing = now.borrow().as_ref().is_some_and(|track| track.playing);
                if let Err(error) = act(&api, &mut link, control, playing).await {
                    tracing::debug!(%error, "Spotify control failed");
                }
                // Spotify takes a moment to say what it plays now.
                tokio::time::sleep(Duration::from_millis(400)).await;
            }
        }
    }
}

/// The same track at the same place: only the time it was read changed.
fn same_but_time(old: Option<&SpotifyNow>, new: Option<&SpotifyNow>) -> bool {
    let (Some(old), Some(new)) = (old, new) else {
        return false;
    };
    let expected = old.progress_at(new.at_ms);
    SpotifyNow {
        progress_ms: new.progress_ms,
        at_ms: new.at_ms,
        ..old.clone()
    } == *new
        && expected.abs_diff(new.progress_ms) < 2_000
}

async fn token(api: &Api, link: &mut Link) -> anyhow::Result<Option<String>> {
    if link.id.is_none() {
        let connections = api.connections().await?;
        let spotify = connections
            .as_array()
            .into_iter()
            .flatten()
            .find(|connection| connection["type"] == "spotify" && connection["revoked"] != true);
        link.id = spotify.and_then(|found| found["id"].as_str().map(str::to_owned));
        link.shown = spotify.is_some_and(|found| found["show_activity"] != false);
    }
    let Some(id) = link.id.clone() else {
        return Ok(None);
    };
    if link.token.is_none() {
        link.token = Some(api.spotify_token(&id).await?);
    }
    Ok(link.token.clone())
}

async fn poll(api: &Api, link: &mut Link) -> anyhow::Result<Option<SpotifyNow>> {
    for _ in 0..2 {
        let Some(token) = token(api, link).await? else {
            return Ok(None);
        };
        let (status, body) = api
            .spotify(Method::GET, "/me/player?additional_types=track", &token)
            .await?;
        match status {
            200 => return Ok(body.as_ref().and_then(|body| parse(body, link.shown))),
            204 => return Ok(None),
            // The token ran out: ask Discord for another once.
            401 => link.token = None,
            _ => anyhow::bail!("Spotify answered {status}"),
        }
    }
    Ok(None)
}

fn parse(player: &Value, shown: bool) -> Option<SpotifyNow> {
    let item = &player["item"];
    if item["type"] != "track" {
        return None;
    }
    let artists: Vec<&Value> = item["artists"].as_array()?.iter().collect();
    let images = item["album"]["images"].as_array();
    let cover_url = images
        .and_then(|images| images.first())
        .and_then(|image| image["url"].as_str())
        .map(str::to_owned);
    let cover_id = cover_url
        .as_deref()
        .and_then(|url| url.rsplit('/').next())
        .map(str::to_owned);
    Some(SpotifyNow {
        track_id: item["id"].as_str()?.to_owned(),
        title: item["name"].as_str()?.to_owned(),
        artists: artists
            .iter()
            .filter_map(|artist| artist["name"].as_str())
            .collect::<Vec<_>>()
            .join("; "),
        album: item["album"]["name"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        album_id: item["album"]["id"].as_str().unwrap_or_default().to_owned(),
        artist_ids: artists
            .iter()
            .filter_map(|artist| artist["id"].as_str().map(str::to_owned))
            .collect(),
        cover_url,
        cover_id,
        duration_ms: item["duration_ms"].as_u64().unwrap_or_default(),
        progress_ms: player["progress_ms"].as_u64().unwrap_or_default(),
        at_ms: crate::service::now_ms(),
        playing: player["is_playing"].as_bool().unwrap_or(false),
        shown,
    })
}

async fn act(
    api: &Api,
    link: &mut Link,
    control: SpotifyControl,
    playing: bool,
) -> anyhow::Result<()> {
    let (method, path) = match control {
        SpotifyControl::PlayPause if playing => (Method::PUT, "/me/player/pause"),
        SpotifyControl::PlayPause => (Method::PUT, "/me/player/play"),
        SpotifyControl::Next => (Method::POST, "/me/player/next"),
        SpotifyControl::Previous => (Method::POST, "/me/player/previous"),
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_player_and_makes_discords_activity() {
        let player = json!({
            "is_playing": true,
            "progress_ms": 30_000,
            "item": {
                "type": "track", "id": "t1", "name": "Song", "duration_ms": 200_000,
                "artists": [{"id": "a1", "name": "One"}, {"id": "a2", "name": "Two"}],
                "album": {"id": "al", "name": "Album", "images": [
                    {"url": "https://i.scdn.co/image/ab67616d0000b273abc"}
                ]},
            },
        });
        let mut now = parse(&player, true).unwrap();
        now.at_ms = 1_000_000;
        assert_eq!(now.artists, "One; Two");
        let activity = now.activity("42").unwrap();
        assert_eq!(
            activity["assets"]["large_image"],
            "spotify:ab67616d0000b273abc"
        );
        assert_eq!(activity["timestamps"]["start"], 970_000);
        assert_eq!(activity["timestamps"]["end"], 1_170_000);
        assert_eq!(activity["party"]["id"], "spotify:42");
        assert_eq!(now.progress_at(1_010_000), 40_000);
        now.playing = false;
        assert!(now.activity("42").is_none());
        // An episode isn't a track.
        assert!(parse(&json!({"item": {"type": "episode"}}), true).is_none());
    }
}
