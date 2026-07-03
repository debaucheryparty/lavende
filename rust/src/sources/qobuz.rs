pub mod manager {
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{debug, error};
use crate::{
    common::types::AnyResult,
    config::AppConfig,
    protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track, TrackInfo},
    sources::{
        SourcePlugin,
        playable_track::BoxedTrack,
        qobuz::{token::QobuzTokenTracker, track::QobuzTrack},
    },
};
const API_URL: &str = "https://www.qobuz.com/api.json/0.2/";
fn url_regex() -> &'static regex::Regex {
    static REGEX: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| {
        regex::Regex::new(
            r"https?://(?:www\.|play\.|open\.)?qobuz\.com/(?:(?:[a-z]{2}-[a-z]{2}/)?(?P<type>album|playlist|track|artist)/(?:.+?/)?(?P<id>[a-zA-Z0-9]+)|(?P<type2>playlist)/(?P<id2>\d+))"
        ).unwrap()
    })
}
pub struct QobuzSource {
    client: Arc<reqwest::Client>,
    token_tracker: Arc<QobuzTokenTracker>,
    search_limit: usize,
    album_load_limit: usize,
    playlist_load_limit: usize,
    artist_load_limit: usize,
}
impl QobuzSource {
    pub fn new(config: &AppConfig, client: Arc<reqwest::Client>) -> Result<Self, String> {
        let qobuz_config = config.sources.qobuz.clone().unwrap_or_default();
        let tracker = Arc::new(QobuzTokenTracker::new(
            client.clone(),
            qobuz_config.user_token,
            qobuz_config.app_id,
            qobuz_config.app_secret,
        ));
        tracker.clone().init();
        Ok(Self {
            client,
            token_tracker: tracker,
            search_limit: qobuz_config.search_limit,
            album_load_limit: qobuz_config.album_load_limit,
            playlist_load_limit: qobuz_config.playlist_load_limit,
            artist_load_limit: qobuz_config.artist_load_limit,
        })
    }
    pub async fn api_request(&self, path: &str, params: Vec<(&str, String)>) -> AnyResult<Value> {
        let tokens = self
            .token_tracker
            .get_tokens()
            .await
            .ok_or("Failed to get Qobuz tokens")?;
        let mut url = reqwest::Url::parse(&format!("{API_URL}{path}"))?;
        {
            let mut query = url.query_pairs_mut();
            for (k, v) in params {
                query.append_pair(k, &v);
            }
        }
        let mut request = self
            .base_request(self.client.get(url))
            .header("Accept", "application/json")
            .header("x-app-id", &tokens.app_id);
        if let Some(user_token) = &tokens.user_token {
            request = request.header("x-user-auth-token", user_token);
        }
        let resp = request.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Qobuz API error ({status}): {body}").into());
        }
        let json: Value = resp.json().await?;
        Ok(json)
    }
    pub fn base_request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36")
    }
    pub fn parse_qobuz_track(&self, json: &Value) -> QobuzTrack {
        let identifier = json["id"]
            .as_i64()
            .or_else(|| json["id"].as_str().and_then(|s| s.parse::<i64>().ok()))
            .unwrap_or(0)
            .to_string();
        let title = json["title"].as_str().unwrap_or("Unknown Title").to_owned();
        let (author, artist_url) = if !json["artist"].is_null() && json["artist"].is_object() {
            let name = json["artist"]["name"]["display"]
                .as_str()
                .or_else(|| json["artist"]["name"].as_str())
                .unwrap_or("Unknown Artist")
                .to_owned();
            let url = json["artist"]["id"]
                .as_i64()
                .map(|id| format!("https://open.qobuz.com/artist/{id}"));
            (name, url)
        } else {
            let name = json["album"]["artist"]["name"]
                .as_str()
                .unwrap_or("Unknown Artist")
                .to_owned();
            let url = json["album"]["artist"]["id"]
                .as_i64()
                .map(|id| format!("https://open.qobuz.com/artist/{id}"));
            (name, url)
        };
        let length = json["duration"].as_i64().unwrap_or(0) * 1000;
        let artwork_url = json["album"]["image"]["large"]
            .as_str()
            .map(|s| s.to_owned());
        let isrc = json["isrc"].as_str().map(|s| s.to_owned());
        let uri = format!("https://open.qobuz.com/track/{identifier}");
        let album_name = json["album"]["title"].as_str().map(|s| s.to_owned());
        let album_url = json["album"]["id"]
            .as_i64()
            .map(|id| format!("https://open.qobuz.com/album/{id}"));
        let artist_artwork_url =
            if !json["album"]["artist"].is_null() && !json["album"]["artist"]["image"].is_null() {
                json["album"]["artist"]["image"]
                    .as_str()
                    .map(|s| s.to_owned())
            } else {
                None
            };
        QobuzTrack {
            info: TrackInfo {
                identifier,
                is_seekable: true,
                author,
                length: length as u64,
                is_stream: false,
                position: 0,
                title,
                uri: Some(uri),
                artwork_url,
                isrc,
                source_name: "qobuz".to_owned(),
            },
            album_name,
            album_url,
            artist_url,
            artist_artwork_url,
            token_tracker: self.token_tracker.clone(),
            client: self.client.clone(),
        }
    }
    async fn handle_search(&self, query: &str) -> LoadResult {
        match self
            .api_request(
                "catalog/search",
                vec![
                    ("query", query.to_owned()),
                    ("limit", self.search_limit.to_string()),
                    ("type", "tracks".to_owned()),
                ],
            )
            .await
        {
            Ok(json) => {
                let items = json["tracks"]["items"].as_array();
                if items.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
                    return LoadResult::Empty {};
                }
                let tracks: Vec<Track> = items
                    .unwrap()
                    .iter()
                    .map(|item| Track::new(self.parse_qobuz_track(item).info))
                    .collect();
                LoadResult::Search(tracks)
            }
            Err(e) => {
                error!("Qobuz search error: {e}");
                LoadResult::Empty {}
            }
        }
    }
    async fn handle_isrc(&self, isrc: &str) -> LoadResult {
        match self
            .api_request(
                "catalog/search",
                vec![
                    ("query", isrc.to_owned()),
                    ("limit", "15".to_owned()),
                    ("type", "tracks".to_owned()),
                ],
            )
            .await
        {
            Ok(json) => {
                let items = json["tracks"]["items"].as_array();
                if items.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
                    return LoadResult::Empty {};
                }
                let track = Track::new(self.parse_qobuz_track(&items.unwrap()[0]).info);
                LoadResult::Track(track)
            }
            Err(e) => {
                error!("Qobuz ISRC search error: {e}");
                LoadResult::Empty {}
            }
        }
    }
    async fn handle_recommendations(&self, track_id: &str) -> LoadResult {
        let track_json = match self
            .api_request("track/get", vec![("track_id", track_id.to_owned())])
            .await
        {
            Ok(j) => j,
            Err(_) => return LoadResult::Empty {},
        };
        let artist_id = track_json["performer"]["id"]
            .as_i64()
            .or_else(|| track_json["artist"]["id"].as_i64())
            .unwrap_or(0);
        let track_id_i64 = track_id.parse::<i64>().unwrap_or(0);
        let payload = json!({
            "limit": 50,
            "listened_tracks_ids": [track_id_i64],
            "track_to_analysed": [
                {
                    "track_id": track_id_i64,
                    "artist_id": artist_id
                }
            ]
        });
        let tokens = match self.token_tracker.get_tokens().await {
            Some(t) => t,
            None => return LoadResult::Empty {},
        };
        let mut request = self
            .base_request(self.client.post(format!("{API_URL}dynamic/suggest")))
            .header("Accept", "application/json")
            .header("x-app-id", &tokens.app_id)
            .json(&payload);
        if let Some(user_token) = &tokens.user_token {
            request = request.header("x-user-auth-token", user_token);
        }
        let resp = match request.send().await {
            Ok(r) => r,
            Err(e) => {
                error!("Qobuz recommendations request error: {e}");
                return LoadResult::Empty {};
            }
        };
        if !resp.status().is_success() {
            return LoadResult::Empty {};
        }
        let json: Value = resp.json().await.unwrap_or(json!({}));
        let items = json["tracks"]["items"].as_array();
        if items.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
            return LoadResult::Empty {};
        }
        let tracks: Vec<Track> = items
            .unwrap()
            .iter()
            .map(|item| Track::new(self.parse_qobuz_track(item).info))
            .collect();
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: "Qobuz Recommendations".to_owned(),
                selected_track: -1,
            },
            plugin_info: json!({
                "type": "recommendations",
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }
    async fn handle_album(&self, id: &str) -> LoadResult {
        match self
            .api_request(
                "album/get",
                vec![
                    ("album_id", id.to_owned()),
                    ("limit", self.album_load_limit.to_string()),
                    ("offset", "0".to_owned()),
                ],
            )
            .await
        {
            Ok(mut json) => {
                let title = json["title"].as_str().unwrap_or("Unknown Album").to_owned();
                let author = json["artist"]["name"]
                    .as_str()
                    .or_else(|| json["artist"]["name"]["display"].as_str())
                    .unwrap_or("Unknown Artist")
                    .to_owned();
                let artwork_url = json["image"]["large"].as_str().map(|s| s.to_owned());
                let uri = format!("https://open.qobuz.com/album/{id}");
                let tracks_json = json["tracks"]["items"].take();
                if tracks_json
                    .as_array()
                    .as_ref()
                    .map(|a| a.is_empty())
                    .unwrap_or(true)
                {
                    return LoadResult::Empty {};
                }
                let tracks: Vec<Track> = tracks_json
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| {
                        let mut item = item.clone();
                        item["album"] = json.clone();
                        Track::new(self.parse_qobuz_track(&item).info)
                    })
                    .collect();
                let track_count = tracks.len();
                LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name: title,
                        selected_track: -1,
                    },
                    plugin_info: json!({
                        "type": "album",
                        "url": uri,
                        "artworkUrl": artwork_url,
                        "author": author,
                        "totalTracks": track_count
                    }),
                    tracks,
                })
            }
            Err(_) => LoadResult::Empty {},
        }
    }
    async fn handle_playlist(&self, id: &str) -> LoadResult {
        match self
            .api_request(
                "playlist/get",
                vec![
                    ("playlist_id", id.to_owned()),
                    ("extra", "tracks".to_owned()),
                    ("limit", self.playlist_load_limit.to_string()),
                    ("offset", "0".to_owned()),
                ],
            )
            .await
        {
            Ok(json) => {
                let items = json["tracks"]["items"].as_array();
                if items.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
                    return LoadResult::Empty {};
                }
                let name = json["name"]
                    .as_str()
                    .unwrap_or("Unknown Playlist")
                    .to_owned();
                let author = json["owner"]["name"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_owned();
                let artwork_url = json["images300"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned());
                let url = json["url"]
                    .as_str()
                    .map(|s| s.to_owned())
                    .unwrap_or_else(|| format!("https://open.qobuz.com/playlist/{id}"));
                let tracks: Vec<Track> = items
                    .unwrap()
                    .iter()
                    .map(|item| Track::new(self.parse_qobuz_track(item).info))
                    .collect();
                let track_count = tracks.len();
                LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name,
                        selected_track: -1,
                    },
                    plugin_info: json!({
                        "type": "playlist",
                        "url": url,
                        "artworkUrl": artwork_url,
                        "author": author,
                        "totalTracks": track_count
                    }),
                    tracks,
                })
            }
            Err(_) => LoadResult::Empty {},
        }
    }
    async fn handle_artist(&self, id: &str) -> LoadResult {
        match self
            .api_request("artist/page", vec![("artist_id", id.to_owned())])
            .await
        {
            Ok(json) => {
                let top_tracks = json["top_tracks"].as_array();
                if top_tracks.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
                    return LoadResult::Empty {};
                }
                let name = json["name"]["display"]
                    .as_str()
                    .or_else(|| json["name"].as_str())
                    .unwrap_or("Unknown Artist")
                    .to_owned();
                let artwork_url = json["images"]["potrait"]["hash"]
                    .as_str()
                    .filter(|h| !h.is_empty())
                    .map(|h| {
                        format!("https://static.qobuz.com/images/artists/covers/large/{h}.jpg")
                    });
                let uri = format!("https://open.qobuz.com/artist/{id}");
                let tracks: Vec<Track> = top_tracks
                    .unwrap()
                    .iter()
                    .take(self.artist_load_limit)
                    .map(|item| Track::new(self.parse_qobuz_track(item).info))
                    .collect();
                let track_count = tracks.len();
                LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name: format!("{name}'s Top Tracks"),
                        selected_track: -1,
                    },
                    plugin_info: json!({
                        "type": "artist",
                        "url": uri,
                        "artworkUrl": artwork_url,
                        "author": name,
                        "totalTracks": track_count
                    }),
                    tracks,
                })
            }
            Err(_) => LoadResult::Empty {},
        }
    }
}
#[async_trait]
impl SourcePlugin for QobuzSource {
    fn name(&self) -> &str {
        "qobuz"
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
        vec!["qbsearch:"]
    }
    fn isrc_prefixes(&self) -> Vec<&str> {
        vec!["qbisrc:"]
    }
    fn rec_prefixes(&self) -> Vec<&str> {
        vec!["qbrec:"]
    }
    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        if let Some(prefix) = self
            .search_prefixes()
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            return self.handle_search(&identifier[prefix.len()..]).await;
        }
        if let Some(prefix) = self
            .isrc_prefixes()
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            return self.handle_isrc(&identifier[prefix.len()..]).await;
        }
        if let Some(prefix) = self
            .rec_prefixes()
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            return self
                .handle_recommendations(&identifier[prefix.len()..])
                .await;
        }
        if let Some(caps) = url_regex().captures(identifier) {
            let type_ = caps
                .name("type")
                .or_else(|| caps.name("type2"))
                .map(|m| m.as_str())
                .unwrap_or("");
            let id = caps
                .name("id")
                .or_else(|| caps.name("id2"))
                .map(|m| m.as_str())
                .unwrap_or("");
            return match type_ {
                "track" => {
                    match self
                        .api_request("track/get", vec![("track_id", id.to_owned())])
                        .await
                    {
                        Ok(json) => {
                            LoadResult::Track(Track::new(self.parse_qobuz_track(&json).info))
                        }
                        Err(_) => LoadResult::Empty {},
                    }
                }
                "album" => self.handle_album(id).await,
                "playlist" => self.handle_playlist(id).await,
                "artist" => self.handle_artist(id).await,
                _ => LoadResult::Empty {},
            };
        }
        LoadResult::Empty {}
    }
    async fn get_track(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let id = if identifier.contains("qobuz.com/track/") {
            identifier
                .split("track/")
                .nth(1)?
                .split('/')
                .next()?
                .split('?')
                .next()?
        } else {
            identifier
        };
        let tokens = self.token_tracker.get_tokens().await?;
        if tokens.user_token.is_none() {
            debug!("Qobuz: No user token, returning None to trigger mirroring");
            return None;
        }
        match self
            .api_request("track/get", vec![("track_id", id.to_owned())])
            .await
        {
            Ok(json) => Some(Arc::new(self.parse_qobuz_track(&json))),
            Err(_) => None,
        }
    }
}
}
pub mod token {
use std::sync::Arc;
use base64::{Engine as _, engine::general_purpose};
use regex::Regex;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
const WEB_PLAYER_BASE_URL: &str = "https://play.qobuz.com";
pub struct QobuzTokens {
    pub app_id: String,
    pub app_secret: String,
    pub user_token: Option<String>,
}
pub struct QobuzTokenTracker {
    client: Arc<reqwest::Client>,
    tokens: Arc<RwLock<Option<QobuzTokens>>>,
    config_user_token: Option<String>,
    config_app_id: Option<String>,
    config_app_secret: Option<String>,
}
fn bundle_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"<script src="(/resources/\d+\.\d+\.\d+-[a-z]\d{3}/bundle\.js)""#).unwrap()
    })
}
fn app_id_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"production:\{api:\{appId:"(.*?)""#).unwrap())
}
fn seed_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"\):[a-z]\.initialSeed\("(.*?)",window\.utimezone\.(.*?)\)"#).unwrap()
    })
}
fn info_extras_regex(timezone: &str) -> Regex {
    Regex::new(&format!(
        r#"(?s)timezones:\[.*?name:.*?/{}",info:"(?P<info>.*?)",extras:"(?P<extras>.*?)""#,
        regex::escape(timezone)
    ))
    .unwrap()
}
impl QobuzTokenTracker {
    pub fn new(
        client: Arc<reqwest::Client>,
        user_token: Option<String>,
        app_id: Option<String>,
        app_secret: Option<String>,
    ) -> Self {
        Self {
            client,
            tokens: Arc::new(RwLock::new(None)),
            config_user_token: user_token,
            config_app_id: app_id,
            config_app_secret: app_secret,
        }
    }
    pub async fn get_tokens(&self) -> Option<Arc<QobuzTokens>> {
        {
            let tokens = self.tokens.read().await;
            if let Some(t) = &*tokens {
                return Some(Arc::new(QobuzTokens {
                    app_id: t.app_id.to_owned(),
                    app_secret: t.app_secret.to_owned(),
                    user_token: t.user_token.to_owned(),
                }));
            }
        }
        self.refresh_tokens().await
    }
    async fn refresh_tokens(&self) -> Option<Arc<QobuzTokens>> {
        let mut tokens_lock = self.tokens.write().await;
        if let Some(t) = &*tokens_lock {
            return Some(Arc::new(QobuzTokens {
                app_id: t.app_id.to_owned(),
                app_secret: t.app_secret.to_owned(),
                user_token: t.user_token.to_owned(),
            }));
        }
        let app_id;
        let app_secret;
        if let (Some(id), Some(secret)) = (&self.config_app_id, &self.config_app_secret) {
            app_id = id.to_owned();
            app_secret = secret.to_owned();
            debug!("Using configured Qobuz app_id and app_secret");
        } else {
            debug!("Fetching Qobuz bundle.js for credential extraction...");
            match self.fetch_credentials_from_web().await {
                Ok((id, secret)) => {
                    app_id = id;
                    app_secret = secret;
                    info!("Successfully extracted Qobuz credentials: appId={app_id}");
                }
                Err(e) => {
                    error!("Failed to extract Qobuz credentials: {e}");
                    return None;
                }
            }
        }
        let new_tokens = QobuzTokens {
            app_id,
            app_secret,
            user_token: self.config_user_token.to_owned(),
        };
        let arc_tokens = Arc::new(QobuzTokens {
            app_id: new_tokens.app_id.to_owned(),
            app_secret: new_tokens.app_secret.to_owned(),
            user_token: new_tokens.user_token.to_owned(),
        });
        *tokens_lock = Some(new_tokens);
        Some(arc_tokens)
    }
    async fn fetch_credentials_from_web(&self) -> Result<(String, String), String> {
        let login_page = self
            .client
            .get(format!("{WEB_PLAYER_BASE_URL}/login"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        let bundle_path = bundle_regex()
            .captures(&login_page)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| "Failed to find bundle.js path in Qobuz login page".to_owned())?;
        let bundle_js = self
            .client
            .get(format!("{WEB_PLAYER_BASE_URL}{bundle_path}"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        let app_id = app_id_regex()
            .captures(&bundle_js)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_owned())
            .ok_or_else(|| "Failed to extract appId from bundle.js".to_owned())?;
        let seed_captures = seed_regex()
            .captures(&bundle_js)
            .ok_or_else(|| "Failed to extract seed and timezone from bundle.js".to_owned())?;
        let seed = seed_captures.get(1).unwrap().as_str();
        let timezone_raw = seed_captures.get(2).unwrap().as_str();
        let timezone = format!(
            "{}{}",
            &timezone_raw[..1].to_uppercase(),
            &timezone_raw[1..].to_lowercase()
        );
        let info_extras = info_extras_regex(&timezone)
            .captures(&bundle_js)
            .ok_or_else(|| format!("Failed to extract info/extras for timezone {timezone}"))?;
        let info = info_extras.name("info").unwrap().as_str();
        let extras = info_extras.name("extras").unwrap().as_str();
        let mut encoded = format!("{seed}{info}{extras}");
        if encoded.len() > 44 {
            encoded.truncate(encoded.len() - 44);
        }
        let decoded = general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| format!("Failed to decode appSecret: {e}"))?;
        let app_secret =
            String::from_utf8(decoded).map_err(|e| format!("Invalid UTF-8 in appSecret: {e}"))?;
        Ok((app_id, app_secret))
    }
    pub fn init(self: Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            this.get_tokens().await;
        });
    }
}
}
pub mod track {
use std::sync::Arc;
use async_trait::async_trait;
use md5::{Digest, Md5};
use crate::{
    common::types::AnyResult,
    protocol::tracks::TrackInfo,
    sources::{
        http::HttpTrack,
        playable_track::{PlayableTrack, ResolvedTrack},
        qobuz::token::QobuzTokenTracker,
    },
};
pub struct QobuzTrack {
    pub info: TrackInfo,
    pub album_name: Option<String>,
    pub album_url: Option<String>,
    pub artist_url: Option<String>,
    pub artist_artwork_url: Option<String>,
    pub token_tracker: Arc<QobuzTokenTracker>,
    pub client: Arc<reqwest::Client>,
}
#[async_trait]
impl PlayableTrack for QobuzTrack {
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let url = switch_media_url(&self.client, &self.token_tracker, &self.info.identifier)
            .await
            .map_err(|e| {
                format!(
                    "Qobuz: Failed to resolve media URL for {}: {e}",
                    self.info.identifier
                )
            })?
            .ok_or_else(|| "Failed to resolve Qobuz media URL".to_string())?;
        HttpTrack {
            url,
            local_addr: None,
            proxy: None,
        }
        .resolve()
        .await
    }
}
async fn switch_media_url(
    client: &Arc<reqwest::Client>,
    token_tracker: &QobuzTokenTracker,
    track_id: &str,
) -> AnyResult<Option<String>> {
    let tokens = token_tracker
        .get_tokens()
        .await
        .ok_or("Failed to get Qobuz tokens")?;
    if tokens.user_token.is_none() {
        return Ok(None);
    }
    let unix_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let format_id = "5";
    let intent = "stream";
    let sig_data = format!(
        "trackgetFileUrlformat_id{format_id}intent{intent}track_id{track_id}{unix_ts}{}",
        tokens.app_secret
    );
    let mut hasher = Md5::new();
    hasher.update(sig_data.as_bytes());
    let sig = hex::encode(hasher.finalize());
    let mut url = reqwest::Url::parse("https://www.qobuz.com/api.json/0.2/track/getFileUrl")?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("request_ts", &unix_ts.to_string());
        query.append_pair("request_sig", &sig);
        query.append_pair("track_id", track_id);
        query.append_pair("format_id", format_id);
        query.append_pair("intent", intent);
    }
    let mut request = client
        .get(url)
        .header("Accept", "application/json")
        .header("x-app-id", &tokens.app_id);
    if let Some(user_token) = &tokens.user_token {
        request = request.header("x-user-auth-token", user_token);
    }
    let resp = request.send().await?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let json: serde_json::Value = resp.json().await?;
    if let Some(url) = json.get("url").and_then(|v| v.as_str()) {
        let is_sample = json.get("sample").and_then(|v| v.as_bool()).or_else(|| {
            json.get("sample")
                .and_then(|v| v.as_str())
                .map(|s| s == "true")
        });
        if is_sample == Some(true) {
            return Ok(None);
        }
        return Ok(Some(url.to_owned()));
    }
    Ok(None)
}
}
pub use manager::*;
pub use token::*;
pub use track::*;