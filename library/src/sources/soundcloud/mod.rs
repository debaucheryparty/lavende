pub mod api;
pub mod extractor;
pub mod stream;
pub mod token;

use crate::{
    protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use regex::Regex;
use std::sync::{Arc, OnceLock};

use extractor::{
    LikedResponseDto, PlaylistDto, SoundCloudStreamKind, TrackDto, UserResponseDto, parse_track,
    select_format,
};
use stream::{SoundCloudHlsReader, SoundCloudReader};
use token::SoundCloudTokenTracker;

fn track_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^https?://(?:www\.|m\.)?soundcloud\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_-]+)(?:/s-[a-zA-Z0-9_-]+)?/?(?:\?.*)?$").unwrap()
    })
}

fn playlist_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^https?://(?:www\.|m\.)?soundcloud\.com/([a-zA-Z0-9_-]+)/sets/([a-zA-Z0-9_:-]+)(?:/[a-zA-Z0-9_-]+)?/?(?:\?.*)?$").unwrap()
    })
}

fn liked_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^https?://(?:www\.|m\.)?soundcloud\.com/([a-zA-Z0-9_-]+)/likes/?(?:\?.*)?$")
            .unwrap()
    })
}

fn short_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^https://on\.soundcloud\.com/[a-zA-Z0-9_-]+/?(?:\?.*)?$").unwrap()
    })
}

fn mobile_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^https://soundcloud\.app\.goo\.gl/[a-zA-Z0-9_-]+/?(?:\?.*)?$").unwrap()
    })
}

fn liked_user_urn_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#""urn":"soundcloud:users:(\d+)","username":"([^"]+)""#).unwrap())
}

fn user_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^https?://(?:www\.|m\.)?soundcloud\.com/([a-zA-Z0-9_-]+)(?:/(tracks|popular-tracks|albums|sets|reposts|spotlight))?/?(?:\?.*)?$").unwrap()
    })
}

fn search_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^https?://(?:www\.|m\.)?soundcloud\.com/search(?:/(?:sounds|people|albums|sets))?/?(?:\?.*)?$").unwrap()
    })
}

pub struct SoundCloudTrack {
    pub stream_url: String,
    pub kind: SoundCloudStreamKind,
    pub bitrate_bps: u64,
    pub local_addr: Option<std::net::IpAddr>,
    pub proxy: Option<crate::config::HttpProxyConfig>,
}

#[async_trait]
impl crate::sources::playable_track::PlayableTrack for SoundCloudTrack {
    fn supports_seek(&self) -> bool {
        true
    }

    async fn resolve(&self) -> Result<crate::sources::playable_track::ResolvedTrack, String> {
        let (reader, hint) = match self.kind {
            SoundCloudStreamKind::ProgressiveMp3 => (
                SoundCloudReader::new(&self.stream_url, self.local_addr, self.proxy.clone())
                    .await
                    .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                    .map_err(|e| format!("Failed to open stream: {e}"))?,
                crate::common::types::AudioFormat::Mp3,
            ),
            SoundCloudStreamKind::ProgressiveAac => (
                SoundCloudReader::new(&self.stream_url, self.local_addr, self.proxy.clone())
                    .await
                    .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                    .map_err(|e| format!("Failed to open stream: {e}"))?,
                crate::common::types::AudioFormat::Mp4,
            ),
            SoundCloudStreamKind::HlsOpus => (
                SoundCloudHlsReader::new(
                    &self.stream_url,
                    self.bitrate_bps,
                    self.local_addr,
                    self.proxy.clone(),
                )
                .await
                .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                .map_err(|e| format!("Failed to init HLS reader: {e}"))?,
                crate::common::types::AudioFormat::Opus,
            ),
            SoundCloudStreamKind::HlsMp3 => (
                SoundCloudHlsReader::new(
                    &self.stream_url,
                    self.bitrate_bps,
                    self.local_addr,
                    self.proxy.clone(),
                )
                .await
                .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                .map_err(|e| format!("Failed to init HLS reader: {e}"))?,
                crate::common::types::AudioFormat::Mp3,
            ),
            SoundCloudStreamKind::HlsAac => (
                SoundCloudHlsReader::new(
                    &self.stream_url,
                    self.bitrate_bps,
                    self.local_addr,
                    self.proxy.clone(),
                )
                .await
                .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                .map_err(|e| format!("Failed to init HLS reader: {e}"))?,
                crate::common::types::AudioFormat::Aac,
            ),
        };
        Ok(crate::sources::playable_track::ResolvedTrack::new(
            reader,
            Some(hint),
        ))
    }
}

pub struct SoundCloudSource {
    client: Arc<reqwest::Client>,
    config: crate::config::SoundCloudConfig,
    token_tracker: Arc<SoundCloudTokenTracker>,
}

impl SoundCloudSource {
    pub fn new(
        config: crate::config::SoundCloudConfig,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let token_tracker = Arc::new(SoundCloudTokenTracker::new(client.clone(), &config));
        token_tracker.clone().init();
        Ok(Self {
            client,
            config,
            token_tracker,
        })
    }

    async fn resolve_short_url(&self, url: &str) -> Option<String> {
        let resp = self.client.head(url).send().await.ok()?;
        let location = resp.headers().get("location")?.to_str().ok()?.to_owned();
        Some(location)
    }

    async fn resolve_mobile_url(&self, url: &str) -> Option<String> {
        let resp = self.client.get(url).send().await.ok()?;
        Some(resp.url().to_string())
    }

    async fn get_track_from_url(
        &self,
        url: &str,
        client_id: &str,
        local_addr: Option<std::net::IpAddr>,
    ) -> Option<BoxedTrack> {
        let json = api::api_resolve(&self.client, url, client_id).await.ok()?;
        let track_dto: TrackDto = serde_json::from_value(json).ok()?;

        let transcodings = track_dto
            .media
            .as_ref()
            .and_then(|m| m.transcodings.clone())
            .unwrap_or_default();
        if transcodings.is_empty() {
            return None;
        }

        let (kind, lookup_url) = select_format(&transcodings)?;
        let stream_url = api::resolve_stream_url(&self.client, &lookup_url, client_id)
            .await
            .ok()?;

        if stream_url.contains("cf-preview-media.sndcdn.com") || stream_url.contains("/preview/") {
            return None;
        }

        Some(Arc::new(SoundCloudTrack {
            stream_url,
            kind,
            bitrate_bps: 128_000,
            local_addr,
            proxy: self.config.proxy.clone(),
        }))
    }

    async fn search_tracks(&self, query: &str) -> LoadResult {
        let client_id = match self.token_tracker.get_client_id().await {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let limit = self.config.search_limit;

        match api::search_tracks_api(&self.client, query, &client_id, limit).await {
            Ok(json) => {
                let response: extractor::SearchResponseDto = match serde_json::from_value(json) {
                    Ok(res) => res,
                    Err(_) => return LoadResult::Empty {},
                };
                let tracks: Vec<Track> = response
                    .collection
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|item| parse_track(item).ok())
                    .collect();

                if tracks.is_empty() {
                    LoadResult::Empty {}
                } else {
                    LoadResult::Search(tracks)
                }
            }
            Err(_) => LoadResult::Empty {},
        }
    }

    async fn load_single_track(&self, url: &str) -> LoadResult {
        let client_id = match self.token_tracker.get_client_id().await {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let json = match api::api_resolve(&self.client, url, &client_id).await {
            Ok(j) => j,
            Err(_) => return LoadResult::Empty {},
        };
        let track_dto: TrackDto = match serde_json::from_value(json) {
            Ok(t) => t,
            Err(_) => return LoadResult::Empty {},
        };

        match parse_track(&track_dto) {
            Ok(track) => LoadResult::Track(track),
            Err(_) => LoadResult::Empty {},
        }
    }

    async fn load_playlist(&self, url: &str) -> LoadResult {
        let client_id = match self.token_tracker.get_client_id().await {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let json = match api::api_resolve(&self.client, url, &client_id).await {
            Ok(v) => v,
            Err(_) => return LoadResult::Empty {},
        };
        let playlist_dto: PlaylistDto = match serde_json::from_value(json) {
            Ok(p) => p,
            Err(_) => return LoadResult::Empty {},
        };
        if playlist_dto.kind.as_deref() != Some("playlist") {
            return LoadResult::Empty {};
        }
        let name = playlist_dto
            .title
            .clone()
            .unwrap_or_else(|| "Untitled playlist".to_owned());
        let raw_tracks = playlist_dto.tracks.clone().unwrap_or_default();
        let mut complete: Vec<Track> = Vec::new();
        let mut stub_ids: Vec<String> = Vec::new();
        for t in &raw_tracks {
            if let Ok(track_dto) = serde_json::from_value::<TrackDto>(t.clone()) {
                if track_dto.title.is_some() {
                    if let Ok(track) = parse_track(&track_dto) {
                        complete.push(track);
                    }
                } else if let Some(id) = track_dto
                    .id
                    .as_str()
                    .map(|s| s.to_owned())
                    .or_else(|| track_dto.id.as_i64().map(|n| n.to_string()))
                {
                    stub_ids.push(id);
                }
            }
        }
        let playlist_limit = self.config.playlist_load_limit;
        let needed = stub_ids
            .iter()
            .take(playlist_limit.saturating_sub(complete.len()))
            .cloned()
            .collect::<Vec<_>>();
        for chunk in needed.chunks(50) {
            let ids = chunk.join(",");
            if let Ok(json) = api::load_tracks_batch_api(&self.client, &ids, &client_id).await {
                if let Some(arr) = json.as_array() {
                    for item in arr {
                        if let Ok(track_dto) = serde_json::from_value::<TrackDto>(item.clone()) {
                            if let Ok(track) = parse_track(&track_dto) {
                                complete.push(track);
                            }
                        }
                    }
                }
            }
        }
        complete.truncate(playlist_limit);
        if complete.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name,
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
                "type": playlist_dto.kind.as_deref().unwrap_or("playlist"),
                "url": url,
                "artworkUrl": playlist_dto.artwork_url.as_ref().map(|s| s.replace("-large", "-t500x500")),
                "author": playlist_dto.user.as_ref().and_then(|u| u.username.clone()),
                "totalTracks": playlist_dto.track_count.unwrap_or(complete.len() as u64)
            }),
            tracks: complete,
        })
    }

    async fn load_liked_tracks(&self, url: &str) -> LoadResult {
        let client_id = match self.token_tracker.get_client_id().await {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let html = match self.client.get(url).send().await {
            Ok(r) => match r.text().await {
                Ok(t) => t,
                Err(_) => return LoadResult::Empty {},
            },
            Err(_) => return LoadResult::Empty {},
        };
        let caps = match liked_user_urn_re().captures(&html) {
            Some(c) => c,
            None => return LoadResult::Empty {},
        };
        let user_id = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let user_name = caps.get(2).map(|m| m.as_str()).unwrap_or("User");
        match api::load_liked_tracks_api(&self.client, user_id, &client_id).await {
            Ok(json) => {
                let response: LikedResponseDto = match serde_json::from_value(json) {
                    Ok(r) => r,
                    Err(_) => return LoadResult::Empty {},
                };
                let tracks: Vec<Track> = response
                    .collection
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|item| item.track.as_ref().and_then(|t| parse_track(t).ok()))
                    .collect();
                if tracks.is_empty() {
                    return LoadResult::Empty {};
                }
                LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name: format!("Liked by {}", user_name),
                        selected_track: -1,
                    },
                    plugin_info: serde_json::json!({
                        "type": "playlist",
                        "url": url,
                        "author": user_name,
                        "totalTracks": tracks.len()
                    }),
                    tracks,
                })
            }
            Err(_) => LoadResult::Empty {},
        }
    }

    async fn load_user(&self, url: &str) -> LoadResult {
        let client_id = match self.token_tracker.get_client_id().await {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let caps = match user_url_re().captures(url) {
            Some(c) => c,
            None => return LoadResult::Empty {},
        };
        let username = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let sub_path = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        let clean_url = format!("https://soundcloud.com/{username}");
        let json = match api::api_resolve(&self.client, &clean_url, &client_id).await {
            Ok(j) => j,
            Err(_) => return LoadResult::Empty {},
        };
        let user_resp: UserResponseDto = match serde_json::from_value(json) {
            Ok(r) => r,
            Err(_) => return LoadResult::Empty {},
        };
        let user_id = match user_resp.id {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let user_name = user_resp
            .username
            .unwrap_or_else(|| "Unknown User".to_owned());
        match sub_path {
            "popular-tracks" => {
                self.load_collection_tracks(
                    user_id,
                    &user_name,
                    "toptracks",
                    "Popular tracks from",
                    &client_id,
                )
                .await
            }
            "albums" => {
                self.load_collection_tracks(
                    user_id,
                    &user_name,
                    "albums",
                    "Albums from",
                    &client_id,
                )
                .await
            }
            "sets" => {
                self.load_collection_tracks(
                    user_id,
                    &user_name,
                    "playlists",
                    "Sets from",
                    &client_id,
                )
                .await
            }
            "reposts" => {
                self.load_collection_tracks(
                    user_id,
                    &user_name,
                    "reposts",
                    "Reposts from",
                    &client_id,
                )
                .await
            }
            "tracks" => {
                self.load_collection_tracks(
                    user_id,
                    &user_name,
                    "tracks",
                    "Tracks from",
                    &client_id,
                )
                .await
            }
            "" | "spotlight" => {
                let result = self
                    .load_collection_tracks(
                        user_id,
                        &user_name,
                        "spotlight",
                        "Spotlight tracks from",
                        &client_id,
                    )
                    .await;
                if matches!(result, LoadResult::Empty {}) && sub_path.is_empty() {
                    self.load_collection_tracks(
                        user_id,
                        &user_name,
                        "tracks",
                        "Tracks from",
                        &client_id,
                    )
                    .await
                } else {
                    result
                }
            }
            _ => LoadResult::Empty {},
        }
    }

    async fn load_collection_tracks(
        &self,
        user_id: u64,
        user_name: &str,
        endpoint: &str,
        playlist_prefix: &str,
        client_id: &str,
    ) -> LoadResult {
        let json = match api::load_collection_tracks_api(&self.client, user_id, endpoint, client_id)
            .await
        {
            Ok(j) => j,
            Err(_) => return LoadResult::Empty {},
        };
        let mut tracks = Vec::new();
        if let Some(collection) = json.get("collection").and_then(|v| v.as_array()) {
            for item in collection {
                let track_json = if item.get("track").is_some() {
                    item.get("track")
                } else if item.get("kind").and_then(|v| v.as_str()) == Some("track") {
                    Some(item)
                } else if item.get("playlist").is_some() {
                    None
                } else {
                    Some(item)
                };
                if let Some(tj) = track_json {
                    if let Ok(track_dto) = serde_json::from_value::<TrackDto>(tj.clone()) {
                        if let Ok(track) = parse_track(&track_dto) {
                            tracks.push(track);
                        }
                    }
                }
            }
        }

        if tracks.is_empty() {
            return LoadResult::Empty {};
        }

        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: format!("{} {}", playlist_prefix, user_name),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
                "type": match endpoint { "albums" => "album", "playlists" | "sets" => "playlist", _ => "artist" },
                "url": format!("https://soundcloud.com/{}/{}", user_name, endpoint),
                "author": user_name,
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }
}

#[async_trait]
impl SourcePlugin for SoundCloudSource {
    fn name(&self) -> &str {
        "soundcloud"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        if self
            .search_prefixes()
            .into_iter()
            .any(|p| identifier.starts_with(p))
        {
            return true;
        }
        let url = identifier
            .strip_prefix("https://m.")
            .map(|s| format!("https://{s}"))
            .unwrap_or_else(|| identifier.to_owned());
        short_url_re().is_match(&url)
            || mobile_url_re().is_match(identifier)
            || liked_url_re().is_match(&url)
            || playlist_url_re().is_match(&url)
            || user_url_re().is_match(&url)
            || search_url_re().is_match(&url)
            || track_url_re().is_match(&url)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["scsearch:"]
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
            let query = identifier.strip_prefix(prefix).unwrap();
            return self.search_tracks(query.trim()).await;
        }
        let url = if mobile_url_re().is_match(identifier) {
            match self.resolve_mobile_url(identifier).await {
                Some(u) => u,
                None => return LoadResult::Empty {},
            }
        } else if short_url_re().is_match(identifier) {
            match self.resolve_short_url(identifier).await {
                Some(u) => u,
                None => return LoadResult::Empty {},
            }
        } else {
            identifier
                .strip_prefix("https://m.")
                .map(|s| format!("https://{s}"))
                .unwrap_or_else(|| identifier.to_owned())
        };
        if search_url_re().is_match(&url)
            && let Ok(uri) = reqwest::Url::parse(&url)
            && let Some((_, query)) = uri.query_pairs().find(|(k, _)| k == "q")
        {
            return self.search_tracks(&query).await;
        }
        if liked_url_re().is_match(&url) {
            return self.load_liked_tracks(&url).await;
        }
        if playlist_url_re().is_match(&url) {
            return self.load_playlist(&url).await;
        }
        if user_url_re().is_match(&url) {
            return self.load_user(&url).await;
        }
        if track_url_re().is_match(&url) {
            return self.load_single_track(&url).await;
        }
        LoadResult::Empty {}
    }

    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let url = if mobile_url_re().is_match(identifier) {
            self.resolve_mobile_url(identifier).await?
        } else if short_url_re().is_match(identifier) {
            self.resolve_short_url(identifier).await?
        } else {
            identifier
                .strip_prefix("https://m.")
                .map(|s| format!("https://{s}"))
                .unwrap_or_else(|| identifier.to_owned())
        };
        let client_id = self.token_tracker.get_client_id().await?;
        let local_addr = routeplanner.and_then(|rp| rp.get_address());
        self.get_track_from_url(&url, &client_id, local_addr).await
    }

    fn get_proxy_config(&self) -> Option<crate::config::HttpProxyConfig> {
        self.config.proxy.clone()
    }
}
