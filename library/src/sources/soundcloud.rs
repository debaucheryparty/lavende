pub mod manager {
    use super::{
        token::SoundCloudTokenTracker,
        track::{SoundCloudStreamKind, SoundCloudTrack},
    };
    use crate::{
        protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track, TrackInfo},
        sources::{SourcePlugin, playable_track::BoxedTrack},
    };
    use async_trait::async_trait;
    use regex::Regex;
    use serde_json::Value;
    use std::sync::{Arc, OnceLock};
    use tracing::{debug, error, trace, warn};
    const BASE_URL: &str = "https://api-v2.soundcloud.com";
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
            Regex::new(
                r"^https?://(?:www\.|m\.)?soundcloud\.com/([a-zA-Z0-9_-]+)/likes/?(?:\?.*)?$",
            )
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
        RE.get_or_init(|| {
            Regex::new(r#""urn":"soundcloud:users:(\d+)","username":"([^"]+)""#).unwrap()
        })
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
        async fn api_resolve(&self, url: &str, client_id: &str) -> Option<Value> {
            let req_url = format!(
                "{}/resolve?url={}&client_id={}",
                BASE_URL,
                urlencoding::encode(url),
                client_id
            );
            debug!("SoundCloud: Resolving URL: {}", req_url);
            let builder = self.client.get(&req_url);
            let resp = builder.send().await.ok()?;
            if resp.status().as_u16() == 401 {
                self.token_tracker.invalidate().await;
                return None;
            }
            if !resp.status().is_success() {
                warn!(
                    "SoundCloud: API resolve failed with status: {} for {}",
                    resp.status(),
                    url
                );
                return None;
            }
            let json: Value = resp.json().await.ok()?;
            trace!("SoundCloud: API resolve response: {:?}", json);
            Some(json)
        }
        fn parse_track(&self, json: &Value) -> Result<Track, String> {
            let id = json
                .get("id")
                .and_then(|v| {
                    v.as_str()
                        .map(|s| s.to_owned())
                        .or_else(|| Some(v.to_string()))
                })
                .ok_or_else(|| "Missing track ID".to_owned())?;
            let title = json
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_owned();
            trace!("SoundCloud: Parsing track {}: {}", id, title);
            if json.get("policy").and_then(|v| v.as_str()) == Some("BLOCK") {
                trace!(
                    "SoundCloud: Track '{}' is blocked by policy (likely geo-blocked). Returning metadata for mirroring.",
                    title
                );
            }
            if json.get("monetization_model").and_then(|v| v.as_str()) == Some("SUB_HIGH_TIER") {
                trace!("SoundCloud: Track '{}' is a Go+ (premium) track", title);
            }
            if let Some(transcodings) = json
                .get("media")
                .and_then(|m| m.get("transcodings"))
                .and_then(|v| v.as_array())
            {
                let all_preview = !transcodings.is_empty()
                    && transcodings.iter().all(|t| {
                        let snipped = t.get("snipped").and_then(|v| v.as_bool()).unwrap_or(false);
                        let url = t.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        snipped
                            || url.contains("/preview/")
                            || url.contains("cf-preview-media.sndcdn.com")
                    });
                if all_preview {
                    trace!(
                        "SoundCloud: Track '{}' only has preview transcodings. Returning metadata for mirroring.",
                        title
                    );
                }
            }
            let author = json
                .get("user")
                .and_then(|u| u.get("username"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_owned();
            let duration = json
                .get("full_duration")
                .or_else(|| json.get("duration"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let uri = json
                .get("permalink_url")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned());
            let artwork_url = json
                .get("artwork_url")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.replace("-large", "-t500x500"));
            let isrc = json
                .get("publisher_metadata")
                .and_then(|m| m.get("isrc"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned());
            let track = Track::new(TrackInfo {
                identifier: id,
                is_seekable: true,
                author,
                length: duration,
                is_stream: false,
                position: 0,
                title,
                uri: uri.clone(),
                artwork_url,
                isrc,
                source_name: "soundcloud".to_owned(),
            });
            Ok(track)
        }
        fn select_format(transcodings: &[Value]) -> Option<(SoundCloudStreamKind, String)> {
            if transcodings.is_empty() {
                return None;
            }
            macro_rules! find_transcoding {
                ($protocol:expr, $mime_contains:expr) => {
                    transcodings.iter().find(|t| {
                        let fmt = t.get("format");
                        let proto = fmt
                            .and_then(|f| f.get("protocol"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let mime = fmt
                            .and_then(|f| f.get("mime_type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let snipped = t.get("snipped").and_then(|v| v.as_bool()).unwrap_or(false);
                        let url = t.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        !snipped
                            && !url.contains("/preview/")
                            && !url.contains("cf-preview-media.sndcdn.com")
                            && proto == $protocol
                            && mime.contains($mime_contains)
                    })
                };
            }
            let selected = find_transcoding!("progressive", "mpeg")
                .or_else(|| find_transcoding!("progressive", "aac"))
                .or_else(|| find_transcoding!("hls", "mpeg"))
                .or_else(|| find_transcoding!("hls", "aac"))
                .or_else(|| find_transcoding!("hls", "mp4"))
                .or_else(|| find_transcoding!("hls", "m4a"))
                .or_else(|| find_transcoding!("hls", "ogg"))
                .or_else(|| {
                    transcodings.iter().find(|t| {
                        t.get("format")
                            .and_then(|f| f.get("protocol"))
                            .and_then(|v| v.as_str())
                            == Some("progressive")
                    })
                })
                .or_else(|| {
                    transcodings.iter().find(|t| {
                        t.get("format")
                            .and_then(|f| f.get("protocol"))
                            .and_then(|v| v.as_str())
                            == Some("hls")
                    })
                })
                .or_else(|| transcodings.first())?;
            let lookup_url = selected.get("url").and_then(|v| v.as_str())?.to_owned();
            let proto = selected
                .get("format")
                .and_then(|f| f.get("protocol"))
                .and_then(|v| v.as_str())
                .unwrap_or("progressive");
            let mime = selected
                .get("format")
                .and_then(|f| f.get("mime_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let kind = if proto == "progressive" {
                if mime.contains("mpeg") || mime.contains("mp3") {
                    SoundCloudStreamKind::ProgressiveMp3
                } else {
                    SoundCloudStreamKind::ProgressiveAac
                }
            } else {
                if mime.contains("ogg") {
                    SoundCloudStreamKind::HlsOpus
                } else if mime.contains("mpeg") || mime.contains("mp3") {
                    SoundCloudStreamKind::HlsMp3
                } else if mime.contains("aac") || mime.contains("mp4") || mime.contains("m4a") {
                    SoundCloudStreamKind::HlsAac
                } else {
                    SoundCloudStreamKind::HlsAac
                }
            };
            Some((kind, lookup_url))
        }
        async fn resolve_stream_url(&self, lookup_url: &str, client_id: &str) -> Option<String> {
            let url = format!("{}?client_id={}", lookup_url, client_id);
            let builder = self.client.get(&url);
            let resp = builder.send().await.ok()?;
            if resp.status().as_u16() == 401 {
                self.token_tracker.invalidate().await;
                return None;
            }
            if !resp.status().is_success() {
                return None;
            }
            let json: Value = resp.json().await.ok()?;
            let stream_url = json
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            if let Some(ref url) = stream_url {
                debug!("SoundCloud: Resolved playback URL: {}", url);
            }
            stream_url
        }
        async fn get_track_from_url(
            &self,
            url: &str,
            client_id: &str,
            local_addr: Option<std::net::IpAddr>,
        ) -> Option<BoxedTrack> {
            let json = self.api_resolve(url, client_id).await?;
            if json.get("kind").and_then(|v| v.as_str()) != Some("track") {
                return None;
            }
            let transcodings = json
                .get("media")
                .and_then(|m| m.get("transcodings"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if transcodings.is_empty() {
                warn!("SoundCloud: No transcodings for track {}", url);
                return None;
            }
            let (kind, lookup_url) = Self::select_format(&transcodings)?;
            trace!("SoundCloud: Selected format {:?} for {}", kind, url);
            let stream_url = self.resolve_stream_url(&lookup_url, client_id).await?;
            if stream_url.contains("cf-preview-media.sndcdn.com")
                || stream_url.contains("/preview/")
            {
                warn!("SoundCloud: Track {} only has a preview URL, skipping", url);
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
            let req_url = format!(
                "{}/search/tracks?q={}&client_id={}&limit={}&offset=0",
                BASE_URL,
                urlencoding::encode(query),
                client_id,
                limit
            );
            let builder = self.client.get(&req_url);
            let resp = match builder.send().await {
                Ok(r) => r,
                Err(e) => {
                    error!("SoundCloud search error: {}", e);
                    return LoadResult::Empty {};
                }
            };
            if !resp.status().is_success() {
                return LoadResult::Empty {};
            }
            let json: Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => return LoadResult::Empty {},
            };
            let tracks: Vec<Track> = json
                .get("collection")
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|item| self.parse_track(item).ok())
                .collect();
            if tracks.is_empty() {
                LoadResult::Empty {}
            } else {
                LoadResult::Search(tracks)
            }
        }
        async fn load_single_track(&self, url: &str) -> LoadResult {
            let client_id = match self.token_tracker.get_client_id().await {
                Some(id) => id,
                None => return LoadResult::Empty {},
            };
            let json = match self.api_resolve(url, &client_id).await {
                Some(v) => v,
                None => return LoadResult::Empty {},
            };
            match self.parse_track(&json) {
                Ok(track) => LoadResult::Track(track),
                Err(msg) => {
                    warn!("SoundCloud: Failed to parse track: {}", msg);
                    LoadResult::Empty {}
                }
            }
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
        async fn load_playlist(&self, url: &str) -> LoadResult {
            let client_id = match self.token_tracker.get_client_id().await {
                Some(id) => id,
                None => return LoadResult::Empty {},
            };
            let json = match self.api_resolve(url, &client_id).await {
                Some(v) => v,
                None => return LoadResult::Empty {},
            };
            if json.get("kind").and_then(|v| v.as_str()) != Some("playlist") {
                return LoadResult::Empty {};
            }
            let name = json
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled playlist")
                .to_owned();
            let raw_tracks = json
                .get("tracks")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut complete: Vec<Track> = Vec::new();
            let mut stub_ids: Vec<String> = Vec::new();
            for t in &raw_tracks {
                if t.get("title").is_some() {
                    if let Ok(track) = self.parse_track(t) {
                        complete.push(track);
                    }
                } else if let Some(id) = t.get("id").map(|v| v.to_string()) {
                    stub_ids.push(id);
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
                let batch_url = format!("{BASE_URL}/tracks?ids={ids}&client_id={client_id}");
                let builder = self.client.get(&batch_url);
                if let Ok(resp) = builder.send().await
                    && let Ok(json) = resp.json::<Value>().await
                    && let Some(arr) = json.as_array()
                {
                    for item in arr {
                        if let Ok(track) = self.parse_track(item) {
                            complete.push(track);
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
                plugin_info: serde_json::json!({ "type": json.get("kind").and_then(|v| v.as_str()).unwrap_or("playlist"), "url": url, "artworkUrl": json.get("artwork_url").and_then(|v| v.as_str()).map(|s| s.replace("-large", "-t500x500")), "author": json.get("user").and_then(|u| u.get("username")).and_then(|v| v.as_str()), "totalTracks": json.get("track_count").and_then(|v| v.as_u64()).unwrap_or(complete.len() as u64) }),
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
            let liked_url = format!(
                "{BASE_URL}/users/{user_id}/likes?limit=200&offset=0&client_id={client_id}"
            );
            let resp = match self.client.get(&liked_url).send().await {
                Ok(r) => r,
                Err(_) => return LoadResult::Empty {},
            };
            let json: Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => return LoadResult::Empty {},
            };
            let tracks: Vec<Track> = json
                .get("collection")
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|item| item.get("track").and_then(|t| self.parse_track(t).ok()))
                .collect();
            if tracks.is_empty() {
                return LoadResult::Empty {};
            }
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: format!("Liked by {}", user_name),
                    selected_track: -1,
                },
                plugin_info: serde_json::json!({ "type": "playlist", "url": url, "author": user_name, "totalTracks": tracks.len() }),
                tracks,
            })
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
            let json = match self.api_resolve(&clean_url, &client_id).await {
                Some(v) => v,
                None => return LoadResult::Empty {},
            };
            if json.get("kind").and_then(|v| v.as_str()) != Some("user") {
                return LoadResult::Empty {};
            }
            let user_id = match json.get("id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => return LoadResult::Empty {},
            };
            let user_name = json
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown User")
                .to_owned();
            debug!(
                "SoundCloud: Loading user '{}' (id={}) with sub-path '{}'",
                user_name, user_id, sub_path
            );
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
            let req_url = format!(
                "{BASE_URL}/users/{user_id}/{endpoint}?client_id={client_id}&limit=200&offset=0&linked_partitioning=1"
            );
            let resp = match self.client.get(&req_url).send().await {
                Ok(r) => r,
                Err(e) => {
                    error!("SoundCloud: Request for {} failed: {}", endpoint, e);
                    return LoadResult::Empty {};
                }
            };
            if !resp.status().is_success() {
                warn!(
                    "SoundCloud: Request for {} returned status {}",
                    endpoint,
                    resp.status()
                );
                return LoadResult::Empty {};
            }
            let mut tracks: Vec<Track> = Vec::new();
            if let Ok(json) = resp.json::<Value>().await {
                let collection = json.get("collection").and_then(|v| v.as_array());
                if let Some(items) = collection {
                    for item in items {
                        let track_json = if item.get("track").is_some() {
                            item.get("track")
                        } else if item.get("kind").and_then(|v| v.as_str()) == Some("track") {
                            Some(item)
                        } else if item.get("playlist").is_some() {
                            None
                        } else {
                            Some(item)
                        };
                        if let Some(tj) = track_json
                            && let Ok(track) = self.parse_track(tj)
                        {
                            tracks.push(track);
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
                plugin_info: serde_json::json!({ "type": match endpoint { "albums" => "album", "playlists" | "sets" => "playlist", _ => "artist" }, "url": format!("https://soundcloud.com/{}/{}", user_name, endpoint), "author": user_name, "totalTracks": tracks.len() }),
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
}
pub mod reader {
    use crate::{
        audio::source::{HttpSource, create_client},
        common::types::AnyResult,
        config::HttpProxyConfig,
        sources::youtube::hls::{
            fetcher::fetch_segment_into, resolver::resolve_playlist,
            ts_demux::extract_adts_from_ts, types::Resource,
        },
    };
    use parking_lot::{Condvar, Mutex};
    use std::{
        io::{Read, Seek, SeekFrom},
        sync::Arc,
        thread,
    };
    use symphonia::core::io::MediaSource;
    use tracing::debug;
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
    const PREFETCH_SEGMENTS: usize = 3;
    const LOW_WATER_BYTES: usize = 128 * 1024;
    pub struct SoundCloudReader {
        inner: HttpSource,
    }
    impl SoundCloudReader {
        pub async fn new(
            url: &str,
            local_addr: Option<std::net::IpAddr>,
            proxy: Option<HttpProxyConfig>,
        ) -> AnyResult<Self> {
            let client = create_client(USER_AGENT.to_owned(), local_addr, proxy, None)?;
            let inner = HttpSource::new(client, url).await?;
            Ok(Self { inner })
        }
    }
    impl Read for SoundCloudReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.inner.read(buf)
        }
    }
    impl Seek for SoundCloudReader {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(pos)
        }
    }
    impl MediaSource for SoundCloudReader {
        fn is_seekable(&self) -> bool {
            self.inner.is_seekable()
        }
        fn byte_len(&self) -> Option<u64> {
            self.inner.byte_len()
        }
    }
    #[derive(Debug, Clone)]
    enum PrefetchCommand {
        Continue,
        Seek(usize),
        Stop,
    }
    struct SharedState {
        next_buf: Vec<u8>,
        need_data: bool,
        pending: Vec<Resource>,
        current_segment_index: usize,
        command: PrefetchCommand,
        seek_done: bool,
        eos: bool,
    }
    pub struct SoundCloudHlsReader {
        buf: Vec<u8>,
        pos: usize,
        total_bytes_read: u64,
        shared: Arc<(Mutex<SharedState>, Condvar)>,
        bg_thread: Option<thread::JoinHandle<()>>,
        all_segments: Vec<Resource>,
        segment_durations: Vec<f64>,
        byte_rate: u64,
    }
    impl SoundCloudHlsReader {
        pub async fn new(
            manifest_url: &str,
            bitrate_bps: u64,
            local_addr: Option<std::net::IpAddr>,
            proxy: Option<HttpProxyConfig>,
        ) -> AnyResult<Self> {
            let client = create_client(USER_AGENT.to_owned(), local_addr, proxy, None)?;
            let (segment_urls, _map_url) = resolve_playlist(&client, manifest_url).await?;
            if segment_urls.is_empty() {
                return Err("SoundCloud HLS: playlist contained no segments".into());
            }
            let segment_durations: Vec<f64> = segment_urls
                .iter()
                .map(|r| r.duration.unwrap_or(0.0))
                .collect();
            let all_segments = segment_urls.clone();
            let byte_rate = bitrate_bps / 8;
            let mut initial_buf = Vec::with_capacity(512 * 1024);
            let first_batch_count = PREFETCH_SEGMENTS.min(segment_urls.len());
            let mut pending = segment_urls;
            let first_batch: Vec<Resource> = pending.drain(..first_batch_count).collect();
            for res in &first_batch {
                let _ = fetch_and_demux_into(&client, res, &mut initial_buf).await;
            }
            debug!(
                "SoundCloud HLS init: {} segments, bitrate={} bps ({} B/s)",
                all_segments.len(),
                bitrate_bps,
                byte_rate
            );
            let shared_state = SharedState {
                next_buf: Vec::with_capacity(512 * 1024),
                need_data: true,
                pending,
                current_segment_index: first_batch.len(),
                command: PrefetchCommand::Continue,
                seek_done: false,
                eos: false,
            };
            let shared = Arc::new((Mutex::new(shared_state), Condvar::new()));
            let shared_bg = Arc::clone(&shared);
            let bg_client = client;
            let bg_all = all_segments.clone();
            let bg_thread = thread::Builder::new()
                .name("sc-hls-prefetch".into())
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(prefetch_loop(shared_bg, bg_client, bg_all));
                })?;
            Ok(Self {
                buf: initial_buf,
                pos: 0,
                total_bytes_read: 0,
                shared,
                bg_thread: Some(bg_thread),
                all_segments,
                segment_durations,
                byte_rate,
            })
        }
        fn seek_to_byte(&mut self, target_byte: u64) -> std::io::Result<u64> {
            let current_byte = self.total_bytes_read;
            let diff = (target_byte as i64) - (current_byte as i64);
            let buf_len = self.buf.len() as i64;
            let current_pos_in_buf = self.pos as i64;
            let new_pos_in_buf = current_pos_in_buf + diff;
            if new_pos_in_buf >= 0 && new_pos_in_buf <= buf_len {
                debug!(
                    "SoundCloud HLS gapless seek (internal buffer): {} -> {} (pos {} -> {})",
                    current_byte, target_byte, self.pos, new_pos_in_buf
                );
                self.pos = new_pos_in_buf as usize;
                self.total_bytes_read = target_byte;
                return Ok(target_byte);
            }
            let target_secs = target_byte as f64 / self.byte_rate as f64;
            let mut segment_start_secs = 0.0;
            let mut target_index = 0;
            for (i, &dur) in self.segment_durations.iter().enumerate() {
                if segment_start_secs + dur <= target_secs {
                    segment_start_secs += dur;
                    target_index = i + 1;
                } else {
                    break;
                }
            }
            if target_index >= self.all_segments.len() {
                target_index = self.all_segments.len().saturating_sub(1);
            }
            let segment_start_byte = (segment_start_secs * self.byte_rate as f64) as u64;
            let skip_in_segment = target_byte.saturating_sub(segment_start_byte);
            debug!(
                "SoundCloud HLS hard seek: target {} -> segment {} (starts at {:.1}s, segment-relative skip={} bytes)",
                target_byte, target_index, segment_start_secs, skip_in_segment
            );
            self.buf.clear();
            self.pos = 0;
            self.total_bytes_read = target_byte;
            {
                let (lock, cvar) = &*self.shared;
                let mut state = lock.lock();
                state.command = PrefetchCommand::Seek(target_index);
                state.need_data = true;
                state.seek_done = false;
                cvar.notify_one();
                while !state.seek_done {
                    cvar.wait(&mut state);
                }
                state.seek_done = false;
                std::mem::swap(&mut self.buf, &mut state.next_buf);
                state.next_buf.clear();
                debug!(
                    "SoundCloud HLS swapped buffers after hard seek. Buffer len: {}",
                    self.buf.len()
                );
                self.pos = (skip_in_segment as usize).min(self.buf.len());
                if self.pos > 0 || skip_in_segment > 0 {
                    debug!(
                        "SoundCloud HLS aligned buffer position to offset {} (skip_in_segment={})",
                        self.pos, skip_in_segment
                    );
                }
                state.need_data = true;
                cvar.notify_one();
            }
            Ok(target_byte)
        }
    }
    impl Read for SoundCloudHlsReader {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            if self.pos < self.buf.len() {
                let remaining = self.buf.len() - self.pos;
                if remaining <= LOW_WATER_BYTES {
                    let (lock, cvar) = &*self.shared;
                    if let Some(mut state) = lock.try_lock()
                        && !state.need_data
                        && !state.eos
                    {
                        state.need_data = true;
                        cvar.notify_one();
                    }
                }
                let n = out.len().min(remaining);
                out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
                self.pos += n;
                self.total_bytes_read += n as u64;
                return Ok(n);
            }
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            if !state.need_data && state.next_buf.is_empty() && !state.eos {
                state.need_data = true;
                cvar.notify_one();
            }
            while state.next_buf.is_empty() && !state.eos {
                cvar.wait(&mut state);
            }
            if state.next_buf.is_empty() && state.eos {
                return Ok(0);
            }
            let next_len = state.next_buf.len();
            self.buf.clear();
            self.pos = 0;
            std::mem::swap(&mut self.buf, &mut state.next_buf);
            state.next_buf.clear();
            debug!(
                "SoundCloud HLS buffer swap: replaced active with next_buf ({} bytes)",
                next_len
            );
            state.need_data = true;
            cvar.notify_one();
            drop(state);
            self.read(out)
        }
    }
    impl Seek for SoundCloudHlsReader {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            match pos {
                SeekFrom::Start(n) => self.seek_to_byte(n),
                SeekFrom::Current(delta) => {
                    let target = self.total_bytes_read.saturating_add_signed(delta);
                    self.seek_to_byte(target)
                }
                SeekFrom::End(_) => {
                    let total = self.byte_len().unwrap_or(0);
                    self.seek_to_byte(total)
                }
            }
        }
    }
    impl MediaSource for SoundCloudHlsReader {
        fn is_seekable(&self) -> bool {
            true
        }
        fn byte_len(&self) -> Option<u64> {
            let total_dur: f64 = self.segment_durations.iter().sum();
            Some((total_dur * self.byte_rate as f64) as u64)
        }
    }
    impl Drop for SoundCloudHlsReader {
        fn drop(&mut self) {
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            state.command = PrefetchCommand::Stop;
            state.need_data = true;
            cvar.notify_one();
            drop(state);
            if let Some(handle) = self.bg_thread.take() {
                let _ = handle.join();
            }
        }
    }
    async fn prefetch_loop(
        shared: Arc<(Mutex<SharedState>, Condvar)>,
        client: reqwest::Client,
        all_segments: Vec<Resource>,
    ) {
        let (lock, cvar) = &*shared;
        loop {
            enum Action {
                Stop,
                Seek {
                    target_index: usize,
                    batch: Vec<Resource>,
                },
                Fetch {
                    batch: Vec<Resource>,
                    current_idx: usize,
                },
                Eos,
            }
            let action = {
                let mut state = lock.lock();
                while !state.need_data {
                    cvar.wait(&mut state);
                }
                match std::mem::replace(&mut state.command, PrefetchCommand::Continue) {
                    PrefetchCommand::Stop => Action::Stop,
                    PrefetchCommand::Seek(target_index) => {
                        state.next_buf.clear();
                        state.eos = false;
                        state.current_segment_index = target_index;
                        state.pending = all_segments[target_index..].to_vec();
                        let count = if !state.pending.is_empty() { 1 } else { 0 };
                        let batch = state.pending.drain(..count).collect();
                        Action::Seek {
                            target_index,
                            batch,
                        }
                    }
                    PrefetchCommand::Continue => {
                        if state.pending.is_empty() {
                            state.eos = true;
                            state.need_data = false;
                            cvar.notify_one();
                            Action::Eos
                        } else {
                            let count = PREFETCH_SEGMENTS.min(state.pending.len());
                            let batch = state.pending.drain(..count).collect();
                            let current_idx = state.current_segment_index;
                            Action::Fetch { batch, current_idx }
                        }
                    }
                }
            };
            match action {
                Action::Stop => return,
                Action::Eos => continue,
                Action::Seek {
                    target_index,
                    batch,
                } => {
                    let mut tmp_buf = Vec::new();
                    for res in &batch {
                        debug!(
                            "SoundCloud HLS prefetcher: fetching seek target segment {}",
                            target_index
                        );
                        let _ = fetch_and_demux_into(&client, res, &mut tmp_buf).await;
                    }
                    debug!(
                        "SoundCloud HLS prefetcher: seek target fetched ({} bytes)",
                        tmp_buf.len()
                    );
                    let mut state = lock.lock();
                    state.next_buf.extend_from_slice(&tmp_buf);
                    state.current_segment_index += batch.len();
                    state.need_data = false;
                    state.seek_done = true;
                    state.eos = state.pending.is_empty();
                    cvar.notify_one();
                }
                Action::Fetch { batch, current_idx } => {
                    let mut tmp_buf = Vec::with_capacity(256 * 1024);
                    for res in &batch {
                        {
                            let s = lock.lock();
                            if !matches!(s.command, PrefetchCommand::Continue) {
                                break;
                            }
                        }
                        let _ = fetch_and_demux_into(&client, res, &mut tmp_buf).await;
                    }
                    let mut state = lock.lock();
                    if !matches!(state.command, PrefetchCommand::Continue) {
                        continue;
                    }
                    state.next_buf.extend_from_slice(&tmp_buf);
                    state.current_segment_index = current_idx + batch.len();
                    state.eos = state.pending.is_empty();
                    state.need_data = false;
                    cvar.notify_one();
                }
            }
        }
    }
    async fn fetch_and_demux_into(
        client: &reqwest::Client,
        res: &Resource,
        out: &mut Vec<u8>,
    ) -> AnyResult<()> {
        let mut raw = Vec::new();
        fetch_segment_into(client, res, &mut raw).await?;
        if raw.first() == Some(&0x47) {
            let adts = extract_adts_from_ts(&raw);
            if !adts.is_empty() {
                out.extend_from_slice(&adts);
            } else {
                out.extend_from_slice(&raw);
            }
        } else {
            out.extend_from_slice(&raw);
        }
        Ok(())
    }
}
pub mod token {
    use crate::common::types::SharedRw;
    use regex::Regex;
    use std::{
        sync::{Arc, OnceLock},
        time::{Duration, Instant},
    };
    use tokio::sync::RwLock;
    use tracing::{debug, error, info, trace, warn};
    const SOUNDCLOUD_URL: &str = "https://soundcloud.com";
    const CLIENT_ID_REFRESH_INTERVAL: Duration = Duration::from_secs(3600);
    fn asset_re() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| {
            Regex::new(r"https://a-v2\.sndcdn\.com/assets/[a-zA-Z0-9_-]+\.js").unwrap()
        })
    }
    fn client_id_re() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r#"[^_]client_id[:"=]+\s*"?([a-zA-Z0-9_-]{20,})"?"#).unwrap())
    }
    pub struct SoundCloudTokenTracker {
        client: Arc<reqwest::Client>,
        client_id: SharedRw<CachedClientId>,
    }
    struct CachedClientId {
        value: Option<String>,
        last_updated: Option<Instant>,
    }
    impl CachedClientId {
        fn is_stale(&self) -> bool {
            match self.last_updated {
                None => true,
                Some(t) => t.elapsed() > CLIENT_ID_REFRESH_INTERVAL,
            }
        }
    }
    impl SoundCloudTokenTracker {
        pub fn new(client: Arc<reqwest::Client>, config: &crate::config::SoundCloudConfig) -> Self {
            let cached = CachedClientId {
                value: config.client_id.clone(),
                last_updated: if config.client_id.is_some() {
                    Some(Instant::now())
                } else {
                    None
                },
            };
            Self {
                client,
                client_id: Arc::new(RwLock::new(cached)),
            }
        }
        pub async fn get_client_id(&self) -> Option<String> {
            {
                let guard = self.client_id.read().await;
                if !guard.is_stale()
                    && let Some(id) = &guard.value
                {
                    return Some(id.clone());
                }
            }
            self.refresh_client_id().await
        }
        pub async fn refresh_client_id(&self) -> Option<String> {
            debug!("Refreshing SoundCloud client_id...");
            trace!("SoundCloud: Fetching client_id from soundcloud.com...");
            let html = match self.client.get(SOUNDCLOUD_URL).send().await {
                Ok(r) => match r.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        error!("SoundCloud: Failed to read main page: {}", e);
                        return None;
                    }
                },
                Err(e) => {
                    error!("SoundCloud: Failed to fetch main page: {}", e);
                    return None;
                }
            };
            if let Some(caps) = client_id_re().captures(&html)
                && let Some(m) = caps.get(1)
            {
                let id = m.as_str().to_owned();
                trace!("SoundCloud: Found client_id in main page: {id}");
                self.store_client_id(id.clone()).await;
                info!("Successfully refreshed SoundCloud client_id");
                return Some(id);
            }
            let asset_urls: Vec<String> = asset_re()
                .find_iter(&html)
                .map(|m| m.as_str().to_owned())
                .collect();
            if asset_urls.is_empty() {
                warn!("SoundCloud: No asset JS URLs found in main page");
                return None;
            }
            trace!(
                "SoundCloud: Found {} asset URLs, probing for client_id",
                asset_urls.len()
            );
            for url in asset_urls.iter().rev().take(9) {
                let js = match self.client.get(url).send().await {
                    Ok(r) => match r.text().await {
                        Ok(t) => t,
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                };
                if let Some(caps) = client_id_re().captures(&js)
                    && let Some(m) = caps.get(1)
                {
                    let id = m.as_str().to_owned();
                    trace!("SoundCloud: Found client_id in asset {url}: {id}");
                    self.store_client_id(id.clone()).await;
                    info!("Successfully refreshed SoundCloud client_id");
                    return Some(id);
                }
            }
            warn!("SoundCloud: client_id not found in any asset scripts");
            None
        }
        pub async fn invalidate(&self) {
            let mut guard = self.client_id.write().await;
            guard.last_updated = None;
        }
        async fn store_client_id(&self, id: String) {
            let mut guard = self.client_id.write().await;
            guard.value = Some(id);
            guard.last_updated = Some(Instant::now());
        }
        pub fn init(self: Arc<Self>) {
            let this = self.clone();
            tokio::spawn(async move {
                this.get_client_id().await;
            });
        }
    }
}
pub mod track {
    use crate::{
        common::types::AudioFormat,
        config::HttpProxyConfig,
        sources::playable_track::{PlayableTrack, ResolvedTrack},
    };
    use async_trait::async_trait;
    use std::net::IpAddr;
    #[derive(Debug, Clone)]
    pub enum SoundCloudStreamKind {
        ProgressiveMp3,
        ProgressiveAac,
        HlsOpus,
        HlsMp3,
        HlsAac,
    }
    pub struct SoundCloudTrack {
        pub stream_url: String,
        pub kind: SoundCloudStreamKind,
        pub bitrate_bps: u64,
        pub local_addr: Option<IpAddr>,
        pub proxy: Option<HttpProxyConfig>,
    }
    #[async_trait]
    impl PlayableTrack for SoundCloudTrack {
        async fn resolve(&self) -> Result<ResolvedTrack, String> {
            let (reader, hint) = match self.kind {
                SoundCloudStreamKind::ProgressiveMp3 => (
                    super::reader::SoundCloudReader::new(
                        &self.stream_url,
                        self.local_addr,
                        self.proxy.clone(),
                    )
                    .await
                    .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                    .map_err(|e| format!("Failed to open stream: {e}"))?,
                    AudioFormat::Mp3,
                ),
                SoundCloudStreamKind::ProgressiveAac => (
                    super::reader::SoundCloudReader::new(
                        &self.stream_url,
                        self.local_addr,
                        self.proxy.clone(),
                    )
                    .await
                    .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                    .map_err(|e| format!("Failed to open stream: {e}"))?,
                    AudioFormat::Mp4,
                ),
                SoundCloudStreamKind::HlsOpus => (
                    super::reader::SoundCloudHlsReader::new(
                        &self.stream_url,
                        self.bitrate_bps,
                        self.local_addr,
                        self.proxy.clone(),
                    )
                    .await
                    .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                    .map_err(|e| format!("Failed to init HLS reader: {e}"))?,
                    AudioFormat::Opus,
                ),
                SoundCloudStreamKind::HlsMp3 => (
                    super::reader::SoundCloudHlsReader::new(
                        &self.stream_url,
                        self.bitrate_bps,
                        self.local_addr,
                        self.proxy.clone(),
                    )
                    .await
                    .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                    .map_err(|e| format!("Failed to init HLS reader: {e}"))?,
                    AudioFormat::Mp3,
                ),
                SoundCloudStreamKind::HlsAac => (
                    super::reader::SoundCloudHlsReader::new(
                        &self.stream_url,
                        self.bitrate_bps,
                        self.local_addr,
                        self.proxy.clone(),
                    )
                    .await
                    .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                    .map_err(|e| format!("Failed to init HLS reader: {e}"))?,
                    AudioFormat::Aac,
                ),
            };
            Ok(ResolvedTrack::new(reader, Some(hint)))
        }
    }
}
pub use manager::SoundCloudSource;
