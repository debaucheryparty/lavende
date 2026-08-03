use crate::{
    protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, SearchResult},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashSet;
use std::sync::Arc;
use token::AppleMusicTokenTracker;

pub mod api;
pub mod extractor;
pub mod token;

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

    pub(crate) async fn resolve_track(&self, id: &str) -> LoadResult {
        let path = format!("/catalog/{}/songs/{}", self.country_code, id);
        let data = match api::api_request(&self.client, &self.token_tracker, &path).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        if let Some(item) = data.pointer("/data/0")
            && let Some(track) = extractor::build_track(item, None)
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
        let data = match api::api_request(&self.client, &self.token_tracker, &path).await {
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
            let extra = api::fetch_paginated_tracks(
                &self.client,
                &self.token_tracker,
                next_url_owned,
                load_limit,
                concurrency,
            )
            .await;
            all_items.extend(extra);
        }
        let mut tracks = Vec::new();
        for item in all_items {
            if let Some(track) = extractor::build_track(&item, artwork.clone()) {
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
        let data = match api::api_request(&self.client, &self.token_tracker, &path).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let tracks_data = data.pointer("/data").and_then(|v| v.as_array());
        let artist_path = format!("/catalog/{}/artists/{}", self.country_code, id);
        let artist_data = api::api_request(&self.client, &self.token_tracker, &artist_path).await;
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
                if let Some(track) = extractor::build_track(item, artwork.clone()) {
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

    pub(crate) async fn search(&self, query: &str) -> LoadResult {
        let encoded_query = urlencoding::encode(query);
        let path = format!(
            "/catalog/{}/search?term={}&limit=10&types=songs",
            self.country_code, encoded_query
        );
        let data = match api::api_request(&self.client, &self.token_tracker, &path).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let songs = data
            .pointer("/results/songs/data")
            .and_then(|v| v.as_array());
        let mut tracks = Vec::new();
        if let Some(items) = songs {
            for item in items {
                if let Some(track) = extractor::build_track(item, None) {
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
        let mut url = format!("{}{}", api::API_BASE, path);
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
        let json = api::api_request(&self.client, &self.token_tracker, &url).await?;
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
                    if let Some(track) = extractor::build_track(content, None) {
                        tracks.push(track);
                    }
                }
                "albums" => {
                    if let Some(album) = extractor::build_collection(content, "album") {
                        albums.push(album);
                    }
                }
                "artists" => {
                    if let Some(artist) = extractor::build_collection(content, "artist") {
                        artists.push(artist);
                    }
                }
                "playlists" => {
                    if let Some(playlist) = extractor::build_collection(content, "playlist") {
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
            if type_str == "album" && song_id.is_some() {
                let s_id = song_id.unwrap();
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
    ) -> Option<SearchResult> {
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
