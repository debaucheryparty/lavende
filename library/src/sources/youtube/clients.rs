pub mod android {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "ANDROID";
    const CLIENT_ID: &str = "3";
    const CLIENT_VERSION: &str = "20.01.35";
    const USER_AGENT: &str = "com.google.android.youtube/20.01.35 (Linux; U; Android 14) identity";
    pub struct AndroidClient {
        http: Arc<reqwest::Client>,
    }
    impl AndroidClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: CLIENT_ID,
                user_agent: USER_AGENT,
                device_make: Some("Google"),
                device_model: Some("Pixel 6"),
                os_name: Some("Android"),
                os_version: Some("14"),
                android_sdk_version: Some("34"),
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for AndroidClient {
        fn name(&self) -> &str {
            "Android"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            core::standard_get_playlist(self, &self.http, playlist_id, context, oauth, || {
                self.config()
            })
            .await
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: None,
                    origin: None,
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod android_vr {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "ANDROID_VR";
    const CLIENT_ID: &str = "28";
    const CLIENT_VERSION: &str = "1.71.26";
    const USER_AGENT: &str = "com.google.android.apps.youtube.vr.oculus/1.71.26 (Linux; U; Android 15; eureka-user Build/AP4A.250205.002) gzip";
    pub struct AndroidVrClient {
        http: Arc<reqwest::Client>,
    }
    impl AndroidVrClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: CLIENT_ID,
                user_agent: USER_AGENT,
                device_make: Some("Google"),
                os_name: Some("Android"),
                os_version: Some("15"),
                android_sdk_version: Some("35"),
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for AndroidVrClient {
        fn name(&self) -> &str {
            "AndroidVR"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            _track_id: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_playlist(
            &self,
            _playlist_id: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            Ok(None)
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: None,
                    origin: None,
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod common {
    use super::YouTubeCipherManager;
    use crate::common::types::AnyResult;
    use regex::Regex;
    use serde_json::{Value, json};
    use std::sync::{Arc, OnceLock};
    pub const INNERTUBE_API: &str = "https://youtubei.googleapis.com";
    #[derive(Debug, Clone)]
    pub struct ClientConfig<'a> {
        pub client_name: &'a str,
        pub client_version: &'a str,
        pub client_id: &'a str,
        pub user_agent: &'a str,
        pub os_name: Option<&'a str>,
        pub os_version: Option<&'a str>,
        pub device_make: Option<&'a str>,
        pub device_model: Option<&'a str>,
        pub platform: Option<&'a str>,
        pub android_sdk_version: Option<&'a str>,
        pub hl: &'a str,
        pub gl: &'a str,
        pub utc_offset_minutes: Option<i32>,
        pub third_party_embed_url: Option<&'a str>,
        pub client_screen: Option<&'a str>,
        pub attestation_request: Option<Value>,
    }
    impl<'a> Default for ClientConfig<'a> {
        fn default() -> Self {
            Self {
                client_name: "",
                client_version: "",
                client_id: "",
                user_agent: "",
                os_name: None,
                os_version: None,
                device_make: None,
                device_model: None,
                platform: None,
                android_sdk_version: None,
                hl: "en",
                gl: "US",
                utc_offset_minutes: None,
                third_party_embed_url: None,
                client_screen: None,
                attestation_request: None,
            }
        }
    }
    impl<'a> ClientConfig<'a> {
        pub fn build_context(&self, visitor_data: Option<&str>) -> Value {
            let mut client = json!({
                "clientName": self.client_name,
                "clientVersion": self.client_version,
                "userAgent": self.user_agent,
                "hl": self.hl,
                "gl": self.gl,
            });
            if let Some(obj) = client.as_object_mut() {
                if let Some(v) = self.os_name {
                    obj.insert("osName".to_string(), v.into());
                }
                if let Some(v) = self.os_version {
                    obj.insert("osVersion".to_string(), v.into());
                }
                if let Some(v) = self.device_make {
                    obj.insert("deviceMake".to_string(), v.into());
                }
                if let Some(v) = self.device_model {
                    obj.insert("deviceModel".to_string(), v.into());
                }
                if let Some(v) = self.platform {
                    obj.insert("platform".to_string(), v.into());
                }
                if let Some(v) = self.android_sdk_version {
                    obj.insert("androidSdkVersion".to_string(), v.into());
                }
                if let Some(v) = self.utc_offset_minutes {
                    obj.insert("utcOffsetMinutes".to_string(), v.into());
                }
                if let Some(v) = self.client_screen {
                    obj.insert("clientScreen".to_string(), v.into());
                }
                if let Some(vd) = visitor_data {
                    obj.insert("visitorData".to_string(), vd.into());
                }
            }
            let mut context = json!({
                "client": client,
                "user": { "lockedSafetyMode": false },
                "request": { "useSsl": true }
            });
            if let Some(url) = self.third_party_embed_url
                && let Some(obj) = context.as_object_mut()
            {
                obj.insert("thirdParty".to_string(), json!({ "embedUrl": url }));
            }
            if let Some(att) = self.attestation_request.clone()
                && let Some(obj) = context.as_object_mut()
            {
                obj.insert("attestationRequest".to_string(), att);
            }
            context
        }
    }
    pub const AUDIO_ITAG_PRIORITY: &[i64] = &[251, 250, 140];
    pub const ITAG_FALLBACK: i64 = 18;
    pub fn decode_signature_cipher(cipher_str: &str) -> Option<(String, String)> {
        let mut url = None;
        let mut sig = None;
        for part in cipher_str.split('&') {
            if let Some((k, v)) = part.split_once('=') {
                let decoded = urlencoding::decode(v).ok()?.to_string();
                match k {
                    "url" => url = Some(decoded),
                    "s" => sig = Some(decoded),
                    _ => {}
                }
            }
        }
        match (url, sig) {
            (Some(u), Some(s)) => Some((u, s)),
            _ => None,
        }
    }
    pub fn select_best_audio_format<'a>(
        adaptive_formats: Option<&'a Vec<Value>>,
        formats: Option<&'a Vec<Value>>,
    ) -> Option<&'a Value> {
        let all: Vec<&Value> = adaptive_formats
            .into_iter()
            .flatten()
            .chain(formats.into_iter().flatten())
            .collect();
        for &target_itag in AUDIO_ITAG_PRIORITY {
            for f in &all {
                let itag = f.get("itag").and_then(|v| v.as_i64()).unwrap_or(-1);
                let mime = f.get("mimeType").and_then(|v| v.as_str()).unwrap_or("");
                if itag == target_itag && mime.starts_with("audio/") {
                    return Some(f);
                }
            }
        }
        for f in &all {
            let itag = f.get("itag").and_then(|v| v.as_i64()).unwrap_or(-1);
            if itag == ITAG_FALLBACK {
                return Some(f);
            }
        }
        let mut best: Option<&Value> = None;
        let mut best_bitrate = 0i64;
        for f in all {
            let mime = f.get("mimeType").and_then(|v| v.as_str()).unwrap_or("");
            if mime.starts_with("audio/") {
                let bitrate = f.get("bitrate").and_then(|v| v.as_i64()).unwrap_or(0);
                if bitrate > best_bitrate {
                    best = Some(f);
                    best_bitrate = bitrate;
                }
            }
        }
        best
    }
    pub async fn resolve_format_url(
        format: &Value,
        player_page_url: &str,
        cipher_manager: &Arc<YouTubeCipherManager>,
    ) -> AnyResult<Option<String>> {
        if let Some(url) = format.get("url").and_then(|u| u.as_str()) {
            let n_param = url
                .split("&n=")
                .nth(1)
                .or_else(|| url.split("?n=").nth(1))
                .and_then(|s| s.split('&').next());
            if n_param.is_none() {
                return Ok(Some(url.to_string()));
            }
            let resolved = cipher_manager
                .resolve_url(url, player_page_url, n_param, None)
                .await?;
            return Ok(Some(resolved));
        }
        let cipher_str = format
            .get("signatureCipher")
            .or_else(|| format.get("cipher"))
            .and_then(|c| c.as_str());
        if let Some(cipher_str) = cipher_str
            && let Some((url, sig)) = decode_signature_cipher(cipher_str)
        {
            let n_param = url
                .split("&n=")
                .nth(1)
                .or_else(|| url.split("?n=").nth(1))
                .and_then(|s| s.split('&').next());
            let resolved = cipher_manager
                .resolve_url(&url, player_page_url, n_param, Some(&sig))
                .await?;
            return Ok(Some(resolved));
        }
        Ok(None)
    }
    static DURATION_REGEX: OnceLock<Regex> = OnceLock::new();
    pub fn is_duration(text: &str) -> bool {
        let re = DURATION_REGEX.get_or_init(|| Regex::new(r"^\d{1,2}:\d{2}(:\d{2})?$").unwrap());
        re.is_match(text)
    }
    pub fn parse_duration(duration: &str) -> u64 {
        let parts: Vec<&str> = duration.split(':').collect();
        let mut ms = 0u64;
        for part in parts {
            if let Ok(num) = part.parse::<u64>() {
                ms = ms * 60 + num;
            }
        }
        ms * 1000
    }
    pub fn extract_thumbnail(renderer: &Value, video_id: Option<&str>) -> Option<String> {
        let thumbnails = renderer
            .get("thumbnail")
            .and_then(|t| t.get("thumbnails"))
            .or_else(|| {
                renderer
                    .get("thumbnail")
                    .and_then(|t| t.get("musicThumbnailRenderer"))
                    .and_then(|t| t.get("thumbnail"))
                    .and_then(|t| t.get("thumbnails"))
            });
        if let Some(list) = thumbnails.and_then(|t| t.as_array())
            && !list.is_empty()
        {
            let lh3 = list.iter().rev().find_map(|t| {
                t.get("url")
                    .and_then(|u| u.as_str())
                    .filter(|u| u.contains("lh3.googleusercontent.com"))
                    .map(|u| u.split('?').next().unwrap_or(u).to_string())
            });
            if let Some(url) = lh3 {
                return Some(url);
            }
            let best = list
                .iter()
                .max_by_key(|t| t.get("width").and_then(|w| w.as_u64()).unwrap_or(0));
            if let Some(url) = best.and_then(|t| t.get("url")).and_then(|u| u.as_str()) {
                let clean = url.split('?').next().unwrap_or(url);
                if clean.contains("i.ytimg.com") {
                    let upgraded = clean
                        .replace("mqdefault", "maxresdefault")
                        .replace("sddefault", "maxresdefault")
                        .replace("hqdefault", "maxresdefault");
                    return Some(upgraded);
                }
                return Some(clean.to_string());
            }
        }
        if let Some(id) = video_id {
            return Some(format!("https://i.ytimg.com/vi/{}/maxresdefault.jpg", id));
        }
        None
    }
    pub struct PlayerRequestOptions<'a> {
        pub http: &'a reqwest::Client,
        pub config: &'a ClientConfig<'a>,
        pub video_id: &'a str,
        pub params: Option<&'a str>,
        pub visitor_data: Option<&'a str>,
        pub signature_timestamp: Option<u32>,
        pub auth_header: Option<String>,
        pub referer: Option<&'a str>,
        pub origin: Option<&'a str>,
        pub po_token: Option<&'a str>,
        pub encrypted_host_flags: Option<String>,
        pub attestation_request: Option<Value>,
        pub serialized_third_party_embed_config: bool,
    }
    pub async fn make_player_request(opts: PlayerRequestOptions<'_>) -> AnyResult<Value> {
        let mut body = json!({
            "context": opts.config.build_context(opts.visitor_data),
            "videoId": opts.video_id,
            "contentCheckOk": true,
            "racyCheckOk": true
        });
        if opts.serialized_third_party_embed_config
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert(
                "serializedThirdPartyEmbedConfig".to_string(),
                "{\"hideInfoBar\":true,\"disableRelatedVideos\":true}".into(),
            );
        }
        if let Some(token) = opts.po_token
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert(
                "serviceIntegrityDimensions".to_string(),
                json!({ "poToken": token }),
            );
        }
        if let Some(p) = opts.params
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert("params".to_string(), p.into());
        }
        if let Some(sts) = opts.signature_timestamp
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert(
                "playbackContext".to_string(),
                json!({
                    "contentPlaybackContext": {
                        "signatureTimestamp": sts
                    }
                }),
            );
        }
        if let Some(flags) = opts.encrypted_host_flags
            && let Some(obj) = body.as_object_mut()
        {
            let playback_context = obj
                .entry("playbackContext".to_string())
                .or_insert_with(|| json!({}));
            let content_playback_context = playback_context
                .as_object_mut()
                .unwrap()
                .entry("contentPlaybackContext".to_string())
                .or_insert_with(|| json!({}));
            content_playback_context
                .as_object_mut()
                .unwrap()
                .insert("encryptedHostFlags".to_string(), flags.into());
        }
        if let Some(att) = opts.attestation_request
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert("attestationRequest".to_string(), att);
        }
        let url = format!("{}/youtubei/v1/player?prettyPrint=false", INNERTUBE_API);
        let mut req = opts
            .http
            .post(&url)
            .header("User-Agent", opts.config.user_agent)
            .header("X-YouTube-Client-Name", opts.config.client_id)
            .header("X-YouTube-Client-Version", opts.config.client_version)
            .header("X-Goog-Api-Format-Version", "2");
        if let Some(vd) = opts.visitor_data {
            req = req.header("X-Goog-Visitor-Id", vd);
        }
        if let Some(auth) = opts.auth_header {
            req = req.header("Authorization", auth);
        }
        if let Some(ref_url) = opts.referer {
            req = req.header("Referer", ref_url);
        }
        if let Some(orig_url) = opts.origin {
            req = req.header("Origin", orig_url);
        }
        let res = req.json(&body).send().await?;
        let status = res.status();
        if !status.is_success() {
            let text = res
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Player request failed (status={}): {}", status, text).into());
        }
        Ok(res.json().await?)
    }
    pub async fn make_next_request(
        http: &reqwest::Client,
        config: &ClientConfig<'_>,
        video_id: Option<&str>,
        playlist_id: Option<&str>,
        visitor_data: Option<&str>,
        auth_header: Option<String>,
    ) -> AnyResult<Value> {
        let mut body = json!({
            "context": config.build_context(visitor_data),
        });
        if let Some(vid) = video_id
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert("videoId".to_string(), vid.into());
        }
        if let Some(pid) = playlist_id
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert("playlistId".to_string(), pid.into());
        }
        let url = format!("{}/youtubei/v1/next?prettyPrint=false", INNERTUBE_API);
        let mut req = http
            .post(&url)
            .header("User-Agent", config.user_agent)
            .header("X-YouTube-Client-Name", config.client_id)
            .header("X-YouTube-Client-Version", config.client_version);
        if let Some(vd) = visitor_data {
            req = req.header("X-Goog-Visitor-Id", vd);
        }
        if let Some(auth) = auth_header {
            req = req.header("Authorization", auth);
        }
        let res = req.json(&body).send().await?;
        let status = res.status();
        if !status.is_success() {
            let text = res
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Next request failed (status={}): {}", status, text).into());
        }
        Ok(res.json().await?)
    }
}
pub mod core {
    use super::{
        YouTubeClient,
        common::{
            INNERTUBE_API, PlayerRequestOptions, make_player_request, resolve_format_url,
            select_best_audio_format,
        },
    };
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager,
            clients::common::ClientConfig,
            extractor::{extract_from_player, extract_track},
            oauth::YouTubeOAuth,
        },
    };
    use serde_json::{Value, json};
    use std::sync::Arc;
    pub fn extract_visitor_data(context: &Value) -> Option<&str> {
        context
            .get("client")
            .and_then(|c| c.get("visitorData"))
            .and_then(|v| v.as_str())
            .or_else(|| context.get("visitorData").and_then(|v| v.as_str()))
    }
    pub async fn standard_search<T: YouTubeClient>(
        client: &T,
        http: &Arc<reqwest::Client>,
        query: &str,
        context: &Value,
        _oauth: Arc<YouTubeOAuth>,
        config_builder: impl FnOnce() -> ClientConfig<'static>,
    ) -> AnyResult<Vec<Track>> {
        let visitor_data = extract_visitor_data(context);
        let config = config_builder();
        let body = json!({
            "context": config.build_context(visitor_data),
            "query": query,
            "params": "EgIQAQ%3D%3D"
        });
        let url = format!("{}/youtubei/v1/search", INNERTUBE_API);
        let mut req = http
            .post(&url)
            .header("User-Agent", client.user_agent())
            .header("X-Goog-Api-Format-Version", "2")
            .header("X-YouTube-Client-Name", client.client_name())
            .header("X-YouTube-Client-Version", client.client_version());
        if let Some(vd) = visitor_data {
            req = req.header("X-Goog-Visitor-Id", vd);
        }
        let res = req.json(&body).send().await?;
        let status = res.status();
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(format!("{} search failed: {} - {}", client.name(), status, text).into());
        }
        let response: Value = res.json().await?;
        let mut tracks = Vec::new();
        if let Some(contents) = response.get("contents") {
            let sections = contents
                .get("sectionListRenderer")
                .and_then(|s| s.get("contents"))
                .and_then(|c| c.as_array())
                .or_else(|| {
                    contents
                        .get("twoColumnSearchResultsRenderer")
                        .and_then(|t| t.get("primaryContents"))
                        .and_then(|p| p.get("sectionListRenderer"))
                        .and_then(|s| s.get("contents"))
                        .and_then(|c| c.as_array())
                });
            if let Some(sections) = sections {
                for section in sections {
                    let items_opt = section
                        .get("itemSectionRenderer")
                        .and_then(|i| i.get("contents"))
                        .and_then(|c| c.as_array());
                    let shelf_items_opt = items_opt
                        .is_none()
                        .then(|| {
                            let shelf = section
                                .get("shelfRenderer")
                                .or_else(|| section.get("richShelfRenderer"))
                                .or_else(|| section.get("reelShelfRenderer"));
                            shelf.and_then(|s| {
                                s.get("content")
                                    .and_then(|c| {
                                        c.get("verticalListRenderer")
                                            .or_else(|| c.get("horizontalListRenderer"))
                                    })
                                    .and_then(|v| v.get("items"))
                                    .or_else(|| {
                                        s.get("content")
                                            .and_then(|c| c.get("richGridRenderer"))
                                            .and_then(|r| r.get("contents"))
                                    })
                                    .and_then(|c| c.as_array())
                            })
                        })
                        .flatten();
                    let items = items_opt.or(shelf_items_opt);
                    if let Some(items) = items {
                        for item in items {
                            let inner = item
                                .get("richItemRenderer")
                                .and_then(|r| r.get("content"))
                                .unwrap_or(item);
                            if let Some(track) = extract_track(inner, "youtube") {
                                tracks.push(track);
                            }
                        }
                    }
                }
            } else if let Some(contents) = contents
                .get("twoColumnSearchResultsRenderer")
                .and_then(|t| t.get("primaryContents"))
                .and_then(|p| p.get("richGridRenderer"))
                .and_then(|r| r.get("contents"))
                .and_then(|c| c.as_array())
            {
                for item in contents {
                    let inner = item
                        .get("richItemRenderer")
                        .and_then(|r| r.get("content"))
                        .unwrap_or(item);
                    if let Some(track) = extract_track(inner, "youtube") {
                        tracks.push(track);
                    }
                }
            } else {
                tracing::debug!(
                    "Search: No standard sections found in response contents. keys: {:?}",
                    contents.as_object().map(|o| o.keys().collect::<Vec<_>>())
                );
            }
        } else {
            tracing::debug!(
                "Search: No contents found in response. keys: {:?}",
                response.as_object().map(|o| o.keys().collect::<Vec<_>>())
            );
        }
        Ok(tracks)
    }
    pub struct StandardPlayerOptions<'a, F>
    where
        F: FnOnce() -> ClientConfig<'static>,
    {
        pub http: &'a Arc<reqwest::Client>,
        pub track_id: &'a str,
        pub context: &'a Value,
        pub oauth: Arc<YouTubeOAuth>,
        pub signature_timestamp: Option<u32>,
        pub encrypted_host_flags: Option<String>,
        pub config_builder: F,
    }
    pub async fn standard_get_track_info<T, F>(
        client: &T,
        opts: StandardPlayerOptions<'_, F>,
    ) -> AnyResult<Option<Track>>
    where
        T: YouTubeClient,
        F: FnOnce() -> ClientConfig<'static>,
    {
        let visitor_data = extract_visitor_data(opts.context);
        let config = (opts.config_builder)();
        let body = make_player_request(PlayerRequestOptions {
            http: opts.http,
            config: &config,
            video_id: opts.track_id,
            params: None,
            visitor_data,
            signature_timestamp: opts.signature_timestamp,
            auth_header: if client.supports_oauth() {
                opts.oauth.get_auth_header().await
            } else {
                None
            },
            referer: None,
            origin: None,
            po_token: None,
            encrypted_host_flags: opts.encrypted_host_flags,
            attestation_request: None,
            serialized_third_party_embed_config: client.is_embedded(),
        })
        .await?;
        Ok(extract_from_player(&body, "youtube"))
    }
    pub struct StandardUrlOptions<'a, F>
    where
        F: FnOnce() -> ClientConfig<'static>,
    {
        pub http: &'a Arc<reqwest::Client>,
        pub track_id: &'a str,
        pub context: &'a Value,
        pub cipher_manager: Arc<YouTubeCipherManager>,
        pub oauth: Arc<YouTubeOAuth>,
        pub signature_timestamp: Option<u32>,
        pub encrypted_host_flags: Option<String>,
        pub config_builder: F,
    }
    pub async fn standard_get_track_url<T, F>(
        client: &T,
        opts: StandardUrlOptions<'_, F>,
    ) -> AnyResult<Option<String>>
    where
        T: YouTubeClient,
        F: FnOnce() -> ClientConfig<'static>,
    {
        let visitor_data = extract_visitor_data(opts.context);
        let config = (opts.config_builder)();
        let body = make_player_request(PlayerRequestOptions {
            http: opts.http,
            config: &config,
            video_id: opts.track_id,
            params: None,
            visitor_data,
            signature_timestamp: opts.signature_timestamp,
            auth_header: if client.supports_oauth() {
                opts.oauth.get_auth_header().await
            } else {
                None
            },
            referer: None,
            origin: None,
            po_token: None,
            encrypted_host_flags: opts.encrypted_host_flags,
            attestation_request: None,
            serialized_third_party_embed_config: client.is_embedded(),
        })
        .await?;
        if let Err(e) = crate::sources::youtube::utils::parse_playability_status(&body) {
            tracing::warn!(
                "{} player: video {} not playable: {}",
                client.name(),
                opts.track_id,
                e
            );
            return Err(e.into());
        }
        let streaming_data = match body.get("streamingData") {
            Some(sd) => sd,
            None => {
                tracing::error!(
                    "{} player: no streamingData for {}",
                    client.name(),
                    opts.track_id
                );
                return Ok(None);
            }
        };
        if let Some(hls) = streaming_data
            .get("hlsManifestUrl")
            .and_then(|v| v.as_str())
        {
            tracing::debug!(
                "{} player: using HLS manifest for {}",
                client.name(),
                opts.track_id
            );
            return Ok(Some(hls.to_string()));
        }
        let adaptive = streaming_data
            .get("adaptiveFormats")
            .and_then(|v| v.as_array());
        let formats = streaming_data.get("formats").and_then(|v| v.as_array());
        let player_page_url = format!("https://www.youtube.com/watch?v={}", opts.track_id);
        if let Some(best) = select_best_audio_format(adaptive, formats) {
            match resolve_format_url(best, &player_page_url, &opts.cipher_manager).await {
                Ok(Some(url)) => {
                    return Ok(Some(url));
                }
                Ok(None) => {
                    tracing::warn!(
                        "{} player: best format had no resolvable URL for {}",
                        client.name(),
                        opts.track_id
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "{} player: cipher resolution failed for {}: {}",
                        client.name(),
                        opts.track_id,
                        e
                    );
                    return Err(e);
                }
            }
        }
        Ok(None)
    }
    pub async fn standard_get_playlist<F>(
        client: &dyn YouTubeClient,
        http: &reqwest::Client,
        playlist_id: &str,
        context: &Value,
        oauth: Arc<YouTubeOAuth>,
        config_builder: F,
    ) -> AnyResult<Option<(Vec<Track>, String)>>
    where
        F: Fn() -> ClientConfig<'static>,
    {
        let visitor_data = extract_visitor_data(context);
        let config = config_builder();
        let browse_body = json!({
            "context": config.build_context(visitor_data),
            "browseId": if playlist_id.starts_with("VL") { playlist_id.to_string() } else { format!("VL{}", playlist_id) },
        });
        let browse_url = "https://www.youtube.com/youtubei/v1/browse?prettyPrint=false";
        let mut browse_req = http
            .post(browse_url)
            .header("User-Agent", client.user_agent())
            .header("X-YouTube-Client-Name", client.client_name())
            .header("X-YouTube-Client-Version", client.client_version());
        if let Some(auth) = oauth.get_auth_header().await {
            browse_req = browse_req.header("Authorization", auth);
        }
        if let Some(vd) = visitor_data {
            browse_req = browse_req.header("X-Goog-Visitor-Id", vd);
        }
        if let Ok(res) = browse_req.json(&browse_body).send().await
            && res.status().is_success()
        {
            let body: Value = res.json().await?;
            if let Some(result) =
                crate::sources::youtube::extractor::extract_from_browse(&body, "youtube")
            {
                return Ok(Some(result));
            }
        }
        let next_body = json!({
            "context": config.build_context(visitor_data),
            "playlistId": playlist_id,
            "enablePersistentPlaylistPanel": true,
        });
        let next_url = "https://www.youtube.com/youtubei/v1/next?prettyPrint=false";
        let mut next_req = http
            .post(next_url)
            .header("User-Agent", client.user_agent())
            .header("X-YouTube-Client-Name", client.client_name())
            .header("X-YouTube-Client-Version", client.client_version());
        if let Some(auth) = oauth.get_auth_header().await {
            next_req = next_req.header("Authorization", auth);
        }
        if let Some(vd) = visitor_data {
            next_req = next_req.header("X-Goog-Visitor-Id", vd);
        }
        if let Ok(res) = next_req.json(&next_body).send().await
            && res.status().is_success()
        {
            let body: Value = res.json().await?;
            if let Some(result) =
                crate::sources::youtube::extractor::extract_from_next(&body, "youtube")
            {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }
}
pub mod ios {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "IOS";
    const CLIENT_VERSION: &str = "21.02.1";
    const USER_AGENT: &str =
        "com.google.ios.youtube/21.02.1 (iPhone16,2; U; CPU iOS 18_2 like Mac OS X;)";
    pub struct IosClient {
        http: Arc<reqwest::Client>,
    }
    impl IosClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: "5",
                user_agent: USER_AGENT,
                device_make: Some("Apple"),
                device_model: Some("iPhone16,2"),
                os_name: Some("iPhone"),
                os_version: Some("18.2.22C152"),
                utc_offset_minutes: Some(0),
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for IosClient {
        fn name(&self) -> &str {
            "IOS"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            _playlist_id: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            Ok(None)
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: None,
                    origin: None,
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod music_android {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::{Track, TrackInfo},
        sources::youtube::{
            cipher::YouTubeCipherManager,
            clients::common::{ClientConfig, extract_thumbnail, is_duration, parse_duration},
            oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::sync::Arc;
    const CLIENT_NAME: &str = "ANDROID_MUSIC";
    const CLIENT_VERSION: &str = "8.47.54";
    const USER_AGENT: &str =
        "com.google.android.apps.youtube.music/8.47.54 (Linux; U; Android 14 gzip)";
    const INNERTUBE_API: &str = "https://music.youtube.com";
    pub struct MusicAndroidClient {
        http: Arc<reqwest::Client>,
    }
    impl MusicAndroidClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: "67",
                user_agent: USER_AGENT,
                device_make: Some("Google"),
                device_model: Some("Pixel 6"),
                os_name: Some("Android"),
                os_version: Some("14"),
                android_sdk_version: Some("30"),
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for MusicAndroidClient {
        fn name(&self) -> &str {
            "MusicAndroid"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        fn can_handle_request(&self, identifier: &str) -> bool {
            !identifier.contains("list=") || identifier.contains("list=RD")
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            let visitor_data = core::extract_visitor_data(context);
            let body = json!({
                "context": self.config().build_context(None),
                "query": query,
                "params": "EgWKAQIIAWoQEAMQBBAJEAoQBRAREBAQFQ%3D%3D"
            });
            let url = format!("{}/youtubei/v1/search?prettyPrint=false", INNERTUBE_API);
            let mut req = self
                .http
                .post(&url)
                .header("X-Goog-Api-Format-Version", "2")
                .header("User-Agent", USER_AGENT);
            if let Some(vd) = visitor_data {
                req = req.header("X-Goog-Visitor-Id", vd);
            }
            let req = req.json(&body);
            let res = req.send().await?;
            if !res.status().is_success() {
                let status = res.status();
                let err_body = res.text().await.unwrap_or_default();
                return Err(
                    format!("Music Android search failed: {} - {}", status, err_body).into(),
                );
            }
            let response: Value = res.json().await.unwrap_or_default();
            let mut tracks = Vec::new();
            let tab_content = response
                .get("contents")
                .and_then(|c| c.get("tabbedSearchResultsRenderer"))
                .and_then(|t| t.get("tabs"))
                .and_then(|t| t.get(0))
                .and_then(|t| t.get("tabRenderer"))
                .and_then(|t| t.get("content"));
            let mut videos = None;
            fn find_shelf(contents: &Value) -> Option<&Vec<Value>> {
                if let Some(sections) = contents.as_array() {
                    for section in sections {
                        if let Some(shelf) = section.get("musicShelfRenderer") {
                            return shelf.get("contents").and_then(|c| c.as_array());
                        }
                    }
                }
                None
            }
            if let Some(tab) = tab_content {
                if let Some(section_list) = tab.get("sectionListRenderer")
                    && let Some(contents) = section_list.get("contents")
                {
                    videos = find_shelf(contents);
                }
                if videos.is_none()
                    && let Some(split_view) = tab.get("musicSplitViewRenderer")
                    && let Some(main_content) = split_view.get("mainContent")
                    && let Some(section_list) = main_content.get("sectionListRenderer")
                    && let Some(contents) = section_list.get("contents")
                {
                    videos = find_shelf(contents);
                }
            }
            if let Some(items) = videos {
                for item in items {
                    let renderer = item
                        .get("musicResponsiveListItemRenderer")
                        .or_else(|| item.get("musicTwoColumnItemRenderer"))
                        .or_else(|| {
                            if item.get("videoId").is_some() {
                                Some(item)
                            } else {
                                None
                            }
                        });
                    if let Some(renderer) = renderer {
                        let id = renderer
                            .get("playlistItemData")
                            .and_then(|d| d.get("videoId"))
                            .and_then(|v| v.as_str())
                            .or_else(|| {
                                renderer
                                    .get("navigationEndpoint")
                                    .and_then(|n| n.get("watchEndpoint"))
                                    .and_then(|w| w.get("videoId"))
                                    .and_then(|v| v.as_str())
                            })
                            .or_else(|| {
                                renderer
                                    .get("doubleTapCommand")
                                    .and_then(|c| c.get("watchEndpoint"))
                                    .and_then(|w| w.get("videoId"))
                                    .and_then(|v| v.as_str())
                            })
                            .or_else(|| renderer.get("videoId").and_then(|v| v.as_str()));
                        if let Some(id) = id {
                            let mut title = renderer
                                .get("title")
                                .and_then(|t| t.get("runs"))
                                .and_then(|r| r.get(0))
                                .and_then(|r| r.get("text"))
                                .and_then(|t| t.as_str())
                                .or_else(|| {
                                    renderer
                                        .get("title")
                                        .and_then(|t| t.get("simpleText"))
                                        .and_then(|t| t.as_str())
                                })
                                .or_else(|| renderer.get("title").and_then(|t| t.as_str()))
                                .unwrap_or("Unknown Title");
                            if title == "Unknown Title"
                                && let Some(flex_cols) =
                                    renderer.get("flexColumns").and_then(|c| c.as_array())
                                && !flex_cols.is_empty()
                                && let Some(t) = flex_cols[0]
                                    .get("musicResponsiveListItemFlexColumnRenderer")
                                    .and_then(|r| r.get("text"))
                                    .and_then(|t| t.get("runs"))
                                    .and_then(|r| r.get(0))
                                    .and_then(|r| r.get("text"))
                                    .and_then(|t| t.as_str())
                            {
                                title = t;
                            }
                            let mut author = "Unknown Artist".to_string();
                            let subtitle_runs = renderer
                                .get("subtitle")
                                .and_then(|s| s.get("runs"))
                                .and_then(|r| r.as_array());
                            let long_byline_runs = renderer
                                .get("longBylineText")
                                .and_then(|l| l.get("runs"))
                                .and_then(|r| r.as_array());
                            let short_byline_runs = renderer
                                .get("shortBylineText")
                                .and_then(|s| s.get("runs"))
                                .and_then(|r| r.as_array());
                            if let Some(runs) = subtitle_runs {
                                if !runs.is_empty()
                                    && let Some(a) = runs[0].get("text").and_then(|t| t.as_str())
                                {
                                    author = a.to_string();
                                }
                            } else if let Some(runs) = long_byline_runs {
                                if !runs.is_empty()
                                    && let Some(a) = runs[0].get("text").and_then(|t| t.as_str())
                                {
                                    author = a.to_string();
                                }
                            } else if let Some(runs) = short_byline_runs {
                                if !runs.is_empty()
                                    && let Some(a) = runs[0].get("text").and_then(|t| t.as_str())
                                {
                                    author = a.to_string();
                                }
                            } else if let Some(a) = renderer.get("author").and_then(|a| a.as_str())
                            {
                                author = a.to_string();
                            }
                            if author == "Unknown Artist"
                                && let Some(flex_cols) =
                                    renderer.get("flexColumns").and_then(|c| c.as_array())
                                && flex_cols.len() > 1
                                && let Some(a) = flex_cols[1]
                                    .get("musicResponsiveListItemFlexColumnRenderer")
                                    .and_then(|r| r.get("text"))
                                    .and_then(|t| t.get("runs"))
                                    .and_then(|r| r.get(0))
                                    .and_then(|r| r.get("text"))
                                    .and_then(|t| t.as_str())
                            {
                                author = a.to_string();
                            }
                            let mut length_ms = 0u64;
                            if let Some(runs) = subtitle_runs {
                                for run in runs {
                                    if let Some(text) = run.get("text").and_then(|t| t.as_str())
                                        && is_duration(text)
                                    {
                                        length_ms = parse_duration(text);
                                        break;
                                    }
                                }
                            }
                            if length_ms == 0
                                && let Some(text) = renderer
                                    .get("lengthText")
                                    .and_then(|l| l.get("simpleText"))
                                    .and_then(|t| t.as_str())
                                && is_duration(text)
                            {
                                length_ms = parse_duration(text);
                            }
                            if length_ms == 0
                                && let Some(runs) = renderer
                                    .get("lengthText")
                                    .and_then(|l| l.get("runs"))
                                    .and_then(|r| r.as_array())
                            {
                                for run in runs {
                                    if let Some(text) = run.get("text").and_then(|t| t.as_str())
                                        && is_duration(text)
                                    {
                                        length_ms = parse_duration(text);
                                        break;
                                    }
                                }
                            }
                            if length_ms == 0
                                && let Some(flex_cols) =
                                    renderer.get("flexColumns").and_then(|c| c.as_array())
                            {
                                for column in flex_cols {
                                    if let Some(runs) = column
                                        .get("musicResponsiveListItemFlexColumnRenderer")
                                        .and_then(|r| r.get("text"))
                                        .and_then(|t| t.get("runs"))
                                        .and_then(|r| r.as_array())
                                    {
                                        for run in runs {
                                            if let Some(text) =
                                                run.get("text").and_then(|t| t.as_str())
                                                && is_duration(text)
                                            {
                                                length_ms = parse_duration(text);
                                                break;
                                            }
                                        }
                                    }
                                    if length_ms > 0 {
                                        break;
                                    }
                                }
                            }
                            let artwork_url = extract_thumbnail(renderer, Some(id));
                            let info = TrackInfo {
                                identifier: id.to_string(),
                                is_seekable: true,
                                title: title.to_string(),
                                author,
                                length: length_ms,
                                is_stream: false,
                                uri: Some(format!("https://music.youtube.com/watch?v={}", id)),
                                source_name: "youtube".to_string(),
                                isrc: None,
                                artwork_url,
                                position: 0,
                            };
                            tracks.push(Track::new(info));
                        }
                    }
                }
            }
            Ok(tracks)
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            let visitor_data = core::extract_visitor_data(context);
            let next_body = json!({
                "context": self.config().build_context(visitor_data),
                "playlistId": playlist_id,
                "enablePersistentPlaylistPanel": true,
                "isAudioOnly": true
            });
            let next_url = format!("{}/youtubei/v1/next?prettyPrint=false", INNERTUBE_API);
            let mut next_req = self
                .http
                .post(&next_url)
                .header("User-Agent", USER_AGENT)
                .header("X-YouTube-Client-Name", "67")
                .header("X-YouTube-Client-Version", CLIENT_VERSION);
            if let Some(vd) = visitor_data {
                next_req = next_req.header("X-Goog-Visitor-Id", vd);
            }
            let next_req = next_req.json(&next_body);
            if let Ok(res) = next_req.send().await
                && res.status().is_success()
            {
                let body: Value = res.json().await?;
                if let Some(result) =
                    crate::sources::youtube::extractor::extract_from_next(&body, "youtube")
                {
                    return Ok(Some(result));
                }
                tracing::debug!(
                    "MusicAndroid: /next endpoint returned but extraction failed for playlist {}",
                    playlist_id
                );
            }
            let browse_body = json!({
                "context": self.config().build_context(visitor_data),
                "browseId": if playlist_id.starts_with("VL") { playlist_id.to_string() } else { format!("VL{}", playlist_id) },
            });
            let browse_url = format!("{}/youtubei/v1/browse?prettyPrint=false", INNERTUBE_API);
            let mut browse_req = self
                .http
                .post(&browse_url)
                .header("User-Agent", USER_AGENT)
                .header("X-YouTube-Client-Name", "67")
                .header("X-YouTube-Client-Version", CLIENT_VERSION);
            if let Some(vd) = visitor_data {
                browse_req = browse_req.header("X-Goog-Visitor-Id", vd);
            }
            if let Ok(res) = browse_req.json(&browse_body).send().await
                && res.status().is_success()
            {
                let body: Value = res.json().await?;
                if let Some(result) =
                    crate::sources::youtube::extractor::extract_from_browse(&body, "youtube")
                {
                    return Ok(Some(result));
                }
                tracing::debug!(
                    "MusicAndroid: /browse endpoint returned but extraction failed for playlist {}",
                    playlist_id
                );
            }
            tracing::warn!(
                "MusicAndroid: Both /next and /browse endpoints failed for playlist {}",
                playlist_id
            );
            Ok(None)
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: None,
                    origin: Some(INNERTUBE_API),
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod mweb {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "MWEB";
    const CLIENT_ID: &str = "2";
    const CLIENT_VERSION: &str = "2.20241022.01.00";
    const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";
    pub struct MWebClient {
        http: Arc<reqwest::Client>,
    }
    impl MWebClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: CLIENT_ID,
                user_agent: USER_AGENT,
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for MWebClient {
        fn name(&self) -> &str {
            "MWeb"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            core::standard_get_playlist(self, &self.http, playlist_id, context, oauth, || {
                self.config()
            })
            .await
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: Some("https://m.youtube.com"),
                    origin: None,
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod tv {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "TVHTML5";
    const CLIENT_ID: &str = "7";
    const CLIENT_VERSION: &str = "7.20260113.16.00";
    const USER_AGENT: &str = "Mozilla/5.0 (Fuchsia) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/140.0.0.0 Safari/537.36 CrKey/1.56.500000";
    pub struct TvClient {
        http: Arc<reqwest::Client>,
    }
    impl TvClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: CLIENT_ID,
                user_agent: USER_AGENT,
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for TvClient {
        fn name(&self) -> &str {
            "TV"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        fn supports_oauth(&self) -> bool {
            true
        }
        fn can_handle_request(&self, _identifier: &str) -> bool {
            false
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            core::standard_get_playlist(self, &self.http, playlist_id, context, oauth, || {
                self.config()
            })
            .await
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: oauth.get_auth_header().await,
                    referer: None,
                    origin: None,
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod tv_cast {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME_OVERRIDE: &str = "TVHTML5_CAST";
    const CLIENT_VERSION: &str = "7.20190924";
    const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36 CrKey/1.54.248666";
    pub struct TvCastClient {
        http: Arc<reqwest::Client>,
    }
    impl TvCastClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME_OVERRIDE,
                client_version: CLIENT_VERSION,
                client_id: "7",
                user_agent: USER_AGENT,
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for TvCastClient {
        fn name(&self) -> &str {
            "TV Cast"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME_OVERRIDE
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        fn can_handle_request(&self, _identifier: &str) -> bool {
            false
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            core::standard_get_playlist(self, &self.http, playlist_id, context, oauth, || {
                self.config()
            })
            .await
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: None,
                    origin: None,
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod tv_embedded {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "TVHTML5_SIMPLY_EMBEDDED_PLAYER";
    const CLIENT_ID: &str = "85";
    const CLIENT_VERSION: &str = "2.0";
    const USER_AGENT: &str = "Mozilla/5.0 (Linux armeabi-v7a; Android 7.1.2; Fire OS 6.0) Cobalt/22.lts.3.306369-gold (unlike Gecko) v8/8.8.278.8-jit gles Starboard/13, Amazon_ATV_mediatek8695_2019/NS6294 (Amazon, AFTMM, Wireless) com.amazon.firetv.youtube/22.3.r2.v66.0";
    pub struct TvEmbeddedClient {
        http: Arc<reqwest::Client>,
    }
    impl TvEmbeddedClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: CLIENT_ID,
                user_agent: USER_AGENT,
                third_party_embed_url: Some("https://www.youtube.com/tv"),
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for TvEmbeddedClient {
        fn name(&self) -> &str {
            "TvEmbedded"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        fn supports_oauth(&self) -> bool {
            true
        }
        fn can_handle_request(&self, identifier: &str) -> bool {
            !identifier.contains("list=")
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            core::standard_get_playlist(self, &self.http, playlist_id, context, oauth, || {
                self.config()
            })
            .await
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: Some("2AMB"),
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: oauth.get_auth_header().await,
                    referer: None,
                    origin: None,
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod tv_simply {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::sync::Arc;
    const CLIENT_NAME: &str = "TVHTML5_SIMPLY";
    const CLIENT_VERSION: &str = "1.0";
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36";
    pub struct TvSimplyClient {
        http: Arc<reqwest::Client>,
    }
    impl TvSimplyClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: "TVHTML5_SIMPLY",
                user_agent: USER_AGENT,
                attestation_request: Some(json!({ "omitBotguardData": true })),
                ..Default::default()
            }
        }
        async fn fetch_encrypted_host_flags(&self, video_id: &str) -> Option<String> {
            let url = format!("https://www.youtube.com/embed/{}", video_id);
            let res = self
            .http
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send()
            .await
            .ok()?;
            let html = res.text().await.ok()?;
            let re = regex::Regex::new(r#""encryptedHostFlags":"([^"]+)""#).ok()?;
            re.captures(&html).map(|caps| caps[1].to_string())
        }
    }
    #[async_trait]
    impl YouTubeClient for TvSimplyClient {
        fn name(&self) -> &str {
            "TvSimply"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            let encrypted_host_flags = self.fetch_encrypted_host_flags(track_id).await;
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            core::standard_get_playlist(self, &self.http, playlist_id, context, oauth, || {
                self.config()
            })
            .await
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            let encrypted_host_flags = self.fetch_encrypted_host_flags(track_id).await;
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            let encrypted_host_flags = self.fetch_encrypted_host_flags(track_id).await;
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: Some("2AMB"),
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: None,
                    origin: Some("https://www.youtube.com"),
                    po_token: None,
                    encrypted_host_flags,
                    attestation_request: Some(json!({ "omitBotguardData": true })),
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{
            config::sources::YouTubeCipherConfig, sources::youtube::cipher::YouTubeCipherManager,
        };
        #[tokio::test]
        async fn test_search() {
            let http = Arc::new(reqwest::Client::new());
            let _cipher = Arc::new(YouTubeCipherManager::new(YouTubeCipherConfig::default()));
            let client = TvSimplyClient::new(http);
            let oauth = Arc::new(YouTubeOAuth::new(vec![]));
            let result = client.search("test", &json!({}), oauth).await.unwrap();
            assert!(!result.is_empty(), "Search should return tracks");
        }
        #[tokio::test]
        async fn test_playlist() {
            let http = Arc::new(reqwest::Client::new());
            let _cipher = Arc::new(YouTubeCipherManager::new(YouTubeCipherConfig::default()));
            let client = TvSimplyClient::new(http);
            let oauth = Arc::new(YouTubeOAuth::new(vec![]));
            let result = client
                .get_playlist("PLFsQleAWXsj_4yDeebiIADdH5FMayBiJo", &json!({}), oauth)
                .await
                .unwrap();
            assert!(result.is_some(), "Playlist should return tracks");
            assert!(
                !result.unwrap().0.is_empty(),
                "Playlist should not be empty"
            );
        }
    }
    #[cfg(test)]
    mod get_track_tests {
        use super::*;
        use crate::{
            config::sources::YouTubeCipherConfig, sources::youtube::cipher::YouTubeCipherManager,
        };
        #[tokio::test]
        async fn test_get_track_url() {
            let http = Arc::new(reqwest::Client::new());
            let _cipher = Arc::new(YouTubeCipherManager::new(YouTubeCipherConfig::default()));
            let client = TvSimplyClient::new(http);
            let body = client
                .get_player_body("3Z_x7vBqr6E", None, Arc::new(YouTubeOAuth::new(vec![])))
                .await;
            assert!(body.is_some());
            println!(
                "Body: {}",
                serde_json::to_string_pretty(&body.unwrap()).unwrap()
            );
        }
    }
}
pub mod tv_unplugged {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "TVHTML5_UNPLUGGED";
    const CLIENT_VERSION: &str = "6.13";
    const USER_AGENT: &str = "Mozilla/5.0 (Linux armeabi-v7a; Android 7.1.2; Fire OS 6.0) Cobalt/22.lts.3.306369-gold (unlike Gecko) v8/8.8.278.8-jit gles Starboard/13, Amazon_ATV_mediatek8695_2019/NS6294 (Amazon, AFTMM, Wireless) com.amazon.firetv.youtube/22.3.r2.v66.0";
    pub struct TvUnpluggedClient {
        http: Arc<reqwest::Client>,
    }
    impl TvUnpluggedClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                user_agent: USER_AGENT,
                ..Default::default()
            }
        }
        async fn fetch_encrypted_host_flags(&self, video_id: &str) -> Option<String> {
            let url = format!("https://www.youtube.com/embed/{}", video_id);
            let res = self
            .http
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send()
            .await
            .ok()?;
            let html = res.text().await.ok()?;
            let re = regex::Regex::new(r#""encryptedHostFlags":"([^"]+)""#).ok()?;
            re.captures(&html).map(|caps| caps[1].to_string())
        }
    }
    #[async_trait]
    impl YouTubeClient for TvUnpluggedClient {
        fn name(&self) -> &str {
            "TvUnplugged"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        fn can_handle_request(&self, identifier: &str) -> bool {
            if identifier.contains("list=") && !identifier.contains("list=RD") {
                return false;
            }
            true
        }
        async fn search(
            &self,
            _query: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            Err("TvUnplugged client does not support search".into())
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            let encrypted_host_flags = self.fetch_encrypted_host_flags(track_id).await;
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags,
                    config_builder: || {
                        let mut cfg = self.config();
                        cfg.client_screen = Some("EMBED");
                        cfg
                    },
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            _playlist_id: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            Ok(None)
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            let encrypted_host_flags = self.fetch_encrypted_host_flags(track_id).await;
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags,
                    config_builder: || {
                        let mut cfg = self.config();
                        cfg.client_screen = Some("EMBED");
                        cfg
                    },
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            let encrypted_host_flags = self.fetch_encrypted_host_flags(track_id).await;
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &{
                        let mut cfg = self.config();
                        cfg.client_screen = Some("EMBED");
                        cfg
                    },
                    video_id: track_id,
                    params: Some("2AMB"),
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: None,
                    origin: Some("https://www.youtube.com/"),
                    po_token: None,
                    encrypted_host_flags,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod web {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "WEB";
    const CLIENT_ID: &str = "1";
    const CLIENT_VERSION: &str = "2.20260114.01.00";
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
     AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";
    pub struct WebClient {
        http: Arc<reqwest::Client>,
        pub yt_cipher_url: Option<String>,
        pub yt_cipher_token: Option<String>,
    }
    impl WebClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self {
                http,
                yt_cipher_url: None,
                yt_cipher_token: None,
            }
        }
        pub fn with_cipher_url(
            http: Arc<reqwest::Client>,
            yt_cipher_url: Option<String>,
            yt_cipher_token: Option<String>,
        ) -> Self {
            let mut client = Self::new(http);
            client.yt_cipher_url = yt_cipher_url;
            client.yt_cipher_token = yt_cipher_token;
            client
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: CLIENT_ID,
                user_agent: USER_AGENT,
                platform: Some("DESKTOP"),
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for WebClient {
        fn name(&self) -> &str {
            "Web"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            core::standard_get_playlist(self, &self.http, playlist_id, context, oauth, || {
                self.config()
            })
            .await
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: None,
                    origin: None,
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod web_embedded {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "WEB_EMBEDDED_PLAYER";
    const CLIENT_ID: &str = "56";
    const CLIENT_VERSION: &str = "1.20260128.01.00";
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36,gzip(gfe)";
    pub struct WebEmbeddedClient {
        http: Arc<reqwest::Client>,
    }
    impl WebEmbeddedClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: CLIENT_ID,
                user_agent: USER_AGENT,
                platform: Some("DESKTOP"),
                third_party_embed_url: Some("https://www.google.com/"),
                ..Default::default()
            }
        }
        async fn fetch_encrypted_host_flags(&self, video_id: &str) -> Option<String> {
            let url = format!("https://www.youtube.com/embed/{}", video_id);
            let res = self
            .http
            .get(&url)
            .header("Referer", "https://www.google.com")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .send()
            .await
            .ok()?;
            if !res.status().is_success() {
                return None;
            }
            let body = res.text().await.ok()?;
            let re = regex::Regex::new(r#""encryptedHostFlags":"([^"]+)""#).ok()?;
            re.captures(&body)
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().to_string())
        }
    }
    #[async_trait]
    impl YouTubeClient for WebEmbeddedClient {
        fn name(&self) -> &str {
            "WebEmbedded"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        fn is_embedded(&self) -> bool {
            true
        }
        fn can_handle_request(&self, identifier: &str) -> bool {
            !identifier.contains("list=")
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            _track_id: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_playlist(
            &self,
            _playlist_id: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            Ok(None)
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            let encrypted_host_flags = self.fetch_encrypted_host_flags(track_id).await;
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            let encrypted_host_flags = self.fetch_encrypted_host_flags(track_id).await;
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: Some("https://www.youtube.com"),
                    origin: None,
                    po_token: None,
                    encrypted_host_flags,
                    attestation_request: None,
                    serialized_third_party_embed_config: true,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod web_parent_tools {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::Track,
        sources::youtube::{
            cipher::YouTubeCipherManager, clients::common::ClientConfig, oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Arc;
    const CLIENT_NAME: &str = "WEB_PARENT_TOOLS";
    const CLIENT_ID: &str = "88";
    const CLIENT_VERSION: &str = "1.20220918";
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36,gzip(gfe)";
    pub struct WebParentToolsClient {
        http: Arc<reqwest::Client>,
    }
    impl WebParentToolsClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: CLIENT_ID,
                user_agent: USER_AGENT,
                third_party_embed_url: Some("https://www.youtube.com/"),
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for WebParentToolsClient {
        fn name(&self) -> &str {
            "WebParentTools"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        fn supports_oauth(&self) -> bool {
            true
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            core::standard_search(self, &self.http, query, context, oauth, || self.config()).await
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            core::standard_get_playlist(self, &self.http, playlist_id, context, oauth, || {
                self.config()
            })
            .await
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: Some("2AMB"),
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: oauth.get_auth_header().await,
                    referer: Some("https://www.youtube.com/"),
                    origin: None,
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
pub mod web_remix {
    use super::{YouTubeClient, core};
    use crate::{
        common::types::AnyResult,
        protocol::tracks::{Track, TrackInfo},
        sources::youtube::{
            cipher::YouTubeCipherManager,
            clients::common::{ClientConfig, extract_thumbnail, is_duration, parse_duration},
            oauth::YouTubeOAuth,
        },
    };
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::sync::Arc;
    const CLIENT_NAME: &str = "WEB_REMIX";
    const CLIENT_VERSION: &str = "1.20260121.03.00";
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
     AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";
    const MUSIC_API: &str = "https://music.youtube.com";
    pub struct WebRemixClient {
        http: Arc<reqwest::Client>,
    }
    impl WebRemixClient {
        pub fn new(http: Arc<reqwest::Client>) -> Self {
            Self { http }
        }
        fn config(&self) -> ClientConfig<'static> {
            ClientConfig {
                client_name: CLIENT_NAME,
                client_version: CLIENT_VERSION,
                client_id: "26",
                user_agent: USER_AGENT,
                ..Default::default()
            }
        }
    }
    #[async_trait]
    impl YouTubeClient for WebRemixClient {
        fn name(&self) -> &str {
            "MusicWeb"
        }
        fn client_name(&self) -> &str {
            CLIENT_NAME
        }
        fn client_version(&self) -> &str {
            CLIENT_VERSION
        }
        fn user_agent(&self) -> &str {
            USER_AGENT
        }
        async fn search(
            &self,
            query: &str,
            context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Vec<Track>> {
            let visitor_data = core::extract_visitor_data(context);
            let body = json!({
                "context": self.config().build_context(visitor_data),
                "query": query,
                "params": "EgWKAQIIAWoQEAMQBBAFEBAQCRAKEBUQEQ%3D%3D"
            });
            let url = format!("{}/youtubei/v1/search?prettyPrint=false", MUSIC_API);
            let mut req = self
                .http
                .post(&url)
                .header("User-Agent", USER_AGENT)
                .header("X-Goog-Api-Format-Version", "2")
                .header("Origin", MUSIC_API);
            if let Some(vd) = visitor_data {
                req = req.header("X-Goog-Visitor-Id", vd);
            }
            let req = req.json(&body);
            let res = req.send().await?;
            if !res.status().is_success() {
                return Err(format!("Music search failed: {}", res.status()).into());
            }
            let response: Value = res.json().await?;
            let mut tracks = Vec::new();
            let tab_content = response
                .get("contents")
                .and_then(|c| c.get("tabbedSearchResultsRenderer"))
                .and_then(|t| t.get("tabs"))
                .and_then(|t| t.get(0))
                .and_then(|t| t.get("tabRenderer"))
                .and_then(|t| t.get("content"));
            let mut shelf_contents = None;
            fn find_shelf(content: &Value) -> Option<&Vec<Value>> {
                if let Some(section_list) = content.get("sectionListRenderer")
                    && let Some(sections) = section_list.get("contents").and_then(|c| c.as_array())
                {
                    for section in sections {
                        if let Some(shelf) = section.get("musicShelfRenderer")
                            && let Some(items) = shelf.get("contents").and_then(|c| c.as_array())
                        {
                            return Some(items);
                        }
                    }
                }
                None
            }
            if let Some(tab) = tab_content {
                shelf_contents = find_shelf(tab);
                if shelf_contents.is_none()
                    && let Some(split_view) = tab.get("musicSplitViewRenderer")
                    && let Some(main_content) = split_view.get("mainContent")
                {
                    shelf_contents = find_shelf(main_content);
                }
            }
            if let Some(items) = shelf_contents {
                for item in items {
                    let renderer = item
                        .get("musicResponsiveListItemRenderer")
                        .or_else(|| item.get("musicTwoColumnItemRenderer"));
                    if let Some(renderer) = renderer {
                        let id = renderer
                            .get("playlistItemData")
                            .and_then(|d| d.get("videoId"))
                            .and_then(|v| v.as_str())
                            .or_else(|| {
                                renderer
                                    .get("doubleTapCommand")
                                    .and_then(|c| c.get("watchEndpoint"))
                                    .and_then(|w| w.get("videoId"))
                                    .and_then(|v| v.as_str())
                            })
                            .or_else(|| renderer.get("videoId").and_then(|v| v.as_str()));
                        if let Some(id) = id {
                            let title = renderer
                                .get("flexColumns")
                                .and_then(|c| c.get(0))
                                .and_then(|c| c.get("musicResponsiveListItemFlexColumnRenderer"))
                                .and_then(|r| r.get("text"))
                                .and_then(|t| t.get("runs"))
                                .and_then(|r| r.get(0))
                                .and_then(|r| r.get("text"))
                                .and_then(|t| t.as_str())
                                .unwrap_or("Unknown Title");
                            let mut author = "Unknown Artist".to_string();
                            let mut length_ms = 0u64;
                            if let Some(flex_cols) =
                                renderer.get("flexColumns").and_then(|c| c.as_array())
                            {
                                if flex_cols.len() > 1
                                    && let Some(a) = flex_cols[1]
                                        .get("musicResponsiveListItemFlexColumnRenderer")
                                        .and_then(|r| r.get("text"))
                                        .and_then(|t| t.get("runs"))
                                        .and_then(|r| r.get(0))
                                        .and_then(|r| r.get("text"))
                                        .and_then(|t| t.as_str())
                                {
                                    author = a.to_string();
                                }
                                for col in flex_cols {
                                    if let Some(runs) = col
                                        .get("musicResponsiveListItemFlexColumnRenderer")
                                        .and_then(|r| r.get("text"))
                                        .and_then(|t| t.get("runs"))
                                        .and_then(|r| r.as_array())
                                    {
                                        for run in runs {
                                            if let Some(text) =
                                                run.get("text").and_then(|t| t.as_str())
                                                && is_duration(text)
                                            {
                                                length_ms = parse_duration(text);
                                                break;
                                            }
                                        }
                                    }
                                    if length_ms > 0 {
                                        break;
                                    }
                                }
                            }
                            if author == "Unknown Artist"
                                && let Some(subtitle_runs) = renderer
                                    .get("subtitle")
                                    .and_then(|s| s.get("runs"))
                                    .and_then(|r| r.as_array())
                                && !subtitle_runs.is_empty()
                                && let Some(a) =
                                    subtitle_runs[0].get("text").and_then(|t| t.as_str())
                            {
                                author = a.to_string();
                            }
                            let artwork_url = extract_thumbnail(renderer, Some(id));
                            let info = TrackInfo {
                                identifier: id.to_string(),
                                is_seekable: true,
                                title: title.to_string(),
                                author,
                                length: length_ms,
                                is_stream: false,
                                uri: Some(format!("https://music.youtube.com/watch?v={}", id)),
                                source_name: "youtube".to_string(),
                                isrc: None,
                                artwork_url,
                                position: 0,
                            };
                            tracks.push(Track::new(info));
                        }
                    }
                }
            }
            Ok(tracks)
        }
        async fn get_track_info(
            &self,
            track_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            core::standard_get_track_info(
                self,
                core::StandardPlayerOptions {
                    http: &self.http,
                    track_id,
                    context,
                    oauth,
                    signature_timestamp: None,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_playlist(
            &self,
            playlist_id: &str,
            context: &Value,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<(Vec<Track>, String)>> {
            let visitor_data = core::extract_visitor_data(context);
            let is_mix_playlist = playlist_id.starts_with("RDCLAK5uy_")
                || playlist_id.starts_with("RDAMVM")
                || playlist_id.starts_with("RDMM")
                || playlist_id.starts_with("RD");
            if is_mix_playlist {
                let next_body = json!({
                    "context": self.config().build_context(visitor_data),
                    "playlistId": playlist_id,
                    "enablePersistentPlaylistPanel": true,
                    "isAudioOnly": true
                });
                let next_url = format!("{}/youtubei/v1/next?prettyPrint=false", MUSIC_API);
                let mut next_req = self
                    .http
                    .post(&next_url)
                    .header("User-Agent", USER_AGENT)
                    .header("X-YouTube-Client-Name", "26")
                    .header("X-YouTube-Client-Version", CLIENT_VERSION);
                if let Some(vd) = visitor_data {
                    next_req = next_req.header("X-Goog-Visitor-Id", vd);
                }
                if let Ok(res) = next_req.json(&next_body).send().await
                    && res.status().is_success()
                {
                    let body: Value = res.json().await?;
                    if let Some(result) =
                        crate::sources::youtube::extractor::extract_from_next(&body, "youtube")
                    {
                        return Ok(Some(result));
                    }
                    tracing::debug!(
                        "WebRemix: /next endpoint returned but extraction failed for playlist {}",
                        playlist_id
                    );
                }
            }
            core::standard_get_playlist(self, &self.http, playlist_id, context, oauth, || {
                self.config()
            })
            .await
        }
        async fn resolve_url(
            &self,
            _url: &str,
            _context: &Value,
            _oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<Track>> {
            Ok(None)
        }
        async fn get_track_url(
            &self,
            track_id: &str,
            context: &Value,
            cipher_manager: Arc<YouTubeCipherManager>,
            oauth: Arc<YouTubeOAuth>,
        ) -> AnyResult<Option<String>> {
            let signature_timestamp = cipher_manager.get_signature_timestamp().await.ok();
            core::standard_get_track_url(
                self,
                core::StandardUrlOptions {
                    http: &self.http,
                    track_id,
                    context,
                    cipher_manager,
                    oauth,
                    signature_timestamp,
                    encrypted_host_flags: None,
                    config_builder: || self.config(),
                },
            )
            .await
        }
        async fn get_player_body(
            &self,
            track_id: &str,
            visitor_data: Option<&str>,
            _oauth: Arc<YouTubeOAuth>,
        ) -> Option<serde_json::Value> {
            crate::sources::youtube::clients::common::make_player_request(
                crate::sources::youtube::clients::common::PlayerRequestOptions {
                    http: &self.http,
                    config: &self.config(),
                    video_id: track_id,
                    params: None,
                    visitor_data,
                    signature_timestamp: None,
                    auth_header: None,
                    referer: None,
                    origin: Some(MUSIC_API),
                    po_token: None,
                    encrypted_host_flags: None,
                    attestation_request: None,
                    serialized_third_party_embed_config: false,
                },
            )
            .await
            .ok()
        }
    }
}
use crate::{
    common::types::AnyResult,
    protocol::tracks::Track,
    sources::youtube::{cipher::YouTubeCipherManager, oauth::YouTubeOAuth},
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
#[async_trait]
pub trait YouTubeClient: Send + Sync {
    fn name(&self) -> &str;
    fn client_name(&self) -> &str;
    fn client_version(&self) -> &str;
    fn user_agent(&self) -> &str;
    fn supports_oauth(&self) -> bool {
        false
    }
    fn is_embedded(&self) -> bool {
        false
    }
    fn requires_embed_workaround(&self) -> bool {
        !self.supports_oauth()
    }
    fn can_handle_request(&self, _identifier: &str) -> bool {
        true
    }
    async fn search(
        &self,
        query: &str,
        context: &Value,
        oauth: Arc<YouTubeOAuth>,
    ) -> AnyResult<Vec<Track>>;
    async fn get_track_info(
        &self,
        track_id: &str,
        context: &Value,
        oauth: Arc<YouTubeOAuth>,
    ) -> AnyResult<Option<Track>>;
    async fn resolve_url(
        &self,
        url: &str,
        context: &Value,
        oauth: Arc<YouTubeOAuth>,
    ) -> AnyResult<Option<Track>>;
    async fn get_track_url(
        &self,
        track_id: &str,
        context: &Value,
        cipher_manager: Arc<YouTubeCipherManager>,
        oauth: Arc<YouTubeOAuth>,
    ) -> AnyResult<Option<String>>;
    async fn get_playlist(
        &self,
        playlist_id: &str,
        context: &Value,
        oauth: Arc<YouTubeOAuth>,
    ) -> AnyResult<Option<(Vec<Track>, String)>>;
    async fn get_player_body(
        &self,
        _track_id: &str,
        _visitor_data: Option<&str>,
        _oauth: Arc<YouTubeOAuth>,
    ) -> Option<serde_json::Value> {
        None
    }
}
