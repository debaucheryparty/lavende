pub mod api {
    use serde_json::Value;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tracing::{debug, warn};
    const TWITCH_GQL: &str = "https://gql.twitch.tv/gql";
    const TWITCH_URL: &str = "https://www.twitch.tv";
    const BROWSER_UA: &str =
        "Mozilla/5.0 (X11; Linux x86_64; rv:125.0) Gecko/20100101 Firefox/125.0";
    const METADATA_PAYLOAD: &str = r#"{"operationName":"StreamMetadata","query":"query StreamMetadata($channelLogin: String!) { user(login: $channelLogin) { stream { type } lastBroadcast { title } } }","variables":{"channelLogin":"%s"}}"#;
    const ACCESS_TOKEN_PAYLOAD: &str = r#"{"operationName":"PlaybackAccessToken_Template","query":"query PlaybackAccessToken_Template($login: String!,$isLive:Boolean!,$vodID:ID!,$isVod:Boolean!,$playerType:String!){streamPlaybackAccessToken(channelName:$login,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:$playerType})@include(if:$isLive){value signature __typename}videoPlaybackAccessToken(id:$vodID,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:$playerType})@include(if:$isVod){value signature __typename}}","variables":{"isLive":true,"login":"%s","isVod":false,"vodID":"","playerType":"site"}}"#;
    pub struct TwitchGqlClient {
        http: Arc<reqwest::Client>,
        client_id: RwLock<Option<String>>,
        device_id: RwLock<Option<String>>,
    }
    impl TwitchGqlClient {
        pub fn new(http: Arc<reqwest::Client>, pinned_client_id: Option<String>) -> Self {
            Self {
                http,
                client_id: RwLock::new(pinned_client_id),
                device_id: RwLock::new(None),
            }
        }
        pub fn is_initialized(&self) -> bool {
            self.client_id
                .try_read()
                .map(|g| g.is_some())
                .unwrap_or(false)
        }
        pub async fn init_request_headers(&self) {
            if self.client_id.read().await.is_some() {
                return;
            }
            let resp = match self
                .http
                .get(TWITCH_URL)
                .header("Accept", "text/html")
                .header("User-Agent", BROWSER_UA)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("Twitch: failed to fetch main page: {e}");
                    return;
                }
            };
            let cookie_headers: Vec<String> = resp
                .headers()
                .get_all("set-cookie")
                .iter()
                .filter_map(|v| v.to_str().ok().map(str::to_owned))
                .collect();
            for cookie in &cookie_headers {
                if cookie.contains("unique_id=")
                    && let Some(id) = extract_between(cookie, "unique_id=", ";")
                {
                    *self.device_id.write().await = Some(id.trim().to_string());
                    break;
                }
            }
            let body = match resp.text().await {
                Ok(b) => b,
                Err(e) => {
                    warn!("Twitch: failed to read main page body: {e}");
                    return;
                }
            };
            if let Some(id) = extract_between(&body, "clientId=\"", "\"") {
                debug!("Twitch: initialized client_id from main page");
                *self.client_id.write().await = Some(id.to_string());
            }
        }
        async fn post_raw(&self, body: String) -> Option<Value> {
            let client_id = self.client_id.read().await.clone()?;
            let device_id = self.device_id.read().await.clone();
            let mut req = self
                .http
                .post(TWITCH_GQL)
                .header("Client-ID", client_id)
                .header("Content-Type", "text/plain;charset=UTF-8")
                .body(body);
            if let Some(did) = device_id {
                req = req.header("X-Device-ID", did);
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    debug!("Twitch GQL send error: {e}");
                    return None;
                }
            };
            if !resp.status().is_success() {
                warn!("Twitch GQL HTTP {}", resp.status());
                return None;
            }
            match resp.json::<Value>().await {
                Ok(v) => Some(v),
                Err(e) => {
                    debug!("Twitch GQL JSON parse error: {e}");
                    None
                }
            }
        }
        pub async fn fetch_stream_channel_info(&self, channel: &str) -> Option<Value> {
            let payload = METADATA_PAYLOAD.replace("%s", channel);
            let body = match self.post_raw(payload).await {
                Some(b) => b,
                None => {
                    debug!("Twitch: stream metadata request failed for '{channel}'");
                    return None;
                }
            };
            if let Some(errors) = body["errors"].as_array() {
                for e in errors {
                    debug!(
                        "Twitch GQL error: {}",
                        e["message"].as_str().unwrap_or("unknown")
                    );
                }
                return None;
            }
            Some(body)
        }
        pub async fn fetch_access_token(&self, channel: &str) -> Option<(String, String)> {
            let payload = ACCESS_TOKEN_PAYLOAD.replace("%s", channel);
            let body = match self.post_raw(payload).await {
                Some(b) => b,
                None => {
                    debug!("Twitch: access token request failed for '{channel}'");
                    return None;
                }
            };
            if let Some(errors) = body["errors"].as_array() {
                for e in errors {
                    debug!(
                        "Twitch access token GQL error: {}",
                        e["message"].as_str().unwrap_or("unknown")
                    );
                }
                return None;
            }
            let token = &body["data"]["streamPlaybackAccessToken"];
            let value = match token["value"].as_str() {
                Some(v) => v.to_string(),
                None => {
                    debug!("Twitch: access token 'value' missing for '{channel}'");
                    return None;
                }
            };
            let sig = match token["signature"].as_str() {
                Some(s) => s.to_string(),
                None => {
                    debug!("Twitch: access token 'signature' missing for '{channel}'");
                    return None;
                }
            };
            Some((value, sig))
        }
        pub async fn fetch_text(&self, url: &str) -> Option<String> {
            let client_id = self.client_id.read().await.clone();
            let device_id = self.device_id.read().await.clone();
            let mut req = self.http.get(url);
            if let Some(id) = client_id {
                req = req.header("Client-ID", id);
            }
            if let Some(did) = device_id {
                req = req.header("X-Device-ID", did);
            }
            match req.send().await {
                Ok(resp) => match resp.text().await {
                    Ok(t) => Some(t),
                    Err(e) => {
                        debug!("Twitch: failed to read response body from '{url}': {e}");
                        None
                    }
                },
                Err(e) => {
                    debug!("Twitch: HTTP GET failed for '{url}': {e}");
                    None
                }
            }
        }
    }
    fn extract_between<'a>(src: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
        let start = src.find(prefix)? + prefix.len();
        let end = src[start..].find(suffix)? + start;
        Some(&src[start..end])
    }
}
pub mod manager {
    use super::{api::TwitchGqlClient, track::TwitchTrack};
    use crate::{
        config::TwitchConfig,
        protocol::tracks::{LoadError, LoadResult, Track, TrackInfo},
        sources::{SourcePlugin, playable_track::BoxedTrack},
    };
    use async_trait::async_trait;
    use regex::Regex;
    use std::sync::Arc;
    use tracing::{debug, warn};
    const STREAM_NAME_REGEX: &str = r"(?i)^https?://(?:www\.|go\.|m\.)?twitch\.tv/([^/]+)$";
    const TWITCH_DOMAIN_REGEX: &str = r"(?i)^https?://(?:www\.|go\.|m\.)?twitch\.tv/";
    const TWITCH_IMAGE_PREVIEW_URL: &str =
        "https://static-cdn.jtvnw.net/previews-ttv/live_user_%s-440x248.jpg";
    pub struct TwitchSource {
        gql: Arc<TwitchGqlClient>,
        proxy: Option<crate::config::HttpProxyConfig>,
        stream_name_regex: Regex,
        twitch_domain_regex: Regex,
    }
    impl TwitchSource {
        pub fn new(config: TwitchConfig, client: Arc<reqwest::Client>) -> Self {
            Self {
                gql: Arc::new(TwitchGqlClient::new(client, config.client_id)),
                proxy: config.proxy,
                stream_name_regex: Regex::new(STREAM_NAME_REGEX).unwrap(),
                twitch_domain_regex: Regex::new(TWITCH_DOMAIN_REGEX).unwrap(),
            }
        }
        fn get_channel_identifier_from_url(&self, url: &str) -> Option<String> {
            self.stream_name_regex
                .captures(url)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_lowercase())
        }
        async fn ensure_initialized(&self) {
            if !self.gql.is_initialized() {
                self.gql.init_request_headers().await;
            }
        }
        async fn get_channel_streams_url(&self, channel: &str) -> Option<String> {
            let (token, sig) = self.gql.fetch_access_token(channel).await?;
            Some(format!(
                "https://usher.ttvnw.net/api/channel/hls/{}.m3u8?token={}&sig={}&allow_source=true&allow_spectre=true&allow_audio_only=true&player_backend=html5&expgroup=regular",
                channel,
                urlencoding::encode(&token),
                urlencoding::encode(&sig),
            ))
        }
        async fn fetch_segment_playlist_url(&self, channel: &str) -> Option<String> {
            let streams_url = self.get_channel_streams_url(channel).await?;
            let m3u8 = self.gql.fetch_text(&streams_url).await?;
            let streams = load_channel_streams_list(&m3u8);
            if streams.is_empty() {
                debug!("Twitch: no streams available on channel '{channel}'");
                return None;
            }
            let chosen = streams.last().unwrap();
            debug!(
                "Twitch: chose stream with quality {} from url {}",
                chosen.quality, chosen.url
            );
            Some(chosen.url.clone())
        }
    }
    struct ChannelStreamInfo {
        quality: String,
        url: String,
    }
    fn load_channel_streams_list(m3u8: &str) -> Vec<ChannelStreamInfo> {
        let lines: Vec<&str> = m3u8.lines().collect();
        let mut streams = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if line.starts_with("#EXT-X-STREAM-INF:") {
                let quality = line
                    .split(',')
                    .find_map(|part| {
                        part.trim()
                            .strip_prefix("VIDEO=\"")
                            .and_then(|v| v.strip_suffix('"'))
                    })
                    .unwrap_or("unknown")
                    .to_string();
                if let Some(url_line) = lines.get(i + 1) {
                    let url = url_line.trim();
                    if !url.is_empty() && !url.starts_with('#') {
                        streams.push(ChannelStreamInfo {
                            quality,
                            url: url.to_string(),
                        });
                    }
                }
            }
            i += 1;
        }
        streams
    }
    #[async_trait]
    impl SourcePlugin for TwitchSource {
        fn name(&self) -> &str {
            "twitch"
        }
        fn can_handle(&self, identifier: &str) -> bool {
            self.twitch_domain_regex.is_match(identifier)
        }
        async fn load(
            &self,
            identifier: &str,
            _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
        ) -> LoadResult {
            let stream_name = match self.get_channel_identifier_from_url(identifier) {
                Some(n) => n,
                None => return LoadResult::Empty {},
            };
            self.ensure_initialized().await;
            let channel_info_body = match self.gql.fetch_stream_channel_info(&stream_name).await {
                Some(b) => b,
                None => {
                    return LoadResult::Error(LoadError {
                        message: Some(format!(
                            "Loading Twitch channel information failed for '{stream_name}'"
                        )),
                        severity: crate::common::Severity::Suspicious,
                        cause: "GQL request failed".to_string(),
                        cause_stack_trace: None,
                    });
                }
            };
            let channel_info = &channel_info_body["data"]["user"];
            if channel_info.is_null() || channel_info["stream"]["type"].is_null() {
                return LoadResult::Empty {};
            }
            let title = channel_info["lastBroadcast"]["title"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let thumbnail = TWITCH_IMAGE_PREVIEW_URL.replace("%s", &stream_name);
            LoadResult::Track(Track::new(TrackInfo {
                identifier: stream_name.clone(),
                is_seekable: false,
                author: stream_name.clone(),
                length: 0,
                is_stream: true,
                position: 0,
                title,
                uri: Some(identifier.to_string()),
                artwork_url: Some(thumbnail),
                isrc: None,
                source_name: "twitch".to_string(),
            }))
        }
        async fn get_track(
            &self,
            identifier: &str,
            routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
        ) -> Option<BoxedTrack> {
            let stream_name = self.get_channel_identifier_from_url(identifier)?;
            self.ensure_initialized().await;
            let local_addr = routeplanner.and_then(|rp| rp.get_address());
            let stream_url = match self.fetch_segment_playlist_url(&stream_name).await {
                Some(u) => u,
                None => {
                    warn!("Twitch: failed to resolve stream for '{stream_name}'");
                    return None;
                }
            };
            Some(Arc::new(TwitchTrack {
                stream_url,
                local_addr,
                proxy: self.proxy.clone(),
            }))
        }
        fn get_proxy_config(&self) -> Option<crate::config::HttpProxyConfig> {
            self.proxy.clone()
        }
    }
}
pub mod track {
    use crate::{
        common::types::AudioFormat,
        config::HttpProxyConfig,
        sources::{
            playable_track::{PlayableTrack, ResolvedTrack},
            youtube::hls::{
                fetcher::fetch_segment_into, resolver::fetch_text, ts_demux::extract_adts_from_ts,
                types::Resource, utils::resolve_url,
            },
        },
    };
    use async_trait::async_trait;
    use std::{
        collections::HashSet,
        io::{self, Read, Seek, SeekFrom},
        net::IpAddr,
    };
    use symphonia::core::io::MediaSource;
    pub struct TwitchTrack {
        pub stream_url: String,
        pub local_addr: Option<IpAddr>,
        pub proxy: Option<HttpProxyConfig>,
    }
    #[async_trait]
    impl PlayableTrack for TwitchTrack {
        async fn resolve(&self) -> Result<ResolvedTrack, String> {
            let handle = tokio::runtime::Handle::current();
            let (err_tx, _err_rx) = flume::bounded::<String>(1);
            let reader = Box::new(
                LiveHlsReader::new(
                    self.stream_url.clone(),
                    self.local_addr,
                    self.proxy.clone(),
                    handle,
                    err_tx,
                )
                .await,
            ) as Box<dyn MediaSource>;
            Ok(ResolvedTrack::new(reader, Some(AudioFormat::Aac)))
        }
    }
    struct LiveHlsReader {
        chunk_rx: flume::Receiver<Vec<u8>>,
        current: Vec<u8>,
        pos: usize,
    }
    impl LiveHlsReader {
        pub async fn new(
            manifest_url: String,
            local_addr: Option<IpAddr>,
            proxy: Option<HttpProxyConfig>,
            _handle: tokio::runtime::Handle,
            err_tx: flume::Sender<String>,
        ) -> Self {
            let (chunk_tx, chunk_rx) = flume::bounded::<Vec<u8>>(16);
            tokio::spawn(async move {
                let mut builder =
                    reqwest::Client::builder().timeout(std::time::Duration::from_secs(15));
                if let Some(ip) = local_addr {
                    builder = builder.local_address(ip);
                }
                if let Some(ref cfg) = proxy
                    && let Some(ref url) = cfg.url
                {
                    match reqwest::Proxy::all(url) {
                        Ok(mut p) => {
                            if let (Some(u), Some(pw)) = (&cfg.username, &cfg.password) {
                                p = p.basic_auth(u, pw);
                            }
                            builder = builder.proxy(p);
                        }
                        Err(e) => {
                            tracing::error!("Twitch live HLS: proxy setup failed for {url}: {e}");
                            let _ = err_tx.send(format!("Proxy setup failed: {e}"));
                            return;
                        }
                    }
                }
                let client = match builder.build() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Twitch live HLS: client build failed: {e}");
                        let _ = err_tx.send(format!("Client build failed: {e}"));
                        return;
                    }
                };
                let mut seen: HashSet<String> = HashSet::new();
                let mut seen_history: std::collections::VecDeque<String> =
                    std::collections::VecDeque::with_capacity(50);
                loop {
                    let text = match fetch_text(&client, &manifest_url).await {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::warn!("Twitch: live playlist refresh failed: {e}");
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            continue;
                        }
                    };
                    let (segments, target_duration) = parse_live_playlist(&text, &manifest_url);
                    for seg in segments {
                        if seen.contains(&seg.url) {
                            continue;
                        }
                        let mut raw = Vec::new();
                        if let Err(e) = fetch_segment_into(&client, &seg, &mut raw).await {
                            tracing::warn!("Twitch: segment fetch error: {e}");
                            continue;
                        }
                        let payload = if raw.first() == Some(&0x47) {
                            let adts = extract_adts_from_ts(&raw);
                            if adts.is_empty() {
                                tracing::debug!("Twitch: ADTS extraction failed, skipping segment");
                                continue;
                            }
                            adts
                        } else {
                            raw
                        };
                        if chunk_tx.send(payload).is_err() {
                            return;
                        }
                        if seen.insert(seg.url.clone()) {
                            seen_history.push_back(seg.url);
                            if seen_history.len() > 50
                                && let Some(old) = seen_history.pop_front()
                            {
                                seen.remove(&old);
                            }
                        }
                    }
                    let wait = (target_duration / 2.0).max(1.0);
                    tokio::time::sleep(std::time::Duration::from_secs_f64(wait)).await;
                }
            });
            Self {
                chunk_rx,
                current: Vec::new(),
                pos: 0,
            }
        }
    }
    fn parse_live_playlist(text: &str, base_url: &str) -> (Vec<Resource>, f64) {
        let mut segments = Vec::new();
        let mut target_duration = 6.0f64;
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            if let Some(rest) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
                if let Ok(d) = rest.trim().parse::<f64>() {
                    target_duration = d;
                }
            } else if line.starts_with("#EXTINF:") {
                let duration = line
                    .strip_prefix("#EXTINF:")
                    .and_then(|r| r.split(',').next())
                    .and_then(|d| d.trim().parse::<f64>().ok());
                let mut j = i + 1;
                while j < lines.len() && lines[j].starts_with('#') {
                    j += 1;
                }
                if j < lines.len() && !lines[j].is_empty() {
                    let url = resolve_url(base_url, lines[j]);
                    segments.push(Resource {
                        url,
                        range: None,
                        duration,
                    });
                }
                i = j + 1;
                continue;
            }
            i += 1;
        }
        (segments, target_duration)
    }
    impl Read for LiveHlsReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            loop {
                if self.pos < self.current.len() {
                    let n = buf.len().min(self.current.len() - self.pos);
                    buf[..n].copy_from_slice(&self.current[self.pos..self.pos + n]);
                    self.pos += n;
                    return Ok(n);
                }
                match self
                    .chunk_rx
                    .recv_timeout(std::time::Duration::from_millis(500))
                {
                    Ok(chunk) => {
                        self.current = chunk;
                        self.pos = 0;
                    }
                    Err(flume::RecvTimeoutError::Timeout) => continue,
                    Err(flume::RecvTimeoutError::Disconnected) => return Ok(0),
                }
            }
        }
    }
    impl Seek for LiveHlsReader {
        fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "live streams are not seekable",
            ))
        }
    }
    impl MediaSource for LiveHlsReader {
        fn is_seekable(&self) -> bool {
            false
        }
        fn byte_len(&self) -> Option<u64> {
            None
        }
    }
}
pub use manager::TwitchSource;
