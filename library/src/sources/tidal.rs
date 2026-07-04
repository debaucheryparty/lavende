pub mod client {
use std::sync::Arc;
use serde_json::Value;
use super::{
    error::{TidalError, TidalResult},
    token::TidalTokenTracker,
};
const API_BASE: &str = "https://api.tidal.com/v1";
pub struct TidalClient {
    pub inner: Arc<reqwest::Client>,
    pub token_tracker: Arc<TidalTokenTracker>,
    pub country_code: String,
    pub quality: String,
}
impl TidalClient {
    pub fn new(
        inner: Arc<reqwest::Client>,
        token_tracker: Arc<TidalTokenTracker>,
        country_code: String,
        quality: String,
    ) -> Self {
        Self {
            inner,
            token_tracker,
            country_code,
            quality,
        }
    }
    pub fn base_request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .header(
                reqwest::header::USER_AGENT,
                "TIDAL/3704 CFNetwork/1220.1 Darwin/20.3.0",
            )
            .header("Accept-Language", "en-US")
    }
    pub async fn get_json(&self, path: &str) -> TidalResult<Value> {
        let url = if path.starts_with("http") {
            path.to_owned()
        } else {
            format!("{API_BASE}{path}")
        };
        let url = if !url.contains("countryCode=") {
            if url.contains('?') {
                format!("{url}&countryCode={}", self.country_code)
            } else {
                format!("{url}?countryCode={}", self.country_code)
            }
        } else {
            url
        };
        let mut req = self.base_request(self.inner.get(&url));
        if let Some(t) = self.token_tracker.get_scraper_token().await {
            req = req.header("x-tidal-token", t);
        } else if let Some(t) = self.token_tracker.get_oauth_token().await {
            req = req.header("Authorization", format!("Bearer {t}"));
        } else {
            return Err(TidalError::NoToken);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(TidalError::ApiError { status, body });
        }
        Ok(resp.json().await?)
    }
}
}
pub mod error {
use thiserror::Error;
#[derive(Debug, Error)]
pub enum TidalError {
    #[error("API request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("API returned {status}: {body}")]
    ApiError {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("Failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Authentication failed: {0}")]
    Auth(String),
    #[error("No token available")]
    NoToken,
    #[error("No stream URL in manifest")]
    NoStreamUrl,
    #[error("Other error: {0}")]
    Other(String),
}
pub type TidalResult<T> = Result<T, TidalError>;
}
pub mod manager {
use std::sync::{Arc, OnceLock};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use regex::Regex;
use serde_json::Value;
use tracing::{debug, warn};
use super::{
    client::TidalClient,
    model::{Manifest, PlaybackInfo},
    oauth::TidalOAuth,
    token::TidalTokenTracker,
    track::TidalTrack,
};
use crate::{
    common::types::AudioFormat,
    protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track, TrackInfo},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
fn url_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"https?://(?:(?:listen|www)\.)?tidal\.com/(?:browse/)?(album|track|playlist|mix|artist)/([a-zA-Z0-9\-]+)(?:/.*)?(?:\?.*)?").unwrap()
    })
}
pub struct TidalSource {
    pub client: Arc<TidalClient>,
    playlist_load_limit: usize,
    album_load_limit: usize,
    artist_load_limit: usize,
}
impl TidalSource {
    pub fn new(
        config: Option<crate::config::TidalConfig>,
        http_client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let (country, quality, p_limit, a_limit, art_limit, refresh_token, get_oauth_token) =
            if let Some(c) = config {
                (
                    c.country_code,
                    c.quality,
                    c.playlist_load_limit,
                    c.album_load_limit,
                    c.artist_load_limit,
                    c.refresh_token,
                    c.get_oauth_token,
                )
            } else {
                (
                    "US".to_string(),
                    crate::config::sources::default_tidal_quality(),
                    0,
                    0,
                    0,
                    None,
                    false,
                )
            };
        let oauth = Arc::new(TidalOAuth::new(refresh_token));
        if get_oauth_token {
            let oauth_clone = oauth.clone();
            tokio::spawn(async move {
                oauth_clone.initialize_access_token().await;
            });
        }
        let token_tracker = Arc::new(TidalTokenTracker::new(http_client.clone(), oauth));
        token_tracker.clone().init();
        let client = Arc::new(TidalClient::new(
            http_client,
            token_tracker,
            country,
            quality,
        ));
        Ok(Self {
            client,
            playlist_load_limit: p_limit,
            album_load_limit: a_limit,
            artist_load_limit: art_limit,
        })
    }
    fn parse_track(&self, item: &Value) -> Option<TrackInfo> {
        let id = item.get("id")?.as_u64()?.to_string();
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Title")
            .to_string();
        let artists = item
            .get("artists")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.get("name").and_then(|n| n.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "Unknown Artist".to_owned());
        let length = item.get("duration").and_then(|v| v.as_u64()).unwrap_or(0) * 1000;
        let isrc = item
            .get("isrc")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned());
        let artwork_url = item
            .get("album")
            .and_then(|a| a.get("cover"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| {
                format!(
                    "https://resources.tidal.com/images/{}/1280x1280.jpg",
                    s.replace("-", "/")
                )
            });
        let url = item
            .get("url")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.replace("http://", "https://"));
        Some(TrackInfo {
            title,
            author: artists,
            length,
            identifier: id,
            is_stream: false,
            uri: url,
            artwork_url,
            isrc,
            source_name: "tidal".to_owned(),
            is_seekable: true,
            position: 0,
        })
    }
    async fn get_track_data(&self, id: &str) -> LoadResult {
        match self.client.get_json(&format!("/tracks/{id}")).await {
            Ok(data) => self
                .parse_track(&data)
                .map(|i| LoadResult::Track(Track::new(i)))
                .unwrap_or(LoadResult::Empty {}),
            Err(_) => LoadResult::Empty {},
        }
    }
    async fn get_album_or_playlist(&self, id: &str, type_str: &str) -> LoadResult {
        let info_data = match self.client.get_json(&format!("/{type_str}s/{id}")).await {
            Ok(d) => d,
            Err(_) => return LoadResult::Empty {},
        };
        let title = info_data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_owned();
        let limit = (if type_str == "album" {
            self.album_load_limit
        } else {
            self.playlist_load_limit
        })
        .clamp(1, 100);
        let tracks_data = match self
            .client
            .get_json(&format!("/{type_str}s/{id}/tracks?limit={limit}"))
            .await
        {
            Ok(d) => d,
            Err(_) => return LoadResult::Empty {},
        };
        let mut tracks = Vec::new();
        if let Some(items) = tracks_data.get("items").and_then(|v| v.as_array()) {
            for item in items {
                let track_obj = item.get("item").unwrap_or(item);
                if let Some(info) = self.parse_track(track_obj) {
                    tracks.push(Track::new(info));
                }
            }
        }
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: title,
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
                "type": type_str,
                "url": format!("https://tidal.com/browse/{type_str}/{id}"),
                "totalTracks": info_data.get("numberOfTracks").or_else(|| info_data.get("numberOfSongs")).and_then(|v| v.as_u64()).unwrap_or(tracks.len() as u64)
            }),
            tracks,
        })
    }
    async fn get_mix(&self, id: &str, name_override: Option<String>) -> LoadResult {
        let data = match self
            .client
            .get_json(&format!("/mixes/{id}/items?limit=100"))
            .await
        {
            Ok(d) => d,
            Err(_) => return LoadResult::Empty {},
        };
        let mut tracks = Vec::new();
        if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
            for item in items {
                let track_obj = item.get("item").unwrap_or(item);
                if let Some(info) = self.parse_track(track_obj) {
                    tracks.push(Track::new(info));
                }
            }
        }
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: name_override.unwrap_or_else(|| format!("Mix: {id}")),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({ "type": "playlist", "url": format!("https://tidal.com/browse/mix/{id}"), "totalTracks": tracks.len() }),
            tracks,
        })
    }
    async fn resolve_by_isrc(&self, isrc: &str) -> LoadResult {
        let token = match self.client.token_tracker.get_oauth_token().await {
            Some(t) => t,
            None => {
                warn!("Tidal ISRC lookup requires an OAuth token; none configured");
                return LoadResult::Empty {};
            }
        };
        let url = format!(
            "https://openapi.tidal.com/v2/tracks?countryCode={}",
            self.client.country_code
        );
        let resp = self
            .client
            .base_request(self.client.inner.get(&url))
            .query(&[("filter[isrc]", isrc)])
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await;
        let r = match resp {
            Ok(r) => r,
            Err(e) => {
                debug!("Tidal ISRC request failed: {e}");
                return LoadResult::Empty {};
            }
        };
        if !r.status().is_success() {
            debug!(
                "Tidal ISRC response {}: {}",
                r.status(),
                r.text().await.unwrap_or_default()
            );
            return LoadResult::Empty {};
        }
        let data: serde_json::Value = match r.json().await {
            Ok(d) => d,
            Err(e) => {
                debug!("Tidal ISRC response parse error: {e}");
                return LoadResult::Empty {};
            }
        };
        match data.pointer("/data/0/id").and_then(|v| v.as_str()) {
            Some(id) => self.get_track_data(id).await,
            None => {
                debug!("Tidal ISRC={isrc} not found in catalog");
                LoadResult::Empty {}
            }
        }
    }
    async fn search(&self, query: &str) -> LoadResult {
        let encoded = urlencoding::encode(query);
        match self
            .client
            .get_json(&format!("/search?query={encoded}&limit=10&types=TRACKS"))
            .await
        {
            Ok(data) => {
                let mut tracks = Vec::new();
                if let Some(items) = data.pointer("/tracks/items").and_then(|v| v.as_array()) {
                    for item in items {
                        if let Some(info) = self.parse_track(item) {
                            tracks.push(Track::new(info));
                        }
                    }
                }
                if tracks.is_empty() {
                    LoadResult::Empty {}
                } else {
                    LoadResult::Search(tracks)
                }
            }
            Err(_) => LoadResult::Empty {},
        }
    }
    async fn get_recommendations(&self, id: &str) -> LoadResult {
        if let Ok(data) = self.client.get_json(&format!("/tracks/{id}")).await
            && let Some(mix_id) = data.pointer("/mixes/TRACK_MIX").and_then(|v| v.as_str())
        {
            return self
                .get_mix(mix_id, Some("Tidal Recommendations".to_string()))
                .await;
        }
        LoadResult::Empty {}
    }
    async fn get_artist_top_tracks(&self, id: &str) -> LoadResult {
        let info_data = match self.client.get_json(&format!("/artists/{id}")).await {
            Ok(d) => d,
            Err(_) => return LoadResult::Empty {},
        };
        let name = info_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Artist");
        let limit = self.artist_load_limit.clamp(1, 10);
        let data = match self
            .client
            .get_json(&format!("/artists/{id}/toptracks?limit={limit}"))
            .await
        {
            Ok(d) => d,
            Err(_) => return LoadResult::Empty {},
        };
        let mut tracks = Vec::new();
        if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(info) = self.parse_track(item) {
                    tracks.push(Track::new(info));
                }
            }
        }
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: format!("{name}'s Top Tracks"),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({ "type": "artist", "url": format!("https://tidal.com/browse/artist/{id}"), "totalTracks": tracks.len() }),
            tracks,
        })
    }
}
#[async_trait]
impl SourcePlugin for TidalSource {
    fn name(&self) -> &str {
        "tidal"
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
            || url_regex().is_match(identifier)
    }
    fn search_prefixes(&self) -> Vec<&str> {
        vec!["tdsearch:"]
    }
    fn isrc_prefixes(&self) -> Vec<&str> {
        vec!["tdisrc:"]
    }
    fn rec_prefixes(&self) -> Vec<&str> {
        vec!["tdrec:"]
    }
    fn is_mirror(&self) -> bool {
        false
    }
    async fn load(
        &self,
        identifier: &str,
        _: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        if let Some(prefix) = self
            .search_prefixes()
            .iter()
            .find(|p| identifier.starts_with(**p))
        {
            return self.search(&identifier[prefix.len()..]).await;
        }
        if let Some(prefix) = self
            .isrc_prefixes()
            .iter()
            .find(|p| identifier.starts_with(**p))
        {
            return self.resolve_by_isrc(&identifier[prefix.len()..]).await;
        }
        if let Some(prefix) = self
            .rec_prefixes()
            .iter()
            .find(|p| identifier.starts_with(**p))
        {
            return self.get_recommendations(&identifier[prefix.len()..]).await;
        }
        if let Some(caps) = url_regex().captures(identifier) {
            let type_str = caps.get(1).map_or("", |m| m.as_str());
            let id = caps.get(2).map_or("", |m| m.as_str());
            return match type_str {
                "track" => self.get_track_data(id).await,
                "album" | "playlist" => self.get_album_or_playlist(id, type_str).await,
                "mix" => self.get_mix(id, None).await,
                "artist" => self.get_artist_top_tracks(id).await,
                _ => LoadResult::Empty {},
            };
        }
        LoadResult::Empty {}
    }
    async fn get_track(
        &self,
        identifier: &str,
        _: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let id = if let Some(caps) = url_regex().captures(identifier) {
            let type_str = caps.get(1).map_or("", |m| m.as_str());
            let id = caps.get(2).map_or("", |m| m.as_str());
            if type_str != "track" {
                return None;
            }
            id.to_owned()
        } else {
            identifier.to_owned()
        };
        let token = match self.client.token_tracker.get_oauth_token().await {
            Some(t) => t,
            None => {
                warn!("Tidal playback requires an OAuth login");
                return None;
            }
        };
        let quality = &self.client.quality;
        let url = format!(
            "https://api.tidal.com/v1/tracks/{}/playbackinfo?audioquality={}&playbackmode=STREAM&assetpresentation=FULL&countryCode={}",
            id, quality, self.client.country_code
        );
        debug!("Tidal: Resolving playback info for {}", id);
        let resp = match self
            .client
            .inner
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("User-Agent", "TIDAL/3704 CFNetwork/1220.1 Darwin/20.3.0")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("Tidal: Failed to fetch playback info: {}", e);
                return None;
            }
        };
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("Tidal: Playback API returned {}: {}", status, body);
            return None;
        }
        let info: PlaybackInfo = match resp.json().await {
            Ok(i) => i,
            Err(e) => {
                warn!("Tidal: Failed to parse playback info: {}", e);
                return None;
            }
        };
        let decoded = match general_purpose::STANDARD.decode(&info.manifest) {
            Ok(d) => d,
            Err(e) => {
                warn!("Tidal: Failed to decode manifest: {}", e);
                return None;
            }
        };
        let manifest: Manifest = match serde_json::from_slice(&decoded) {
            Ok(m) => m,
            Err(e) => {
                warn!("Tidal: Failed to parse manifest JSON: {}", e);
                return None;
            }
        };
        let stream_url = match manifest.urls.first() {
            Some(u) => u.clone(),
            None => {
                warn!("Tidal: No stream URL in manifest");
                return None;
            }
        };
        let mut kind = AudioFormat::from_url(&stream_url);
        if kind == AudioFormat::Unknown {
            if quality == "HI_RES_LOSSLESS" {
                kind = AudioFormat::Flac;
            } else if quality == "LOSSLESS" {
                kind = AudioFormat::Mp4;
            } else {
                kind = AudioFormat::Aac;
            }
        }
        Some(Arc::new(TidalTrack {
            identifier: id,
            stream_url,
            kind,
            client: self.client.clone(),
        }))
    }
}
}
pub mod model {
use serde::Deserialize;
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackInfo {
    pub manifest: String,
    pub manifest_mime_type: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub urls: Vec<String>,
    pub mime_type: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}
#[derive(Clone, Debug)]
pub struct TidalToken {
    pub access_token: String,
    pub expiry_ms: u64,
}
#[derive(Clone, Debug)]
pub enum TidalAuthToken {
    OAuth(String),
    Scraper(String),
}
impl TidalAuthToken {
    pub fn value(&self) -> &str {
        match self {
            Self::OAuth(s) | Self::Scraper(s) => s,
        }
    }
}
}
pub mod oauth {
use std::{sync::Arc, time::Duration};
use reqwest::Client;
use tokio::sync::RwLock;
use tracing::{error, info};
use super::model::{DeviceAuthResponse, TokenResponse};
const CLIENT_ID: &str = "fX2JxdmntZWK0ixT";
const CLIENT_SECRET: &str = "1Nn9AfDAjxrgJFJbKNWLeAyKGVGmINuXPPLHVXAvxAg=";
pub struct TidalOAuth {
    pub client: Client,
    pub access_token: RwLock<Option<String>>,
    pub refresh_token: RwLock<Option<String>>,
}
impl TidalOAuth {
    pub fn new(refresh_token: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("TIDAL/3704 CFNetwork/1220.1 Darwin/20.3.0")
                .build()
                .unwrap_or_default(),
            access_token: RwLock::new(None),
            refresh_token: RwLock::new(refresh_token),
        }
    }
    pub async fn get_access_token(&self) -> Option<String> {
        if let Some(token) = &*self.access_token.read().await {
            return Some(token.clone());
        }
        if self.refresh_oauth_token().await.is_ok() {
            return self.access_token.read().await.clone();
        }
        None
    }
    pub async fn get_refresh_token(&self) -> Option<String> {
        self.refresh_token.read().await.clone()
    }
    pub async fn initialize_access_token(self: Arc<Self>) {
        if self.refresh_token.read().await.is_some() {
            return;
        }
        info!("Starting Tidal device authorization flow...");
        let form = [("client_id", CLIENT_ID), ("scope", "r_usr w_usr w_sub")];
        let resp = match self
            .client
            .post("https://auth.tidal.com/v1/oauth2/device_authorization")
            .form(&form)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                error!("Tidal device authorization request failed: {}", e);
                return;
            }
        };
        if !resp.status().is_success() {
            error!("Tidal device authorization failed: {}", resp.status());
            return;
        }
        let data: DeviceAuthResponse = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse device auth response: {}", e);
                return;
            }
        };
        crate::log_println!(
            "\n  ┌────────────────────────────────────────────────────────────┐\n  │                     TIDAL OAUTH LOGIN                      │\n  ├────────────────────────────────────────────────────────────┤\n  │ 1. Visit: {:<48} │\n  │ 2. Log in and authorize the application.                   │\n  └────────────────────────────────────────────────────────────┘\n",
            data.verification_uri_complete
        );
        let oauth = self.clone();
        tokio::spawn(async move {
            oauth.poll_token(data.device_code, data.interval).await;
        });
    }
    async fn poll_token(&self, device_code: String, interval: u64) {
        let mut interval_timer = tokio::time::interval(Duration::from_secs(interval.max(1)));
        loop {
            interval_timer.tick().await;
            let form = [
                ("client_id", CLIENT_ID),
                ("device_code", &device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("scope", "r_usr w_usr w_sub"),
            ];
            let resp = match self
                .client
                .post("https://auth.tidal.com/v1/oauth2/token")
                .basic_auth(CLIENT_ID, Some(CLIENT_SECRET))
                .form(&form)
                .send()
                .await
            {
                Ok(r) => r,
                Err(_) => continue,
            };
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<TokenResponse>().await {
                    let mut at_lock = self.access_token.write().await;
                    let mut rt_lock = self.refresh_token.write().await;
                    *at_lock = Some(data.access_token);
                    *rt_lock = data.refresh_token;
                    info!("Successfully authorized Tidal OAuth");
                    if let Some(ref rt) = *rt_lock {
                        info!("Tidal Refresh Token: {}", rt);
                    }
                    return;
                }
            } else if let Ok(body) = resp.json::<serde_json::Value>().await {
                let error = body["error"].as_str().unwrap_or_default();
                match error {
                    "authorization_pending" => continue,
                    "slow_down" => {
                        interval_timer = tokio::time::interval(Duration::from_secs(interval + 3));
                    }
                    _ => {
                        error!("Tidal OAuth polling failed: {}", error);
                        return;
                    }
                }
            }
        }
    }
    pub async fn refresh_oauth_token(&self) -> Result<(), String> {
        let refresh_token = self.get_refresh_token().await;
        let rt = match refresh_token {
            Some(t) => t,
            None => return Err("No refresh token available".to_string()),
        };
        info!("Refreshing Tidal OAuth token...");
        let form = [
            ("client_id", CLIENT_ID),
            ("refresh_token", &rt),
            ("grant_type", "refresh_token"),
            ("scope", "r_usr w_usr w_sub"),
        ];
        let resp = self
            .client
            .post("https://auth.tidal.com/v1/oauth2/token")
            .basic_auth(CLIENT_ID, Some(CLIENT_SECRET))
            .form(&form)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("Tidal token refresh failed ({}): {}", status, body);
            return Err(format!("Refresh failed: {}", status));
        }
        let data: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
        let mut at_lock = self.access_token.write().await;
        let mut rt_lock = self.refresh_token.write().await;
        *at_lock = Some(data.access_token);
        if data.refresh_token.is_some() {
            *rt_lock = data.refresh_token;
        }
        info!("Successfully refreshed Tidal OAuth token");
        Ok(())
    }
}
}
pub mod token {
use std::sync::{Arc, OnceLock};
use regex::Regex;
use tokio::sync::RwLock;
use tracing::{error, info};
use super::{model::TidalToken, oauth::TidalOAuth};
fn script_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"src="(/assets/index-[^"]+\.js)""#).unwrap())
}
fn client_id_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"clientId\s*[:=]\s*"([^"]+)""#).unwrap())
}
pub struct TidalTokenTracker {
    pub token: RwLock<Option<TidalToken>>,
    pub client: Arc<reqwest::Client>,
    pub oauth: Arc<TidalOAuth>,
}
impl TidalTokenTracker {
    pub fn new(client: Arc<reqwest::Client>, oauth: Arc<TidalOAuth>) -> Self {
        Self {
            token: RwLock::new(None),
            client,
            oauth,
        }
    }
    pub async fn get_scraper_token(&self) -> Option<String> {
        {
            let lock = self.token.read().await;
            if let Some(token) = &*lock
                && self.is_valid(token)
            {
                return Some(token.access_token.clone());
            }
        }
        self.refresh_token().await
    }
    pub async fn get_oauth_token(&self) -> Option<String> {
        self.oauth.get_access_token().await
    }
    fn is_valid(&self, token: &TidalToken) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        token.expiry_ms > now + 10_000
    }
    async fn refresh_token(&self) -> Option<String> {
        info!("Fetching new Tidal API token via scraper...");
        let listen_url = "https://listen.tidal.com";
        let resp = self.client.get(listen_url).send().await.ok()?;
        if !resp.status().is_success() {
            error!("Tidal listen page returned status: {}", resp.status());
            return None;
        }
        let html = resp.text().await.unwrap_or_default();
        let script_path = script_regex().captures(&html)?.get(1)?.as_str();
        let script_url = format!("https://listen.tidal.com{}", script_path);
        let js_resp = self.client.get(&script_url).send().await.ok()?;
        let js_content = js_resp.text().await.unwrap_or_default();
        let mut matches = client_id_regex().captures_iter(&js_content);
        matches.next(); 
        let token_str = matches.next()?.get(1)?.as_str().to_owned();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let token = TidalToken {
            access_token: token_str.clone(),
            expiry_ms: now + (24 * 60 * 60 * 1000),
        };
        let mut lock = self.token.write().await;
        *lock = Some(token);
        info!("Successfully refreshed Tidal scraper token");
        Some(token_str)
    }
    pub fn init(self: Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            this.get_scraper_token().await;
        });
    }
    pub async fn has_oauth_refresh_token(&self) -> bool {
        self.oauth.get_refresh_token().await.is_some()
    }
}
}
pub mod track {
use std::sync::Arc;
use async_trait::async_trait;
use tracing::debug;
use super::client::TidalClient;
use crate::{
    audio::source::HttpSource,
    common::types::AudioFormat,
    sources::playable_track::{PlayableTrack, ResolvedTrack},
};
pub struct TidalTrack {
    pub identifier: String,
    pub stream_url: String,
    pub kind: AudioFormat,
    pub client: Arc<TidalClient>,
}
#[async_trait]
impl PlayableTrack for TidalTrack {
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        debug!(
            "TidalTrack: resolving {} with quality {}",
            self.identifier, self.client.quality
        );
        let client_inner = (*self.client.inner).clone();
        let stream_url = self.stream_url.clone();
        let kind = self.kind;
        let reader = HttpSource::new(client_inner, &stream_url)
            .await
            .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
            .map_err(|e| format!("Failed to initialize source: {e}"))?;
        Ok(ResolvedTrack::new(reader, Some(kind)))
    }
}
}
pub use manager::TidalSource;