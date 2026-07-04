pub mod helpers {
use serde_json::Value;
use super::{DeezerSource, PUBLIC_API_BASE};
impl DeezerSource {
    pub(crate) async fn get_json_public(&self, path: &str) -> Option<Value> {
        let url = format!("{PUBLIC_API_BASE}/{path}");
        match self.client.get(&url).send().await {
            Ok(res) => {
                if res.status().is_success() {
                    res.json().await.ok()
                } else {
                    tracing::warn!(
                        "Deezer public API request failed: {url} (Status: {})",
                        res.status()
                    );
                    None
                }
            }
            Err(e) => {
                tracing::error!(
                    "Deezer public API request error: {url} (Error: {e}). If this is a connectivity error, check your proxy settings.",
                );
                None
            }
        }
    }
}
}
pub mod metadata {
use super::DeezerSource;
use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track};
impl DeezerSource {
    pub(crate) async fn get_track_by_isrc(&self, isrc: &str) -> Option<Track> {
        let url = format!("track/isrc:{isrc}");
        tracing::debug!("DeezerSource: Fetching metadata for ISRC: {isrc} (URL: {url})");
        let json = self.get_json_public(&url).await?;
        if json.get("id").is_some() {
            let res = self.parse_track(&json);
            if let Some(ref t) = res {
                tracing::debug!(
                    "DeezerSource: Found track for ISRC {isrc}: {}",
                    t.info.identifier
                );
            } else {
                tracing::debug!("DeezerSource: Failed to parse track for ISRC {isrc}");
            }
            res
        } else {
            tracing::debug!("DeezerSource: No track found for ISRC {isrc}");
            None
        }
    }
    pub(crate) async fn get_album(&self, id: &str) -> LoadResult {
        let json = match self.get_json_public(&format!("album/{id}")).await {
            Some(j) => j,
            None => return LoadResult::Empty {},
        };
        let tracks_json = match self
            .get_json_public(&format!("album/{id}/tracks?limit=10000"))
            .await
        {
            Some(j) => j,
            None => return LoadResult::Empty {},
        };
        let mut tracks = Vec::new();
        let artwork_url = json
            .get("cover_xl")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        if let Some(data) = tracks_json.get("data").and_then(|d| d.as_array()) {
            for item in data {
                if let Some(mut track) = self.parse_track(item) {
                    if track.info.artwork_url.is_none() {
                        track.info.artwork_url = artwork_url.clone();
                    }
                    tracks.push(track);
                }
            }
        }
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: json
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Album")
                    .to_owned(),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
              "type": "album",
              "url": format!("https://www.deezer.com/album/{id}"),
              "artworkUrl": json.get("cover_xl").and_then(|v| v.as_str()),
              "author": json.get("artist").and_then(|v| v.get("name")).and_then(|v| v.as_str()),
              "totalTracks": json.get("nb_tracks").and_then(|v| v.as_u64()).unwrap_or(tracks.len() as u64)
            }),
            tracks,
        })
    }
    pub(crate) async fn get_playlist(&self, id: &str) -> LoadResult {
        let json = match self.get_json_public(&format!("playlist/{id}")).await {
            Some(j) => j,
            None => return LoadResult::Empty {},
        };
        let tracks_json = match self
            .get_json_public(&format!("playlist/{id}/tracks?limit=10000"))
            .await
        {
            Some(j) => j,
            None => return LoadResult::Empty {},
        };
        let mut tracks = Vec::new();
        if let Some(data) = tracks_json.get("data").and_then(|d| d.as_array()) {
            for item in data {
                if let Some(track) = self.parse_track(item) {
                    tracks.push(track);
                }
            }
        }
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: json
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Playlist")
                    .to_owned(),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
              "type": "playlist",
              "url": format!("https://www.deezer.com/playlist/{id}"),
              "artworkUrl": json.get("picture_xl").and_then(|v| v.as_str()),
              "author": json.get("creator").and_then(|v| v.get("name")).and_then(|v| v.as_str()),
              "totalTracks": json.get("nb_tracks").and_then(|v| v.as_u64()).unwrap_or(tracks.len() as u64)
            }),
            tracks,
        })
    }
    pub(crate) async fn get_artist(&self, id: &str) -> LoadResult {
        let json = match self.get_json_public(&format!("artist/{id}")).await {
            Some(j) => j,
            None => return LoadResult::Empty {},
        };
        let tracks_json = match self
            .get_json_public(&format!("artist/{id}/top?limit=50"))
            .await
        {
            Some(j) => j,
            None => return LoadResult::Empty {},
        };
        let artwork_url = json
            .get("picture_xl")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let author = json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Artist")
            .to_owned();
        let mut tracks = Vec::new();
        if let Some(data) = tracks_json.get("data").and_then(|d| d.as_array()) {
            for item in data {
                if let Some(mut track) = self.parse_track(item) {
                    if track.info.artwork_url.is_none() {
                        track.info.artwork_url = artwork_url.clone();
                    }
                    tracks.push(track);
                }
            }
        }
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: format!("{author}'s Top Tracks"),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
              "type": "artist",
              "url": format!("https://www.deezer.com/artist/{id}"),
              "artworkUrl": json.get("picture_xl").and_then(|v| v.as_str()),
              "author": author,
              "totalTracks": tracks.len()
            }),
            tracks,
        })
    }
}
}
pub mod parser {
use serde_json::{Value, json};
use super::DeezerSource;
use crate::protocol::tracks::{Track, TrackInfo};
impl DeezerSource {
    pub(crate) fn parse_track(&self, json: &Value) -> Option<Track> {
        let id = json.get("id").and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        })?;
        let title = json.get("title")?.as_str()?.to_owned();
        let artist = json.get("artist")?.get("name")?.as_str()?.to_owned();
        let duration = json.get("duration")?.as_u64()? * 1000;
        if let Some(readable) = json.get("readable").and_then(|v| v.as_bool())
            && !readable
        {
            let countries = json
                .get("available_countries")
                .and_then(|v| {
                    v.as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|c| c.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .or_else(|| v.as_str().map(|s| s.to_owned()))
                })
                .unwrap_or_default();
            tracing::debug!(
                "Deezer track {} ({}) is marked as not readable. Available countries: {}. It might fail unless a fallback is found.",
                title,
                id,
                countries
            );
        }
        let isrc = json
            .get("isrc")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned());
        let artwork_url = json
            .get("album")
            .and_then(|a| a.get("cover_xl"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .or_else(|| {
                json.get("md5_image").and_then(|v| v.as_str()).map(|id| {
                    format!(
                        "https://cdn-images.dzcdn.net/images/cover/{id}/1000x1000-000000-80-0-0.jpg"
                    )
                })
            });
        let uri = json
            .get("link")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned());
        let mut track = Track::new(TrackInfo {
            identifier: id,
            is_seekable: true,
            author: artist,
            length: duration,
            is_stream: false,
            position: 0,
            title,
            uri: uri.clone(),
            artwork_url,
            isrc,
            source_name: "deezer".to_owned(),
        });
        let album_name = json.pointer("/album/title").and_then(|v| v.as_str());
        let album_url = json
            .pointer("/album/id")
            .and_then(|v| {
                v.as_str()
                    .map(|s| s.to_owned())
                    .or_else(|| v.as_u64().map(|id| id.to_string()))
            })
            .map(|id| format!("https://www.deezer.com/album/{id}"));
        let artist_url = json
            .pointer("/artist/id")
            .and_then(|v| {
                v.as_str()
                    .map(|s| s.to_owned())
                    .or_else(|| v.as_u64().map(|id| id.to_string()))
            })
            .map(|id| format!("https://www.deezer.com/artist/{id}"));
        let artist_artwork_url = json.pointer("/artist/picture_xl").and_then(|v| v.as_str());
        let preview_url = json.get("preview").and_then(|v| v.as_str());
        track.plugin_info = json!({
            "albumName": album_name,
            "albumUrl": album_url,
            "artistUrl": artist_url,
            "artistArtworkUrl": artist_artwork_url,
            "previewUrl": preview_url,
            "isPreview": false
        });
        Some(track)
    }
    pub(crate) fn parse_recommendation_track(&self, json: &Value) -> Option<Track> {
        let id = json.get("SNG_ID").and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        })?;
        let title = json.get("SNG_TITLE")?.as_str()?.to_owned();
        let artist = json.get("ART_NAME")?.as_str()?.to_owned();
        let duration = json.get("DURATION")?.as_u64()? * 1000;
        if let Some(readable) = json.get("READABLE").and_then(|v| v.as_bool())
            && !readable
        {
            tracing::debug!(
                "Deezer recommendation track {} ({}) is marked as not readable. It might fail unless a fallback is found.",
                title,
                id
            );
        }
        let isrc = json
            .get("ISRC")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let album_pic = json
            .get("ALB_PICTURE")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let artwork_url = if !album_pic.is_empty() {
            Some(format!(
                "https://cdn-images.dzcdn.net/images/cover/{album_pic}/1000x1000-000000-80-0-0.jpg"
            ))
        } else {
            None
        };
        let uri_val = Some(format!("https://deezer.com/track/{id}"));
        let mut track = Track::new(TrackInfo {
            identifier: id.clone(),
            is_seekable: true,
            author: artist,
            length: duration,
            is_stream: false,
            position: 0,
            title,
            uri: uri_val.clone(),
            artwork_url,
            isrc,
            source_name: "deezer".to_owned(),
        });
        let album_name = json.get("ALB_TITLE").and_then(|v| v.as_str());
        let album_url = json
            .get("ALB_ID")
            .and_then(|v| {
                v.as_str()
                    .map(|s| s.to_owned())
                    .or_else(|| v.as_u64().map(|id| id.to_string()))
            })
            .map(|id| format!("https://www.deezer.com/album/{id}"));
        let artist_url = json
            .get("ART_ID")
            .and_then(|v| {
                v.as_str()
                    .map(|s| s.to_owned())
                    .or_else(|| v.as_u64().map(|id| id.to_string()))
            })
            .map(|id| format!("https://www.deezer.com/artist/{id}"));
        let artist_artwork_url = json
            .pointer("/ARTISTS/0/ART_PICTURE")
            .and_then(|v| v.as_str())
            .map(|id| {
                format!(
                    "https://cdn-images.dzcdn.net/images/cover/{id}/1000x1000-000000-80-0-0.jpg"
                )
            });
        let preview_url = json.pointer("/MEDIA/0/HREF").and_then(|v| v.as_str());
        track.plugin_info = json!({
            "albumName": album_name,
            "albumUrl": album_url,
            "artistUrl": artist_url,
            "artistArtworkUrl": artist_artwork_url,
            "previewUrl": preview_url,
            "isPreview": false
        });
        Some(track)
    }
}
}
pub mod recommendations {
use serde_json::Value;
use super::{DeezerSource, PRIVATE_API_BASE};
use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track};
impl DeezerSource {
    pub(crate) async fn get_recommendations(&self, query: &str) -> LoadResult {
        let tokens = match self.token_tracker.get_token().await {
            Some(t) => t,
            None => return LoadResult::Empty {},
        };
        let (method, payload) =
            if let Some(artist_id) = query.strip_prefix(super::REC_ARTIST_PREFIX) {
                (
                    "song.getSmartRadio",
                    serde_json::json!({ "art_id": artist_id }),
                )
            } else {
                let track_id = query.strip_prefix(super::REC_TRACK_PREFIX).unwrap_or(query);
                (
                    "song.getSearchTrackMix",
                    serde_json::json!({ "sng_id": track_id, "start_with_input_track": "true" }),
                )
            };
        let url = format!(
            "{PRIVATE_API_BASE}?method={method}&input=3&api_version=1.0&api_token={}",
            tokens.api_token
        );
        let res = match self
            .client
            .post(&url)
            .header(
                "Cookie",
                format!(
                    "sid={}; dzr_uniq_id={}",
                    tokens.session_id, tokens.dzr_uniq_id
                ),
            )
            .json(&payload)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return LoadResult::Empty {},
        };
        let json: Value = res.json().await.unwrap_or(Value::Null);
        let data = json.get("results").and_then(|r| r.get("data"));
        let tracks: Vec<Track> = if let Some(arr) = data.and_then(|d| d.as_array()) {
            arr.iter()
                .filter_map(|item| self.parse_recommendation_track(item))
                .collect()
        } else if let Some(obj) = data.and_then(|d| d.as_object()) {
            obj.values()
                .filter_map(|item| self.parse_recommendation_track(item))
                .collect()
        } else {
            Vec::new()
        };
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: "Deezer Recommendations".to_owned(),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
              "type": "recommendations",
              "totalTracks": tracks.len()
            }),
            tracks,
        })
    }
}
}
pub mod search {
use super::DeezerSource;
use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track};
impl DeezerSource {
    pub(crate) async fn search(&self, query: &str) -> LoadResult {
        let url = format!("search?q={}", urlencoding::encode(query));
        if let Some(json) = self.get_json_public(&url).await
            && let Some(data) = json.get("data").and_then(|v| v.as_array())
        {
            if data.is_empty() {
                return LoadResult::Empty {};
            }
            let tracks: Vec<Track> = data
                .iter()
                .filter_map(|item| self.parse_track(item))
                .collect();
            if tracks.is_empty() {
                return LoadResult::Empty {};
            }
            return LoadResult::Search(tracks);
        }
        LoadResult::Empty {}
    }
    pub(crate) async fn get_autocomplete(
        &self,
        query: &str,
        types: &[String],
    ) -> Option<crate::protocol::tracks::SearchResult> {
        let url = format!("search/autocomplete?q={}", urlencoding::encode(query));
        let json = self.get_json_public(&url).await?;
        let all_types = types.is_empty();
        let mut tracks = Vec::new();
        let mut albums = Vec::new();
        let mut artists = Vec::new();
        let mut playlists = Vec::new();
        let texts = Vec::new();
        if (all_types || types.contains(&"album".to_owned()))
            && let Some(data) = json.pointer("/albums/data").and_then(|v| v.as_array())
        {
            for album in data {
                let title = album
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Album")
                    .to_owned();
                let link = album
                    .get("link")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let cover_xl = album
                    .get("cover_xl")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let artist_name = album
                    .pointer("/artist/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Artist")
                    .to_owned();
                let nb_tracks = album.get("nb_tracks").and_then(|v| v.as_u64()).unwrap_or(0);
                albums.push(PlaylistData {
                    info: PlaylistInfo {
                        name: title,
                        selected_track: -1,
                    },
                    plugin_info: serde_json::json!({
                      "type": "album",
                      "url": link,
                      "artworkUrl": if cover_xl.is_empty() { None } else { Some(cover_xl) },
                      "author": artist_name,
                      "totalTracks": nb_tracks
                    }),
                    tracks: Vec::new(),
                });
            }
        }
        if (all_types || types.contains(&"artist".to_owned()))
            && let Some(data) = json.pointer("/artists/data").and_then(|v| v.as_array())
        {
            for artist in data {
                let name = artist
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Artist")
                    .to_owned();
                let link = artist
                    .get("link")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let picture_xl = artist
                    .get("picture_xl")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                artists.push(PlaylistData {
                    info: PlaylistInfo {
                        name: format!("{name}'s Top Tracks"),
                        selected_track: -1,
                    },
                    plugin_info: serde_json::json!({
                      "type": "artist",
                      "url": link,
                      "artworkUrl": if picture_xl.is_empty() { None } else { Some(picture_xl) },
                      "author": name,
                      "totalTracks": 0
                    }),
                    tracks: Vec::new(),
                });
            }
        }
        if (all_types || types.contains(&"playlist".to_owned()))
            && let Some(data) = json.pointer("/playlists/data").and_then(|v| v.as_array())
        {
            for playlist in data {
                let title = playlist
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Playlist")
                    .to_owned();
                let link = playlist
                    .get("link")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let picture_xl = playlist
                    .get("picture_xl")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let creator_name = playlist
                    .pointer("/creator/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Creator")
                    .to_owned();
                let nb_tracks = playlist
                    .get("nb_tracks")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                playlists.push(PlaylistData {
                    info: PlaylistInfo {
                        name: title,
                        selected_track: -1,
                    },
                    plugin_info: serde_json::json!({
                      "type": "playlist",
                      "url": link,
                      "artworkUrl": if picture_xl.is_empty() { None } else { Some(picture_xl) },
                      "author": creator_name,
                      "totalTracks": nb_tracks
                    }),
                    tracks: Vec::new(),
                });
            }
        }
        if (all_types || types.contains(&"track".to_owned()))
            && let Some(data) = json.pointer("/tracks/data").and_then(|v| v.as_array())
        {
            for track in data {
                if let Some(parsed) = self.parse_track(track) {
                    tracks.push(parsed);
                }
            }
        }
        Some(crate::protocol::tracks::SearchResult {
            tracks,
            albums,
            artists,
            playlists,
            texts,
            plugin: serde_json::json!({}),
        })
    }
}
}
pub mod token {
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use serde_json::Value;
use tracing::{debug, error};
use crate::common::types::Shared;
#[derive(Debug, Clone)]
pub struct DeezerTokens {
    pub session_id: String,
    pub dzr_uniq_id: String,
    pub api_token: String,
    pub license_token: String,
    pub expire_at: Instant,
    pub arl_index: usize,
}
pub struct DeezerTokenTracker {
    client: Arc<reqwest::Client>,
    arls: Vec<String>,
    tokens: Shared<Vec<Option<DeezerTokens>>>,
    current_index: AtomicUsize,
}
impl DeezerTokenTracker {
    pub fn new(client: Arc<reqwest::Client>, arls: Vec<String>) -> Self {
        let size = arls.len();
        Self {
            client,
            arls,
            tokens: Arc::new(tokio::sync::Mutex::new(vec![None; size])),
            current_index: AtomicUsize::new(0),
        }
    }
    pub async fn get_token(&self) -> Option<DeezerTokens> {
        let index = self.current_index.fetch_add(1, Ordering::Relaxed) % self.arls.len();
        self.get_token_at(index).await
    }
    pub async fn get_token_at(&self, index: usize) -> Option<DeezerTokens> {
        {
            let guard = self.tokens.lock().await;
            if let Some(tokens) = &guard[index]
                && Instant::now() < tokens.expire_at
            {
                return Some(tokens.clone());
            }
        }
        self.refresh_session(index).await
    }
    pub async fn invalidate_token(&self, index: usize) {
        let mut guard = self.tokens.lock().await;
        guard[index] = None;
    }
    async fn refresh_session(&self, index: usize) -> Option<DeezerTokens> {
        let arl = &self.arls[index];
        let initial_cookie = format!("arl={arl}");
        let url = "https://www.deezer.com/ajax/gw-light.php?method=deezer.getUserData&input=3&api_version=1.0&api_token=";
        let req = self.client.get(url).header("Cookie", initial_cookie);
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                error!("DeezerTokenTracker: Failed to refresh session (index {index}): {e}");
                return None;
            }
        };
        let mut session_id = String::new();
        let mut dzr_uniq_id = String::new();
        for cookie in resp.cookies() {
            match cookie.name() {
                "sid" => session_id = cookie.value().to_owned(),
                "dzr_uniq_id" => dzr_uniq_id = cookie.value().to_owned(),
                _ => {}
            }
        }
        let body: Value = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                error!("DeezerTokenTracker: Failed to parse session response: {e}");
                return None;
            }
        };
        let api_token = body
            .get("results")
            .and_then(|r| r.get("checkForm"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned())?;
        let license_token = body
            .get("results")
            .and_then(|r| r.get("USER"))
            .and_then(|u| u.get("OPTIONS"))
            .and_then(|o| o.get("license_token"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned())
            .unwrap_or_default();
        let tokens = DeezerTokens {
            session_id,
            dzr_uniq_id,
            api_token,
            license_token,
            expire_at: Instant::now() + Duration::from_secs(3600),
            arl_index: index,
        };
        {
            let mut guard = self.tokens.lock().await;
            guard[index] = Some(tokens.clone());
        }
        debug!("DeezerTokenTracker: Refreshed tokens for index {index}");
        Some(tokens)
    }
}
}
pub mod track {
use std::{net::IpAddr, sync::Arc};
use async_trait::async_trait;
use tracing::{debug, error, warn};
use crate::{
    common::types::AudioFormat,
    config::HttpProxyConfig,
    sources::{
        deezer::{reader::DeezerReader, token::DeezerTokenTracker},
        playable_track::{PlayableTrack, ResolvedTrack},
    },
};
pub struct DeezerTrack {
    pub client: Arc<reqwest::Client>,
    pub track_id: String,
    pub token_tracker: Arc<DeezerTokenTracker>,
    pub master_key: String,
    pub local_addr: Option<IpAddr>,
    pub proxy: Option<HttpProxyConfig>,
}
struct ResolvedUrl {
    cdn_url: String,
    track_id: String,
    arl_index: usize,
}
#[async_trait]
impl PlayableTrack for DeezerTrack {
    fn supports_seek(&self) -> bool {
        true
    }
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        const MAX_RETRIES: u32 = 3;
        let mut last_arl: Option<usize> = None;
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0
                && let Some(idx) = last_arl.take()
            {
                self.token_tracker.invalidate_token(idx).await;
            }
            let resolved =
                match resolve_cdn_url(&self.client, &self.token_tracker, &self.track_id).await {
                    Some(r) => r,
                    None => continue,
                };
            last_arl = Some(resolved.arl_index);
            let master_key = self.master_key.clone();
            let local_addr = self.local_addr;
            let proxy = self.proxy.clone();
            let cdn_url = resolved.cdn_url.clone();
            let effective_id = resolved.track_id.clone();
            let reader_result =
                DeezerReader::new(&cdn_url, &effective_id, &master_key, local_addr, proxy)
                    .await
                    .map(|r| {
                        (
                            Box::new(r) as Box<dyn symphonia::core::io::MediaSource>,
                            cdn_url,
                        )
                    })
                    .map_err(|e| e.to_string());
            let (reader, final_url) = match reader_result {
                Ok(v) => v,
                Err(e) => {
                    warn!(
                        "Deezer CDN open failed for {} (attempt {}/{}): {e} — rotating ARL",
                        self.track_id,
                        attempt + 1,
                        MAX_RETRIES + 1,
                    );
                    continue;
                }
            };
            let hint = AudioFormat::from_url(&final_url);
            return Ok(ResolvedTrack::new(reader, Some(hint)));
        }
        error!("Deezer: all retries exhausted for {}", self.track_id);
        Err("Failed to open Deezer stream after retries".to_string())
    }
}
async fn resolve_cdn_url(
    client: &Arc<reqwest::Client>,
    token_tracker: &Arc<DeezerTokenTracker>,
    track_id: &str,
) -> Option<ResolvedUrl> {
    let tokens = token_tracker.get_token().await?;
    let arl_index = tokens.arl_index;
    let song_url = format!(
        "https://www.deezer.com/ajax/gw-light.php?method=song.getData&input=3&api_version=1.0&api_token={}",
        tokens.api_token
    );
    let json: serde_json::Value = match client
        .post(&song_url)
        .header(
            "Cookie",
            format!(
                "sid={}; dzr_uniq_id={}",
                tokens.session_id, tokens.dzr_uniq_id
            ),
        )
        .json(&serde_json::json!({ "sng_id": track_id }))
        .send()
        .await
    {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(_) => {
                token_tracker.invalidate_token(arl_index).await;
                return None;
            }
        },
        Err(e) => {
            debug!("Deezer: song.getData failed: {e}");
            token_tracker.invalidate_token(arl_index).await;
            return None;
        }
    };
    if json
        .get("error")
        .and_then(|v| v.as_array())
        .is_some_and(|e| !e.is_empty())
    {
        debug!("Deezer: API error in song.getData");
        token_tracker.invalidate_token(arl_index).await;
        return None;
    }
    let mut results = json.get("results")?.clone();
    let rights = results.get("RIGHTS");
    if is_rights_empty(rights)
        && let Some(fallback) = results.get("FALLBACK")
        && !fallback
            .get("TRACK_TOKEN")
            .map(|v| v.is_null())
            .unwrap_or(true)
    {
        let fallback_id = fallback.get("SNG_ID").and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        });
        if let Some(id) = fallback_id {
            debug!("Deezer: track {track_id} has no RIGHTS, using FALLBACK {id}");
            results = fallback.clone();
            let track_token = results.get("TRACK_TOKEN").and_then(|v| v.as_str())?;
            return fetch_media_url(client, token_tracker, &tokens, track_token, &id, arl_index)
                .await;
        } else {
            warn!("Deezer: track {track_id} FALLBACK SNG_ID has unexpected format");
        }
    }
    let track_token = results.get("TRACK_TOKEN").and_then(|v| v.as_str())?;
    fetch_media_url(
        client,
        token_tracker,
        &tokens,
        track_token,
        track_id,
        arl_index,
    )
    .await
}
async fn fetch_media_url(
    client: &Arc<reqwest::Client>,
    token_tracker: &Arc<DeezerTokenTracker>,
    tokens: &crate::sources::deezer::token::DeezerTokens,
    track_token: &str,
    effective_track_id: &str,
    arl_index: usize,
) -> Option<ResolvedUrl> {
    let body = serde_json::json!({
        "license_token": tokens.license_token,
        "media": [{ "type": "FULL", "formats": [
            { "cipher": "BF_CBC_STRIPE", "format": "MP3_128" },
            { "cipher": "BF_CBC_STRIPE", "format": "MP3_64" }
        ]}],
        "track_tokens": [track_token]
    });
    let json: serde_json::Value = match client
        .post("https://media.deezer.com/v1/get_url")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(_) => {
                token_tracker.invalidate_token(arl_index).await;
                return None;
            }
        },
        Err(e) => {
            debug!("Deezer: get_url failed: {e}");
            token_tracker.invalidate_token(arl_index).await;
            return None;
        }
    };
    if json
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|d| d.get("errors"))
        .and_then(|e| e.as_array())
        .is_some_and(|e| !e.is_empty())
    {
        debug!("Deezer: get_url returned errors");
        token_tracker.invalidate_token(arl_index).await;
        return None;
    }
    let cdn_url = json
        .get("data")?
        .get(0)?
        .get("media")?
        .get(0)?
        .get("sources")?
        .get(0)?
        .get("url")?
        .as_str()?
        .to_owned();
    Some(ResolvedUrl {
        cdn_url,
        track_id: effective_track_id.to_owned(),
        arl_index,
    })
}
pub(super) async fn verify_track_resolvable(
    client: &Arc<reqwest::Client>,
    track_id: &str,
    token_tracker: &DeezerTokenTracker,
) -> Option<String> {
    let tokens = token_tracker.get_token().await?;
    let song_url = format!(
        "https://www.deezer.com/ajax/gw-light.php?method=song.getData&input=3&api_version=1.0&api_token={}",
        tokens.api_token
    );
    let json: serde_json::Value = client
        .post(&song_url)
        .header(
            "Cookie",
            format!(
                "sid={}; dzr_uniq_id={}",
                tokens.session_id, tokens.dzr_uniq_id
            ),
        )
        .json(&serde_json::json!({ "sng_id": track_id }))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    if json
        .get("error")
        .and_then(|v| v.as_array())
        .is_some_and(|e| !e.is_empty())
    {
        token_tracker.invalidate_token(tokens.arl_index).await;
        return None;
    }
    let mut results = match json.get("results") {
        Some(r) => r.clone(),
        None => {
            token_tracker.invalidate_token(tokens.arl_index).await;
            return None;
        }
    };
    let rights = results.get("RIGHTS");
    if is_rights_empty(rights)
        && let Some(fallback) = results.get("FALLBACK")
        && !fallback
            .get("TRACK_TOKEN")
            .map(|v| v.is_null())
            .unwrap_or(true)
    {
        let has_id = fallback.get("SNG_ID").and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        });
        if has_id.is_some() {
            results = fallback.clone();
        } else {
            warn!(
                "Deezer: track {track_id} FALLBACK SNG_ID has unexpected format: {:?}",
                fallback.get("SNG_ID")
            );
        }
    }
    let track_token = results
        .get("TRACK_TOKEN")
        .and_then(|v| v.as_str())?
        .to_owned();
    let media_json: serde_json::Value = client
        .post("https://media.deezer.com/v1/get_url")
        .json(&serde_json::json!({
            "license_token": tokens.license_token,
            "media": [{ "type": "FULL", "formats": [
                { "cipher": "BF_CBC_STRIPE", "format": "MP3_128" }
            ]}],
            "track_tokens": [track_token]
        }))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    if media_json
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|d| d.get("errors"))
        .and_then(|e| e.as_array())
        .is_some_and(|e| !e.is_empty())
    {
        token_tracker.invalidate_token(tokens.arl_index).await;
        return None;
    }
    media_json
        .get("data")?
        .get(0)?
        .get("media")?
        .get(0)?
        .get("sources")?
        .get(0)?
        .get("url")?
        .as_str()
        .map(|s| s.to_owned())
}
fn is_rights_empty(rights: Option<&serde_json::Value>) -> bool {
    rights
        .map(|v| {
            v.as_array()
                .map(|a| a.is_empty())
                .or_else(|| v.as_object().map(|o| o.is_empty()))
                .unwrap_or(true)
        })
        .unwrap_or(true)
}
}
pub mod reader {
pub mod crypt {
use cbc::cipher::{BlockDecryptMut, KeyIvInit};
use md5::{Digest, Md5};
use tracing::warn;
type BlowfishCbc = cbc::Decryptor<blowfish::Blowfish>;
pub const CHUNK_SIZE: usize = 2048;
pub struct DeezerCrypt {
    key: [u8; 16],
}
impl DeezerCrypt {
    pub fn new(track_id: &str, master_key: &str) -> Self {
        let hash = Md5::digest(track_id.as_bytes());
        let hash_hex = hex::encode(hash);
        let hash_bytes = hash_hex.as_bytes();
        let master_bytes = master_key.as_bytes();
        let mut key = [0u8; 16];
        for i in 0..16 {
            key[i] = hash_bytes[i] ^ hash_bytes[i + 16] ^ master_bytes[i];
        }
        Self { key }
    }
    pub fn decrypt_chunk(&self, chunk_index: u64, chunk: &[u8], dest: &mut Vec<u8>) {
        if chunk_index.is_multiple_of(3) {
            let iv = [0, 1, 2, 3, 4, 5, 6, 7];
            let mut buffer = [0u8; CHUNK_SIZE];
            let len = std::cmp::min(chunk.len(), CHUNK_SIZE);
            buffer[..len].copy_from_slice(&chunk[..len]);
            if let Ok(cipher) = BlowfishCbc::new_from_slices(&self.key, &iv) {
                if let Ok(decrypted) =
                    cipher.decrypt_padded_mut::<cbc::cipher::block_padding::NoPadding>(&mut buffer)
                {
                    dest.extend_from_slice(decrypted);
                    return;
                } else {
                    warn!(
                        "Blowfish decryption failed for chunk {}, falling back to raw",
                        chunk_index
                    );
                }
            }
        }
        dest.extend_from_slice(chunk);
    }
}
}
pub mod remote_reader {
use std::io::{Read, Seek, SeekFrom};
use symphonia::core::io::MediaSource;
use crate::{
    audio::source::{AudioSource, HttpSource, create_client},
    common::types::AnyResult,
};
pub struct DeezerRemoteReader {
    inner: HttpSource,
}
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36";
impl DeezerRemoteReader {
    pub async fn new(
        url: &str,
        local_addr: Option<std::net::IpAddr>,
        proxy: Option<crate::config::HttpProxyConfig>,
    ) -> AnyResult<Self> {
        let client = create_client(USER_AGENT.to_owned(), local_addr, proxy, None)?;
        let inner = HttpSource::new(client, url).await?;
        Ok(Self { inner })
    }
}
impl Read for DeezerRemoteReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}
impl Seek for DeezerRemoteReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}
impl MediaSource for DeezerRemoteReader {
    fn is_seekable(&self) -> bool {
        self.inner.is_seekable()
    }
    fn byte_len(&self) -> Option<u64> {
        self.inner.byte_len()
    }
}
impl DeezerRemoteReader {
    pub fn content_type(&self) -> Option<String> {
        self.inner.content_type()
    }
}
}
use crate::common::types::AnyResult;
use std::io::{Read, Seek, SeekFrom};
use symphonia::core::io::MediaSource;
use tracing::debug;
use self::{
    crypt::{CHUNK_SIZE, DeezerCrypt},
    remote_reader::DeezerRemoteReader,
};
pub struct DeezerReader {
    source: DeezerRemoteReader,
    crypt: DeezerCrypt,
    pos: u64,
    raw_buf: Vec<u8>,
    ready_buf: Vec<u8>,
    skip_pending: usize,
}
impl DeezerReader {
    pub async fn new(
        url: &str,
        track_id: &str,
        master_key: &str,
        local_addr: Option<std::net::IpAddr>,
        proxy: Option<crate::config::HttpProxyConfig>,
    ) -> AnyResult<Self> {
        debug!("Initializing DeezerReader for track {}", track_id);
        let source = DeezerRemoteReader::new(url, local_addr, proxy).await?;
        let crypt = DeezerCrypt::new(track_id, master_key);
        Ok(Self {
            source,
            crypt,
            pos: 0,
            raw_buf: Vec::with_capacity(CHUNK_SIZE * 2),
            ready_buf: Vec::with_capacity(CHUNK_SIZE * 2),
            skip_pending: 0,
        })
    }
}
impl Read for DeezerReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.skip_pending > 0 && !self.ready_buf.is_empty() {
                let to_skip = std::cmp::min(self.skip_pending, self.ready_buf.len());
                self.ready_buf.drain(..to_skip);
                self.skip_pending -= to_skip;
            }
            if self.skip_pending == 0 && !self.ready_buf.is_empty() {
                let n = std::cmp::min(buf.len(), self.ready_buf.len());
                buf[..n].copy_from_slice(&self.ready_buf[..n]);
                self.ready_buf.drain(..n);
                return Ok(n);
            }
            let mut tmp = [0u8; CHUNK_SIZE];
            let n = self.source.read(&mut tmp)?;
            if n == 0 {
                if self.raw_buf.is_empty() {
                    return Ok(0);
                }
                let leftovers = self.raw_buf.clone();
                let chunk_idx = self.pos / CHUNK_SIZE as u64;
                self.crypt
                    .decrypt_chunk(chunk_idx, &leftovers, &mut self.ready_buf);
                self.pos += leftovers.len() as u64;
                self.raw_buf.clear();
                continue;
            }
            self.raw_buf.extend_from_slice(&tmp[..n]);
            while self.raw_buf.len() >= CHUNK_SIZE {
                let chunk: Vec<u8> = self.raw_buf.drain(..CHUNK_SIZE).collect();
                let chunk_idx = self.pos / CHUNK_SIZE as u64;
                self.crypt
                    .decrypt_chunk(chunk_idx, &chunk, &mut self.ready_buf);
                self.pos += CHUNK_SIZE as u64;
            }
        }
    }
}
impl Seek for DeezerReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::Current(0) => {
                let buffered = self.ready_buf.len() as u64 + self.raw_buf.len() as u64;
                return Ok(self.pos.saturating_sub(buffered));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Only SeekFrom::Start is supported",
                ));
            }
        };
        let aligned_pos = (target / CHUNK_SIZE as u64) * CHUNK_SIZE as u64;
        let skip = (target - aligned_pos) as usize;
        let new_pos = self.source.seek(SeekFrom::Start(aligned_pos))?;
        self.pos = new_pos;
        self.raw_buf.clear();
        self.ready_buf.clear();
        self.skip_pending = skip;
        Ok(target)
    }
}
impl MediaSource for DeezerReader {
    fn is_seekable(&self) -> bool {
        self.source.is_seekable()
    }
    fn byte_len(&self) -> Option<u64> {
        self.source.byte_len()
    }
}
}
use std::sync::{Arc, OnceLock};
use async_trait::async_trait;
use regex::Regex;
use token::DeezerTokenTracker;
use track::DeezerTrack;
use crate::{
    protocol::tracks::LoadResult,
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
const PUBLIC_API_BASE: &str = "https://api.deezer.com";
const PRIVATE_API_BASE: &str = "https://www.deezer.com/ajax/gw-light.php";
pub(crate) const REC_ARTIST_PREFIX: &str = "artist=";
pub(crate) const REC_TRACK_PREFIX: &str = "track=";
fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?deezer\.com/(?:[a-z]+(?:-[a-z]+)?/)?(?<type>track|album|playlist|artist)/(?<id>\d+)").unwrap()
    })
}
fn share_url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https?://(?:deezer\.page\.link|link\.deezer\.com)/\S*").unwrap()
    })
}
pub struct DeezerSource {
    client: Arc<reqwest::Client>,
    config: crate::config::DeezerConfig,
    pub token_tracker: Arc<DeezerTokenTracker>,
}
const DECRYPTION_KEY_HASH: [u8; 32] = [
    52, 76, 41, 138, 120, 133, 48, 72, 198, 74, 16, 75, 82, 101, 186, 223, 15, 190, 111, 218, 176,
    71, 103, 11, 181, 136, 155, 247, 66, 203, 218, 240,
];
impl DeezerSource {
    pub fn new(
        config: crate::config::DeezerConfig,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let mut arls = config.arls.clone().unwrap_or_default();
        arls.retain(|s| !s.is_empty());
        arls.sort();
        arls.dedup();
        if arls.is_empty() {
            return Err("Deezer arls must be set".to_owned());
        }
        if let Some(ref key) = config.master_decryption_key {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(key.as_bytes());
            if hasher.finalize().as_slice() != DECRYPTION_KEY_HASH {
                tracing::warn!("Deezer master decryption key is invalid, playback may not work!");
            }
        }
        let token_tracker = Arc::new(DeezerTokenTracker::new(client.clone(), arls));
        Ok(Self {
            client,
            config,
            token_tracker,
        })
    }
    async fn resolve_share_url(&self, identifier: &str) -> Option<String> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.6094.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .ok()?;
        let res = client.get(identifier).send().await.ok()?;
        if !res.status().is_redirection() {
            return None;
        }
        let loc = res.headers().get("location")?.to_str().ok()?;
        let mut url = loc.to_owned();
        if let Some(pos) = url.find("dest=") {
            let dest = &url[pos + 5..];
            let end = dest.find('&').unwrap_or(dest.len());
            if let Ok(decoded) = urlencoding::decode(&dest[..end]) {
                url = decoded.into_owned();
            }
        }
        if let Some(pos) = url.find('?') {
            url.truncate(pos);
        }
        if url.ends_with("/404") {
            return None;
        }
        Some(url)
    }
}
#[async_trait]
impl SourcePlugin for DeezerSource {
    fn name(&self) -> &str {
        "deezer"
    }
    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || self
                .isrc_prefixes()
                .iter()
                .any(|p| identifier.starts_with(p))
            || self
                .rec_prefixes()
                .iter()
                .any(|p| identifier.starts_with(p))
            || share_url_regex().is_match(identifier)
            || url_regex().is_match(identifier)
    }
    fn search_prefixes(&self) -> Vec<&str> {
        vec!["dzsearch:"]
    }
    fn isrc_prefixes(&self) -> Vec<&str> {
        vec!["dzisrc:"]
    }
    fn rec_prefixes(&self) -> Vec<&str> {
        vec!["dzrec:"]
    }
    async fn load(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        for prefix in self.search_prefixes() {
            if let Some(query) = identifier.strip_prefix(prefix) {
                return self.search(query).await;
            }
        }
        for prefix in self.isrc_prefixes() {
            if let Some(isrc) = identifier.strip_prefix(prefix) {
                if let Some(track) = self.get_track_by_isrc(isrc).await {
                    return LoadResult::Track(track);
                }
                return LoadResult::Empty {};
            }
        }
        for prefix in self.rec_prefixes() {
            if let Some(query) = identifier.strip_prefix(prefix) {
                return self.get_recommendations(query).await;
            }
        }
        if share_url_regex().is_match(identifier) {
            if let Some(resolved) = self.resolve_share_url(identifier).await {
                return self.load(&resolved, routeplanner).await;
            }
            return LoadResult::Empty {};
        }
        if let Some(caps) = url_regex().captures(identifier) {
            let type_ = caps.name("type").map(|m| m.as_str()).unwrap_or("");
            let id = caps.name("id").map(|m| m.as_str()).unwrap_or("");
            return match type_ {
                "track" => {
                    if let Some(json) = self.get_json_public(&format!("track/{id}")).await
                        && let Some(track) = self.parse_track(&json)
                    {
                        return LoadResult::Track(track);
                    }
                    LoadResult::Empty {}
                }
                "album" => self.get_album(id).await,
                "playlist" => self.get_playlist(id).await,
                "artist" => self.get_artist(id).await,
                _ => LoadResult::Empty {},
            };
        }
        LoadResult::Empty {}
    }
    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let track_id = if let Some(caps) = url_regex().captures(identifier) {
            caps.name("id").map(|m| m.as_str())?.to_owned()
        } else {
            identifier.to_owned()
        };
        let resolved =
            track::verify_track_resolvable(&self.client, &track_id, &self.token_tracker).await;
        if resolved.is_none() {
            tracing::warn!("Deezer: no stream URL for track {track_id}, falling back to mirrors");
            return None;
        }
        Some(Arc::new(DeezerTrack {
            client: self.client.clone(),
            track_id,
            token_tracker: self.token_tracker.clone(),
            master_key: self
                .config
                .master_decryption_key
                .clone()
                .unwrap_or_default(),
            local_addr: routeplanner.and_then(|rp| rp.get_address()),
            proxy: self.config.proxy.clone(),
        }))
    }
    async fn load_search(
        &self,
        query: &str,
        types: &[String],
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<crate::protocol::tracks::SearchResult> {
        let mut q = query;
        for prefix in self.search_prefixes() {
            if let Some(stripped) = query.strip_prefix(prefix) {
                q = stripped;
                break;
            }
        }
        self.get_autocomplete(q, types).await
    }
}