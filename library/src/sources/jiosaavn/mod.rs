pub mod api;
pub mod decrypt;
pub mod extractor;
pub mod stream;

use crate::{
    common::types::AudioFormat,
    protocol::tracks::{LoadError, LoadResult, PlaylistData, PlaylistInfo},
    sources::{
        SourcePlugin,
        playable_track::{BoxedTrack, PlayableTrack, ResolvedTrack},
    },
};
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;
use std::sync::{Arc, OnceLock};

use decrypt::decrypt_url;
use extractor::{JioSaavnTrackDto, clean_string, parse_track};
use stream::JioSaavnReader;

fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?(?:jiosaavn|saavn)\.com/(?:(?<type>p/album|s/featured|s/artist|s/song|album|featured|song|s/playlist|artist)/)(?:[^/]+/)+(?<id>[A-Za-z0-9_,-]+)").unwrap()
    })
}

pub struct JioSaavnTrack {
    pub encrypted_url: String,
    pub secret_key: Vec<u8>,
    pub is_320: bool,
    pub local_addr: Option<std::net::IpAddr>,
    pub proxy: Option<crate::config::HttpProxyConfig>,
}

#[async_trait]
impl PlayableTrack for JioSaavnTrack {
    fn supports_seek(&self) -> bool {
        true
    }

    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let url = self.resolve_url().ok_or_else(|| {
            "Failed to decrypt JioSaavn URL. Check secretKey in config.toml".to_string()
        })?;
        let hint = format_hint_from_url(&url);
        let reader = JioSaavnReader::new(&url, self.local_addr, self.proxy.clone())
            .await
            .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
            .map_err(|e| format!("Failed to open stream: {e}"))?;
        Ok(ResolvedTrack::new(reader, hint))
    }
}

impl JioSaavnTrack {
    fn resolve_url(&self) -> Option<String> {
        let mut url = decrypt_url(&self.encrypted_url, &self.secret_key)?;
        if self.is_320 {
            url = url.replace("_96.mp4", "_320.mp4");
        }
        Some(url)
    }
}

fn format_hint_from_url(url: &str) -> Option<AudioFormat> {
    std::path::Path::new(url)
        .extension()
        .and_then(|s| s.to_str())
        .map(AudioFormat::from_ext)
        .filter(|f| *f != AudioFormat::Unknown)
        .or(Some(AudioFormat::Mp4))
}

pub struct JioSaavnSource {
    client: Arc<reqwest::Client>,
    secret_key: Vec<u8>,
    proxy: Option<crate::config::HttpProxyConfig>,
    api_url: String,
    search_limit: usize,
    recommendations_limit: usize,
    playlist_load_limit: usize,
    album_load_limit: usize,
    artist_load_limit: usize,
}

impl JioSaavnSource {
    pub fn new(
        config: Option<crate::config::JioSaavnConfig>,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let (
            secret_key,
            search_limit,
            recommendations_limit,
            playlist_load_limit,
            album_load_limit,
            artist_load_limit,
            proxy,
            api_url,
        ) = if let Some(c) = config {
            (
                c.decryption
                    .and_then(|d| d.secret_key)
                    .unwrap_or_else(|| "38346591".to_owned()),
                c.search_limit,
                c.recommendations_limit,
                c.playlist_load_limit,
                c.album_load_limit,
                c.artist_load_limit,
                c.proxy,
                c.api_url
                    .unwrap_or_else(|| "https://www.jiosaavn.com/api.php".to_owned()),
            )
        } else {
            (
                "38346591".to_owned(),
                10,
                10,
                50,
                50,
                20,
                None,
                "https://www.jiosaavn.com/api.php".to_owned(),
            )
        };
        Ok(Self {
            client,
            secret_key: secret_key.into_bytes(),
            proxy,
            search_limit,
            recommendations_limit,
            playlist_load_limit,
            album_load_limit,
            artist_load_limit,
            api_url,
        })
    }

    pub async fn fetch_metadata(&self, id: &str) -> Option<Value> {
        let params = vec![
            ("__call", "webapi.get"),
            ("api_version", "4"),
            ("_format", "json"),
            ("_marker", "0"),
            ("ctx", "web6dot0"),
            ("token", id),
            ("type", "song"),
        ];
        api::get_json(&self.client, &self.api_url, &params)
            .await
            .ok()
            .and_then(|json| {
                json.get("songs")
                    .and_then(|s| s.get(0))
                    .cloned()
                    .or_else(|| (json.get("id").is_some()).then_some(json))
            })
    }

    pub async fn resolve_list(&self, type_: &str, id: &str) -> LoadResult {
        let t = if type_ == "featured" || type_ == "s/playlist" {
            "playlist"
        } else {
            type_
        };
        let n = if type_ == "artist" {
            self.artist_load_limit
        } else if type_ == "album" {
            self.album_load_limit
        } else {
            self.playlist_load_limit
        };
        let n_str = n.to_string();
        let mut params = vec![
            ("__call", "webapi.get"),
            ("api_version", "4"),
            ("_format", "json"),
            ("_marker", "0"),
            ("ctx", "web6dot0"),
            ("token", id),
            ("type", t),
        ];
        if type_ == "artist" {
            params.push(("n_song", &n_str));
        } else {
            params.push(("n", &n_str));
        }
        match api::get_json(&self.client, &self.api_url, &params).await {
            Ok(data) => {
                let list = data
                    .get("list")
                    .or_else(|| data.get("topSongs"))
                    .and_then(|v| v.as_array());
                if let Some(arr) = list
                    && !arr.is_empty()
                {
                    let tracks: Vec<_> = arr
                        .iter()
                        .filter_map(|item| {
                            let dto =
                                serde_json::from_value::<JioSaavnTrackDto>(item.clone()).ok()?;
                            parse_track(&dto)
                        })
                        .collect();
                    let mut name = clean_string(
                        data.get("title")
                            .or_else(|| data.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                    );
                    if type_ == "artist" {
                        name = format!("{name}'s Top Tracks");
                    }
                    return LoadResult::Playlist(PlaylistData {
                        info: PlaylistInfo {
                            name,
                            selected_track: -1,
                        },
                        plugin_info: serde_json::json!({
                            "url": data.get("perma_url").and_then(|v| v.as_str()),
                            "type": type_,
                            "artworkUrl": data.get("image").and_then(|v| v.as_str()).map(|s| s.replace("150x150", "500x500").replace("50x50", "500x500")),
                            "author": data.get("subtitle").or_else(|| data.get("header_desc")).and_then(|v| v.as_str()).map(|s| s.split(',').take(3).collect::<Vec<_>>().join(", ")),
                            "totalTracks": data.get("list_count").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(tracks.len() as u64)
                        }),
                        tracks,
                    });
                }
                LoadResult::Empty {}
            }
            Err(e) => LoadResult::Error(LoadError {
                message: Some("JioSaavn list fetch failed".to_owned()),
                severity: crate::common::Severity::Common,
                cause: e.to_string(),
                cause_stack_trace: None,
            }),
        }
    }

    pub async fn search(&self, query: &str) -> LoadResult {
        let params = vec![
            ("__call", "search.getResults"),
            ("api_version", "4"),
            ("_format", "json"),
            ("_marker", "0"),
            ("cc", "in"),
            ("ctx", "web6dot0"),
            ("includeMetaTags", "1"),
            ("q", query),
        ];

        match api::get_json(&self.client, &self.api_url, &params).await {
            Ok(json) => {
                if let Some(results) = json.get("results").and_then(|v| v.as_array()) {
                    if results.is_empty() {
                        return LoadResult::Empty {};
                    }
                    let tracks: Vec<_> = results
                        .iter()
                        .take(self.search_limit)
                        .filter_map(|item| {
                            let dto =
                                serde_json::from_value::<JioSaavnTrackDto>(item.clone()).ok()?;
                            parse_track(&dto)
                        })
                        .collect();
                    return LoadResult::Search(tracks);
                }
                LoadResult::Empty {}
            }
            Err(e) => LoadResult::Error(LoadError {
                message: Some("JioSaavn search failed".to_owned()),
                severity: crate::common::Severity::Common,
                cause: e.to_string(),
                cause_stack_trace: None,
            }),
        }
    }

    pub async fn get_recommendations(&self, query: &str) -> LoadResult {
        let mut id = query.to_owned();
        let id_regex = Regex::new(r"^[A-Za-z0-9_,-]+$").unwrap();
        if !id_regex.is_match(query) {
            if let LoadResult::Search(tracks) = self.search(query).await {
                if let Some(first) = tracks.first() {
                    id = first.info.identifier.clone();
                } else {
                    return LoadResult::Empty {};
                }
            } else {
                return LoadResult::Empty {};
            }
        }
        let encoded_id = format!("[\"{id}\"]");
        let params = vec![
            ("__call", "webradio.createEntityStation"),
            ("api_version", "4"),
            ("_format", "json"),
            ("_marker", "0"),
            ("ctx", "android"),
            ("entity_id", &encoded_id),
            ("entity_type", "queue"),
        ];
        let station_id = api::get_json(&self.client, &self.api_url, &params)
            .await
            .ok()
            .and_then(|json| {
                json.get("stationid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned())
            });
        if let Some(sid) = station_id {
            let k_limit = self.recommendations_limit.to_string();
            let params = vec![
                ("__call", "webradio.getSong"),
                ("api_version", "4"),
                ("_format", "json"),
                ("_marker", "0"),
                ("ctx", "android"),
                ("stationid", &sid),
                ("k", &k_limit),
            ];
            if let Ok(json) = api::get_json(&self.client, &self.api_url, &params).await
                && let Some(obj) = json.as_object()
            {
                let tracks: Vec<_> = obj
                    .values()
                    .filter_map(|v| {
                        let song_val = v.get("song")?;
                        let dto =
                            serde_json::from_value::<JioSaavnTrackDto>(song_val.clone()).ok()?;
                        parse_track(&dto)
                    })
                    .collect();
                if !tracks.is_empty() {
                    return LoadResult::Playlist(PlaylistData {
                        info: PlaylistInfo {
                            name: "JioSaavn Recommendations".to_owned(),
                            selected_track: -1,
                        },
                        plugin_info: serde_json::json!({
                            "type": "recommendations",
                            "totalTracks": tracks.len()
                        }),
                        tracks,
                    });
                }
            }
        }
        if let Some(metadata) = self.fetch_metadata(&id).await
            && let Some(artist_ids) = metadata.get("primary_artists_id").and_then(|v| v.as_str())
        {
            let params = vec![
                ("__call", "search.artistOtherTopSongs"),
                ("api_version", "4"),
                ("_format", "json"),
                ("_marker", "0"),
                ("ctx", "wap6dot0"),
                ("artist_ids", artist_ids),
                ("song_id", &id),
                ("language", "unknown"),
            ];
            if let Ok(json) = api::get_json(&self.client, &self.api_url, &params).await
                && let Some(arr) = json.as_array()
            {
                let tracks: Vec<_> = arr
                    .iter()
                    .take(self.recommendations_limit)
                    .filter_map(|item| {
                        let dto = serde_json::from_value::<JioSaavnTrackDto>(item.clone()).ok()?;
                        parse_track(&dto)
                    })
                    .collect();
                if !tracks.is_empty() {
                    return LoadResult::Playlist(PlaylistData {
                        info: PlaylistInfo {
                            name: "JioSaavn Recommendations".to_owned(),
                            selected_track: -1,
                        },
                        plugin_info: serde_json::json!({
                            "type": "recommendations",
                            "totalTracks": tracks.len()
                        }),
                        tracks,
                    });
                }
            }
        }
        LoadResult::Empty {}
    }
}

#[async_trait]
impl SourcePlugin for JioSaavnSource {
    fn name(&self) -> &str {
        "jiosaavn"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || self
                .rec_prefixes()
                .iter()
                .any(|p| identifier.starts_with(p))
            || url_regex().is_match(identifier)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["saavnsearch:", "saavn:"]
    }

    fn rec_prefixes(&self) -> Vec<&str> {
        vec!["saavnrec:"]
    }

    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        if let Some(prefix) = self
            .search_prefixes()
            .iter()
            .find(|p| identifier.starts_with(*p))
        {
            let query = &identifier[prefix.len()..];
            return self.search(query.trim()).await;
        }
        if let Some(prefix) = self
            .rec_prefixes()
            .iter()
            .find(|p| identifier.starts_with(*p))
        {
            let query = &identifier[prefix.len()..];
            return self.get_recommendations(query.trim()).await;
        }
        if let Some(caps) = url_regex().captures(identifier) {
            let t = caps.name("type").map(|m| m.as_str()).unwrap_or("");
            let id = caps.name("id").map(|m| m.as_str()).unwrap_or("");
            if t == "song" {
                if let Some(meta) = self.fetch_metadata(id).await {
                    if let Ok(dto) = serde_json::from_value::<JioSaavnTrackDto>(meta) {
                        if let Some(track) = parse_track(&dto) {
                            return LoadResult::Track(track);
                        }
                    }
                }
            } else {
                return self.resolve_list(t, id).await;
            }
        }
        LoadResult::Empty {}
    }

    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let meta = if let Some(caps) = url_regex().captures(identifier) {
            let t = caps.name("type").map(|m| m.as_str()).unwrap_or("");
            let id = caps.name("id").map(|m| m.as_str()).unwrap_or("");
            if t == "song" {
                self.fetch_metadata(id).await?
            } else {
                return None;
            }
        } else {
            self.fetch_metadata(identifier).await?
        };

        let media_url = meta
            .pointer("/more_info/encrypted_media_url")
            .or_else(|| meta.get("encrypted_media_url"))
            .and_then(|v| v.as_str())?;

        let is_320 = meta
            .pointer("/more_info/320kbps")
            .or_else(|| meta.get("320kbps"))
            .and_then(|v| v.as_str())
            .map(|s| s == "true")
            .unwrap_or(false)
            || meta
                .pointer("/more_info/320kbps")
                .or_else(|| meta.get("320kbps"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

        let local_addr = routeplanner.and_then(|rp| rp.get_address());

        Some(Arc::new(JioSaavnTrack {
            encrypted_url: media_url.to_owned(),
            secret_key: self.secret_key.clone(),
            is_320,
            local_addr,
            proxy: self.proxy.clone(),
        }))
    }
}
