use crate::{
    protocol::tracks::{LoadResult, Track},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;
use std::sync::{Arc, OnceLock};

pub mod api;
pub mod extractor;
pub mod reader;
pub mod recommendations;
pub mod token;
pub mod track;

const PUBLIC_API_BASE: &str = "https://api.deezer.com";
const PRIVATE_API_BASE: &str = "https://www.deezer.com/ajax/gw-light.php";
const REC_TRACK_PREFIX: &str = "dzrec:track:";
const REC_ARTIST_PREFIX: &str = "dzrec:artist:";

fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"https?://(?:www\.)?deezer\.com/(?:[a-z]{2}/)?(?:track|album|playlist|artist)/(\d+)",
        )
        .expect("deezer URL regex is a valid literal")
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
    pub token_tracker: Arc<token::DeezerTokenTracker>,
    _search_limit: usize,
    _playlist_load_limit: usize,
    _album_load_limit: usize,
}

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
            const DECRYPTION_KEY_HASH: [u8; 32] = [
                52, 76, 41, 138, 120, 133, 48, 72, 198, 74, 16, 75, 82, 101, 186, 223, 15, 190,
                111, 218, 176, 71, 103, 11, 181, 136, 155, 247, 66, 203, 218, 240,
            ];
            if hasher.finalize().as_slice() != DECRYPTION_KEY_HASH {
                tracing::warn!("Deezer master decryption key is invalid, playback may not work!");
            }
        }
        let token_tracker = Arc::new(token::DeezerTokenTracker::new(client.clone(), arls));
        Ok(Self {
            client,
            token_tracker,
            config,
            _search_limit: 10,
            _playlist_load_limit: 10000,
            _album_load_limit: 10000,
        })
    }

    async fn search(&self, query: &str) -> LoadResult {
        let url = format!("search?q={}", urlencoding::encode(query));
        if let Some(json) = self.get_json_public(&url).await {
            if let Some(data) = json.get("data").and_then(|v| v.as_array()) {
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
        }
        LoadResult::Empty {}
    }

    async fn get_track_by_isrc(&self, isrc: &str) -> Option<Track> {
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

    pub(crate) async fn get_json_public(&self, path: &str) -> Option<Value> {
        let url = format!("{PUBLIC_API_BASE}/{path}");
        self.client.get(&url).send().await.ok()?.json().await.ok()
    }

    pub(crate) fn parse_track(&self, json: &Value) -> Option<Track> {
        extractor::parse_track(json)
    }

    pub(crate) fn parse_recommendation_track(&self, json: &Value) -> Option<Track> {
        extractor::parse_recommendation_track(json)
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

        if all_types || types.contains(&"album".to_owned()) {
            if let Some(data) = json.pointer("/albums/data").and_then(|v| v.as_array()) {
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
                    albums.push(crate::protocol::tracks::PlaylistData {
                        info: crate::protocol::tracks::PlaylistInfo {
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
        }

        if all_types || types.contains(&"artist".to_owned()) {
            if let Some(data) = json.pointer("/artists/data").and_then(|v| v.as_array()) {
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
                    artists.push(crate::protocol::tracks::PlaylistData {
                        info: crate::protocol::tracks::PlaylistInfo {
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
        }

        if all_types || types.contains(&"playlist".to_owned()) {
            if let Some(data) = json.pointer("/playlists/data").and_then(|v| v.as_array()) {
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
                    playlists.push(crate::protocol::tracks::PlaylistData {
                        info: crate::protocol::tracks::PlaylistInfo {
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
        }

        if all_types || types.contains(&"track".to_owned()) {
            if let Some(data) = json.pointer("/tracks/data").and_then(|v| v.as_array()) {
                for track in data {
                    if let Some(parsed) = self.parse_track(track) {
                        tracks.push(parsed);
                    }
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

#[async_trait]
impl SourcePlugin for DeezerSource {
    fn name(&self) -> &str {
        "deezer"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || identifier.starts_with(REC_TRACK_PREFIX)
            || identifier.starts_with(REC_ARTIST_PREFIX)
            || share_url_regex().is_match(identifier)
            || url_regex().is_match(identifier)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["dzsearch:", "deezersearch:"]
    }

    async fn load(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        if identifier.starts_with(REC_TRACK_PREFIX) || identifier.starts_with(REC_ARTIST_PREFIX) {
            let query = if identifier.starts_with(REC_TRACK_PREFIX) {
                identifier
                    .strip_prefix(REC_TRACK_PREFIX)
                    .unwrap_or(identifier)
            } else {
                identifier
                    .strip_prefix(REC_ARTIST_PREFIX)
                    .unwrap_or(identifier)
            };
            return recommendations::get_recommendations(self, query).await;
        }

        for prefix in self.search_prefixes() {
            if let Some(query) = identifier.strip_prefix(prefix) {
                return self.search(query.trim()).await;
            }
        }

        if let Some(prefix) = vec!["dzisrc:"]
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            if let Some(isrc) = identifier.strip_prefix(prefix) {
                if let Some(track) = self.get_track_by_isrc(isrc).await {
                    return LoadResult::Track(track);
                }
                return LoadResult::Empty {};
            }
        }

        if share_url_regex().is_match(identifier) {
            if let Some(resolved) = self.resolve_share_url(identifier).await {
                return self.load(&resolved, routeplanner).await;
            }
            return LoadResult::Empty {};
        }

        if let Some(caps) = url_regex().captures(identifier) {
            let id = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if id.is_empty() {
                return LoadResult::Empty {};
            }

            if identifier.contains("/track/") {
                return api::get_track(self, id).await;
            } else if identifier.contains("/album/") {
                return api::get_album(self, id).await;
            } else if identifier.contains("/playlist/") {
                return api::get_playlist(self, id).await;
            } else if identifier.contains("/artist/") {
                return api::get_artist(self, id).await;
            }
        }

        LoadResult::Empty {}
    }

    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let track_id = if let Some(caps) = url_regex().captures(identifier) {
            caps.get(1).map(|m| m.as_str())?.to_owned()
        } else {
            identifier.to_owned()
        };
        let resolved =
            track::verify_track_resolvable(&self.client, &track_id, &self.token_tracker).await;
        if resolved.is_none() {
            tracing::warn!("Deezer: no stream URL for track {track_id}, falling back to mirrors");
            return None;
        }
        Some(Arc::new(track::DeezerTrack {
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
