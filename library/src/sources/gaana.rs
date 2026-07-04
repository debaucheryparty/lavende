pub mod crypto {
    use cbc::cipher::{BlockDecryptMut, KeyIvInit};
    use tracing::warn;
    type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
    const CRYPTO_KEY: &[u8; 16] = b"gy1t#b@jl(b$wtme";
    const CRYPTO_IV: &[u8; 16] = b"xC4dmVJAq14BfntX";
    const HLS_BASE_URL: &str = "https://vodhlsgaana-ebw.akamaized.net/";
    pub fn decrypt_stream_path(encrypted_data: &str) -> Option<String> {
        if encrypted_data.is_empty() {
            return None;
        }
        let offset = encrypted_data.chars().next()?.to_digit(10)? as usize;
        let skip = offset + 16;
        if skip >= encrypted_data.len() {
            warn!(
                "Gaana: encrypted data too short (len={}, skip={})",
                encrypted_data.len(),
                skip
            );
            return None;
        }
        let ciphertext_b64 = &encrypted_data[skip..];
        let padded = format!(
            "{}{}",
            ciphertext_b64,
            &"==="[..(4 - ciphertext_b64.len() % 4) % 4]
        );
        let ciphertext =
            match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &padded) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Gaana: base64 decode failed: {}", e);
                    return None;
                }
            };
        if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
            warn!("Gaana: invalid ciphertext length: {}", ciphertext.len());
            return None;
        }
        let mut buf = ciphertext;
        let cipher = Aes128CbcDec::new_from_slices(CRYPTO_KEY, CRYPTO_IV).ok()?;
        let decrypted =
            match cipher.decrypt_padded_mut::<cbc::cipher::block_padding::NoPadding>(&mut buf) {
                Ok(d) => d,
                Err(e) => {
                    warn!("Gaana: AES decryption failed: {}", e);
                    return None;
                }
            };
        let raw_text: String = decrypted
            .iter()
            .filter(|&&b| (32..=126).contains(&b))
            .map(|&b| b as char)
            .collect();
        if let Some(idx) = raw_text.find("hls/") {
            let path = &raw_text[idx..];
            Some(format!("{HLS_BASE_URL}{path}"))
        } else {
            warn!("Gaana: No /hls/ path found in decrypted text");
            None
        }
    }
}
pub mod manager {
    use super::track::GaanaTrack;
    use crate::{
        protocol::tracks::{LoadError, LoadResult, PlaylistData, PlaylistInfo, Track, TrackInfo},
        sources::{SourcePlugin, playable_track::BoxedTrack},
    };
    use async_trait::async_trait;
    use regex::Regex;
    use serde_json::Value;
    use std::sync::{Arc, OnceLock};
    use tracing::warn;
    const API_URL: &str = "https://gaana.com/apiv2";
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
    fn url_regex() -> &'static Regex {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        REGEX.get_or_init(|| {
        Regex::new(
        r"(?:https?://)?(?:www\.)?gaana\.com/(?P<type>song|album|playlist|artist)/(?P<seokey>[\w-]+)",
      )
      .unwrap()
    })
    }
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
            let url = format!(
                "{}?{}",
                API_URL,
                params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&")
            );
            let resp = match self
                .base_request(self.client.post(&url))
                .header("Referer", format!("https://gaana.com/{}", referer_path))
                .header("Content-Length", "0")
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("Gaana API request failed: {}", e);
                    return None;
                }
            };
            if !resp.status().is_success() {
                return None;
            }
            let text = resp.text().await.ok()?;
            serde_json::from_str::<Value>(&text).ok()
        }
        fn base_request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
            builder
                .header("User-Agent", USER_AGENT)
                .header("Accept", "application/json, text/plain, */*")
                .header("Accept-Language", "en-US,en;q=0.9")
                .header("Accept-Encoding", "gzip, deflate, br")
                .header("Origin", "https://gaana.com")
                .header("Connection", "keep-alive")
                .header("Sec-Fetch-Dest", "empty")
                .header("Sec-Fetch-Mode", "cors")
                .header("Sec-Fetch-Site", "same-origin")
                .header(
                    "sec-ch-ua",
                    "\"Chromium\";v=\"136\", \"Google Chrome\";v=\"136\", \"Not.A/Brand\";v=\"99\"",
                )
                .header("sec-ch-ua-mobile", "?0")
                .header("sec-ch-ua-platform", "\"Windows\"")
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
                        severity: crate::common::Severity::Common,
                        cause: String::new(),
                        cause_stack_trace: None,
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
                if let Some(key) = seokey
                    && let LoadResult::Track(track) = self.load_song(key).await
                {
                    results.push(track);
                }
            }
            if results.is_empty() {
                LoadResult::Empty {}
            } else {
                LoadResult::Search(results)
            }
        }
        fn extract_isrc(&self, json: &Value) -> Option<String> {
            if let Some(isrc) = json.get("isrc").and_then(|v| v.as_str()) {
                return Some(isrc.to_owned());
            }
            if let Some(info) = json.get("entity_info").and_then(|v| v.as_array()) {
                return info.iter().find_map(|e| {
                    if e.get("key").and_then(|k| k.as_str()) == Some("isrc") {
                        e.get("value")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_owned())
                    } else {
                        None
                    }
                });
            }
            None
        }
        fn parse_track(&self, json: &Value) -> Option<Track> {
            let id = json
                .get("track_id")
                .and_then(|v| {
                    v.as_str().map(|s| s.to_owned()).or_else(|| {
                        v.as_i64()
                            .map(|i| i.to_string())
                            .or_else(|| v.as_u64().map(|i| i.to_string()))
                    })
                })
                .or_else(|| {
                    json.get("entity_id").and_then(|v| {
                        v.as_str()
                            .map(|s| s.to_owned())
                            .or_else(|| v.as_i64().map(|i| i.to_string()))
                    })
                })?;
            let title = json
                .get("track_title")
                .and_then(|v| v.as_str())
                .or_else(|| json.get("name").and_then(|v| v.as_str()))?;
            let duration = json
                .get("duration")
                .and_then(|v| {
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
                .unwrap_or(0)
                * 1000;
            let author = if let Some(artist_arr) = json.get("artist").and_then(|v| v.as_array()) {
                let names: Vec<&str> = artist_arr
                    .iter()
                    .filter_map(|a| a.get("name").and_then(|v| v.as_str()))
                    .collect();
                if names.is_empty() {
                    "Unknown Artist".to_owned()
                } else {
                    names.join(", ")
                }
            } else {
                "Unknown Artist".to_owned()
            };
            let seokey = json.get("seokey").and_then(|v| v.as_str());
            let uri = seokey.map(|s| format!("https://gaana.com/song/{s}"));
            let artwork_url = json
                .get("artwork_large")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    json.get("atw")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                })
                .map(|s| s.to_owned());
            let isrc = self.extract_isrc(json);
            let track_info = TrackInfo {
                identifier: id,
                is_seekable: true,
                author,
                length: duration,
                is_stream: false,
                position: 0,
                title: title.to_owned(),
                uri,
                artwork_url,
                isrc,
                source_name: "gaana".to_owned(),
            };
            Some(Track::new(track_info))
        }
        fn parse_entity_track(&self, json: &Value) -> Option<Track> {
            let id = json.get("entity_id").and_then(|v| {
                v.as_str()
                    .map(|s| s.to_owned())
                    .or_else(|| v.as_i64().map(|i| i.to_string()))
            })?;
            let title = json.get("name").and_then(|v| v.as_str())?;
            let entity_info = json.get("entity_info").and_then(|v| v.as_array());
            let get_entity_value = |key: &str| -> Option<&Value> {
                entity_info?.iter().find_map(|e| {
                    if e.get("key").and_then(|k| k.as_str()) == Some(key) {
                        e.get("value")
                    } else {
                        None
                    }
                })
            };
            let duration = get_entity_value("duration")
                .and_then(|v| {
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
                .unwrap_or(0)
                * 1000;
            let author = get_entity_value("artist")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "Unknown Artist".to_owned());
            let seokey = json.get("seokey").and_then(|v| v.as_str());
            let uri = seokey.map(|s| format!("https://gaana.com/song/{s}"));
            let artwork_url = json
                .get("atw")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            let isrc = self.extract_isrc(json);
            let track_info = TrackInfo {
                identifier: id,
                is_seekable: true,
                author,
                length: duration,
                is_stream: false,
                position: 0,
                title: title.to_owned(),
                uri,
                artwork_url,
                isrc,
                source_name: "gaana".to_owned(),
            };
            Some(Track::new(track_info))
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
                || url_regex().is_match(identifier)
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
            if let Some(caps) = url_regex().captures(identifier) {
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
            let track_id = if let Some(caps) = url_regex().captures(identifier) {
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
            let stream_url = super::track::fetch_stream_url_internal(
                &self.client,
                &track_id,
                &self.stream_quality,
            )
            .await;
            if stream_url.is_none() {
                warn!("Gaana: no stream URL for track {track_id}, falling back to mirrors");
                return None;
            }
            Some(Arc::new(GaanaTrack {
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
}
pub mod reader {
    use crate::{
        audio::source::{HttpSource, create_client},
        common::types::AnyResult,
    };
    use std::io::{Read, Seek, SeekFrom};
    use symphonia::core::io::MediaSource;
    pub struct GaanaReader {
        inner: HttpSource,
    }
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
    impl GaanaReader {
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
    impl Read for GaanaReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.inner.read(buf)
        }
    }
    impl Seek for GaanaReader {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(pos)
        }
    }
    impl MediaSource for GaanaReader {
        fn is_seekable(&self) -> bool {
            self.inner.is_seekable()
        }
        fn byte_len(&self) -> Option<u64> {
            self.inner.byte_len()
        }
    }
}
pub mod track {
    use crate::{
        common::types::AudioFormat,
        config::HttpProxyConfig,
        sources::{
            gaana::crypto::decrypt_stream_path,
            playable_track::{PlayableTrack, ResolvedTrack},
        },
    };
    use async_trait::async_trait;
    use std::{net::IpAddr, sync::Arc};
    pub struct GaanaTrack {
        pub client: Arc<reqwest::Client>,
        pub track_id: String,
        pub stream_quality: String,
        pub local_addr: Option<IpAddr>,
        pub proxy: Option<HttpProxyConfig>,
    }
    #[async_trait]
    impl PlayableTrack for GaanaTrack {
        async fn resolve(&self) -> Result<ResolvedTrack, String> {
            let url = fetch_stream_url_internal(&self.client, &self.track_id, &self.stream_quality)
                .await
                .ok_or_else(|| {
                    format!(
                        "GaanaTrack: Failed to fetch stream URL for {}",
                        self.track_id
                    )
                })?;
            let local_addr = self.local_addr;
            let proxy = self.proxy.clone();
            let is_hls = url.contains(".m3u8") || url.contains("/api/manifest/hls_");
            if is_hls {
                crate::sources::youtube::hls::HlsReader::new(&url, local_addr, None, None, proxy)
                    .await
                    .map(|r| {
                        ResolvedTrack::new(
                            Box::new(r) as Box<dyn symphonia::core::io::MediaSource>,
                            Some(AudioFormat::Aac),
                        )
                    })
                    .map_err(|e| format!("Failed to init HLS reader: {e}"))
            } else {
                let hint = std::path::Path::new(&url)
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(AudioFormat::from_ext);
                super::reader::GaanaReader::new(&url, local_addr, proxy)
                    .await
                    .map(|r| {
                        ResolvedTrack::new(
                            Box::new(r) as Box<dyn symphonia::core::io::MediaSource>,
                            hint,
                        )
                    })
                    .map_err(|e| format!("Failed to init reader: {e}"))
            }
        }
    }
    pub(super) async fn fetch_stream_url_internal(
        client: &Arc<reqwest::Client>,
        track_id: &str,
        quality: &str,
    ) -> Option<String> {
        let body = format!(
            "quality={}&track_id={}&stream_format=mp4",
            urlencoding::encode(quality),
            urlencoding::encode(track_id)
        );
        let resp = client
        .post("https://gaana.com/api/stream-url")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36")
        .header("Referer", "https://gaana.com/")
        .header("Origin", "https://gaana.com")
        .header("Accept", "application/json, text/plain, */*")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let data: serde_json::Value = resp.json().await.ok()?;
        let encrypted_path = data.get("data")?.get("stream_path")?.as_str()?;
        decrypt_stream_path(encrypted_path)
    }
}
pub use manager::GaanaSource;
pub use track::GaanaTrack;
