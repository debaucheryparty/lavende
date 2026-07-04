pub mod helpers {
    use super::{API_BASE, AppleMusicSource};
    use serde_json::Value;
    use tracing::{error, warn};
    impl AppleMusicSource {
        pub(crate) async fn api_request(&self, path: &str) -> Option<Value> {
            let token = self.token_tracker.get_token().await?;
            let origin = self.token_tracker.get_origin().await;
            let url = if path.starts_with("http") {
                path.to_owned()
            } else {
                format!("{}{}", API_BASE, path)
            };
            let mut req = self.client.get(&url).bearer_auth(token);
            if let Some(o) = origin {
                req = req.header("Origin", format!("https://{}", o));
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    error!("Apple Music API request failed: {}", e);
                    return None;
                }
            };
            if !resp.status().is_success() {
                warn!("Apple Music API returned {} for {}", resp.status(), url);
                return None;
            }
            resp.json().await.ok()
        }
    }
}
pub mod metadata {
    use super::AppleMusicSource;
    use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo};
    use futures::future::join_all;
    use serde_json::Value;
    use std::sync::Arc;
    use tokio::sync::Semaphore;
    impl AppleMusicSource {
        pub(crate) async fn resolve_track(&self, id: &str) -> LoadResult {
            let path = format!("/catalog/{}/songs/{}", self.country_code, id);
            let data = match self.api_request(&path).await {
                Some(d) => d,
                None => return LoadResult::Empty {},
            };
            if let Some(item) = data.pointer("/data/0")
                && let Some(track) = self.build_track(item, None)
            {
                return LoadResult::Track(track);
            }
            LoadResult::Empty {}
        }
        pub(crate) async fn resolve_album(&self, id: &str) -> LoadResult {
            self.resolve_collection(id, "album").await
        }
        pub(crate) async fn resolve_playlist(&self, id: &str) -> LoadResult {
            self.resolve_collection(id, "playlist").await
        }
        async fn resolve_collection(&self, id: &str, kind: &str) -> LoadResult {
            let plural = match kind {
                "album" => "albums",
                "playlist" => "playlists",
                _ => return LoadResult::Empty {},
            };
            let path = format!("/catalog/{}/{}/{}", self.country_code, plural, id);
            let data = match self.api_request(&path).await {
                Some(d) => d,
                None => return LoadResult::Empty {},
            };
            let collection = match data.pointer("/data/0") {
                Some(c) => c,
                None => return LoadResult::Empty {},
            };
            let attributes = match collection.get("attributes") {
                Some(a) => a,
                None => return LoadResult::Empty {},
            };
            let name = attributes
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_owned();
            let artwork = attributes
                .pointer("/artwork/url")
                .and_then(|v| v.as_str())
                .map(|s| s.replace("{w}", "1000").replace("{h}", "1000"));
            let tracks_rel = match collection
                .get("relationships")
                .and_then(|r| r.get("tracks"))
            {
                Some(t) => t,
                None => return LoadResult::Empty {},
            };
            let mut all_items = tracks_rel
                .get("data")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let next_url = tracks_rel.get("next").and_then(|v| v.as_str());
            let (load_limit, concurrency) = if kind == "album" {
                (self.album_load_limit, self.album_page_load_concurrency)
            } else {
                (
                    self.playlist_load_limit,
                    self.playlist_page_load_concurrency,
                )
            };
            if next_url.is_some() && (load_limit == 0 || load_limit > 1) {
                let next_url_owned = next_url.map(|s| s.to_owned());
                let extra = self
                    .fetch_paginated_tracks(next_url_owned, load_limit, concurrency)
                    .await;
                all_items.extend(extra);
            }
            let mut tracks = Vec::new();
            for item in all_items {
                if let Some(track) = self.build_track(&item, artwork.clone()) {
                    tracks.push(track);
                }
            }
            if tracks.is_empty() {
                return LoadResult::Empty {};
            }
            let author = if kind == "album" {
                attributes.get("artistName").and_then(|v| v.as_str())
            } else {
                attributes.get("curatorName").and_then(|v| v.as_str())
            };
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name,
                    selected_track: -1,
                },
                plugin_info: serde_json::json!({
                    "type": kind,
                    "url": attributes.get("url").and_then(|v| v.as_str()),
                    "artworkUrl": artwork,
                    "author": author,
                    "totalTracks": attributes.get("trackCount").and_then(|v| v.as_u64()).unwrap_or(tracks.len() as u64)
                }),
                tracks,
            })
        }
        pub(crate) async fn resolve_artist(&self, id: &str) -> LoadResult {
            let path = format!(
                "/catalog/{}/artists/{}/view/top-songs",
                self.country_code, id
            );
            let data = match self.api_request(&path).await {
                Some(d) => d,
                None => return LoadResult::Empty {},
            };
            let tracks_data = data.pointer("/data").and_then(|v| v.as_array());
            let artist_path = format!("/catalog/{}/artists/{}", self.country_code, id);
            let artist_data = self.api_request(&artist_path).await;
            let (artist_name, artwork) = if let Some(ad) = artist_data {
                let name = ad
                    .pointer("/data/0/attributes/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Artist")
                    .to_owned();
                let art = ad
                    .pointer("/data/0/attributes/artwork/url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.replace("{w}", "1000").replace("{h}", "1000"));
                (name, art)
            } else {
                ("Artist".to_owned(), None)
            };
            let mut tracks = Vec::new();
            if let Some(items) = tracks_data {
                for item in items {
                    if let Some(track) = self.build_track(item, artwork.clone()) {
                        tracks.push(track);
                    }
                }
            }
            if tracks.is_empty() {
                return LoadResult::Empty {};
            }
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: format!("{}'s Top Tracks", artist_name),
                    selected_track: -1,
                },
                plugin_info: serde_json::json!({
                    "type": "artist",
                    "url": format!("https://music.apple.com/artist/{}", id),
                    "artworkUrl": artwork,
                    "author": artist_name,
                    "totalTracks": tracks.len()
                }),
                tracks,
            })
        }
        async fn fetch_paginated_tracks(
            &self,
            next_url: Option<String>,
            load_limit: usize,
            concurrency: usize,
        ) -> Vec<Value> {
            let initial_next = match next_url {
                Some(u) => u,
                None => return Vec::new(),
            };
            if initial_next.contains("offset=") {
                let base_url = initial_next
                    .split("offset=")
                    .next()
                    .unwrap_or(&initial_next)
                    .to_owned();
                let offset: usize = initial_next
                    .split("offset=")
                    .nth(1)
                    .and_then(|s| s.split('&').next())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100);
                let mut all_items = Vec::new();
                let mut current_offset = offset;
                let mut limit_reached = false;
                let mut pages_fetched = 1;
                while !limit_reached {
                    let mut futs = Vec::new();
                    let semaphore = Arc::new(Semaphore::new(concurrency));
                    for _ in 0..concurrency {
                        if load_limit > 0 && pages_fetched >= load_limit {
                            limit_reached = true;
                            break;
                        }
                        let url = format!("{}offset={}", base_url, current_offset);
                        let sem = semaphore.clone();
                        futs.push(async move {
                            let _permit = sem.acquire().await.ok();
                            self.api_request(&url).await
                        });
                        current_offset += 100;
                        pages_fetched += 1;
                    }
                    if futs.is_empty() {
                        break;
                    }
                    let results = join_all(futs).await;
                    let mut added_on_this_step = 0;
                    for res in results {
                        if let Some(data) = res {
                            if let Some(items) = data.get("data").and_then(|v| v.as_array()) {
                                all_items.extend(items.iter().cloned());
                                added_on_this_step += items.len();
                                if items.len() < 100 {
                                    limit_reached = true;
                                }
                            } else {
                                limit_reached = true;
                            }
                        } else {
                            limit_reached = true;
                        }
                    }
                    if added_on_this_step == 0 {
                        break;
                    }
                }
                return all_items;
            }
            let mut next = Some(initial_next);
            let mut all_items = Vec::new();
            let mut pages_fetched = 1;
            while let Some(url) = next {
                if load_limit > 0 && pages_fetched >= load_limit {
                    break;
                }
                let data = match self.api_request(&url).await {
                    Some(d) => d,
                    None => break,
                };
                if let Some(items) = data.get("data").and_then(|v| v.as_array()) {
                    all_items.extend(items.iter().cloned());
                }
                next = data
                    .get("next")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned());
                pages_fetched += 1;
            }
            all_items
        }
    }
}
pub mod parser {
    use super::AppleMusicSource;
    use crate::protocol::tracks::{Track, TrackInfo};
    use serde_json::{Value, json};
    impl AppleMusicSource {
        pub(crate) fn build_track(
            &self,
            item: &Value,
            artwork_override: Option<String>,
        ) -> Option<Track> {
            let attributes = item.get("attributes")?;
            let id = item.get("id")?.as_str()?.to_owned();
            let title = attributes
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Title")
                .to_owned();
            let author = attributes
                .get("artistName")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Artist")
                .to_owned();
            let length = attributes
                .get("durationInMillis")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let isrc = attributes
                .get("isrc")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            let artwork_url = artwork_override.or_else(|| {
                attributes
                    .get("artwork")
                    .and_then(|a| a.get("url"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.replace("{w}", "1000").replace("{h}", "1000"))
            });
            let url = attributes
                .get("url")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned());
            let mut track = Track::new(TrackInfo {
                title,
                author,
                length,
                identifier: id,
                is_stream: false,
                uri: url,
                artwork_url,
                isrc,
                source_name: "applemusic".to_owned(),
                is_seekable: true,
                position: 0,
            });
            let album_name = attributes.get("albumName").and_then(|v| v.as_str());
            let artist_url = attributes.get("artistUrl").and_then(|v| v.as_str());
            let preview_url = attributes
                .pointer("/previews/0/url")
                .and_then(|v| v.as_str());
            let album_url = track
                .info
                .uri
                .as_ref()
                .and_then(|u| u.split('?').next().map(|s| s.to_owned()));
            track.plugin_info = json!({
                "albumName": album_name,
                "albumUrl": album_url,
                "artistUrl": artist_url,
                "artistArtworkUrl": null,
                "previewUrl": preview_url,
                "isPreview": false
            });
            Some(track)
        }
    }
}
pub mod search {
    use super::{API_BASE, AppleMusicSource};
    use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, SearchResult};
    use serde_json::Value;
    use std::collections::HashSet;
    impl AppleMusicSource {
        pub(crate) async fn search(&self, query: &str) -> LoadResult {
            let encoded_query = urlencoding::encode(query);
            let path = format!(
                "/catalog/{}/search?term={}&limit=10&types=songs",
                self.country_code, encoded_query
            );
            let data = match self.api_request(&path).await {
                Some(d) => d,
                None => return LoadResult::Empty {},
            };
            let songs = data
                .pointer("/results/songs/data")
                .and_then(|v| v.as_array());
            let mut tracks = Vec::new();
            if let Some(items) = songs {
                for item in items {
                    if let Some(track) = self.build_track(item, None) {
                        tracks.push(track);
                    }
                }
            }
            if tracks.is_empty() {
                LoadResult::Empty {}
            } else {
                LoadResult::Search(tracks)
            }
        }
        pub(crate) async fn get_search_suggestions(
            &self,
            query: &str,
            types: &[String],
        ) -> Option<SearchResult> {
            let mut kinds = HashSet::new();
            let mut am_types = Vec::new();
            let all_types = types.is_empty();
            if all_types
                || types.contains(&"track".to_owned())
                || types.contains(&"album".to_owned())
                || types.contains(&"artist".to_owned())
                || types.contains(&"playlist".to_owned())
            {
                kinds.insert("topResults");
            }
            if types.contains(&"text".to_owned()) {
                kinds.insert("terms");
            }
            if all_types || types.contains(&"track".to_owned()) {
                am_types.push("songs");
            }
            if all_types || types.contains(&"album".to_owned()) {
                am_types.push("albums");
            }
            if all_types || types.contains(&"artist".to_owned()) {
                am_types.push("artists");
            }
            if all_types || types.contains(&"playlist".to_owned()) {
                am_types.push("playlists");
            }
            let kinds_str = kinds.into_iter().collect::<Vec<_>>().join(",");
            let types_str = am_types.join(",");
            let mut params = vec![
                ("term", query.to_owned()),
                ("extend", "artistUrl".to_owned()),
                ("kinds", kinds_str),
            ];
            if !types_str.is_empty() {
                params.push(("types", types_str));
            }
            let path = format!("/catalog/{}/search/suggestions", self.country_code);
            let mut url = format!("{}{}", API_BASE, path);
            if !params.is_empty() {
                url.push('?');
                for (i, (k, v)) in params.iter().enumerate() {
                    if i > 0 {
                        url.push('&');
                    }
                    url.push_str(k);
                    url.push('=');
                    url.push_str(&urlencoding::encode(v));
                }
            }
            let json = self.api_request(&url).await?;
            let suggestions = json.pointer("/results/suggestions")?.as_array()?;
            let mut tracks = Vec::new();
            let mut albums = Vec::new();
            let mut artists = Vec::new();
            let mut playlists = Vec::new();
            for suggestion in suggestions {
                let kind = suggestion
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if kind == "terms" {
                    continue;
                }
                let content = match suggestion.get("content") {
                    Some(c) => c,
                    None => continue,
                };
                let type_ = content.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match type_ {
                    "songs" => {
                        if let Some(track) = self.build_track(content, None) {
                            tracks.push(track);
                        }
                    }
                    "albums" => {
                        if let Some(album) = self.build_collection(content, "album") {
                            albums.push(album);
                        }
                    }
                    "artists" => {
                        if let Some(artist) = self.build_collection(content, "artist") {
                            artists.push(artist);
                        }
                    }
                    "playlists" => {
                        if let Some(playlist) = self.build_collection(content, "playlist") {
                            playlists.push(playlist);
                        }
                    }
                    _ => {}
                }
            }
            Some(SearchResult {
                tracks,
                albums,
                artists,
                playlists,
                texts: Vec::new(),
                plugin: serde_json::json!({}),
            })
        }
        fn build_collection(&self, content: &Value, kind: &str) -> Option<PlaylistData> {
            let attributes = content.get("attributes")?;
            let url = attributes.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let name = attributes
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let artwork = attributes
                .pointer("/artwork/url")
                .and_then(|v| v.as_str())
                .map(|s| s.replace("{w}", "500").replace("{h}", "500"));
            let (author, track_count, display_name) = match kind {
                "album" => (
                    attributes
                        .get("artistName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Artist")
                        .to_owned(),
                    attributes
                        .get("trackCount")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    name.to_owned(),
                ),
                "artist" => (name.to_owned(), 0, format!("{}'s Top Tracks", name)),
                "playlist" => (
                    attributes
                        .get("curatorName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Apple Music")
                        .to_owned(),
                    attributes
                        .get("trackCount")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    name.to_owned(),
                ),
                _ => return None,
            };
            Some(PlaylistData {
                info: PlaylistInfo {
                    name: display_name,
                    selected_track: -1,
                },
                plugin_info: serde_json::json!({
                    "type": kind,
                    "url": url,
                    "author": author,
                    "artworkUrl": artwork,
                    "totalTracks": track_count
                }),
                tracks: Vec::new(),
            })
        }
    }
}
pub mod token {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use regex::Regex;
    use serde_json::Value;
    use std::{
        sync::{Arc, OnceLock},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    use tokio::sync::RwLock;
    use tracing::{error, info, warn};
    #[derive(Clone, Debug)]
    pub struct AppleMusicToken {
        pub access_token: String,
        pub origin: Option<String>,
        pub expiry_ms: u64,
    }
    pub struct AppleMusicTokenTracker {
        token: RwLock<Option<AppleMusicToken>>,
        client: Arc<reqwest::Client>,
    }
    impl AppleMusicTokenTracker {
        pub fn new(client: Arc<reqwest::Client>) -> Self {
            Self {
                token: RwLock::new(None),
                client,
            }
        }
        pub async fn get_token(&self) -> Option<String> {
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
        pub async fn get_origin(&self) -> Option<String> {
            let lock = self.token.read().await;
            lock.as_ref().and_then(|t| t.origin.clone())
        }
        fn is_valid(&self, token: &AppleMusicToken) -> bool {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            token.expiry_ms > now + 10_000
        }
        async fn refresh_token(&self) -> Option<String> {
            info!("Fetching new Apple Music API token...");
            let browse_url = "https://music.apple.com";
            let resp = match self.client.get(browse_url).send().await {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to fetch Apple Music root page: {}", e);
                    return None;
                }
            };
            if !resp.status().is_success() {
                error!("Apple Music root page returned status: {}", resp.status());
                return None;
            }
            let html = resp.text().await.unwrap_or_default();
            static SCRIPT_REGEX: OnceLock<Regex> = OnceLock::new();
            static INDEX_REGEX: OnceLock<Regex> = OnceLock::new();
            let script_re = SCRIPT_REGEX.get_or_init(|| {
                Regex::new(
                    r#"<script\s+type="module"\s+crossorigin\s+src="(/assets/index[^"]+\.js)""#,
                )
                .unwrap()
            });
            let script_path = match script_re.captures(&html) {
                Some(caps) => caps.get(1).map(|m| m.as_str()),
                None => {
                    let index_re = INDEX_REGEX
                        .get_or_init(|| Regex::new(r#"/assets/index[^"]+\.js"#).unwrap());
                    index_re.find(&html).map(|m| m.as_str())
                }
            };
            let script_path = match script_path {
                Some(p) => p,
                None => {
                    error!("Could not find index JS in Apple Music HTML");
                    return None;
                }
            };
            let script_url = if script_path.starts_with("http") {
                script_path.to_owned()
            } else {
                format!("https://music.apple.com{}", script_path)
            };
            let js_resp = match self.client.get(&script_url).send().await {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to fetch Apple Music JS bundle: {}", e);
                    return None;
                }
            };
            let js_content = js_resp.text().await.unwrap_or_default();
            static TOKEN_REGEX: OnceLock<Regex> = OnceLock::new();
            let token_re =
                TOKEN_REGEX.get_or_init(|| Regex::new(r#"(ey[\w-]+\.[\w-]+\.[\w-]+)"#).unwrap());
            let token_str = match token_re.find(&js_content) {
                Some(m) => m.as_str().to_owned(),
                None => {
                    error!("Could not find bearer token in Apple Music JS");
                    return None;
                }
            };
            let (origin, expiry_ms) = self.parse_jwt(&token_str).unwrap_or((None, 0));
            let token = AppleMusicToken {
                access_token: token_str.clone(),
                origin,
                expiry_ms,
            };
            let mut lock = self.token.write().await;
            *lock = Some(token);
            info!("Successfully refreshed Apple Music token");
            Some(token_str)
        }
        fn parse_jwt(&self, token: &str) -> Option<(Option<String>, u64)> {
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() < 2 {
                return None;
            }
            let payload_part = parts[1];
            let decoded = URL_SAFE_NO_PAD.decode(payload_part).ok()?;
            let json: Value = serde_json::from_slice(&decoded).ok()?;
            let origin = json
                .get("root_https_origin")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            let exp = json
                .get("exp")
                .and_then(|v| v.as_u64())
                .map(|e| e * 1000)
                .unwrap_or(0);
            Some((origin, exp))
        }
        pub fn init(self: Arc<Self>) {
            let this = self.clone();
            tokio::spawn(async move {
                let mut backoff = Duration::from_secs(1);
                loop {
                    if this.refresh_token().await.is_some() {
                        break;
                    }
                    warn!(
                        "Failed to initialize Apple Music token, retrying in {:?}...",
                        backoff
                    );
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(Duration::from_secs(60));
                }
            });
        }
    }
}
use crate::{
    protocol::tracks::LoadResult,
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use regex::Regex;
use std::sync::Arc;
use token::AppleMusicTokenTracker;
const API_BASE: &str = "https://api.music.apple.com/v1";
pub struct AppleMusicSource {
    client: Arc<reqwest::Client>,
    token_tracker: Arc<AppleMusicTokenTracker>,
    country_code: String,
    playlist_load_limit: usize,
    album_load_limit: usize,
    playlist_page_load_concurrency: usize,
    album_page_load_concurrency: usize,
    url_regex: Regex,
}
impl AppleMusicSource {
    pub fn new(
        config: Option<crate::config::AppleMusicConfig>,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let (country, p_limit, a_limit, p_conc, a_conc) = if let Some(c) = config {
            (
                c.country_code,
                c.playlist_load_limit,
                c.album_load_limit,
                c.playlist_page_load_concurrency,
                c.album_page_load_concurrency,
            )
        } else {
            ("us".to_owned(), 0, 0, 5, 5)
        };
        let token_tracker = Arc::new(AppleMusicTokenTracker::new(client.clone()));
        token_tracker.clone().init();
        Ok(Self {
            token_tracker,
            client,
            country_code: country,
            playlist_load_limit: p_limit,
            album_load_limit: a_limit,
            playlist_page_load_concurrency: p_conc,
            album_page_load_concurrency: a_conc,
            url_regex: Regex::new(r"https?://(?:www\.)?music\.apple\.com/(?:[a-zA-Z]{2}/)?(album|playlist|artist|song)/[^/]+/([a-zA-Z0-9\-.]+)(?:\?i=(\d+))?").unwrap(),
        })
    }
}
#[async_trait]
impl SourcePlugin for AppleMusicSource {
    fn name(&self) -> &str {
        "applemusic"
    }
    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || self.url_regex.is_match(identifier)
    }
    fn search_prefixes(&self) -> Vec<&str> {
        vec!["amsearch:"]
    }
    fn is_mirror(&self) -> bool {
        true
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
            let query = &identifier[prefix.len()..];
            return self.search(query).await;
        }
        if let Some(caps) = self.url_regex.captures(identifier) {
            let type_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let id = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let song_id = caps.get(3).map(|m| m.as_str());
            if type_str == "album"
                && let Some(s_id) = song_id
            {
                return self.resolve_track(s_id).await;
            }
            match type_str {
                "song" => return self.resolve_track(id).await,
                "album" => return self.resolve_album(id).await,
                "playlist" => return self.resolve_playlist(id).await,
                "artist" => return self.resolve_artist(id).await,
                _ => return LoadResult::Empty {},
            }
        }
        LoadResult::Empty {}
    }
    async fn load_search(
        &self,
        query: &str,
        types: &[String],
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<crate::protocol::tracks::SearchResult> {
        let q = if let Some(prefix) = self
            .search_prefixes()
            .into_iter()
            .find(|p| query.starts_with(p))
        {
            &query[prefix.len()..]
        } else {
            query
        };
        self.get_search_suggestions(q, types).await
    }
    async fn get_track(
        &self,
        _identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        None
    }
}
