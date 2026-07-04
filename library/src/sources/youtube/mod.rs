pub mod utils {
use std::sync::Arc;
use symphonia::core::io::MediaSource;
use crate::{
    common::types::AudioFormat,
    sources::{
        http::reader::HttpReader,
        youtube::{cipher::YouTubeCipherManager, hls::HlsReader, reader::YoutubeReader},
    },
};
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36";
pub fn detect_audio_kind(url: &str, is_hls: bool) -> AudioFormat {
    if is_hls {
        AudioFormat::Aac
    } else {
        AudioFormat::from_url(url)
    }
}
pub async fn create_reader(
    url: &str,
    client_name: &str,
    local_addr: Option<std::net::IpAddr>,
    proxy: Option<crate::config::HttpProxyConfig>,
    cipher_manager: Arc<YouTubeCipherManager>,
) -> AnyResult<Box<dyn MediaSource>> {
    if url.contains(".m3u8") || url.contains("/playlist") {
        Ok(Box::new(
            HlsReader::new(url, local_addr, Some(cipher_manager), None, proxy).await?,
        ))
    } else if client_name == "TV" {
        Ok(Box::new(YoutubeReader::new(url, local_addr, proxy).await?))
    } else {
        Ok(Box::new(HttpReader::new(url, local_addr, proxy).await?))
    }
}
type AnyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub fn parse_playability_status(body: &serde_json::Value) -> Result<(), String> {
    let playability = body
        .get("playabilityStatus")
        .and_then(|p| p.get("status"))
        .and_then(|s| s.as_str())
        .unwrap_or("UNKNOWN");
    if playability == "OK" {
        return Ok(());
    }
    let p = body.get("playabilityStatus");
    let reason = p
        .and_then(|p| p.get("reason"))
        .and_then(|r| r.as_str())
        .unwrap_or("unknown reason");
    match playability {
        "ERROR" => Err(reason.to_string()),
        "UNPLAYABLE" => {
            if reason == "unknown reason" {
                Err("This video is unplayable.".to_string())
            } else {
                Err(reason.to_string())
            }
        }
        "LOGIN_REQUIRED" => {
            if reason.contains("This video is private") {
                Err("This is a private video.".to_string())
            } else if reason.contains("This video may be inappropriate for some users") {
                Err("This video requires age verification.".to_string())
            } else {
                Err("This video requires login.".to_string())
            }
        }
        "CONTENT_CHECK_REQUIRED" => Err(reason.to_string()),
        "LIVE_STREAM_OFFLINE" => {
            if let Some(err_screen) = p.and_then(|p| p.get("errorScreen"))
                && err_screen.get("ypcTrailerRenderer").is_some()
            {
                return Err("This trailer cannot be loaded.".to_string());
            }
            Err(reason.to_string())
        }
        _ => Err("This video cannot be viewed anonymously.".to_string()),
    }
}
}
pub mod ua {
pub mod yt_ua {
    pub const IOS: &str =
        "com.google.ios.youtube/21.02.1 (iPhone16,2; U; CPU iOS 18_2 like Mac OS X;)";
    pub const ANDROID: &str = "com.google.android.youtube/20.01.35 (Linux; U; Android 14) identity";
    pub const ANDROID_VR: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8 Pro Build/UQ1A.240205.002; wv) \
         AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 \
         Chrome/121.0.6167.164 Mobile Safari/537.36 YouTubeVR/1.42.15 (gzip)";
    pub const TVHTML5: &str = "Mozilla/5.0 (Fuchsia) AppleWebKit/537.36 (KHTML, like Gecko) \
         Chrome/140.0.0.0 Safari/537.36 CrKey/1.56.500000";
    pub const MWEB: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_2 like Mac OS X) \
         AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
    pub const WEB_EMBEDDED: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
    pub const TVHTML5_SIMPLY: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36";
    pub const TVHTML5_UNPLUGGED: &str = "Mozilla/5.0 (Linux armeabi-v7a; Android 7.1.2; Fire OS 6.0) Cobalt/22.lts.3.306369-gold (unlike Gecko) v8/8.8.278.8-jit gles Starboard/13, Amazon_ATV_mediatek8695_2019/NS6294 (Amazon, AFTMM, Wireless) com.amazon.firetv.youtube/22.3.r2.v66.0";
}
pub fn get_youtube_ua(url: &str) -> Option<&'static str> {
    if !(url.contains("googlevideo.com") || url.contains("youtube.com")) {
        return None;
    }
    extract_param(url, "c=").and_then(|client| match client {
        "IOS" => Some(yt_ua::IOS),
        "ANDROID" => Some(yt_ua::ANDROID),
        "ANDROID_VR" => Some(yt_ua::ANDROID_VR),
        "TVHTML5" => Some(yt_ua::TVHTML5),
        "MWEB" => Some(yt_ua::MWEB),
        "WEB_EMBEDDED_PLAYER" => Some(yt_ua::WEB_EMBEDDED),
        "TVHTML5_SIMPLY" => Some(yt_ua::TVHTML5_SIMPLY),
        "TVHTML5_UNPLUGGED" => Some(yt_ua::TVHTML5_UNPLUGGED),
        _ => None,
    })
}
fn extract_param<'a>(url: &'a str, key: &str) -> Option<&'a str> {
    let query_start = url.find('?')?;
    let query = &url[query_start + 1..];
    for part in query.split('&') {
        if let Some(val) = part.strip_prefix(key) {
            return Some(val.split('#').next().unwrap_or(val));
        }
    }
    None
}
}
pub mod oauth {
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::common::types::AnyResult;
const CLIENT_ID: &str = "861556708454-d6dlm3lh05idd8npek18k6be8ba3oc68.apps.googleusercontent.com";
const CLIENT_SECRET: &str = "SboVhoG9s0rNafixCSGGKXAT";
const SCOPES: &str = "http://gdata.youtube.com https://www.googleapis.com/auth/youtube";
pub struct YouTubeOAuth {
    refresh_tokens: RwLock<Vec<String>>,
    current_token_index: RwLock<usize>,
    access_token: RwLock<Option<String>>,
    token_expiry: RwLock<u64>,
    client: reqwest::Client,
}
impl YouTubeOAuth {
    pub fn new(refresh_tokens: Vec<String>) -> Self {
        Self {
            refresh_tokens: RwLock::new(refresh_tokens),
            current_token_index: RwLock::new(0),
            access_token: RwLock::new(None),
            token_expiry: RwLock::new(0),
            client: reqwest::Client::new(),
        }
    }
    pub async fn initialize_access_token(self: std::sync::Arc<Self>) {
        if !self.refresh_tokens.read().await.is_empty() {
            return;
        }
        match self.fetch_device_code().await {
            Ok(response) => {
                let verification_url = response["verification_url"].as_str().unwrap_or_default();
                let user_code = response["user_code"].as_str().unwrap_or_default();
                let device_code = response["device_code"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let interval = response["interval"].as_u64().unwrap_or(5);
                let inner_width = 60;
                let top_border = format!("  ┌{}┐", "─".repeat(inner_width));
                let sep_border = format!("  ├{}┤", "─".repeat(inner_width));
                let bot_border = format!("  └{}┘", "─".repeat(inner_width));
                let warning = "!!! USE A BURNER ACCOUNT FOR YOUTUBE OAUTH !!!";
                let warning_pad = inner_width.saturating_sub(warning.len());
                let warning_left = warning_pad / 2;
                let warning_right = warning_pad - warning_left;
                crate::log_println!("\n\x1b[1;33m{}\x1b[0m", top_border);
                crate::log_println!(
                    "\x1b[1;33m  │\x1b[0m{:left$}\x1b[1;31m{}\x1b[0m{:right$}\x1b[1;33m│\x1b[0m",
                    "",
                    warning,
                    "",
                    left = warning_left,
                    right = warning_right
                );
                crate::log_println!("\x1b[1;33m{}\x1b[0m", sep_border);
                let s1_prefix = " 1. Visit: ";
                let s1_padding =
                    inner_width.saturating_sub(s1_prefix.len() + verification_url.len());
                crate::log_println!(
                    "\x1b[1;33m  │\x1b[0m\x1b[1;36m{}\x1b[0m\x1b[4;34m{}\x1b[0m{:pad$}\x1b[1;33m│\x1b[0m",
                    s1_prefix,
                    verification_url,
                    "",
                    pad = s1_padding
                );
                let s2_prefix = " 2. Enter code: ";
                let s2_code = format!(" {} ", user_code);
                let s2_padding = inner_width.saturating_sub(s2_prefix.len() + s2_code.len());
                crate::log_println!(
                    "\x1b[1;33m  │\x1b[0m\x1b[1;36m{}\x1b[0m\x1b[1;42;30m{}\x1b[0m{:pad$}\x1b[1;33m│\x1b[0m",
                    s2_prefix,
                    s2_code,
                    "",
                    pad = s2_padding
                );
                crate::log_println!("\x1b[1;33m{}\x1b[0m\n", bot_border);
                let oauth = self.clone();
                tokio::spawn(async move {
                    oauth.poll_for_token(device_code, interval).await;
                });
            }
            Err(e) => {
                tracing::error!("Failed to fetch YouTube device code: {}", e);
            }
        }
    }
    async fn fetch_device_code(&self) -> AnyResult<Value> {
        let res = self
            .client
            .post("https://www.youtube.com/o/oauth2/device/code")
            .json(&json!({
                "client_id": CLIENT_ID,
                "scope": SCOPES,
                "device_id": Uuid::new_v4().to_string().replace("-", ""),
                "device_model": "ytlr::"
            }))
            .send()
            .await?;
        Ok(res.json().await?)
    }
    async fn poll_for_token(&self, device_code: String, interval: u64) {
        let mut interval_timer = tokio::time::interval(std::time::Duration::from_secs(interval));
        loop {
            interval_timer.tick().await;
            match self
                .fetch_refresh_token_from_device_code(&device_code)
                .await
            {
                Ok(response) => {
                    if let Some(error) = response["error"].as_str() {
                        match error {
                            "authorization_pending" => continue,
                            "slow_down" => {
                                interval_timer = tokio::time::interval(
                                    std::time::Duration::from_secs(interval + 5),
                                );
                                continue;
                            }
                            "expired_token" => {
                                tracing::error!(
                                    "OAUTH INTEGRATION: The device token has expired. OAuth integration has been canceled."
                                );
                                break;
                            }
                            "access_denied" => {
                                tracing::error!(
                                    "OAUTH INTEGRATION: Account linking was denied. OAuth integration has been canceled."
                                );
                                break;
                            }
                            _ => {
                                tracing::error!("Unhandled OAuth2 error: {}", error);
                                break;
                            }
                        }
                    }
                    if let Some(refresh_token) = response["refresh_token"].as_str() {
                        let mut tokens = self.refresh_tokens.write().await;
                        tokens.push(refresh_token.to_string());
                        crate::log_println!(
                            "\x1b[1;32mOAUTH INTEGRATION: Token retrieved successfully!\x1b[0m"
                        );
                        crate::log_println!("\x1b[1;32mRefresh token:\x1b[0m {}", refresh_token);
                        break;
                    }
                }
                Err(e) => {
                    crate::log_println!(
                        "\x1b[1;31mFailed to fetch YouTube OAuth2 token:\x1b[0m {}",
                        e
                    );
                    break;
                }
            }
        }
    }
    async fn fetch_refresh_token_from_device_code(&self, device_code: &str) -> AnyResult<Value> {
        let res = self
            .client
            .post("https://www.youtube.com/o/oauth2/token")
            .json(&json!({
                "client_id": CLIENT_ID,
                "client_secret": CLIENT_SECRET,
                "code": device_code,
                "grant_type": "http://oauth.net/grant_type/device/1.0"
            }))
            .send()
            .await?;
        Ok(res.json().await?)
    }
    pub async fn get_access_token(&self, idx: usize) -> Option<String> {
        let tokens = self.refresh_tokens.read().await;
        let max_tokens = tokens.len();
        if max_tokens == 0 {
            return None;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        {
            let expiry = self.token_expiry.read().await;
            let token = self.access_token.read().await;
            if let Some(t) = token.as_ref()
                && now < *expiry
            {
                return Some(t.clone());
            }
        }
        let refresh_token = &tokens[idx % max_tokens];
        if refresh_token.is_empty() {
            return None;
        }
        match self.refresh_token_request(refresh_token).await {
            Ok((new_token, expires_in)) => {
                let mut token_store = self.access_token.write().await;
                let mut expiry_store = self.token_expiry.write().await;
                *token_store = Some(new_token.clone());
                *expiry_store = now + expires_in - 30; 
                Some(new_token)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to refresh YouTube token for index {}: {}",
                    idx % max_tokens,
                    e
                );
                None
            }
        }
    }
    async fn refresh_token_request(&self, refresh_token: &str) -> AnyResult<(String, u64)> {
        let res = self
            .client
            .post("https://www.youtube.com/o/oauth2/token")
            .json(&json!({
                "client_id": CLIENT_ID,
                "client_secret": CLIENT_SECRET,
                "refresh_token": refresh_token,
                "grant_type": "refresh_token"
            }))
            .send()
            .await?;
        let status = res.status();
        if status == 200 {
            let body: Value = res.json().await?;
            if let Some(access_token) = body.get("access_token").and_then(|t| t.as_str()) {
                let expires_in = body
                    .get("expires_in")
                    .and_then(|e| e.as_u64())
                    .unwrap_or(3600);
                return Ok((access_token.to_string(), expires_in));
            }
        }
        Err(format!("OAuth refresh failed with status: {}", status).into())
    }
    pub async fn get_auth_header(&self) -> Option<String> {
        let tokens = self.refresh_tokens.read().await;
        if tokens.is_empty() {
            return None;
        }
        let num_tokens = tokens.len();
        let idx = {
            let mut current_idx = self.current_token_index.write().await;
            let val = *current_idx;
            *current_idx = (val + 1) % num_tokens;
            val
        };
        drop(tokens); 
        self.get_access_token(idx)
            .await
            .map(|t| format!("Bearer {}", t))
    }
    pub async fn get_refresh_tokens(&self) -> Vec<String> {
        self.refresh_tokens.read().await.clone()
    }
    pub async fn refresh_with_token(&self, refresh_token: &str) -> AnyResult<serde_json::Value> {
        let res = self
            .client
            .post("https://www.youtube.com/o/oauth2/token")
            .json(&json!({
                "client_id": CLIENT_ID,
                "client_secret": CLIENT_SECRET,
                "refresh_token": refresh_token,
                "grant_type": "refresh_token"
            }))
            .send()
            .await?;
        let status = res.status();
        let body: serde_json::Value = res.json().await?;
        if status.is_success() {
            Ok(body)
        } else {
            Err(format!("OAuth refresh failed: status={}, body={}", status, body).into())
        }
    }
}
}
pub mod cipher {
use std::time::{Duration, Instant};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use crate::{common::types::AnyResult, config::sources::YouTubeCipherConfig};
#[derive(Clone)]
pub struct CachedPlayerScript {
    pub url: String,
    pub signature_timestamp: String,
    pub expire_timestamp_ms: Instant,
}
pub struct YouTubeCipherManager {
    config: YouTubeCipherConfig,
    client: reqwest::Client,
    cached_player_script: RwLock<Option<CachedPlayerScript>>,
}
impl YouTubeCipherManager {
    pub fn new(config: YouTubeCipherConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            cached_player_script: RwLock::new(None),
        }
    }
    pub async fn get_cached_player_script(&self) -> AnyResult<CachedPlayerScript> {
        {
            let cache = self.cached_player_script.read().await;
            if let Some(script) = &*cache
                && Instant::now() < script.expire_timestamp_ms
            {
                return Ok(script.clone());
            }
        }
        let mut cache = self.cached_player_script.write().await;
        if let Some(script) = &*cache
            && Instant::now() < script.expire_timestamp_ms
        {
            return Ok(script.clone());
        }
        let script = self.get_player_script().await?;
        *cache = Some(script.clone());
        Ok(script)
    }
    async fn get_player_script(&self) -> AnyResult<CachedPlayerScript> {
        let res = self
            .client
            .get("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36")
            .send()
            .await?;
        let text = res.text().await?;
        let re = regex::Regex::new(r#""jsUrl":"([^"]+)""#)?;
        let mut script_url = if let Some(caps) = re.captures(&text) {
            caps[1].to_string()
        } else {
            let res = self
                .client
                .get("https://www.youtube.com/embed/")
                .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36")
                .send()
                .await?;
            let text = res.text().await?;
            if let Some(caps) = re.captures(&text) {
                caps[1].to_string()
            } else {
                return Err("Could not find jsUrl in player script".into());
            }
        };
        let locale_re = regex::Regex::new(r"/([a-z]{2}_[A-Z]{2})/")?;
        script_url = locale_re.replace(&script_url, "/en_US/").to_string();
        let full_url = if script_url.starts_with("http") {
            script_url
        } else {
            format!("https://www.youtube.com{}", script_url)
        };
        let signature_timestamp = self.get_timestamp(&full_url).await?;
        Ok(CachedPlayerScript {
            url: full_url,
            signature_timestamp,
            expire_timestamp_ms: Instant::now() + Duration::from_secs(12 * 60 * 60),
        })
    }
    pub async fn get_timestamp(&self, source_url: &str) -> AnyResult<String> {
        if let Some(url) = &self.config.url {
            let mut headers = reqwest::header::HeaderMap::new();
            if let Some(token) = &self.config.token {
                headers.insert(reqwest::header::AUTHORIZATION, token.parse()?);
            }
            if let Ok(res) = self
                .client
                .post(format!("{}/get_sts", url.trim_end_matches('/')))
                .headers(headers)
                .json(&json!({ "player_url": source_url }))
                .send()
                .await
            {
                if res.status() == 200 {
                    if let Ok(body) = res.json::<Value>().await {
                        if let Some(sts) = body.get("sts").and_then(|v| v.as_str()) {
                            return Ok(sts.to_string());
                        }
                    }
                }
            }
        }
        let res = self.client.get(source_url).send().await?;
        let text = res.text().await?;
        let re = regex::Regex::new(r#"(?:signatureTimestamp|sts):(\d+)"#)?;
        if let Some(caps) = re.captures(&text) {
            Ok(caps[1].to_string())
        } else {
            Err("Could not find STS in player script".into())
        }
    }
    pub async fn get_signature_timestamp(&self) -> AnyResult<u32> {
        let script = self.get_cached_player_script().await?;
        script
            .signature_timestamp
            .parse::<u32>()
            .map_err(|e| e.into())
    }
    pub async fn resolve_url(
        &self,
        stream_url: &str,
        player_url: &str, 
        n_param: Option<&str>,
        sig: Option<&str>,
    ) -> AnyResult<String> {
        let url = self
            .config
            .url
            .as_ref()
            .ok_or("Remote cipher URL not configured")?;
        let player_url = if let Ok(script) = self.get_cached_player_script().await {
            script.url
        } else {
            player_url.to_string()
        };
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(token) = &self.config.token {
            headers.insert(reqwest::header::AUTHORIZATION, token.parse()?);
        }
        let mut body = json!({
            "stream_url": stream_url,
            "player_url": player_url,
        });
        if let Some(n) = n_param {
            body["n_param"] = json!(n);
        }
        if let Some(s) = sig {
            body["encrypted_signature"] = json!(s);
            body["signature_key"] = json!("sig");
        }
        let res = self
            .client
            .post(format!("{}/resolve_url", url.trim_end_matches('/')))
            .headers(headers)
            .json(&body)
            .send()
            .await?;
        let status = res.status();
        if status == 200 {
            let body: Value = res.json().await?;
            if let Some(resolved) = body.get("resolved_url").and_then(|v| v.as_str()) {
                return Ok(resolved.to_string());
            }
            return Err("Resolved URL missing in response".into());
        }
        let err_body = res.text().await?;
        Err(format!("Failed to resolve URL with status {}: {}", status, err_body).into())
    }
}
}
pub mod reader {
use std::io::{Read, Seek, SeekFrom};
use symphonia::core::io::MediaSource;
use super::ua::get_youtube_ua;
use crate::{
    audio::source::{SegmentedSource, create_client},
    common::types::AnyResult,
};
pub struct YoutubeReader {
    inner: SegmentedSource,
}
impl YoutubeReader {
    pub async fn new(
        url: &str,
        local_addr: Option<std::net::IpAddr>,
        proxy: Option<crate::config::HttpProxyConfig>,
    ) -> AnyResult<Self> {
        let user_agent = get_youtube_ua(url)
            .map(str::to_string)
            .unwrap_or_else(crate::common::utils::default_user_agent);
        let client = create_client(user_agent, local_addr, proxy, None)?;
        let inner = SegmentedSource::new(client, url).await?;
        Ok(Self { inner })
    }
}
impl Read for YoutubeReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}
impl Seek for YoutubeReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}
impl MediaSource for YoutubeReader {
    fn is_seekable(&self) -> bool {
        self.inner.is_seekable()
    }
    fn byte_len(&self) -> Option<u64> {
        self.inner.byte_len()
    }
}
}
pub mod extractor {
use serde_json::Value;
use crate::protocol::tracks::{Track, TrackInfo};
pub fn extract_from_player(body: &Value, source_name: &str) -> Option<Track> {
    let details = body
        .get("videoDetails")
        .or_else(|| body.get("video_details"))?;
    let video_id = details
        .get("videoId")
        .or_else(|| details.get("video_id"))?
        .as_str()?;
    let title = details.get("title")?.as_str()?.to_string();
    let author = details.get("author")?.as_str()?.to_string();
    let is_stream = details
        .get("isLiveContent")
        .or_else(|| details.get("is_live_content"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let length_seconds = details
        .get("lengthSeconds")
        .or_else(|| details.get("length_seconds"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let artwork_url = details
        .get("thumbnail")
        .and_then(|t| t.get("thumbnails"))
        .and_then(|arr| arr.as_array())
        .and_then(|arr| arr.last())
        .and_then(|thumb| thumb.get("url"))
        .and_then(|url| url.as_str())
        .map(|s| s.to_string());
    let track = Track::new(TrackInfo {
        identifier: video_id.to_string(),
        is_seekable: !is_stream,
        author,
        length: if is_stream {
            9223372036854775807
        } else {
            length_seconds * 1000
        },
        is_stream,
        position: 0,
        title,
        uri: Some(format!("https://www.youtube.com/watch?v={}", video_id)),
        artwork_url,
        isrc: None,
        source_name: source_name.to_string(),
    });
    Some(track)
}
pub fn extract_from_next(body: &Value, source_name: &str) -> Option<(Vec<Track>, String)> {
    let contents_root = body.get("contents").and_then(|c| {
        c.get("singleColumnWatchNextResults")
            .or_else(|| c.get("singleColumnMusicWatchNextResultsRenderer"))
            .or_else(|| c.get("twoColumnWatchNextResults"))
    })?;
    let playlist_content = contents_root
        .get("playlist")
        .and_then(|p| p.get("playlist"))
        .and_then(|p| p.get("contents"))
        .and_then(|c| c.as_array())
        .or_else(|| {
            contents_root
                .get("tabbedRenderer")
                .and_then(|t| t.get("watchNextTabbedResultsRenderer"))
                .and_then(|w| w.get("tabs"))
                .and_then(|t| t.get(0))
                .and_then(|t| t.get("tabRenderer"))
                .and_then(|t| t.get("content"))
                .and_then(|c| c.get("musicQueueRenderer"))
                .and_then(|music_queue| {
                    music_queue
                        .get("content")
                        .and_then(|c| c.get("playlistPanelRenderer"))
                        .and_then(|p| p.get("contents"))
                        .or_else(|| music_queue.get("contents"))
                        .and_then(|c| c.as_array())
                })
        })?;
    if playlist_content.is_empty() {
        return None;
    }
    let mut tracks = Vec::new();
    for item in playlist_content {
        if let Some(track) = extract_track(item, source_name) {
            tracks.push(track);
        }
    }
    if tracks.is_empty() {
        return None;
    }
    let title = contents_root
        .get("tabbedRenderer")
        .and_then(|t| t.get("watchNextTabbedResultsRenderer"))
        .and_then(|t| t.get("tabs"))
        .and_then(|t| t.get(0))
        .and_then(|t| t.get("tabRenderer"))
        .and_then(|t| t.get("content"))
        .and_then(|c| c.get("musicQueueRenderer"))
        .and_then(|m| m.get("header"))
        .and_then(|h| h.get("musicQueueHeaderRenderer"))
        .and_then(|m| m.get("subtitle"))
        .and_then(get_text)
        .or_else(|| {
            contents_root
                .get("playlist")
                .and_then(|p| p.get("playlist"))
                .and_then(|p| p.get("title"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Unknown Playlist".to_string());
    Some((tracks, title))
}
pub fn extract_from_browse(body: &Value, source_name: &str) -> Option<(Vec<Track>, String)> {
    let title = body
        .get("header")
        .and_then(|h| {
            h.get("playlistHeaderRenderer")
                .or_else(|| h.get("musicAlbumReleaseHeaderRenderer"))
                .or_else(|| h.get("musicDetailHeaderRenderer"))
                .or_else(|| {
                    h.get("musicEditablePlaylistDetailHeaderRenderer")
                        .and_then(|m| m.get("header"))
                        .and_then(|h| h.get("musicDetailHeaderRenderer"))
                })
        })
        .and_then(|h| h.get("title"))
        .and_then(get_text)
        .unwrap_or_else(|| "Unknown Playlist".to_string());
    let mut tracks = Vec::new();
    if let Some(section_list) = find_section_list(body)
        && let Some(contents) = section_list.get("contents").and_then(|c| c.as_array())
    {
        for section in contents {
            if let Some(list) = section
                .get("itemSectionRenderer")
                .and_then(|i| i.get("contents"))
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("playlistVideoListRenderer"))
                .and_then(|p| p.get("contents"))
                .and_then(|c| c.as_array())
            {
                for item in list {
                    if let Some(track) = extract_track(item, source_name) {
                        tracks.push(track);
                    }
                }
            }
            if let Some(list) = section
                .get("musicShelfRenderer")
                .and_then(|s| s.get("contents"))
                .and_then(|c| c.as_array())
            {
                for item in list {
                    if let Some(track) = extract_track(item, source_name) {
                        tracks.push(track);
                    }
                }
            }
            if let Some(shelf) = section.get("musicPlaylistShelfRenderer")
                && let Some(list) = shelf.get("contents").and_then(|c| c.as_array())
            {
                for item in list {
                    if let Some(track) = extract_track(item, source_name) {
                        tracks.push(track);
                    }
                }
            }
        }
    }
    if tracks.is_empty()
        && let Some(contents) = body
            .get("contents")
            .and_then(|c| c.get("singleColumnBrowseResultsRenderer"))
            .and_then(|s| s.get("tabs"))
            .and_then(|t| t.as_array())
            .and_then(|t| t.first())
            .and_then(|t| t.get("tabRenderer"))
            .and_then(|t| t.get("content"))
            .and_then(|c| c.get("sectionListRenderer"))
            .and_then(|s| s.get("contents"))
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("musicPlaylistShelfRenderer"))
        && let Some(list) = contents.get("contents").and_then(|c| c.as_array())
    {
        for item in list {
            if let Some(track) = extract_track(item, source_name) {
                tracks.push(track);
            }
        }
    }
    if tracks.is_empty()
        && let Some(list) = find_music_playlist_shelf(body)
    {
        for item in list {
            if let Some(track) = extract_track(item, source_name) {
                tracks.push(track);
            }
        }
    }
    if tracks.is_empty()
        && let Some(continuation_contents) = body
            .get("onResponseReceivedActions")
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .and_then(|a| a.get("appendContinuationItemsAction"))
            .and_then(|a| a.get("continuationItems"))
            .and_then(|c| c.as_array())
    {
        for item in continuation_contents {
            if let Some(track) = extract_track(item, source_name) {
                tracks.push(track);
            }
        }
    }
    if tracks.is_empty() {
        return None;
    }
    Some((tracks, title))
}
fn find_music_playlist_shelf(value: &Value) -> Option<&Vec<Value>> {
    if let Some(shelf) = value.get("musicPlaylistShelfRenderer") {
        return shelf.get("contents").and_then(|c| c.as_array());
    }
    if let Some(obj) = value.as_object() {
        for (_, val) in obj {
            if let Some(list) = find_music_playlist_shelf(val) {
                return Some(list);
            }
        }
    }
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(list) = find_music_playlist_shelf(item) {
                return Some(list);
            }
        }
    }
    None
}
pub fn find_section_list(value: &Value) -> Option<&Value> {
    if let Some(list) = value.get("sectionListRenderer") {
        return Some(list);
    }
    if let Some(contents) = value.get("contents")
        && let Some(list) = find_section_list(contents)
    {
        return Some(list);
    }
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(list) = find_section_list(item) {
                return Some(list);
            }
        }
    }
    if let Some(tabs) = value.get("tabs").and_then(|t| t.as_array()) {
        for tab in tabs {
            if let Some(content) = tab.get("tabRenderer").and_then(|tr| tr.get("content"))
                && let Some(list) = find_section_list(content)
            {
                return Some(list);
            }
        }
    }
    if let Some(primary) = value
        .get("twoColumnSearchResultsRenderer")
        .and_then(|t| t.get("primaryContents"))
    {
        return find_section_list(primary);
    }
    None
}
pub fn extract_track(item: &Value, source_name: &str) -> Option<Track> {
    let renderer = item
        .get("videoRenderer")
        .or_else(|| item.get("compactVideoRenderer"))
        .or_else(|| item.get("playlistVideoRenderer"))
        .or_else(|| item.get("musicResponsiveListItemRenderer"))
        .or_else(|| item.get("musicTwoColumnItemRenderer"))
        .or_else(|| item.get("playlistPanelVideoRenderer"))
        .or_else(|| item.get("gridVideoRenderer"))?;
    let video_id = renderer
        .get("videoId")
        .and_then(|v| v.as_str())
        .or_else(|| {
            renderer
                .get("playlistItemData")
                .and_then(|d| d.get("videoId"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            renderer
                .get("doubleTapCommand")
                .and_then(|c| c.get("watchEndpoint"))
                .and_then(|w| w.get("videoId"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            renderer
                .get("navigationEndpoint")
                .and_then(|n| n.get("watchEndpoint"))
                .and_then(|w| w.get("videoId"))
                .and_then(|v| v.as_str())
        })?;
    let title = get_text(renderer.get("title").or_else(|| {
        renderer
            .get("flexColumns")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("musicResponsiveListItemFlexColumnRenderer"))
            .and_then(|r| r.get("text"))
    })?)
    .unwrap_or_else(|| "Unknown Title".to_string());
    let author = extract_author(renderer).unwrap_or_else(|| "Unknown Artist".to_string());
    let is_stream = renderer
        .get("isLive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || renderer
            .get("badges")
            .and_then(|b| b.as_array())
            .map(|arr| {
                arr.iter().any(|badge| {
                    badge
                        .get("metadataBadgeRenderer")
                        .and_then(|mbr| mbr.get("label"))
                        .and_then(|l| l.as_str())
                        .map(|s| s == "LIVE")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
    let length_ms = if is_stream {
        9223372036854775807
    } else {
        renderer
            .get("lengthText")
            .and_then(get_text)
            .map(|s| parse_duration(&s))
            .or_else(|| {
                renderer
                    .get("lengthSeconds")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<i64>().ok())
                    .map(|s| s * 1000)
            })
            .unwrap_or(0)
    };
    Some(Track::new(TrackInfo {
        identifier: video_id.to_string(),
        is_seekable: !is_stream,
        author,
        length: length_ms as u64,
        is_stream,
        position: 0,
        title,
        uri: Some(format!("https://www.youtube.com/watch?v={}", video_id)),
        artwork_url: get_thumbnail(renderer),
        isrc: None,
        source_name: source_name.to_string(),
    }))
}
fn extract_author(renderer: &Value) -> Option<String> {
    if let Some(subtitle) = renderer.get("subtitle")
        && let Some(text) = get_first_subtitle_run(subtitle)
    {
        let artist = text.split(" • ").next().unwrap_or(&text).trim();
        if !artist.is_empty() {
            return Some(artist.to_string());
        }
    }
    if let Some(author) = renderer
        .get("menu")
        .and_then(|m| m.get("menuRenderer"))
        .and_then(|m| m.get("title"))
        .and_then(|t| t.get("musicMenuTitleRenderer"))
        .and_then(|m| m.get("secondaryText"))
        .and_then(get_first_text)
    {
        return Some(author);
    }
    if let Some(text) = renderer.get("longBylineText").and_then(get_first_text) {
        return Some(text);
    }
    if let Some(text) = renderer.get("shortBylineText").and_then(get_first_text) {
        return Some(text);
    }
    if let Some(text) = renderer.get("ownerText").and_then(get_first_text) {
        return Some(text);
    }
    if let Some(flex) = renderer
        .get("flexColumns")
        .and_then(|c| c.get(1))
        .and_then(|c| c.get("musicResponsiveListItemFlexColumnRenderer"))
        .and_then(|r| r.get("text"))
        && let Some(runs) = flex.get("runs").and_then(|r| r.as_array())
        && let Some(text) = runs.first().and_then(|r| r.get("text")).and_then(|t| t.as_str())
    {
        return Some(text.to_string());
    }
    None
}
fn get_first_subtitle_run(subtitle: &Value) -> Option<String> {
    if let Some(runs) = subtitle.get("runs").and_then(|r| r.as_array()) {
        return runs
            .first()
            .and_then(|r| r.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
    }
    if let Some(simple_text) = subtitle.get("simpleText").and_then(|v| v.as_str()) {
        return Some(simple_text.to_string());
    }
    if let Some(s) = subtitle.as_str() {
        return Some(s.to_string());
    }
    None
}
fn get_text(obj: &Value) -> Option<String> {
    if let Some(s) = obj.as_str() {
        return Some(s.to_string());
    }
    if let Some(simple_text) = obj.get("simpleText").and_then(|v| v.as_str()) {
        return Some(simple_text.to_string());
    }
    if let Some(runs) = obj.get("runs").and_then(|v| v.as_array()) {
        let mut text = String::new();
        for run in runs {
            if let Some(t) = run.get("text").and_then(|v| v.as_str()) {
                text.push_str(t);
            }
        }
        return Some(text);
    }
    None
}
fn get_first_text(obj: &Value) -> Option<String> {
    if let Some(s) = obj.as_str() {
        return Some(s.to_string());
    }
    if let Some(simple_text) = obj.get("simpleText").and_then(|v| v.as_str()) {
        return Some(simple_text.to_string());
    }
    if let Some(runs) = obj.get("runs").and_then(|v| v.as_array()) {
        return runs
            .first()
            .and_then(|run| run.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
    }
    None
}
fn parse_duration(s: &str) -> i64 {
    let parts: Vec<&str> = s.split(':').collect();
    let mut seconds = 0;
    for part in parts {
        seconds = seconds * 60 + part.parse::<i64>().unwrap_or(0);
    }
    seconds * 1000
}
fn get_thumbnail(renderer: &Value) -> Option<String> {
    renderer
        .get("thumbnail")
        .and_then(|t| t.get("thumbnails"))
        .and_then(|arr| arr.as_array())
        .and_then(|arr| arr.last()) 
        .and_then(|thumb| thumb.get("url"))
        .and_then(|url| url.as_str())
        .map(|s| s.split('?').next().unwrap_or(s).to_string())
}
}
pub mod track {
use std::{net::IpAddr, sync::Arc};
use async_trait::async_trait;
use tracing::{debug, error, info, warn};
use crate::{
    config::HttpProxyConfig,
    sources::{
        playable_track::{PlayableTrack, ResolvedTrack},
        youtube::{
            cipher::YouTubeCipherManager,
            clients::YouTubeClient,
            oauth::YouTubeOAuth,
            utils::{create_reader, detect_audio_kind},
        },
    },
};
pub struct YoutubeTrack {
    pub identifier: String,
    pub clients: Vec<Arc<dyn YouTubeClient>>,
    pub oauth: Arc<YouTubeOAuth>,
    pub cipher_manager: Arc<YouTubeCipherManager>,
    pub visitor_data: Option<String>,
    pub local_addr: Option<IpAddr>,
    pub proxy: Option<HttpProxyConfig>,
}
#[async_trait]
impl PlayableTrack for YoutubeTrack {
    fn supports_seek(&self) -> bool {
        true
    }
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let context = serde_json::json!({ "visitorData": self.visitor_data });
        let mut last_error = String::from("No clients available");
        for client in &self.clients {
            let name = client.name().to_string();
            let url = match client
                .get_track_url(
                    &self.identifier,
                    &context,
                    self.cipher_manager.clone(),
                    self.oauth.clone(),
                )
                .await
            {
                Ok(Some(url)) => {
                    info!(
                        "YoutubeTrack: resolved '{}' using '{name}'",
                        self.identifier
                    );
                    url
                }
                Ok(None) => {
                    debug!(
                        "YoutubeTrack: client '{name}' returned no URL for '{}'",
                        self.identifier
                    );
                    continue;
                }
                Err(e) => {
                    warn!(
                        "YoutubeTrack: client '{name}' failed for '{}': {e}",
                        self.identifier
                    );
                    last_error = e.to_string();
                    continue;
                }
            };
            let is_hls = url.contains(".m3u8") || url.contains("/playlist");
            let hint = Some(detect_audio_kind(&url, is_hls));
            let proxy = self.proxy.clone();
            let local_addr = self.local_addr;
            let cipher = self.cipher_manager.clone();
            let url_clone = url.clone();
            let name_clone = name.clone();
            match create_reader(&url_clone, &name_clone, local_addr, proxy, cipher).await {
                Ok(reader) => return Ok(ResolvedTrack::new(reader, hint)),
                Err(e) => {
                    warn!("YoutubeTrack: reader failed for '{name}': {e} — trying next client");
                    last_error = e.to_string();
                    continue;
                }
            }
        }
        error!(
            "YoutubeTrack: all clients failed for '{}': {last_error}",
            self.identifier
        );
        Err(format!("All clients failed: {last_error}"))
    }
}
fn is_playability_error(msg: &str) -> bool {
    msg.contains("This video ")
        || msg.contains("This is a private video")
        || msg.contains("This trailer cannot be loaded")
}
}
use std::sync::Arc;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tracing::{debug, warn};
use crate::{
    common::types::SharedRw,
    config::sources::YouTubeConfig,
    protocol::tracks::*,
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
pub mod clients;
pub mod hls;
use cipher::YouTubeCipherManager;
use clients::{
    YouTubeClient, android::AndroidClient, android_vr::AndroidVrClient, ios::IosClient,
    music_android::MusicAndroidClient, mweb::MWebClient, tv::TvClient, tv_cast::TvCastClient,
    tv_embedded::TvEmbeddedClient, tv_simply::TvSimplyClient, tv_unplugged::TvUnpluggedClient,
    web::WebClient, web_embedded::WebEmbeddedClient, web_parent_tools::WebParentToolsClient,
    web_remix::WebRemixClient,
};
use oauth::YouTubeOAuth;
pub struct YouTubeSource {
    search_prefixes: Vec<String>,
    rec_prefixes: Vec<String>,
    url_regex: Regex,
    search_clients: Vec<Arc<dyn YouTubeClient>>,
    music_search_clients: Vec<Arc<dyn YouTubeClient>>,
    playback_clients: Vec<Arc<dyn YouTubeClient>>,
    resolve_clients: Vec<Arc<dyn YouTubeClient>>,
    oauth: Arc<YouTubeOAuth>,
    cipher_manager: Arc<YouTubeCipherManager>,
    visitor_data: SharedRw<Option<String>>,
    #[allow(dead_code)]
    http: Arc<reqwest::Client>,
}
pub struct YoutubeStreamContext {
    pub clients: Vec<Arc<dyn YouTubeClient>>,
    pub oauth: Arc<YouTubeOAuth>,
    pub cipher_manager: Arc<YouTubeCipherManager>,
    pub visitor_data: SharedRw<Option<String>>,
    pub http: Arc<reqwest::Client>,
}
impl YouTubeSource {
    pub fn new(config: Option<YouTubeConfig>, http: Arc<reqwest::Client>) -> Self {
        let config = config.unwrap_or_default();
        let oauth = Arc::new(YouTubeOAuth::new(config.refresh_tokens.clone()));
        let cipher_manager = Arc::new(YouTubeCipherManager::new(config.cipher.clone()));
        if config.get_oauth_token && config.refresh_tokens.is_empty() {
            let oauth_clone = oauth.clone();
            tokio::spawn(async move {
                oauth_clone.initialize_access_token().await;
            });
        }
        let cm_clone = cipher_manager.clone();
        tokio::spawn(async move {
            debug!("YouTubeSource: Warming cipher cache...");
            if let Err(e) = cm_clone.get_cached_player_script().await {
                warn!("YouTubeSource: Failed to warm cipher cache: {}", e);
            } else {
                debug!("YouTubeSource: Cipher cache warmed.");
            }
        });
        let visitor_data = Arc::new(RwLock::new(None));
        let vd_clone = visitor_data.clone();
        let http_clone = http.clone();
        tokio::spawn(async move {
            loop {
                if let Some(vd) = Self::refresh_visitor_data(&http_clone).await {
                    let mut lock = vd_clone.write().await;
                    *lock = Some(vd);
                    tracing::debug!("YouTube visitorData refreshed.");
                } else {
                    tracing::warn!("Failed to refresh YouTube visitorData.");
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
        let cipher_url = config.cipher.url.clone();
        let cipher_token = config.cipher.token.clone();
        let create_client = |name: &str| -> Option<Arc<dyn YouTubeClient>> {
            match name.to_uppercase().as_str() {
                "WEB" => Some(Arc::new(WebClient::with_cipher_url(
                    http.clone(),
                    cipher_url.clone(),
                    cipher_token.clone(),
                ))),
                "WEB_REMIX" | "REMIX" | "MUSIC_WEB" => {
                    Some(Arc::new(WebRemixClient::new(http.clone())))
                }
                "MWEB" => Some(Arc::new(MWebClient::new(http.clone()))),
                "ANDROID" => Some(Arc::new(AndroidClient::new(http.clone()))),
                "IOS" => Some(Arc::new(IosClient::new(http.clone()))),
                "TV" | "TVHTML5" => Some(Arc::new(TvClient::new(http.clone()))),
                "TV_CAST" | "TVHTML5_CAST" => Some(Arc::new(TvCastClient::new(http.clone()))),
                "TVHTML5_SIMPLY_EMBEDDED_PLAYER" | "TV_EMBEDDED" => {
                    Some(Arc::new(TvEmbeddedClient::new(http.clone())))
                }
                "TVHTML5_SIMPLY" | "TV_SIMPLY" => Some(Arc::new(TvSimplyClient::new(http.clone()))),
                "TVHTML5_UNPLUGGED" | "TV_UNPLUGGED" => {
                    Some(Arc::new(TvUnpluggedClient::new(http.clone())))
                }
                "MUSIC" | "MUSIC_ANDROID" | "ANDROID_MUSIC" => {
                    Some(Arc::new(MusicAndroidClient::new(http.clone())))
                }
                "ANDROID_VR" | "ANDROIDVR" => Some(Arc::new(AndroidVrClient::new(http.clone()))),
                "WEB_EMBEDDED" | "WEBEMBEDDED" => {
                    Some(Arc::new(WebEmbeddedClient::new(http.clone())))
                }
                "WEB_PARENT_TOOLS" | "WEBPARENTTOOLS" => {
                    Some(Arc::new(WebParentToolsClient::new(http.clone())))
                }
                _ => {
                    tracing::warn!("Unknown YouTube client: {}", name);
                    None
                }
            }
        };
        let mut search_clients = Vec::new();
        for name in &config.clients.search {
            if let Some(client) = create_client(name) {
                search_clients.push(client);
            }
        }
        if search_clients.is_empty() {
            tracing::warn!("No valid YouTube search clients configured! Fallback to Web.");
            search_clients.push(Arc::new(WebClient::new(http.clone())));
        }
        let search_client_names: Vec<String> = search_clients
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        tracing::debug!(
            "YouTube Search Clients initialized: {:?}",
            search_client_names
        );
        let mut playback_clients = Vec::new();
        for name in &config.clients.playback {
            if let Some(client) = create_client(name) {
                playback_clients.push(client);
            }
        }
        if playback_clients.is_empty() {
            tracing::warn!("No valid YouTube playback clients configured! Fallback to Web.");
            playback_clients.push(Arc::new(WebClient::new(http.clone())));
        }
        let mut resolve_clients = Vec::new();
        for name in &config.clients.resolve {
            if let Some(client) = create_client(name) {
                resolve_clients.push(client);
            }
        }
        if resolve_clients.is_empty() {
            tracing::warn!("No valid YouTube resolve clients configured! Fallback to Web.");
            resolve_clients.push(Arc::new(WebClient::new(http.clone())));
        }
        let music_search_clients: Vec<Arc<dyn YouTubeClient>> = vec![
            Arc::new(MusicAndroidClient::new(http.clone())),
            Arc::new(WebRemixClient::new(http.clone())),
        ];
        tracing::info!(
            "YouTube source initialized with {} search, {} playback, and {} resolve clients.",
            search_clients.len(),
            playback_clients.len(),
            resolve_clients.len()
        );
        Self {
            search_prefixes: vec!["ytsearch:".to_string(), "ytmsearch:".to_string()],
            rec_prefixes: vec!["ytrec:".to_string()],
            url_regex: Regex::new(r"(?:youtube\.com|youtu\.be)").unwrap(),
            search_clients,
            music_search_clients,
            playback_clients,
            resolve_clients,
            oauth,
            cipher_manager,
            visitor_data,
            http,
        }
    }
    pub fn stream_context(&self) -> Arc<YoutubeStreamContext> {
        Arc::new(YoutubeStreamContext {
            clients: self.playback_clients.clone(),
            oauth: self.oauth.clone(),
            cipher_manager: self.cipher_manager.clone(),
            visitor_data: self.visitor_data.clone(),
            http: self.http.clone(),
        })
    }
    async fn refresh_visitor_data(http: &reqwest::Client) -> Option<String> {
        match http
            .get("https://www.youtube.com/embed")
            .header("Cookie", "YSC=cz5kYp3ZuIE; VISITOR_INFO1_LIVE=U-0T5oUyzf8;")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36")
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => {
                if let Ok(text) = res.text().await {
                    let re = regex::Regex::new(r#""VISITOR_DATA":"([^"]+)""#).ok();
                    if let Some(vd) = re.as_ref().and_then(|r| r.captures(&text)).and_then(|c| c.get(1)) {
                        let raw = vd.as_str();
                        let decoded = urlencoding::decode(raw)
                            .map(|s| s.into_owned())
                            .unwrap_or_else(|_| raw.to_string());
                        tracing::debug!("YouTube: visitorData refreshed from embed page.");
                        return Some(decoded);
                    }
                }
            }
            Ok(res) => {
                tracing::warn!("YouTube embed page returned status {}; falling back to guide API.", res.status());
            }
            Err(e) => {
                tracing::warn!("YouTube embed page request failed: {}; falling back to guide API.", e);
            }
        }
        let body = json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": "2.20260114.01.00",
                    "hl": "en",
                    "gl": "US"
                }
            }
        });
        match http
            .post("https://www.youtube.com/youtubei/v1/guide")
            .json(&body)
            .send()
            .await
        {
            Ok(res) => {
                if let Ok(json) = res.json::<Value>().await
                    && let Some(vd) = json
                        .get("responseContext")
                        .and_then(|rc| rc.get("visitorData"))
                        .and_then(|vd| vd.as_str())
                {
                    let decoded = urlencoding::decode(vd)
                        .map(|s| s.into_owned())
                        .unwrap_or_else(|_| vd.to_string());
                    tracing::debug!("YouTube: visitorData refreshed via guide API fallback.");
                    return Some(decoded);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch visitor data via guide API: {}", e);
            }
        }
        None
    }
    fn extract_playlist_id(&self, identifier: &str) -> Option<String> {
        if identifier.contains("list=") {
            return Some(
                identifier
                    .split("list=")
                    .nth(1)
                    .unwrap_or(identifier)
                    .split('&')
                    .next()
                    .unwrap_or(identifier)
                    .to_string(),
            );
        }
        None
    }
    fn extract_id(&self, identifier: &str) -> String {
        if identifier.contains("v=") {
            identifier
                .split("v=")
                .nth(1)
                .unwrap_or(identifier)
                .split('&')
                .next()
                .unwrap_or(identifier)
                .to_string()
        } else if identifier.contains("youtu.be/") {
            identifier
                .split("youtu.be/")
                .nth(1)
                .unwrap_or(identifier)
                .split('?')
                .next()
                .unwrap_or(identifier)
                .to_string()
        } else if identifier.contains("/live/") {
            identifier
                .split("/live/")
                .nth(1)
                .unwrap_or(identifier)
                .split('?')
                .next()
                .unwrap_or(identifier)
                .to_string()
        } else if identifier.contains("/shorts/") {
            identifier
                .split("/shorts/")
                .nth(1)
                .unwrap_or(identifier)
                .split('?')
                .next()
                .unwrap_or(identifier)
                .to_string()
        } else {
            identifier.to_string()
        }
    }
    fn prioritize_clients<'a>(
        &'a self,
        clients: &'a [Arc<dyn YouTubeClient>],
        prefer_music: bool,
    ) -> Vec<&'a Arc<dyn YouTubeClient>> {
        let is_music =
            |c: &&Arc<dyn YouTubeClient>| c.name().contains("Music") || c.name().contains("Remix");
        let mut ordered = Vec::with_capacity(clients.len());
        if prefer_music {
            ordered.extend(clients.iter().filter(|c| is_music(c)));
            ordered.extend(clients.iter().filter(|c| !is_music(c)));
        } else {
            ordered.extend(clients.iter().filter(|c| !is_music(c)));
            ordered.extend(clients.iter().filter(|c| is_music(c)));
        }
        ordered
    }
    fn fallback_clients<'a>(
        &'a self,
        tried: &[&Arc<dyn YouTubeClient>],
        prefer_music: bool,
    ) -> Vec<&'a Arc<dyn YouTubeClient>> {
        let tried_names: std::collections::HashSet<&str> = tried.iter().map(|c| c.name()).collect();
        let all_pools: &[&[Arc<dyn YouTubeClient>]] = &[
            &self.resolve_clients,
            &self.playback_clients,
            &self.search_clients,
        ];
        let mut seen = tried_names.clone();
        let mut fallback: Vec<&Arc<dyn YouTubeClient>> = Vec::new();
        for pool in all_pools {
            for client in pool.iter() {
                if seen.insert(client.name()) {
                    fallback.push(client);
                }
            }
        }
        self.prioritize_clients_slice(&fallback, prefer_music)
    }
    fn prioritize_clients_slice<'a>(
        &self,
        clients: &[&'a Arc<dyn YouTubeClient>],
        prefer_music: bool,
    ) -> Vec<&'a Arc<dyn YouTubeClient>> {
        let is_music =
            |c: &&&Arc<dyn YouTubeClient>| c.name().contains("Music") || c.name().contains("Remix");
        let mut ordered = Vec::with_capacity(clients.len());
        if prefer_music {
            ordered.extend(clients.iter().filter(|c| is_music(c)).copied());
            ordered.extend(clients.iter().filter(|c| !is_music(c)).copied());
        } else {
            ordered.extend(clients.iter().filter(|c| !is_music(c)).copied());
            ordered.extend(clients.iter().filter(|c| is_music(c)).copied());
        }
        ordered
    }
    pub fn cipher_manager(&self) -> Arc<YouTubeCipherManager> {
        self.cipher_manager.clone()
    }
}
#[async_trait]
impl SourcePlugin for YouTubeSource {
    fn name(&self) -> &str {
        "youtube"
    }
    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes
            .iter()
            .any(|p| identifier.starts_with(p))
            || self.rec_prefixes.iter().any(|p| identifier.starts_with(p))
            || self.url_regex.is_match(identifier)
    }
    fn search_prefixes(&self) -> Vec<&str> {
        self.search_prefixes.iter().map(|s| s.as_str()).collect()
    }
    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        let visitor_data = self.visitor_data.read().await.clone();
        let context = if let Some(vd) = visitor_data {
            json!({ "visitorData": vd })
        } else {
            json!({})
        };
        if let Some(prefix) = self
            .search_prefixes
            .iter()
            .find(|p| identifier.starts_with(*p))
        {
            return self.handle_search(identifier, prefix, &context).await;
        }
        if let Some(prefix) = self
            .rec_prefixes
            .iter()
            .find(|p| identifier.starts_with(*p))
        {
            return self
                .handle_recommendations(identifier, prefix, &context)
                .await;
        }
        if self.url_regex.is_match(identifier) {
            return self.handle_url(identifier, &context).await;
        }
        LoadResult::Empty {}
    }
    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let visitor_data = self.visitor_data.read().await.clone();
        let id = self.extract_id(identifier);
        let is_music_url = identifier.contains("music.youtube.com");
        let clients_to_try = self.prioritize_clients(&self.playback_clients, is_music_url);
        let clients = clients_to_try.into_iter().cloned().collect();
        Some(Arc::new(track::YoutubeTrack {
            identifier: id,
            clients,
            oauth: self.oauth.clone(),
            cipher_manager: self.cipher_manager.clone(),
            visitor_data,
            local_addr: routeplanner.and_then(|rp| rp.get_address()),
            proxy: None,
        }))
    }
}
impl YouTubeSource {
    async fn handle_search(&self, identifier: &str, prefix: &str, context: &Value) -> LoadResult {
        let prefer_music = prefix == "ytmsearch:";
        let query = &identifier[prefix.len()..];
        let is_music =
            |c: &Arc<dyn YouTubeClient>| c.name().contains("Music") || c.name().contains("Remix");
        if prefer_music {
            let mut music_clients: Vec<&Arc<dyn YouTubeClient>> = Vec::new();
            let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
            tracing::debug!(
                "Search: prefer_music=true. search_clients count: {}",
                self.search_clients.len()
            );
            for c in &self.music_search_clients {
                if seen.insert(c.name()) {
                    music_clients.push(c);
                }
            }
            for pool in [
                &self.search_clients[..],
                &self.resolve_clients[..],
                &self.playback_clients[..],
            ] {
                for c in pool {
                    if is_music(c) && seen.insert(c.name()) {
                        music_clients.push(c);
                    }
                }
            }
            for client in &music_clients {
                if !client.can_handle_request(identifier) {
                    continue;
                }
                tracing::debug!("Searching '{}' with {}", query, client.name());
                match client.search(query, context, self.oauth.clone()).await {
                    Ok(tracks) if !tracks.is_empty() => return LoadResult::Search(tracks),
                    Ok(_) => continue,
                    Err(e) => tracing::warn!("Music search error with {}: {}", client.name(), e),
                }
            }
            tracing::debug!(
                "All music clients returned empty for '{}', falling back to regular search",
                query
            );
        }
        let primary: Vec<&Arc<dyn YouTubeClient>> = self
            .search_clients
            .iter()
            .filter(|c| !is_music(c))
            .collect();
        for client in &primary {
            if !client.can_handle_request(identifier) {
                continue;
            }
            tracing::debug!("Searching '{}' with {}", query, client.name());
            match client.search(query, context, self.oauth.clone()).await {
                Ok(tracks) if !tracks.is_empty() => return LoadResult::Search(tracks),
                Ok(_) => continue,
                Err(e) => tracing::warn!("Search error with {}: {}", client.name(), e),
            }
        }
        let mut seen_search: std::collections::HashSet<&str> =
            primary.iter().map(|c| c.name()).collect();
        let secondary_search: Vec<&Arc<dyn YouTubeClient>> = self
            .search_clients
            .iter()
            .filter(|c| is_music(c) && seen_search.insert(c.name()))
            .collect();
        for client in &secondary_search {
            if !client.can_handle_request(identifier) {
                continue;
            }
            tracing::debug!("Secondary search '{}' with {}", query, client.name());
            match client.search(query, context, self.oauth.clone()).await {
                Ok(tracks) if !tracks.is_empty() => return LoadResult::Search(tracks),
                Ok(_) => continue,
                Err(e) => tracing::warn!("Secondary search error with {}: {}", client.name(), e),
            }
        }
        let tried: Vec<&Arc<dyn YouTubeClient>> =
            primary.into_iter().chain(secondary_search).collect();
        let fallback = self.fallback_clients(&tried, false);
        if !fallback.is_empty() {
            tracing::debug!(
                "All search clients failed for '{}', trying fallback clients",
                query
            );
        }
        for client in fallback {
            if !client.can_handle_request(identifier) {
                continue;
            }
            tracing::debug!("Fallback search '{}' with {}", query, client.name());
            match client.search(query, context, self.oauth.clone()).await {
                Ok(tracks) if !tracks.is_empty() => return LoadResult::Search(tracks),
                Ok(_) => continue,
                Err(e) => tracing::warn!("Fallback search error with {}: {}", client.name(), e),
            }
        }
        LoadResult::Empty {}
    }
    async fn handle_recommendations(
        &self,
        identifier: &str,
        prefix: &str,
        context: &Value,
    ) -> LoadResult {
        let seed_id = &identifier[prefix.len()..];
        let playlist_id = format!("RD{}", seed_id);
        let clients = self.prioritize_clients(&self.resolve_clients, true);
        for client in clients {
            if !client.can_handle_request(&playlist_id) {
                continue;
            }
            match client
                .get_playlist(&playlist_id, context, self.oauth.clone())
                .await
            {
                Ok(Some((tracks, title))) => {
                    let filtered: Vec<Track> = tracks
                        .into_iter()
                        .filter(|t| t.info.identifier != seed_id)
                        .collect();
                    return LoadResult::Playlist(PlaylistData {
                        info: PlaylistInfo {
                            name: format!("Recommendations: {}", title),
                            selected_track: -1,
                        },
                        plugin_info: json!({
                          "type": "recommendations",
                          "totalTracks": filtered.len()
                        }),
                        tracks: filtered,
                    });
                }
                _ => continue,
            }
        }
        LoadResult::Empty {}
    }
    async fn handle_url(&self, identifier: &str, context: &Value) -> LoadResult {
        let is_music_url = identifier.contains("music.youtube.com");
        if let Some(playlist_id) = self.extract_playlist_id(identifier) {
            let mut playlist_clients: Vec<&Arc<dyn YouTubeClient>> = Vec::new();
            for c in self.prioritize_clients(&self.resolve_clients, is_music_url) {
                if !playlist_clients.iter().any(|x| x.name() == c.name()) {
                    playlist_clients.push(c);
                }
            }
            for c in self.fallback_clients(&playlist_clients, is_music_url) {
                if !playlist_clients.iter().any(|x| x.name() == c.name()) {
                    playlist_clients.push(c);
                }
            }
            for client in &playlist_clients {
                if !client.can_handle_request(identifier) {
                    continue;
                }
                tracing::debug!("Fetching playlist '{}' with {}", playlist_id, client.name());
                match client
                    .get_playlist(&playlist_id, context, self.oauth.clone())
                    .await
                {
                    Ok(Some((tracks, title))) => {
                        return LoadResult::Playlist(PlaylistData {
                            info: PlaylistInfo {
                                name: title,
                                selected_track: -1,
                            },
                            plugin_info: json!({
                                "type": "playlist",
                                "url": format!("https://www.youtube.com/playlist?list={}", playlist_id),
                                "artworkUrl": tracks.first().and_then(|t| t.info.artwork_url.clone()),
                                "totalTracks": tracks.len()
                            }),
                            tracks,
                        });
                    }
                    _ => continue,
                }
            }
        }
        let id = self.extract_id(identifier);
        let resolve_clients: Vec<&Arc<dyn YouTubeClient>> = self.resolve_clients.iter().collect();
        for client in &resolve_clients {
            if !client.can_handle_request(identifier) {
                continue;
            }
            tracing::debug!("Resolving track '{}' with {}", id, client.name());
            match client
                .get_track_info(&id, context, self.oauth.clone())
                .await
            {
                Ok(Some(mut track)) => {
                    if is_music_url {
                        track.info.uri = Some(format!("https://music.youtube.com/watch?v={}", id));
                    }
                    return LoadResult::Track(track);
                }
                Ok(None) => continue,
                Err(e) => tracing::warn!("Resolve error with {}: {}", client.name(), e),
            }
        }
        let fallback = self.fallback_clients(&resolve_clients, false);
        if !fallback.is_empty() {
            tracing::debug!(
                "All resolve clients failed for '{}', trying {} fallback client(s)",
                id,
                fallback.len()
            );
        }
        for client in fallback {
            if !client.can_handle_request(identifier) {
                continue;
            }
            tracing::debug!("Fallback resolve '{}' with {}", id, client.name());
            if let Ok(Some(mut track)) = client
                .get_track_info(&id, context, self.oauth.clone())
                .await
            {
                if is_music_url {
                    track.info.uri = Some(format!("https://music.youtube.com/watch?v={}", id));
                }
                return LoadResult::Track(track);
            }
        }
        LoadResult::Empty {}
    }
}