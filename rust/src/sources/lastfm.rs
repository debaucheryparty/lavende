pub mod helpers {
use std::sync::Arc;
use serde_json::Value;
pub async fn get_json(client: &Arc<reqwest::Client>, url: &str) -> Option<Value> {
    let res = match client.get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .send()
        .await {
            Ok(r) => r,
            Err(e) => {
                let redacted = if let Some(pos) = url.find("api_key=") {
                    let end = url[pos..].find('&').map(|e| pos + e).unwrap_or(url.len());
                    let mut s = url.to_owned();
                    s.replace_range(pos + 8..end, "REDACTED");
                    s
                } else {
                    url.to_owned()
                };
                tracing::debug!("Last.fm: API request failed for {}: {}", redacted, e);
                return None;
            }
        };
    if !res.status().is_success() {
        let redacted = if let Some(pos) = url.find("api_key=") {
            let end = url[pos..].find('&').map(|e| pos + e).unwrap_or(url.len());
            let mut s = url.to_owned();
            s.replace_range(pos + 8..end, "REDACTED");
            s
        } else {
            url.to_owned()
        };
        tracing::debug!(
            "Last.fm: API returned error status {} for {}",
            res.status(),
            redacted
        );
        return None;
    }
    res.json().await.ok()
}
pub fn unescape_html(input: &str) -> String {
    let mut result = input.to_owned();
    loop {
        let next = result
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&#x27;", "'");
        if next == result {
            break;
        }
        result = next;
    }
    result
}
pub fn parse_duration_to_ms(duration: &str) -> u64 {
    let parts: Vec<&str> = duration.split(':').collect();
    if parts.len() == 2 {
        let minutes = parts[0].trim().parse::<u64>().unwrap_or(0);
        let seconds = parts[1].trim().parse::<u64>().unwrap_or(0);
        (minutes * 60 + seconds) * 1000
    } else if parts.len() == 3 {
        let hours = parts[0].trim().parse::<u64>().unwrap_or(0);
        let minutes = parts[1].trim().parse::<u64>().unwrap_or(0);
        let seconds = parts[2].trim().parse::<u64>().unwrap_or(0);
        (hours * 3600 + minutes * 60 + seconds) * 1000
    } else {
        0
    }
}
}
pub mod metadata {
use regex::Regex;
use super::{
    LastFMSource,
    helpers::{get_json, parse_duration_to_ms, unescape_html},
};
use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track, TrackInfo};
impl LastFMSource {
    pub async fn resolve_url(&self, url: &str) -> LoadResult {
        let caps = match crate::sources::lastfm::path_regex().captures(url) {
            Some(c) => c,
            None => {
                tracing::debug!("Last.fm: URL path failed to match regex: {}", url);
                return LoadResult::Empty {};
            }
        };
        let type_ = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let p1 = urlencoding::decode(caps.get(2).map(|m| m.as_str()).unwrap_or(""))
            .unwrap_or_default()
            .to_string();
        let p2 = urlencoding::decode(caps.get(3).map(|m| m.as_str()).unwrap_or(""))
            .unwrap_or_default()
            .to_string();
        let p3 = urlencoding::decode(caps.get(4).map(|m| m.as_str()).unwrap_or(""))
            .unwrap_or_default()
            .to_string();
        if type_ == "user" {
            return self.resolve_user(&p1, url).await;
        }
        if p3.is_empty() {
            if p2 == "_" || p2.is_empty() {
                self.resolve_artist(&p1, url).await
            } else {
                self.resolve_album(&p1, &p2, url).await
            }
        } else {
            self.resolve_track(&p1, &p3, url).await
        }
    }
    pub async fn resolve_track(&self, artist: &str, title: &str, url: &str) -> LoadResult {
        let mut artwork_url = None;
        let mut length = 0;
        if let Some(ref key) = self.api_key {
            let api_url = format!(
                "https://ws.audioscrobbler.com/2.0/?method=track.getInfo&api_key={}&artist={}&track={}&format=json",
                key,
                urlencoding::encode(artist),
                urlencoding::encode(title)
            );
            if let Some(json) = get_json(&self.http, &api_url).await {
                artwork_url = json["track"]["album"]["image"]
                    .as_array()
                    .or_else(|| json["track"]["image"].as_array())
                    .and_then(|images| images.last())
                    .and_then(|img| img["#text"].as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.replace("/34s/", "/300x300/"));
                length = json["track"]["duration"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .or_else(|| json["track"]["duration"].as_u64())
                    .unwrap_or(0);
            }
        }
        if (artwork_url.is_none() || length == 0)
            && let Ok(res) = self.http.get(url).send().await
            && let Ok(body) = res.text().await
        {
            if artwork_url.is_none()
                && let Some(caps) = Regex::new(
                    r#"(?i)<img[^>]*?class="[^"]*header-new-background-image[^"]*"[^>]*?src="([^"]+)""#,
                )
                .ok()
                .and_then(|r| r.captures(&body))
            {
                artwork_url = caps
                    .get(1)
                    .map(|m| m.as_str().replace("/64s/", "/300x300/"));
            }
            if length == 0
                && let Some(caps) = Regex::new(
                    r#"(?i)<dt[^>]*?>\s*Length\s*</dt>\s*<dd[^>]*?class="[^"]*catalogue-metadata-description[^"]*"[^>]*?>\s*(\d+:\d+(?::\d+)?)\s*</dd>"#,
                )
                .ok()
                .and_then(|r| r.captures(&body))
            {
                length = parse_duration_to_ms(
                    caps.get(1).map(|m| m.as_str()).unwrap_or("0:00"),
                );
            }
        }
        let canonical_url = crate::sources::lastfm::construct_track_url(artist, title);
        LoadResult::Track(Track::new(TrackInfo {
            identifier: canonical_url.clone(),
            is_seekable: true,
            author: artist.to_owned(),
            title: title.to_owned(),
            length,
            uri: Some(canonical_url),
            artwork_url,
            source_name: "lastfm".to_owned(),
            ..Default::default()
        }))
    }
    pub async fn resolve_album(&self, artist: &str, album: &str, url: &str) -> LoadResult {
        if let Some(ref key) = self.api_key {
            let api_url = format!(
                "https://ws.audioscrobbler.com/2.0/?method=album.getinfo&api_key={}&artist={}&album={}&format=json",
                key,
                urlencoding::encode(artist),
                urlencoding::encode(album)
            );
            if let Some(json) = get_json(&self.http, &api_url).await
                && let Some(tracks) = json["album"]["tracks"]["track"].as_array()
            {
                let artwork_url = json["album"]["image"]
                    .as_array()
                    .and_then(|images| images.last())
                    .and_then(|img| img["#text"].as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.replace("/34s/", "/300x300/"));
                let mut results = Vec::new();
                for t in tracks {
                    let title = t["name"].as_str().unwrap_or("Unknown").to_owned();
                    let t_url = crate::sources::lastfm::construct_track_url(artist, &title);
                    let length = t["duration"]
                        .as_str()
                        .and_then(|s| s.parse::<u64>().ok())
                        .or_else(|| t["duration"].as_u64())
                        .unwrap_or(0)
                        * 1000;
                    results.push(Track::new(TrackInfo {
                        identifier: t_url.clone(),
                        is_seekable: true,
                        author: artist.to_owned(),
                        title,
                        length,
                        uri: Some(t_url),
                        artwork_url: artwork_url.clone(),
                        source_name: "lastfm".to_owned(),
                        ..Default::default()
                    }));
                }
                return LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name: format!("{} - {}", artist, album),
                        selected_track: -1,
                    },
                    plugin_info: serde_json::json!({}),
                    tracks: results,
                });
            }
        }
        let body = match self.http.get(url).send().await {
            Ok(r) => r.text().await.unwrap_or_else(|e| {
                tracing::debug!(
                    "Last.fm: failed to get response text for album {}: {}",
                    url,
                    e
                );
                Default::default()
            }),
            Err(e) => {
                tracing::debug!("Last.fm: album scraping request failed for {}: {}", url, e);
                return LoadResult::Empty {};
            }
        };
        let mut results = Vec::new();
        for caps in crate::sources::lastfm::search_regex().captures_iter(&body) {
            let artwork_url = caps
                .get(1)
                .map(|m| m.as_str().replace("/64s/", "/300x300/"));
            let title = unescape_html(caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown"));
            let full_url = crate::sources::lastfm::construct_track_url(artist, &title);
            results.push(Track::new(TrackInfo {
                identifier: full_url.clone(),
                is_seekable: true,
                author: artist.to_owned(),
                title: title.to_owned(),
                uri: Some(full_url),
                artwork_url,
                source_name: "lastfm".to_owned(),
                ..Default::default()
            }));
        }
        if results.is_empty() {
            tracing::debug!(
                "Last.fm: album/artist search yielded no tracks on page {}, trying track fallback",
                url
            );
            self.resolve_track(artist, album, url).await
        } else {
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: format!("{} - {}", artist, album),
                    selected_track: -1,
                },
                plugin_info: serde_json::json!({}),
                tracks: results,
            })
        }
    }
    pub async fn resolve_artist(&self, artist: &str, url: &str) -> LoadResult {
        if let Some(ref key) = self.api_key {
            let api_url = format!(
                "https://ws.audioscrobbler.com/2.0/?method=artist.gettoptracks&api_key={}&artist={}&limit=50&format=json",
                key,
                urlencoding::encode(artist)
            );
            if let Some(json) = get_json(&self.http, &api_url).await
                && let Some(tracks) = json["toptracks"]["track"].as_array()
            {
                let mut results = Vec::new();
                for t in tracks {
                    let title = t["name"].as_str().unwrap_or("Unknown").to_owned();
                    let t_url = crate::sources::lastfm::construct_track_url(artist, &title);
                    let artwork_url = t["image"]
                        .as_array()
                        .and_then(|images| images.last())
                        .and_then(|img| img["#text"].as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.replace("/34s/", "/300x300/"));
                    let length = t["duration"]
                        .as_str()
                        .and_then(|s| s.parse::<u64>().ok())
                        .or_else(|| t["duration"].as_u64())
                        .unwrap_or(0)
                        * 1000;
                    results.push(Track::new(TrackInfo {
                        identifier: t_url.clone(),
                        is_seekable: true,
                        author: artist.to_owned(),
                        title,
                        length,
                        uri: Some(t_url),
                        artwork_url,
                        source_name: "lastfm".to_owned(),
                        ..Default::default()
                    }));
                }
                return LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name: format!("{} - Top Tracks", artist),
                        selected_track: -1,
                    },
                    plugin_info: serde_json::json!({}),
                    tracks: results,
                });
            }
        }
        let body = match self.http.get(url).send().await {
            Ok(r) => r.text().await.unwrap_or_else(|e| {
                tracing::debug!(
                    "Last.fm: failed to get response text for artist tracks {}: {}",
                    url,
                    e
                );
                Default::default()
            }),
            Err(e) => {
                tracing::debug!(
                    "Last.fm: artist tracks scraping request failed for {}: {}",
                    url,
                    e
                );
                return LoadResult::Empty {};
            }
        };
        let mut results = Vec::new();
        for caps in crate::sources::lastfm::search_regex().captures_iter(&body) {
            let artwork_url = caps
                .get(1)
                .map(|m| m.as_str().replace("/64s/", "/300x300/"));
            let title = unescape_html(caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown"));
            let full_url = crate::sources::lastfm::construct_track_url(artist, &title);
            results.push(Track::new(TrackInfo {
                identifier: full_url.clone(),
                is_seekable: true,
                author: artist.to_owned(),
                title: title.to_owned(),
                uri: Some(full_url),
                artwork_url,
                source_name: "lastfm".to_owned(),
                ..Default::default()
            }));
        }
        if results.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: format!("{} - Top Tracks", artist),
                    selected_track: -1,
                },
                plugin_info: serde_json::json!({}),
                tracks: results,
            })
        }
    }
    pub async fn resolve_user(&self, username: &str, url: &str) -> LoadResult {
        if let Some(ref key) = self.api_key {
            let api_url = format!(
                "https://ws.audioscrobbler.com/2.0/?method=user.gettoptracks&user={}&api_key={}&limit=50&format=json",
                urlencoding::encode(username),
                key
            );
            if let Some(json) = get_json(&self.http, &api_url).await
                && let Some(tracks) = json["toptracks"]["track"].as_array()
            {
                let mut results = Vec::new();
                for t in tracks {
                    let title = t["name"].as_str().unwrap_or("Unknown").to_owned();
                    let artist = t["artist"]["name"].as_str().unwrap_or("Unknown").to_owned();
                    let t_url = crate::sources::lastfm::construct_track_url(&artist, &title);
                    let artwork_url = t["image"]
                        .as_array()
                        .and_then(|images| images.last())
                        .and_then(|img| img["#text"].as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.replace("/34s/", "/300x300/"));
                    let length = t["duration"]
                        .as_str()
                        .and_then(|s| s.parse::<u64>().ok())
                        .or_else(|| t["duration"].as_u64())
                        .unwrap_or(0)
                        * 1000;
                    results.push(Track::new(TrackInfo {
                        identifier: t_url.clone(),
                        is_seekable: true,
                        author: artist,
                        title,
                        length,
                        uri: Some(t_url),
                        artwork_url,
                        source_name: "lastfm".to_owned(),
                        ..Default::default()
                    }));
                }
                return LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name: format!("{}'s Top Tracks", username),
                        selected_track: -1,
                    },
                    plugin_info: serde_json::json!({}),
                    tracks: results,
                });
            }
        }
        let body = match self.http.get(url).send().await {
            Ok(r) => r.text().await.unwrap_or_else(|e| {
                tracing::debug!(
                    "Last.fm: failed to get response text for user profile {}: {}",
                    url,
                    e
                );
                Default::default()
            }),
            Err(e) => {
                tracing::debug!(
                    "Last.fm: user profile scraping request failed for {}: {}",
                    url,
                    e
                );
                return LoadResult::Empty {};
            }
        };
        let mut results = Vec::new();
        for caps in crate::sources::lastfm::search_regex().captures_iter(&body) {
            let artwork_url = caps
                .get(1)
                .map(|m| m.as_str().replace("/64s/", "/300x300/"));
            let title = unescape_html(caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown"));
            let artist = unescape_html(caps.get(4).map(|m| m.as_str()).unwrap_or("Unknown"));
            let full_url = crate::sources::lastfm::construct_track_url(&artist, &title);
            results.push(Track::new(TrackInfo {
                identifier: full_url.clone(),
                is_seekable: true,
                author: artist,
                title,
                uri: Some(full_url),
                artwork_url,
                source_name: "lastfm".to_owned(),
                ..Default::default()
            }));
        }
        if results.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: format!("{}'s Recent Tracks", username),
                    selected_track: -1,
                },
                plugin_info: serde_json::json!({}),
                tracks: results,
            })
        }
    }
}
}
pub mod search {
use super::{
    LastFMSource,
    helpers::{get_json, unescape_html},
};
use crate::protocol::tracks::{LoadResult, Track, TrackInfo};
impl LastFMSource {
    pub async fn search_tracks(&self, query: &str) -> LoadResult {
        if let Some(ref key) = self.api_key {
            self.search_api(query, key).await
        } else {
            self.search_scraping(query).await
        }
    }
    async fn search_api(&self, query: &str, api_key: &str) -> LoadResult {
        let url = format!(
            "https://ws.audioscrobbler.com/2.0/?method=track.search&track={}&api_key={}&limit={}&format=json",
            urlencoding::encode(query),
            api_key,
            self.search_limit
        );
        let json = match get_json(&self.http, &url).await {
            Some(j) => j,
            None => return LoadResult::Empty {},
        };
        let tracks = match json["results"]["trackmatches"]["track"].as_array() {
            Some(t) => t,
            None => {
                tracing::debug!(
                    "Last.fm: API response missing trackmatches for search '{}'",
                    query
                );
                return LoadResult::Empty {};
            }
        };
        let results: Vec<Track> = tracks
            .iter()
            .map(|t| {
                let title = t["name"].as_str().unwrap_or("Unknown").to_owned();
                let artist = t["artist"].as_str().unwrap_or("Unknown").to_owned();
                let uri = crate::sources::lastfm::construct_track_url(&artist, &title);
                let artwork_url = t["image"]
                    .as_array()
                    .and_then(|images| images.last())
                    .and_then(|img| img["#text"].as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.replace("/34s/", "/300x300/"));
                Track::new(TrackInfo {
                    identifier: uri.clone(),
                    is_seekable: true,
                    author: artist,
                    title,
                    uri: Some(uri),
                    artwork_url,
                    source_name: "lastfm".to_owned(),
                    ..Default::default()
                })
            })
            .collect();
        if results.is_empty() {
            tracing::debug!("Last.fm: API search returned no tracks for '{}'", query);
            LoadResult::Empty {}
        } else {
            LoadResult::Search(results)
        }
    }
    async fn search_scraping(&self, query: &str) -> LoadResult {
        let url = format!(
            "https://www.last.fm/search/tracks?q={}",
            urlencoding::encode(query)
        );
        let body = match self.http.get(&url).send().await {
            Ok(r) => r.text().await.unwrap_or_else(|e| {
                tracing::debug!(
                    "Last.fm: failed to get response text for search scraping '{}': {}",
                    query,
                    e
                );
                Default::default()
            }),
            Err(e) => {
                tracing::debug!(
                    "Last.fm: search scraping request failed for '{}': {}",
                    query,
                    e
                );
                return LoadResult::Empty {};
            }
        };
        let mut results = Vec::new();
        for caps in crate::sources::lastfm::search_regex().captures_iter(&body) {
            let artwork_url = caps
                .get(1)
                .map(|m| m.as_str().replace("/64s/", "/300x300/"));
            let title = unescape_html(caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown"));
            let artist = unescape_html(caps.get(4).map(|m| m.as_str()).unwrap_or("Unknown"));
            let full_url = crate::sources::lastfm::construct_track_url(&artist, &title);
            results.push(Track::new(TrackInfo {
                identifier: full_url.clone(),
                is_seekable: true,
                author: artist,
                title,
                uri: Some(full_url),
                artwork_url,
                source_name: "lastfm".to_owned(),
                ..Default::default()
            }));
            if results.len() >= self.search_limit {
                break;
            }
        }
        if results.is_empty() {
            tracing::debug!(
                "Last.fm: search scraping found no tracks for '{}' on page {}",
                query,
                url
            );
            LoadResult::Empty {}
        } else {
            LoadResult::Search(results)
        }
    }
}
}
use std::sync::{Arc, OnceLock};
use async_trait::async_trait;
use regex::Regex;
use crate::{
    protocol::tracks::LoadResult,
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
pub fn path_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?last\.fm/(?:[a-z]{2}/)?(music|user)/([^/]+)(?:/([^/]+)(?:/([^/]+))?)?")
            .expect("lastfm path regex is a valid literal")
    })
}
pub fn search_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)<tr[^>]*?>[\s\S]*?<img[^>]*?src="([^"]+)"[\s\S]*?data-track-name="([^"]+)"[\s\S]*?data-track-url="([^"]+)"[\s\S]*?data-artist-name="([^"]+)"#)
            .expect("lastfm search regex is a valid literal")
    })
}
pub fn encode_path_segment(segment: &str) -> String {
    urlencoding::encode(segment).replace("%20", "+")
}
pub fn construct_track_url(artist: &str, track: &str) -> String {
    format!(
        "https://www.last.fm/music/{}/_/{}",
        encode_path_segment(artist),
        encode_path_segment(track)
    )
}
pub struct LastFMSource {
    pub http: Arc<reqwest::Client>,
    pub api_key: Option<String>,
    pub search_limit: usize,
}
impl LastFMSource {
    pub fn new(
        config: Option<crate::config::sources::LastFmConfig>,
        http: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let (api_key, search_limit) = if let Some(c) = config {
            (c.api_key, c.search_limit)
        } else {
            (None, 10)
        };
        Ok(Self {
            http,
            api_key,
            search_limit,
        })
    }
}
#[async_trait]
impl SourcePlugin for LastFMSource {
    fn name(&self) -> &str {
        "lastfm"
    }
    fn search_prefixes(&self) -> Vec<&str> {
        vec!["lfsearch:", "lfmsearch:"]
    }
    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || path_regex().is_match(identifier)
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
            self.search_tracks(query).await
        } else {
            self.resolve_url(identifier).await
        }
    }
    async fn get_track(
        &self,
        _identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        None
    }
}