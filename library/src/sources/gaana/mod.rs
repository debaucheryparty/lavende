use crate::{
    protocol::tracks::{LoadError, LoadResult, PlaylistData, PlaylistInfo, Track},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::warn;

pub mod api;
pub mod crypto;
pub mod extractor;
pub mod reader;
pub mod track;

pub struct GaanaSource {
    client: Arc<reqwest::Client>,
    stream_quality: String,
    proxy: Option<crate::config::HttpProxyConfig>,
    search_limit: usize,
    playlist_load_limit: usize,
    album_load_limit: usize,
    artist_load_limit: usize,
}

impl GaanaSource {
    pub fn new(
        config: Option<crate::config::GaanaConfig>,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let (
            stream_quality,
            search_limit,
            playlist_load_limit,
            album_load_limit,
            artist_load_limit,
            proxy,
        ) = if let Some(c) = config {
            (
                c.stream_quality.unwrap_or_else(|| "high".to_owned()),
                c.search_limit,
                c.playlist_load_limit,
                c.album_load_limit,
                c.artist_load_limit,
                c.proxy,
            )
        } else {
            ("high".to_owned(), 10, 50, 50, 20, None)
        };
        Ok(Self {
            client,
            stream_quality,
            proxy,
            search_limit,
            playlist_load_limit,
            album_load_limit,
            artist_load_limit,
        })
    }

    async fn get_json(&self, params: &[(&str, &str)], referer_path: &str) -> Option<Value> {
        api::get_json(&self.client, params, referer_path).await
    }

    async fn load_song(&self, seokey: &str) -> LoadResult {
        let params = [("type", "songDetail"), ("seokey", seokey)];
        let data = match self.get_json(&params, &format!("song/{seokey}")).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let tracks = match data.get("tracks").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => return LoadResult::Empty {},
        };
        match self.parse_track(&tracks[0]) {
            Some(track) => LoadResult::Track(track),
            None => LoadResult::Empty {},
        }
    }

    async fn load_album(&self, seokey: &str) -> LoadResult {
        let params = [("type", "albumDetail"), ("seokey", seokey)];
        let data = match self.get_json(&params, &format!("album/{seokey}")).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let tracks_arr = match data.get("tracks").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => return LoadResult::Empty {},
        };
        let album = data.get("album").unwrap_or(&Value::Null);
        let name = album
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Album");
        let tracks: Vec<Track> = tracks_arr
            .iter()
            .take(self.album_load_limit)
            .filter_map(|t| self.parse_track(t))
            .collect();
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: name.to_owned(),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
                "type": "album",
                "url": format!("https://gaana.com/album/{seokey}"),
                "artworkUrl": album.get("atw").or_else(|| album.get("artwork_large")).and_then(|v| v.as_str()),
                "author": album.get("artist").and_then(|a| a.as_array()).and_then(|arr| arr.first()).and_then(|a| a.get("name")).and_then(|v| v.as_str()),
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }

    async fn load_playlist(&self, seokey: &str) -> LoadResult {
        let params = [("type", "playlistDetail"), ("seokey", seokey)];
        let data = match self.get_json(&params, &format!("playlist/{seokey}")).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let tracks_arr = match data.get("tracks").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => return LoadResult::Empty {},
        };
        let playlist = data.get("playlist").unwrap_or(&Value::Null);
        let name = playlist
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Playlist");
        let tracks: Vec<Track> = tracks_arr
            .iter()
            .take(self.playlist_load_limit)
            .filter_map(|t| self.parse_track(t))
            .collect();
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: name.to_owned(),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
                "type": "playlist",
                "url": format!("https://gaana.com/playlist/{seokey}"),
                "artworkUrl": playlist.get("atw").or_else(|| playlist.get("artwork_large")).and_then(|v| v.as_str()),
                "author": playlist.get("created_by").and_then(|v| v.as_str()),
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }

    async fn load_artist(&self, seokey: &str) -> LoadResult {
        let detail_params = [("type", "artistDetail"), ("seokey", seokey)];
        let detail = match self
            .get_json(&detail_params, &format!("artist/{seokey}"))
            .await
        {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let artist_arr = match detail.get("artist").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => return LoadResult::Empty {},
        };
        let artist_data = &artist_arr[0];
        let artist_id = match artist_data.get("artist_id").and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_i64().map(|i| i.to_string()))
        }) {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let artist_name = artist_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Artist");
        let tracks_params = [
            ("language", ""),
            ("order", "0"),
            ("page", "0"),
            ("sortBy", "popularity"),
            ("type", "artistTrackList"),
            ("id", &artist_id),
        ];
        let tracks_data = match self
            .get_json(&tracks_params, &format!("artist/{seokey}"))
            .await
        {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let entities_arr = match tracks_data.get("entities").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => return LoadResult::Empty {},
        };
        let tracks: Vec<Track> = entities_arr
            .iter()
            .take(self.artist_load_limit)
            .filter_map(|t| self.parse_entity_track(t))
            .collect();
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: format!("{artist_name}'s Top Tracks"),
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
                "type": "artist",
                "url": format!("https://gaana.com/artist/{seokey}"),
                "artworkUrl": artist_data.get("atw").and_then(|v| v.as_str()),
                "author": artist_name,
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }

    async fn search(&self, query: &str) -> LoadResult {
        let params = [
            ("country", "IN"),
            ("page", "0"),
            ("secType", "track"),
            ("type", "search"),
            ("keyword", query),
        ];
        let data = match self
            .get_json(&params, &format!("search/{}", urlencoding::encode(query)))
            .await
        {
            Some(d) => d,
            None => {
                return LoadResult::Error(LoadError {
                    message: Some("Gaana search failed".to_owned()),
                    cause: String::new(),
                    cause_stack_trace: None,
                    severity: crate::common::Severity::Common,
                });
            }
        };
        let gr = match data.get("gr").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return LoadResult::Empty {},
        };
        let track_group = gr
            .iter()
            .find(|g| g.get("ty").and_then(|v| v.as_str()) == Some("Track"));
        let items = match track_group
            .and_then(|g| g.get("gd"))
            .and_then(|v| v.as_array())
        {
            Some(arr) if !arr.is_empty() => arr,
            _ => return LoadResult::Empty {},
        };
        let mut results = Vec::new();
        for item in items.iter().take(self.search_limit) {
            let seokey = item
                .get("seo")
                .and_then(|v| v.as_str())
                .or_else(|| item.get("id").and_then(|v| v.as_str()));
            if let Some(key) = seokey {
                if let LoadResult::Track(track) = self.load_song(key).await {
                    results.push(track);
                }
            }
        }
        if results.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Search(results)
        }
    }

    pub fn parse_track(&self, json: &Value) -> Option<Track> {
        extractor::parse_track(json)
    }

    pub fn parse_entity_track(&self, json: &Value) -> Option<Track> {
        extractor::parse_entity_track(json)
    }
}

#[async_trait]
impl SourcePlugin for GaanaSource {
    fn name(&self) -> &str {
        "gaana"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || extractor::url_regex().is_match(identifier)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["gnsearch:", "gaanasearch:"]
    }

    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        for prefix in self.search_prefixes() {
            if let Some(query) = identifier.strip_prefix(prefix) {
                return self.search(query.trim()).await;
            }
        }
        if let Some(caps) = extractor::url_regex().captures(identifier) {
            let type_ = caps.name("type").map(|m| m.as_str()).unwrap_or("");
            let seokey = caps.name("seokey").map(|m| m.as_str()).unwrap_or("");
            if seokey.is_empty() || type_.is_empty() {
                return LoadResult::Empty {};
            }
            return match type_ {
                "song" => self.load_song(seokey).await,
                "album" => self.load_album(seokey).await,
                "playlist" => self.load_playlist(seokey).await,
                "artist" => self.load_artist(seokey).await,
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
        let track_id = if let Some(caps) = extractor::url_regex().captures(identifier) {
            if caps.name("type").map(|m| m.as_str()) != Some("song") {
                return None;
            }
            let seokey = caps.name("seokey").map(|m| m.as_str())?;
            let params = [("type", "songDetail"), ("seokey", seokey)];
            let data = self.get_json(&params, &format!("song/{seokey}")).await?;
            data.get("tracks")?
                .as_array()?
                .first()?
                .get("track_id")?
                .as_str()?
                .to_owned()
        } else {
            identifier.to_owned()
        };
        let stream_url =
            api::fetch_stream_url_internal(&self.client, &track_id, &self.stream_quality).await;
        if stream_url.is_none() {
            warn!("Gaana: no stream URL for track {track_id}, falling back to mirrors");
            return None;
        }
        Some(Arc::new(track::GaanaTrack {
            client: self.client.clone(),
            track_id,
            stream_quality: self.stream_quality.clone(),
            local_addr: routeplanner.and_then(|rp| rp.get_address()),
            proxy: self.proxy.clone(),
        }))
    }

    fn get_proxy_config(&self) -> Option<crate::config::HttpProxyConfig> {
        self.proxy.clone()
    }
}
