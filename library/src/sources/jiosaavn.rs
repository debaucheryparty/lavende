pub mod helpers {
    use serde_json::Value;
    use tracing::warn;
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36";
    pub async fn get_json(
        client: &reqwest::Client,
        api_url: &str,
        params: &[(&str, &str)],
    ) -> Option<Value> {
        let resp = match client
            .get(api_url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Referer", "https://www.jiosaavn.com/")
            .header("Origin", "https://www.jiosaavn.com")
            .query(params)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("JioSaavn request failed: {e}");
                return None;
            }
        };
        if !resp.status().is_success() {
            warn!("JioSaavn API error status: {}", resp.status());
            return None;
        }
        let text = match resp.text().await {
            Ok(text) => text,
            Err(e) => {
                warn!("Failed to read JioSaavn response body: {e}");
                return None;
            }
        };
        serde_json::from_str(&text).ok()
    }
    pub fn clean_string(s: &str) -> String {
        s.replace("&quot;", "\"").replace("&amp;", "&")
    }
}
pub mod metadata {
    use super::{
        JioSaavnSource,
        helpers::{clean_string, get_json},
        parser::parse_track,
    };
    use crate::protocol::tracks::{LoadError, LoadResult, PlaylistData, PlaylistInfo};
    use serde_json::Value;
    impl JioSaavnSource {
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
            get_json(&self.client, &self.api_url, &params)
                .await
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
            if let Some(data) = get_json(&self.client, &self.api_url, &params).await {
                let list = data
                    .get("list")
                    .or_else(|| data.get("topSongs"))
                    .and_then(|v| v.as_array());
                if let Some(arr) = list
                    && !arr.is_empty()
                {
                    let tracks: Vec<_> = arr.iter().filter_map(parse_track).collect();
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
            } else {
                LoadResult::Error(LoadError {
                    message: Some("JioSaavn list fetch failed".to_owned()),
                    severity: crate::common::Severity::Common,
                    cause: String::new(),
                    cause_stack_trace: None,
                })
            }
        }
    }
}
pub mod parser {
    use crate::protocol::tracks::{PlaylistData, PlaylistInfo, Track, TrackInfo};
    use serde_json::Value;
    pub fn parse_track(v: &Value) -> Option<Track> {
        let id = v.get("id").and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_i64().map(|i| i.to_string()))
        })?;
        let title = super::helpers::clean_string(
            v.get("title")
                .or_else(|| v.get("song"))
                .and_then(|v| v.as_str())?,
        );
        let duration = v
            .pointer("/more_info/duration")
            .or_else(|| v.get("duration"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .or_else(|| v.pointer("/more_info/duration").and_then(|v| v.as_u64()))
            .or_else(|| v.get("duration").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let artwork_url = v
            .get("image")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.replace("150x150", "500x500").replace("50x50", "500x500"));
        let artists_str = v
            .pointer("/more_info/artistMap/primary_artists")
            .and_then(|v| v.as_array())
            .filter(|arr| !arr.is_empty())
            .or_else(|| {
                v.pointer("/more_info/artistMap/artists")
                    .and_then(|v| v.as_array())
            })
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a.get("name").and_then(|v| v.as_str()))
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .or_else(|| {
                v.pointer("/more_info/music")
                    .or_else(|| v.get("subtitle"))
                    .or_else(|| v.get("primary_artists"))
                    .or_else(|| v.get("singers"))
                    .or_else(|| v.get("header_desc"))
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        s.split(',')
                            .map(|part| part.trim())
                            .take(3)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
            })
            .unwrap_or_else(|| "Unknown Artist".to_owned());
        let author = super::helpers::clean_string(&artists_str);
        let mut track = Track::new(TrackInfo {
            title,
            author,
            length: duration * 1000,
            identifier: id,
            source_name: "jiosaavn".to_owned(),
            uri: v
                .get("perma_url")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned()),
            artwork_url,
            is_stream: false,
            is_seekable: true,
            ..Default::default()
        });
        track.plugin_info = serde_json::json!({
            "albumName": v.get("album").or_else(|| v.pointer("/more_info/album")).and_then(|v| v.as_str()),
            "albumUrl": v.get("album_url").or_else(|| v.pointer("/more_info/album_url")).and_then(|v| v.as_str()),
            "artistUrl": v.pointer("/more_info/artistMap/primary_artists/0/perma_url").and_then(|v| v.as_str()),
            "artistArtworkUrl": v.pointer("/more_info/artistMap/primary_artists/0/image").and_then(|v| v.as_str()).map(|s| s.replace("150x150", "500x500").replace("50x50", "500x500")),
            "previewUrl": v.get("media_preview_url").or_else(|| v.pointer("/more_info/media_preview_url")).or_else(|| v.get("vlink")).or_else(|| v.pointer("/more_info/vlink")).and_then(|v| v.as_str()),
            "isPreview": false
        });
        Some(track)
    }
    pub fn parse_search_item(v: &Value) -> Option<Track> {
        let id = v.get("id").and_then(|v| v.as_str())?;
        let title = super::helpers::clean_string(v.get("title").and_then(|v| v.as_str())?);
        let artwork_url = v
            .get("image")
            .and_then(|v| v.as_str())
            .map(|s| s.replace("150x150", "500x500").replace("50x50", "500x500"));
        let author_str = v
            .get("subtitle")
            .or_else(|| v.get("description"))
            .and_then(|v| v.as_str())
            .map(|s| {
                s.split(',')
                    .map(|part| part.trim())
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "Unknown Artist".to_owned());
        let author = super::helpers::clean_string(&author_str);
        let mut track = Track::new(TrackInfo {
            title,
            author,
            length: 0,
            identifier: id.to_owned(),
            source_name: "jiosaavn".to_owned(),
            uri: v.get("url").and_then(|v| v.as_str()).map(|s| s.to_owned()),
            artwork_url,
            is_stream: false,
            is_seekable: true,
            ..Default::default()
        });
        track.plugin_info = serde_json::json!({
            "albumName": v.get("album").and_then(|v| v.as_str()),
            "previewUrl": v.get("vlink").or_else(|| v.pointer("/more_info/vlink")).and_then(|v| v.as_str()),
            "isPreview": true
        });
        Some(track)
    }
    pub fn parse_search_playlist(v: &Value, type_: &str) -> Option<PlaylistData> {
        let title = super::helpers::clean_string(v.get("title").and_then(|v| v.as_str())?);
        let artwork_url = v
            .get("image")
            .and_then(|v| v.as_str())
            .map(|s| s.replace("150x150", "500x500").replace("50x50", "500x500"));
        let mut url = v
            .get("url")
            .or_else(|| v.get("perma_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        if url.is_empty() {
            url = v
                .get("perma_url")
                .or_else(|| v.get("permaurl"))
                .or_else(|| v.get("token"))
                .or_else(|| v.pointer("/more_info/perma_url"))
                .or_else(|| v.pointer("/more_info/token"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
        }
        if url.is_empty()
            && let Some(id) = v.get("id").and_then(|v| v.as_str())
        {
            if id.starts_with('/') || id.starts_with("http") {
                url = id.to_owned();
            } else {
                let path_type = match type_ {
                    "playlist" => "s/playlist",
                    "featured" => "featured",
                    "album" => "album",
                    "artist" => "artist",
                    _ => type_,
                };
                let slug = title
                    .to_lowercase()
                    .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
                    .replace(' ', "-");
                url = format!("/{path_type}/{slug}/{id}");
            }
        }
        if !url.is_empty() && !url.starts_with("http") {
            url = format!("https://www.jiosaavn.com{url}");
        }
        let total_tracks = v
            .pointer("/more_info/song_count")
            .or_else(|| v.pointer("/more_info/track_count"))
            .or_else(|| v.get("song_count"))
            .or_else(|| v.get("track_count"))
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .or_else(|| v.as_u64())
            })
            .unwrap_or(0);
        let author_raw = v
            .pointer("/more_info/artist_name")
            .or_else(|| v.pointer("/more_info/music"))
            .or_else(|| v.get("music"))
            .or_else(|| v.get("subtitle"))
            .or_else(|| v.get("description"))
            .and_then(|v| v.as_str())
            .map(|s| {
                s.split(',')
                    .map(|part| part.trim())
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|s| !s.is_empty());
        let final_author = if type_ == "artist" {
            title.clone()
        } else {
            author_raw.unwrap_or_else(|| "Unknown Author".to_owned())
        };
        Some(PlaylistData {
            info: PlaylistInfo {
                name: title,
                selected_track: -1,
            },
            plugin_info: serde_json::json!({
                "url": url,
                "type": type_,
                "artworkUrl": artwork_url,
                "author": super::helpers::clean_string(&final_author),
                "totalTracks": total_tracks
            }),
            tracks: Vec::new(),
        })
    }
}
pub mod reader {
    use crate::{
        audio::source::{HttpSource, create_client},
        common::types::AnyResult,
    };
    use std::io::{Read, Seek, SeekFrom};
    use symphonia::core::io::MediaSource;
    pub struct JioSaavnReader {
        inner: HttpSource,
    }
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36";
    impl JioSaavnReader {
        pub async fn new(
            url: &str,
            local_addr: Option<std::net::IpAddr>,
            proxy: Option<crate::config::HttpProxyConfig>,
        ) -> AnyResult<Self> {
            let client = create_client(USER_AGENT.to_owned(), local_addr, proxy, None)?;
            let inner = HttpSource::new(client, url).await?;
            Ok(Self { inner })
        }
    }
    impl Read for JioSaavnReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.inner.read(buf)
        }
    }
    impl Seek for JioSaavnReader {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(pos)
        }
    }
    impl MediaSource for JioSaavnReader {
        fn is_seekable(&self) -> bool {
            self.inner.is_seekable()
        }
        fn byte_len(&self) -> Option<u64> {
            self.inner.byte_len()
        }
    }
}
pub mod recommendations {
    use super::{JioSaavnSource, helpers::get_json, parser::parse_track};
    use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo};
    use regex::Regex;
    impl JioSaavnSource {
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
            let station_id = get_json(&self.client, &self.api_url, &params)
                .await
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
                if let Some(json) = get_json(&self.client, &self.api_url, &params).await
                    && let Some(obj) = json.as_object()
                {
                    let tracks: Vec<_> = obj
                        .values()
                        .filter_map(|v| v.get("song"))
                        .filter_map(parse_track)
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
                && let Some(artist_ids) =
                    metadata.get("primary_artists_id").and_then(|v| v.as_str())
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
                if let Some(json) = get_json(&self.client, &self.api_url, &params).await
                    && let Some(arr) = json.as_array()
                {
                    let tracks: Vec<_> = arr
                        .iter()
                        .take(self.recommendations_limit)
                        .filter_map(parse_track)
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
}
pub mod search {
    use super::{
        JioSaavnSource,
        helpers::get_json,
        parser::{parse_search_item, parse_search_playlist, parse_track},
    };
    use crate::protocol::tracks::{LoadError, LoadResult, SearchResult};
    use tracing::debug;
    impl JioSaavnSource {
        pub async fn search(&self, query: &str) -> LoadResult {
            debug!("JioSaavn searching: {query}");
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
            if let Some(json) = get_json(&self.client, &self.api_url, &params).await {
                if let Some(results) = json.get("results").and_then(|v| v.as_array()) {
                    if results.is_empty() {
                        return LoadResult::Empty {};
                    }
                    let tracks: Vec<_> = results
                        .iter()
                        .take(self.search_limit)
                        .filter_map(parse_track)
                        .collect();
                    return LoadResult::Search(tracks);
                }
                LoadResult::Empty {}
            } else {
                LoadResult::Error(LoadError {
                    message: Some("JioSaavn search failed".to_owned()),
                    severity: crate::common::Severity::Common,
                    cause: String::new(),
                    cause_stack_trace: None,
                })
            }
        }
        pub async fn get_autocomplete(
            &self,
            query: &str,
            types: &[String],
        ) -> Option<SearchResult> {
            debug!("JioSaavn get_autocomplete: {query}");
            let params = vec![
                ("__call", "autocomplete.get"),
                ("api_version", "4"),
                ("_format", "json"),
                ("_marker", "0"),
                ("ctx", "web6dot0"),
                ("query", query),
            ];
            let json = get_json(&self.client, &self.api_url, &params).await?;
            let mut tracks = Vec::new();
            let mut albums = Vec::new();
            let mut artists = Vec::new();
            let mut playlists = Vec::new();
            let texts = Vec::new();
            let all_types = types.is_empty();
            if (all_types || types.contains(&"track".to_owned()))
                && let Some(songs) = json
                    .get("songs")
                    .and_then(|v| v.get("data"))
                    .and_then(|v| v.as_array())
            {
                for item in songs {
                    if let Some(track) = parse_search_item(item) {
                        tracks.push(track);
                    }
                }
            }
            if !tracks.is_empty() {
                let pids: Vec<String> = tracks.iter().map(|t| t.info.identifier.clone()).collect();
                let pids_str = pids.join(",");
                let details_params = vec![
                    ("__call", "song.getDetails"),
                    ("_format", "json"),
                    ("pids", &pids_str),
                ];
                if let Some(details_json) =
                    get_json(&self.client, &self.api_url, &details_params).await
                {
                    for track in &mut tracks {
                        if let Some(detail) = details_json.get(&track.info.identifier) {
                            if let Some(duration) = detail
                                .get("duration")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<u64>().ok())
                                .or_else(|| detail.get("duration").and_then(|v| v.as_u64()))
                            {
                                track.info.length = duration * 1000;
                            }
                            if let Some(perma_url) =
                                detail.get("perma_url").and_then(|v| v.as_str())
                            {
                                track.info.uri = Some(perma_url.to_owned());
                            }
                            track.plugin_info = serde_json::json!({
                                "albumName": detail
                                    .get("album")
                                    .or_else(|| detail.pointer("/more_info/album"))
                                    .and_then(|v| v.as_str()),
                                "albumUrl": detail
                                    .get("album_url")
                                    .or_else(|| detail.pointer("/more_info/album_url"))
                                    .and_then(|v| v.as_str()),
                                "artistUrl": detail
                                    .pointer("/more_info/artistMap/primary_artists/0/perma_url")
                                    .and_then(|v| v.as_str()),
                                "artistArtworkUrl": detail
                                    .pointer("/more_info/artistMap/primary_artists/0/image")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.replace("150x150", "500x500").replace("50x50", "500x500")),
                                "previewUrl": detail
                                    .get("media_preview_url")
                                    .or_else(|| detail.pointer("/more_info/media_preview_url"))
                                    .or_else(|| detail.get("vlink"))
                                    .or_else(|| detail.pointer("/more_info/vlink"))
                                    .and_then(|v| v.as_str()),
                                "isPreview": false
                            });
                            if let Some(artists) =
                                detail.get("primary_artists").and_then(|v| v.as_str())
                                && !artists.is_empty()
                            {
                                let limited_artists = artists
                                    .split(',')
                                    .map(|s| s.trim())
                                    .take(3)
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                track.info.author = super::helpers::clean_string(&limited_artists);
                            }
                            track.encoded = track.encode();
                        }
                    }
                }
            }
            if (all_types || types.contains(&"album".to_owned()))
                && let Some(data) = json
                    .get("albums")
                    .and_then(|v| v.get("data"))
                    .and_then(|v| v.as_array())
            {
                for item in data {
                    if let Some(pd) = parse_search_playlist(item, "album") {
                        albums.push(pd);
                    }
                }
            }
            if (all_types || types.contains(&"artist".to_owned()))
                && let Some(data) = json
                    .get("artists")
                    .and_then(|v| v.get("data"))
                    .and_then(|v| v.as_array())
            {
                for item in data {
                    if let Some(pd) = parse_search_playlist(item, "artist") {
                        artists.push(pd);
                    }
                }
            }
            if (all_types || types.contains(&"playlist".to_owned()))
                && let Some(data) = json
                    .get("playlists")
                    .and_then(|v| v.get("data"))
                    .and_then(|v| v.as_array())
            {
                for item in data {
                    if let Some(pd) = parse_search_playlist(item, "playlist") {
                        playlists.push(pd);
                    }
                }
            }
            if (all_types || types.is_empty())
                && let Some(top_data) = json
                    .get("topquery")
                    .and_then(|v| v.get("data"))
                    .and_then(|v| v.as_array())
            {
                for item in top_data {
                    let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match item_type {
                        "song" => {
                            if let Some(track) = parse_search_item(item)
                                && !tracks
                                    .iter()
                                    .any(|t| t.info.identifier == track.info.identifier)
                            {
                                tracks.insert(0, track);
                            }
                        }
                        "album" => {
                            if let Some(pd) = parse_search_playlist(item, "album")
                                && !albums.iter().any(|a| a.info.name == pd.info.name)
                            {
                                albums.insert(0, pd);
                            }
                        }
                        "artist" => {
                            if let Some(pd) = parse_search_playlist(item, "artist")
                                && !artists.iter().any(|a| a.info.name == pd.info.name)
                            {
                                artists.insert(0, pd);
                            }
                        }
                        "playlist" => {
                            if let Some(pd) = parse_search_playlist(item, "playlist")
                                && !playlists.iter().any(|a| a.info.name == pd.info.name)
                            {
                                playlists.insert(0, pd);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Some(SearchResult {
                tracks,
                albums,
                artists,
                playlists,
                texts,
                plugin: serde_json::json!({}),
            })
        }
    }
}
pub mod track {
    use crate::{
        common::AudioFormat,
        config::HttpProxyConfig,
        sources::playable_track::{PlayableTrack, ResolvedTrack},
    };
    use async_trait::async_trait;
    use base64::prelude::*;
    use des::{
        Des,
        cipher::{BlockDecrypt, KeyInit, generic_array::GenericArray},
    };
    use std::net::IpAddr;
    pub struct JioSaavnTrack {
        pub encrypted_url: String,
        pub secret_key: Vec<u8>,
        pub is_320: bool,
        pub local_addr: Option<IpAddr>,
        pub proxy: Option<HttpProxyConfig>,
    }
    #[async_trait]
    impl PlayableTrack for JioSaavnTrack {
        async fn resolve(&self) -> Result<ResolvedTrack, String> {
            let url = self.resolve_url().ok_or_else(|| {
                "Failed to decrypt JioSaavn URL. Check secretKey in config.toml".to_string()
            })?;
            let hint = format_hint_from_url(&url);
            let reader =
                super::reader::JioSaavnReader::new(&url, self.local_addr, self.proxy.clone())
                    .await
                    .map(|r| Box::new(r) as Box<dyn symphonia::core::io::MediaSource>)
                    .map_err(|e| format!("Failed to open stream: {e}"))?;
            Ok(ResolvedTrack::new(reader, hint))
        }
    }
    impl JioSaavnTrack {
        fn resolve_url(&self) -> Option<String> {
            let mut url = self.decrypt_url(&self.encrypted_url)?;
            if self.is_320 {
                url = url.replace("_96.mp4", "_320.mp4");
            }
            Some(url)
        }
        fn decrypt_url(&self, encrypted: &str) -> Option<String> {
            if self.secret_key.len() != 8 {
                return None;
            }
            let cipher = Des::new_from_slice(&self.secret_key).ok()?;
            let mut data = BASE64_STANDARD.decode(encrypted).ok()?;
            for chunk in data.chunks_mut(8) {
                if chunk.len() == 8 {
                    cipher.decrypt_block(GenericArray::from_mut_slice(chunk));
                }
            }
            if let Some(&last_byte) = data.last() {
                let padding = last_byte as usize;
                if (1..=8).contains(&padding) && data.len() >= padding {
                    data.truncate(data.len() - padding);
                }
            }
            String::from_utf8(data).ok()
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
}
use self::track::JioSaavnTrack;
use crate::{protocol::tracks::LoadResult, sources::playable_track::BoxedTrack};
use async_trait::async_trait;
use regex::Regex;
use std::sync::{Arc, OnceLock};
fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?(?:jiosaavn|saavn)\.com/(?:(?<type>p/album|s/featured|s/artist|s/song|album|featured|song|s/playlist|artist)/)(?:[^/]+/)+(?<id>[A-Za-z0-9_,-]+)").unwrap()
    })
}
pub struct JioSaavnSource {
    pub(crate) client: Arc<reqwest::Client>,
    pub(crate) secret_key: Vec<u8>,
    pub(crate) proxy: Option<crate::config::HttpProxyConfig>,
    pub(crate) api_url: String,
    pub(crate) search_limit: usize,
    pub(crate) recommendations_limit: usize,
    pub(crate) playlist_load_limit: usize,
    pub(crate) album_load_limit: usize,
    pub(crate) artist_load_limit: usize,
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
}
#[async_trait]
impl crate::sources::plugin::SourcePlugin for JioSaavnSource {
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
        vec!["jssearch:"]
    }
    fn rec_prefixes(&self) -> Vec<&str> {
        vec!["jsrec:"]
    }
    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        for prefix in self.rec_prefixes() {
            if let Some(query) = identifier.strip_prefix(prefix) {
                return self.get_recommendations(query).await;
            }
        }
        for prefix in self.search_prefixes() {
            if let Some(query) = identifier.strip_prefix(prefix) {
                return self.search(query).await;
            }
        }
        if let Some(caps) = url_regex().captures(identifier) {
            let type_ = caps.name("type").map(|m| m.as_str()).unwrap_or("");
            let id = caps.name("id").map(|m| m.as_str()).unwrap_or("");
            if id.is_empty() || type_.is_empty() {
                return LoadResult::Empty {};
            }
            let canonical_type = match type_ {
                "p/album" => "album",
                "s/artist" => "artist",
                "s/featured" => "featured",
                other => other,
            };
            if canonical_type == "song" || canonical_type == "s/song" {
                if let Some(track_data) = self.fetch_metadata(id).await {
                    if let Some(track) = parser::parse_track(&track_data) {
                        return LoadResult::Track(track);
                    }
                }
                return LoadResult::Empty {};
            } else {
                return self.resolve_list(canonical_type, id).await;
            }
        }
        LoadResult::Empty {}
    }
    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let id = if let Some(caps) = url_regex().captures(identifier) {
            caps.name("id").map(|m| m.as_str()).unwrap_or(identifier)
        } else {
            identifier
        };
        let track_data = self.fetch_metadata(id).await?;
        let encrypted_url = track_data
            .get("more_info")
            .and_then(|m| m.get("encrypted_media_url"))
            .and_then(|v| v.as_str())?
            .to_owned();
        let is_320 = track_data
            .get("more_info")
            .and_then(|m| m.get("320kbps"))
            .map(|v| v.as_str() == Some("true") || v.as_bool() == Some(true))
            .unwrap_or(false);
        let local_addr = routeplanner.and_then(|rp| rp.get_address());
        Some(Arc::new(JioSaavnTrack {
            encrypted_url,
            secret_key: self.secret_key.clone(),
            is_320,
            local_addr,
            proxy: self.proxy.clone(),
        }))
    }
    fn get_proxy_config(&self) -> Option<crate::config::HttpProxyConfig> {
        self.proxy.clone()
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
