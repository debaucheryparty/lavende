pub mod api {
use std::sync::Arc;
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tracing::{debug, warn};
use super::validators::{is_invalid_album, is_invalid_playlist};
const CONFIG_URL: &str = "https://music.amazon.com/config.json";
const API_BASE: &str = "https://eu.mesk.skill.music.a2z.com/api";
pub const EP_TRACK_INFO: &str = "cosmicTrack/displayCatalogTrack";
pub const EP_ALBUM_INFO: &str = "showCatalogAlbum";
pub const EP_ARTIST_INFO: &str = "explore/v1/showCatalogArtist";
pub const EP_PLAYLIST_INFO: &str = "showCatalogPlaylist";
pub const EP_COMMUNITY_PLAYLIST_INFO: &str = "showLibraryPlaylist";
pub const EP_TRACKS_SEARCH: &str = "searchCatalogTracks";
const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36";
pub struct AmazonMusicClient {
    http: Arc<reqwest::Client>,
    cached_config: RwLock<Option<Value>>,
}
impl AmazonMusicClient {
    pub fn new(http: Arc<reqwest::Client>) -> Self {
        Self {
            http,
            cached_config: RwLock::new(None),
        }
    }
    pub async fn site_config(&self) -> Option<Value> {
        {
            let guard = self.cached_config.read().await;
            if guard.is_some() {
                return guard.clone();
            }
        }
        let resp = match self
            .http
            .get(CONFIG_URL)
            .header("accept", "*/*")
            .header("accept-language", "en-US,en;q=0.9")
            .header("origin", "https://music.amazon.com")
            .header("referer", "https://music.amazon.com/")
            .header("user-agent", USER_AGENT)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("Amazon Music: config fetch failed: {e}");
                return None;
            }
        };
        if !resp.status().is_success() {
            warn!("Amazon Music: config fetch HTTP {}", resp.status());
            return None;
        }
        let config: Value = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                debug!("Amazon Music: config JSON parse error: {e}");
                return None;
            }
        };
        *self.cached_config.write().await = Some(config.clone());
        Some(config)
    }
    pub fn build_amzn_headers(config: &Value, page_url: &str) -> Value {
        let access_token = config["accessToken"].as_str().unwrap_or("");
        let device_id = config["deviceId"].as_str().unwrap_or("");
        let session_id = config["sessionId"].as_str().unwrap_or("");
        let version = config["version"].as_str().unwrap_or("");
        let csrf_token = config["csrf"]["token"].as_str().unwrap_or("");
        let csrf_ts = config["csrf"]["ts"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| config["csrf"]["ts"].as_u64().map(|n| n.to_string()))
            .unwrap_or_default();
        let csrf_rnd = config["csrf"]["rnd"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| config["csrf"]["rnd"].as_u64().map(|n| n.to_string()))
            .unwrap_or_default();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis().to_string())
            .unwrap_or_default();
        let req_id = gen_request_id();
        json!({
            "x-amzn-authentication": serde_json::to_string(&json!({
                "interface": "ClientAuthenticationInterface.v1_0.ClientTokenElement",
                "accessToken": access_token
            })).unwrap_or_default(),
            "x-amzn-device-model": "WEBPLAYER",
            "x-amzn-device-width": "1920",
            "x-amzn-device-family": "WebPlayer",
            "x-amzn-device-id": device_id,
            "x-amzn-user-agent": USER_AGENT,
            "x-amzn-session-id": session_id,
            "x-amzn-device-height": "1080",
            "x-amzn-request-id": req_id,
            "x-amzn-device-language": "en_US",
            "x-amzn-currency-of-preference": "USD",
            "x-amzn-os-version": "1.0",
            "x-amzn-application-version": version,
            "x-amzn-device-time-zone": "Asia/Calcutta",
            "x-amzn-timestamp": ts,
            "x-amzn-csrf": serde_json::to_string(&json!({
                "interface": "CSRFInterface.v1_0.CSRFHeaderElement",
                "token": csrf_token,
                "timestamp": csrf_ts,
                "rndNonce": csrf_rnd
            })).unwrap_or_default(),
            "x-amzn-music-domain": "music.amazon.com",
            "x-amzn-referer": "music.amazon.com",
            "x-amzn-affiliate-tags": "",
            "x-amzn-ref-marker": "",
            "x-amzn-page-url": page_url,
            "x-amzn-weblab-id-overrides": "",
            "x-amzn-video-player-token": "",
            "x-amzn-feature-flags": "hd-supported,uhd-supported",
            "x-amzn-has-profile-id": "",
            "x-amzn-age-band": ""
        })
    }
    pub async fn post_endpoint(
        &self,
        path: &str,
        mut body: Value,
        page_url: &str,
    ) -> Option<Value> {
        let config = self.site_config().await?;
        let amzn_headers = Self::build_amzn_headers(&config, page_url);
        body["headers"] = Value::String(serde_json::to_string(&amzn_headers).unwrap_or_default());
        let url = format!("{API_BASE}/{path}");
        let resp = match self
            .http
            .post(&url)
            .header("authority", "eu.mesk.skill.music.a2z.com")
            .header("accept", "*/*")
            .header("accept-language", "en-US,en;q=0.9")
            .header("content-type", "text/plain;charset=UTF-8")
            .header("origin", "https://music.amazon.com")
            .header("referer", "https://music.amazon.com/")
            .header(
                "sec-ch-ua",
                "\"Chromium\";v=\"125\", \"Not.A/Brand\";v=\"24\"",
            )
            .header("sec-ch-ua-mobile", "?1")
            .header("sec-ch-ua-platform", "\"Android\"")
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "cross-site")
            .header("user-agent", USER_AGENT)
            .body(serde_json::to_string(&body).unwrap_or_default())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                debug!("Amazon Music: POST {path} failed: {e}");
                return None;
            }
        };
        if !resp.status().is_success() {
            warn!("Amazon Music: POST {path} HTTP {}", resp.status());
            *self.cached_config.write().await = None;
            return None;
        }
        match resp.json::<Value>().await {
            Ok(v) => Some(v),
            Err(e) => {
                debug!("Amazon Music: POST {path} JSON parse error: {e}");
                None
            }
        }
    }
    fn entity_body(id: &str) -> Value {
        json!({
            "id": id,
            "userHash": serde_json::to_string(&json!({"level": "LIBRARY_MEMBER"})).unwrap_or_default()
        })
    }
    pub async fn fetch_track(&self, id: &str) -> Option<Value> {
        let page = format!(
            "https://music.amazon.com/tracks/{}",
            urlencoding::encode(id)
        );
        self.post_endpoint(EP_TRACK_INFO, Self::entity_body(id), &page)
            .await
    }
    pub async fn fetch_album(&self, id: &str) -> Option<Value> {
        let page = format!(
            "https://music.amazon.com/albums/{}",
            urlencoding::encode(id)
        );
        self.post_endpoint(EP_ALBUM_INFO, Self::entity_body(id), &page)
            .await
    }
    pub async fn fetch_artist(&self, id: &str) -> Option<Value> {
        let page = format!(
            "https://music.amazon.com/artists/{}",
            urlencoding::encode(id)
        );
        self.post_endpoint(EP_ARTIST_INFO, Self::entity_body(id), &page)
            .await
    }
    pub async fn fetch_playlist(&self, id: &str) -> Option<Value> {
        let page = format!(
            "https://music.amazon.com/playlists/{}",
            urlencoding::encode(id)
        );
        self.post_endpoint(EP_PLAYLIST_INFO, Self::entity_body(id), &page)
            .await
    }
    pub async fn fetch_community_playlist(&self, id: &str) -> Option<Value> {
        let page = format!(
            "https://music.amazon.com/user-playlists/{}",
            urlencoding::encode(id)
        );
        self.post_endpoint(EP_COMMUNITY_PLAYLIST_INFO, Self::entity_body(id), &page)
            .await
    }
    pub async fn search_tracks(&self, query: &str) -> Option<Value> {
        let page = format!(
            "https://music.amazon.com/search/{}/songs",
            urlencoding::encode(query)
        );
        let body = json!({
            "keyword": query,
            "userHash": serde_json::to_string(&json!({"level": "LIBRARY_MEMBER"})).unwrap_or_default()
        });
        self.post_endpoint(EP_TRACKS_SEARCH, body, &page).await
    }
    pub async fn fetch_album_multi_region(
        &self,
        id: &str,
        domain_hint: Option<&str>,
    ) -> Option<Value> {
        let config = self.site_config().await?;
        super::region::fetch_multi_region(
            &self.http,
            id,
            EP_ALBUM_INFO,
            "albums",
            "Album",
            domain_hint,
            is_invalid_album,
            &config,
        )
        .await
    }
    pub async fn fetch_playlist_multi_region(
        &self,
        id: &str,
        domain_hint: Option<&str>,
    ) -> Option<Value> {
        let config = self.site_config().await?;
        super::region::fetch_multi_region(
            &self.http,
            id,
            EP_PLAYLIST_INFO,
            "playlists",
            "Playlist",
            domain_hint,
            is_invalid_playlist,
            &config,
        )
        .await
    }
}
pub fn gen_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(42);
    let mut x = seed ^ (seed << 13);
    x ^= x >> 17;
    x ^= x << 5;
    (0..13)
        .map(|i| {
            x ^= x.wrapping_add(i).wrapping_mul(1664525);
            b"0123456789abcdefghijklmnopqrstuvwxyz"[(x as usize) % 36] as char
        })
        .collect()
}
pub fn duration_str_to_ms(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    let parts: Vec<u64> = s.split(':').filter_map(|p| p.trim().parse().ok()).collect();
    let secs = match parts.as_slice() {
        [h, m, s] => h * 3600 + m * 60 + s,
        [first, second] => {
            if *first >= 60 {
                let h = first / 60;
                let m = first % 60;
                h * 3600 + m * 60 + second
            } else {
                first * 60 + second
            }
        }
        [n] => *n,
        _ => 0,
    };
    secs * 1000
}
pub fn clean_image_url(url: &str) -> String {
    if url.is_empty() {
        return url.to_owned();
    }
    if url.contains("_CLa%7C") {
        return url
            .replace("._AA", ".")
            .replace("_US354", "_US1000")
            .replace("CLa%7C354,354", "CLa%7C1000,1000")
            .replace("0,0,354,354", "0,0,1000,1000")
            .replace("0,0,177,177", "0,0,500,500")
            .replace("177,0,177,177", "500,0,500,500")
            .replace("0,177,177,177", "0,500,500,500")
            .replace("177,177,177,177", "500,500,500,500");
    }
    if let Some(i_pos) = url.find("/I/") {
        let after = &url[i_pos + 3..];
        if let Some(dot_pos) = after.rfind('.') {
            let ext = &after[dot_pos..];
            let id_end = after.find(&['.', '_', '?'][..]).unwrap_or(after.len());
            let id_part = &after[..id_end];
            return format!("{}/I/{}{}", &url[..i_pos], id_part, ext);
        }
    }
    url.to_owned()
}
pub fn clean_song_title(title: &str) -> String {
    if let Some(stripped) = title.trim().strip_prefix(|c: char| c.is_ascii_digit()) {
        let rest = stripped.trim_start_matches(|c: char| c.is_ascii_digit());
        if let Some(clean) = rest.strip_prefix(". ") {
            return clean.to_owned();
        }
    }
    title.to_owned()
}
pub fn normalize_artist(raw: &str) -> String {
    let parts: Vec<&str> = raw
        .split(['&', ','])
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .take(3)
        .collect();
    if parts.is_empty() {
        return raw.trim().to_owned();
    }
    parts.join(", ")
}
}
pub mod crypt {
use aes::Aes128;
use ctr::Ctr128BE;
use tracing::{debug, warn};
type Aes128Ctr = Ctr128BE<Aes128>;
pub struct CencDecryptor {
    key: [u8; 16],
}
impl CencDecryptor {
    pub fn from_hex(hex_key: &str) -> Result<Self, String> {
        let trimmed = hex_key.trim();
        if trimmed.len() != 32 {
            return Err(format!(
                "decryption key must be 32 hex chars, got {}",
                trimmed.len()
            ));
        }
        let mut key = [0u8; 16];
        hex::decode_to_slice(trimmed, &mut key).map_err(|e| format!("invalid hex key: {e}"))?;
        Ok(Self { key })
    }
    pub fn decrypt_buffer(&self, buf: &mut [u8]) -> Result<(), String> {
        patch_moov_headers(buf)?;
        if let Ok(fragments) = locate_fragments(buf) {
            for frag in fragments {
                let _ = self.decrypt_fragment(buf, &frag);
            }
        }
        Ok(())
    }
    fn decrypt_fragment(&self, buf: &mut [u8], frag: &FragmentInfo) -> Result<(), String> {
        let mdat_payload_start = frag.mdat_offset + 8;
        let mdat_payload_end = frag.mdat_offset + frag.mdat_size;
        if mdat_payload_end > buf.len() {
            return Err("mdat extends past buffer".into());
        }
        if frag.sample_ivs.is_empty() {
            debug!("fragment has no sample IVs, skipping decryption");
            return Ok(());
        }
        let mut cursor = mdat_payload_start;
        for (idx, sample) in frag.samples.iter().enumerate() {
            let iv_bytes = frag
                .sample_ivs
                .get(idx)
                .ok_or_else(|| format!("missing IV for sample {idx}"))?;
            let sample_end = cursor + sample.size;
            if sample_end > mdat_payload_end || sample_end > buf.len() {
                warn!("sample {} exceeds mdat boundary, stopping", idx);
                break;
            }
            let mut full_iv = [0u8; 16];
            let copy_len = iv_bytes.len().min(16);
            full_iv[..copy_len].copy_from_slice(&iv_bytes[..copy_len]);
            if !sample.subsamples.is_empty() {
                let mut pos = cursor;
                for sub in &sample.subsamples {
                    pos += sub.clear as usize;
                    let enc_len = sub.encrypted as usize;
                    if pos + enc_len > sample_end {
                        break;
                    }
                    self.ctr_decrypt(&full_iv, &mut buf[pos..pos + enc_len]);
                    pos += enc_len;
                }
            } else {
                self.ctr_decrypt(&full_iv, &mut buf[cursor..sample_end]);
            }
            cursor = sample_end;
        }
        Ok(())
    }
    fn ctr_decrypt(&self, iv: &[u8; 16], data: &mut [u8]) {
        self.ctr_decrypt_with_offset(iv, data, 0);
    }
    fn ctr_decrypt_with_offset(&self, iv: &[u8; 16], data: &mut [u8], offset: usize) {
        use ctr::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
        let mut cipher = Aes128Ctr::new(&self.key.into(), iv.into());
        if offset > 0 {
            cipher.seek(offset as u64);
        }
        cipher.apply_keystream(data);
    }
    pub fn ctr_decrypt_external(&self, iv_bytes: &[u8], data: &mut [u8], offset: usize) {
        let mut full_iv = [0u8; 16];
        let copy_len = iv_bytes.len().min(16);
        full_iv[..copy_len].copy_from_slice(&iv_bytes[..copy_len]);
        self.ctr_decrypt_with_offset(&full_iv, data, offset);
    }
}
pub fn parse_moof_external(moof: &[u8]) -> (Vec<SampleEntry>, u8) {
    parse_moof(moof)
}
pub fn extract_sample_ivs_external(moof: &[u8], iv_size_hint: u8, count: usize) -> Vec<Vec<u8>> {
    extract_sample_ivs(moof, iv_size_hint, count)
}
struct FragmentInfo {
    mdat_offset: usize,
    mdat_size: usize,
    samples: Vec<SampleEntry>,
    sample_ivs: Vec<Vec<u8>>,
}
#[derive(Debug, Clone)]
pub struct SampleEntry {
    pub size: usize,
    pub subsamples: Vec<SubsampleEntry>,
}
#[derive(Debug, Clone)]
pub struct SubsampleEntry {
    pub clear: u32,
    pub encrypted: u32,
}
fn locate_fragments(buf: &[u8]) -> Result<Vec<FragmentInfo>, String> {
    let mut frags = Vec::new();
    let mut pos = 0;
    while pos + 8 <= buf.len() {
        let box_size =
            u32::from_be_bytes([buf[pos], buf[pos + 1], buf[pos + 2], buf[pos + 3]]) as usize;
        let box_type = &buf[pos + 4..pos + 8];
        if box_size < 8 || pos + box_size > buf.len() {
            break;
        }
        if box_type == b"moof" {
            let moof_end = pos + box_size;
            let mdat_offset = moof_end;
            if mdat_offset + 8 > buf.len() {
                pos += box_size;
                continue;
            }
            let mdat_size_raw = u32::from_be_bytes([
                buf[mdat_offset],
                buf[mdat_offset + 1],
                buf[mdat_offset + 2],
                buf[mdat_offset + 3],
            ]) as usize;
            let mdat_type = &buf[mdat_offset + 4..mdat_offset + 8];
            if mdat_type != b"mdat" || mdat_size_raw < 8 {
                pos += box_size;
                continue;
            }
            let (samples, per_sample_iv_size) = parse_moof(&buf[pos..moof_end]);
            let ivs = extract_sample_ivs(&buf[pos..moof_end], per_sample_iv_size, samples.len());
            frags.push(FragmentInfo {
                mdat_offset,
                mdat_size: mdat_size_raw,
                samples,
                sample_ivs: ivs,
            });
            pos = mdat_offset + mdat_size_raw;
            continue;
        }
        pos += box_size;
    }
    Ok(frags)
}
fn parse_moof(moof: &[u8]) -> (Vec<SampleEntry>, u8) {
    let mut samples = Vec::new();
    let mut default_sample_size: u32 = 0;
    let mut per_sample_iv_size: u8 = 0;
    let mut pos = 8;
    while pos + 8 <= moof.len() {
        let sz =
            u32::from_be_bytes([moof[pos], moof[pos + 1], moof[pos + 2], moof[pos + 3]]) as usize;
        let typ = &moof[pos + 4..pos + 8];
        if sz < 8 || pos + sz > moof.len() {
            break;
        }
        if typ == b"traf" {
            let (s, dsz, iv_sz) = parse_traf(&moof[pos..pos + sz]);
            if !s.is_empty() {
                samples = s;
            }
            if dsz > 0 {
                default_sample_size = dsz;
            }
            if iv_sz > 0 {
                per_sample_iv_size = iv_sz;
            }
        }
        pos += sz;
    }
    if default_sample_size > 0 {
        for s in &mut samples {
            if s.size == 0 {
                s.size = default_sample_size as usize;
            }
        }
    }
    (samples, per_sample_iv_size)
}
fn parse_traf(traf: &[u8]) -> (Vec<SampleEntry>, u32, u8) {
    let mut samples = Vec::new();
    let mut default_sample_size: u32 = 0;
    let mut per_sample_iv_size: u8 = 0;
    let mut pos = 8;
    while pos + 8 <= traf.len() {
        let sz =
            u32::from_be_bytes([traf[pos], traf[pos + 1], traf[pos + 2], traf[pos + 3]]) as usize;
        let typ = &traf[pos + 4..pos + 8];
        if sz < 8 || pos + sz > traf.len() {
            break;
        }
        match typ {
            b"tfhd" => {
                default_sample_size = parse_tfhd_default_size(&traf[pos..pos + sz]);
            }
            b"trun" => {
                samples = parse_trun(&traf[pos..pos + sz]);
            }
            b"senc" => {
                let (ivs, subsubs) = parse_senc(&traf[pos..pos + sz], per_sample_iv_size);
                for (i, (iv_data, subs)) in ivs.iter().zip(subsubs.iter()).enumerate() {
                    if i < samples.len() && !subs.is_empty() {
                        samples[i].subsamples = subs.clone();
                    }
                    let _ = iv_data;
                }
            }
            b"sbgp" | b"sgpd" | b"saiz" | b"saio" => {}
            _ => {}
        }
        pos += sz;
    }
    if per_sample_iv_size == 0 {
        per_sample_iv_size = detect_iv_size_from_senc(traf, samples.len());
    }
    (samples, default_sample_size, per_sample_iv_size)
}
fn parse_tfhd_default_size(tfhd: &[u8]) -> u32 {
    if tfhd.len() < 12 {
        return 0;
    }
    let flags = u32::from_be_bytes([0, tfhd[9], tfhd[10], tfhd[11]]);
    let mut off = 12;
    off += 4;
    if flags & 0x01 != 0 {
        off += 8;
    }
    if flags & 0x02 != 0 {
        off += 4;
    }
    if flags & 0x08 != 0 {
        off += 4;
    }
    if flags & 0x10 != 0 && off + 4 <= tfhd.len() {
        return u32::from_be_bytes([tfhd[off], tfhd[off + 1], tfhd[off + 2], tfhd[off + 3]]);
    }
    0
}
fn parse_trun(trun: &[u8]) -> Vec<SampleEntry> {
    if trun.len() < 12 {
        return Vec::new();
    }
    let flags = u32::from_be_bytes([0, trun[9], trun[10], trun[11]]);
    let sample_count = u32::from_be_bytes([trun[12], trun[13], trun[14], trun[15]]) as usize;
    let mut off = 16;
    if flags & 0x01 != 0 {
        off += 4;
    }
    if flags & 0x04 != 0 {
        off += 4;
    }
    let has_duration = flags & 0x100 != 0;
    let has_size = flags & 0x200 != 0;
    let has_flags = flags & 0x400 != 0;
    let has_cts = flags & 0x800 != 0;
    let mut samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        if has_duration {
            off += 4;
        }
        let size = if has_size && off + 4 <= trun.len() {
            let s = u32::from_be_bytes([trun[off], trun[off + 1], trun[off + 2], trun[off + 3]]);
            off += 4;
            s as usize
        } else {
            if has_size {
                off += 4;
            }
            0
        };
        if has_flags {
            off += 4;
        }
        if has_cts {
            off += 4;
        }
        samples.push(SampleEntry {
            size,
            subsamples: Vec::new(),
        });
    }
    samples
}
fn parse_senc(senc: &[u8], iv_size_hint: u8) -> (Vec<Vec<u8>>, Vec<Vec<SubsampleEntry>>) {
    if senc.len() < 12 {
        return (Vec::new(), Vec::new());
    }
    let flags = u32::from_be_bytes([0, senc[9], senc[10], senc[11]]);
    let sample_count = u32::from_be_bytes([senc[12], senc[13], senc[14], senc[15]]) as usize;
    let has_subsamples = flags & 0x02 != 0;
    let iv_size = if iv_size_hint > 0 {
        iv_size_hint as usize
    } else {
        8
    };
    let mut off = 16;
    let mut ivs = Vec::with_capacity(sample_count);
    let mut all_subs = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        if off + iv_size > senc.len() {
            break;
        }
        ivs.push(senc[off..off + iv_size].to_vec());
        off += iv_size;
        let mut subs = Vec::new();
        if has_subsamples {
            if off + 2 > senc.len() {
                break;
            }
            let sub_count = u16::from_be_bytes([senc[off], senc[off + 1]]) as usize;
            off += 2;
            for _ in 0..sub_count {
                if off + 6 > senc.len() {
                    break;
                }
                let clear = u16::from_be_bytes([senc[off], senc[off + 1]]) as u32;
                let encrypted = u32::from_be_bytes([
                    senc[off + 2],
                    senc[off + 3],
                    senc[off + 4],
                    senc[off + 5],
                ]);
                off += 6;
                subs.push(SubsampleEntry { clear, encrypted });
            }
        }
        all_subs.push(subs);
    }
    (ivs, all_subs)
}
fn detect_iv_size_from_senc(traf: &[u8], sample_count: usize) -> u8 {
    if sample_count == 0 {
        return 8;
    }
    let mut pos = 8;
    while pos + 8 <= traf.len() {
        let sz =
            u32::from_be_bytes([traf[pos], traf[pos + 1], traf[pos + 2], traf[pos + 3]]) as usize;
        let typ = &traf[pos + 4..pos + 8];
        if sz < 8 || pos + sz > traf.len() {
            break;
        }
        if typ == b"senc" && sz >= 16 {
            let flags = u32::from_be_bytes([0, traf[pos + 9], traf[pos + 10], traf[pos + 11]]);
            let payload_after_header = sz - 16;
            let has_sub = flags & 0x02 != 0;
            if !has_sub && sample_count > 0 {
                let candidate = payload_after_header / sample_count;
                if candidate == 8 || candidate == 16 {
                    return candidate as u8;
                }
            }
        }
        pos += sz;
    }
    8
}
fn extract_sample_ivs(moof: &[u8], iv_size_hint: u8, sample_count: usize) -> Vec<Vec<u8>> {
    let mut pos = 8;
    while pos + 8 <= moof.len() {
        let sz =
            u32::from_be_bytes([moof[pos], moof[pos + 1], moof[pos + 2], moof[pos + 3]]) as usize;
        let typ = &moof[pos + 4..pos + 8];
        if sz < 8 || pos + sz > moof.len() {
            break;
        }
        if typ == b"traf" {
            return extract_senc_ivs(&moof[pos..pos + sz], iv_size_hint, sample_count);
        }
        pos += sz;
    }
    Vec::new()
}
fn extract_senc_ivs(traf: &[u8], iv_size_hint: u8, _sample_count: usize) -> Vec<Vec<u8>> {
    let mut pos = 8;
    while pos + 8 <= traf.len() {
        let sz =
            u32::from_be_bytes([traf[pos], traf[pos + 1], traf[pos + 2], traf[pos + 3]]) as usize;
        let typ = &traf[pos + 4..pos + 8];
        if sz < 8 || pos + sz > traf.len() {
            break;
        }
        if typ == b"senc" {
            let (ivs, _) = parse_senc(&traf[pos..pos + sz], iv_size_hint);
            return ivs;
        }
        pos += sz;
    }
    Vec::new()
}
pub fn patch_moov_headers(buf: &mut [u8]) -> Result<(), String> {
    let mut pos = 0;
    while pos + 8 <= buf.len() {
        let box_size =
            u32::from_be_bytes([buf[pos], buf[pos + 1], buf[pos + 2], buf[pos + 3]]) as usize;
        let box_type = &buf[pos + 4..pos + 8];
        if box_size < 8 || pos + box_size > buf.len() {
            break;
        }
        if box_type == b"moov" {
            patch_moov_box(&mut buf[pos..pos + box_size]);
            return Ok(());
        }
        pos += box_size;
    }
    Ok(())
}
fn patch_moov_box(moov: &mut [u8]) {
    let mut pos = 8;
    while pos + 8 <= moov.len() {
        let sz =
            u32::from_be_bytes([moov[pos], moov[pos + 1], moov[pos + 2], moov[pos + 3]]) as usize;
        if sz < 8 || pos + sz > moov.len() {
            break;
        }
        let typ = [moov[pos + 4], moov[pos + 5], moov[pos + 6], moov[pos + 7]];
        if &typ == b"trak" {
            patch_trak(&mut moov[pos..pos + sz]);
        }
        pos += sz;
    }
}
fn patch_trak(trak: &mut [u8]) {
    let mut pos = 8;
    while pos + 8 <= trak.len() {
        let sz =
            u32::from_be_bytes([trak[pos], trak[pos + 1], trak[pos + 2], trak[pos + 3]]) as usize;
        if sz < 8 || pos + sz > trak.len() {
            break;
        }
        let typ = [trak[pos + 4], trak[pos + 5], trak[pos + 6], trak[pos + 7]];
        if &typ == b"mdia" {
            patch_mdia(&mut trak[pos..pos + sz]);
        }
        pos += sz;
    }
}
fn patch_mdia(mdia: &mut [u8]) {
    let mut pos = 8;
    while pos + 8 <= mdia.len() {
        let sz =
            u32::from_be_bytes([mdia[pos], mdia[pos + 1], mdia[pos + 2], mdia[pos + 3]]) as usize;
        if sz < 8 || pos + sz > mdia.len() {
            break;
        }
        let typ = [mdia[pos + 4], mdia[pos + 5], mdia[pos + 6], mdia[pos + 7]];
        if &typ == b"minf" {
            patch_minf(&mut mdia[pos..pos + sz]);
        }
        pos += sz;
    }
}
fn patch_minf(minf: &mut [u8]) {
    let mut pos = 8;
    while pos + 8 <= minf.len() {
        let sz =
            u32::from_be_bytes([minf[pos], minf[pos + 1], minf[pos + 2], minf[pos + 3]]) as usize;
        if sz < 8 || pos + sz > minf.len() {
            break;
        }
        let typ = [minf[pos + 4], minf[pos + 5], minf[pos + 6], minf[pos + 7]];
        if &typ == b"stbl" {
            patch_stbl(&mut minf[pos..pos + sz]);
        }
        pos += sz;
    }
}
fn patch_stbl(stbl: &mut [u8]) {
    let mut pos = 8;
    while pos + 8 <= stbl.len() {
        let sz =
            u32::from_be_bytes([stbl[pos], stbl[pos + 1], stbl[pos + 2], stbl[pos + 3]]) as usize;
        if sz < 8 || pos + sz > stbl.len() {
            break;
        }
        let typ = [stbl[pos + 4], stbl[pos + 5], stbl[pos + 6], stbl[pos + 7]];
        if &typ == b"stsd" {
            patch_stsd(&mut stbl[pos..pos + sz]);
        }
        pos += sz;
    }
}
fn patch_stsd(stsd: &mut [u8]) {
    if stsd.len() < 16 {
        return;
    }
    stsd[12] = 0;
    stsd[13] = 0;
    stsd[14] = 0;
    stsd[15] = 1;
    let mut entry_pos = 16;
    while entry_pos + 8 <= stsd.len() {
        let entry_sz = u32::from_be_bytes([
            stsd[entry_pos],
            stsd[entry_pos + 1],
            stsd[entry_pos + 2],
            stsd[entry_pos + 3],
        ]) as usize;
        if entry_sz < 8 || entry_pos + entry_sz > stsd.len() {
            break;
        }
        let codec_tag = [
            stsd[entry_pos + 4],
            stsd[entry_pos + 5],
            stsd[entry_pos + 6],
            stsd[entry_pos + 7],
        ];
        if &codec_tag == b"enca" {
            let original = find_original_codec(&stsd[entry_pos..entry_pos + entry_sz]);
            let replacement = original.unwrap_or(*b"mp4a");
            debug!(
                "patching stsd entry: enca -> {}",
                std::str::from_utf8(&replacement).unwrap_or("????")
            );
            stsd[entry_pos + 4] = replacement[0];
            stsd[entry_pos + 5] = replacement[1];
            stsd[entry_pos + 6] = replacement[2];
            stsd[entry_pos + 7] = replacement[3];
            neutralize_sinf(&mut stsd[entry_pos..entry_pos + entry_sz]);
        }
        entry_pos += entry_sz;
    }
}
fn find_original_codec(entry: &[u8]) -> Option<[u8; 4]> {
    let mut pos = 36;
    while pos + 8 <= entry.len() {
        let sz = u32::from_be_bytes([entry[pos], entry[pos + 1], entry[pos + 2], entry[pos + 3]])
            as usize;
        let typ = &entry[pos + 4..pos + 8];
        if sz < 8 || pos + sz > entry.len() {
            break;
        }
        if typ == b"sinf" {
            return find_frma(&entry[pos..pos + sz]);
        }
        pos += sz;
    }
    None
}
fn find_frma(sinf: &[u8]) -> Option<[u8; 4]> {
    let mut pos = 8;
    while pos + 8 <= sinf.len() {
        let sz =
            u32::from_be_bytes([sinf[pos], sinf[pos + 1], sinf[pos + 2], sinf[pos + 3]]) as usize;
        let typ = &sinf[pos + 4..sinf.len().min(pos + 8)];
        if sz < 8 || pos + sz > sinf.len() {
            break;
        }
        if typ == b"frma" && sz >= 12 {
            return Some([sinf[pos + 8], sinf[pos + 9], sinf[pos + 10], sinf[pos + 11]]);
        }
        pos += sz;
    }
    None
}
fn neutralize_sinf(entry: &mut [u8]) {
    let mut pos = 36;
    while pos + 8 <= entry.len() {
        let sz = u32::from_be_bytes([entry[pos], entry[pos + 1], entry[pos + 2], entry[pos + 3]])
            as usize;
        if sz < 8 || pos + sz > entry.len() {
            break;
        }
        if &entry[pos + 4..pos + 8] == b"sinf" {
            entry[pos + 4] = b'f';
            entry[pos + 5] = b'r';
            entry[pos + 6] = b'e';
            entry[pos + 7] = b'e';
            debug!("neutralized sinf box at offset {pos}");
        }
        pos += sz;
    }
}
pub fn extract_flac_stream_header(moov: &[u8]) -> Option<Vec<u8>> {
    let dfla = find_dfla_in_moov(moov)?;
    if dfla.len() < 12 {
        return None;
    }
    let metadata_blocks = &dfla[12..];
    if metadata_blocks.is_empty() {
        return None;
    }
    if metadata_blocks[0] & 0x7F != 0 {
        warn!(
            "Amazon FLAC: unexpected first metadata block type {}",
            metadata_blocks[0] & 0x7F
        );
        return None;
    }
    let mut out = Vec::with_capacity(4 + metadata_blocks.len());
    out.extend_from_slice(b"fLaC");
    out.extend_from_slice(metadata_blocks);
    ensure_last_metadata_block_flag(&mut out[4..]);
    Some(out)
}
fn ensure_last_metadata_block_flag(metadata_blocks: &mut [u8]) {
    let mut block_starts: Vec<usize> = Vec::new();
    let mut pos = 0;
    while pos + 4 <= metadata_blocks.len() {
        let length = u32::from_be_bytes([
            0,
            metadata_blocks[pos + 1],
            metadata_blocks[pos + 2],
            metadata_blocks[pos + 3],
        ]) as usize;
        if pos + 4 + length > metadata_blocks.len() {
            break;
        }
        block_starts.push(pos);
        pos += 4 + length;
    }
    if block_starts.is_empty() {
        return;
    }
    for &start in &block_starts {
        metadata_blocks[start] &= 0x7F;
    }
    let last = *block_starts.last().unwrap();
    metadata_blocks[last] |= 0x80;
}
fn find_dfla_in_moov(moov: &[u8]) -> Option<&[u8]> {
    let trak = find_child(moov, b"trak")?;
    let mdia = find_child(trak, b"mdia")?;
    let minf = find_child(mdia, b"minf")?;
    let stbl = find_child(minf, b"stbl")?;
    let stsd = find_child(stbl, b"stsd")?;
    if stsd.len() < 16 {
        return None;
    }
    let audio_entry = {
        let entries = &stsd[16..];
        let mut pos = 0;
        loop {
            if pos + 8 > entries.len() {
                return None;
            }
            let sz = u32::from_be_bytes([
                entries[pos],
                entries[pos + 1],
                entries[pos + 2],
                entries[pos + 3],
            ]) as usize;
            if sz < 8 || pos + sz > entries.len() {
                return None;
            }
            let tag = &entries[pos + 4..pos + 8];
            if tag == b"enca" || tag == b"fLaC" {
                break &entries[pos..pos + sz];
            }
            pos += sz;
        }
    };
    if audio_entry.len() < 36 {
        return None;
    }
    find_box_in(&audio_entry[36..], b"dfLa")
}
fn find_child<'a>(parent: &'a [u8], target: &[u8; 4]) -> Option<&'a [u8]> {
    if parent.len() < 8 {
        return None;
    }
    find_box_in(&parent[8..], target)
}
fn find_box_in<'a>(data: &'a [u8], target: &[u8; 4]) -> Option<&'a [u8]> {
    let mut pos = 0;
    while pos + 8 <= data.len() {
        let sz =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        if sz < 8 || pos + sz > data.len() {
            break;
        }
        if &data[pos + 4..pos + 8] == target {
            return Some(&data[pos..pos + sz]);
        }
        pos += sz;
    }
    None
}
}
pub mod direct {
use std::net::IpAddr;
use async_trait::async_trait;
use tracing::debug;
use super::streaming_reader::AmazonStreamingReader;
use crate::{
    audio::source::create_client,
    config::HttpProxyConfig,
    sources::playable_track::{PlayableTrack, ResolvedTrack},
};
pub const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";
pub struct AmazonMusicTrack {
    pub track_id: String,
    pub stream_url: String,
    pub decryption_key: String,
    pub local_addr: Option<IpAddr>,
    pub proxy: Option<HttpProxyConfig>,
}
#[async_trait]
impl PlayableTrack for AmazonMusicTrack {
    fn supports_seek(&self) -> bool {
        true
    }
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        debug!(
            "Amazon Music: opening streaming reader for {}",
            self.track_id
        );
        let stream_client = create_client(UA.to_owned(), self.local_addr, self.proxy.clone(), None)
            .map_err(|e| format!("failed to create client: {e}"))?;
        let content_len = probe_content_length(&stream_client, &self.stream_url)
            .await
            .map_err(|e| format!("failed to probe stream length: {e}"))?;
        let streaming_reader = AmazonStreamingReader::new(
            stream_client,
            &self.stream_url,
            &self.decryption_key,
            content_len,
        )
        .map_err(|e| format!("failed to initialize streaming reader: {e}"))?;
        let reader = Box::new(streaming_reader) as Box<dyn symphonia::core::io::MediaSource>;
        Ok(ResolvedTrack::new(reader, None))
    }
}
async fn probe_content_length(client: &reqwest::Client, url: &str) -> Result<u64, String> {
    let head = client
        .head(url)
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| format!("HEAD request failed: {e}"))?;
    if let Some(len) = head.content_length() {
        return Ok(len);
    }
    let range_resp = client
        .get(url)
        .header("User-Agent", UA)
        .header("Range", "bytes=0-0")
        .send()
        .await
        .map_err(|e| format!("range probe failed: {e}"))?;
    range_resp
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('/').next_back())
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| "could not determine stream length".to_string())
}
}
pub mod manager {
use std::{net::IpAddr, sync::Arc};
use async_trait::async_trait;
use regex::Regex;
use serde_json::json;
use tracing::debug;
use super::{
    api::AmazonMusicClient,
    direct::AmazonMusicTrack,
    parsers::{
        parse_album_tracks, parse_artist_top_songs, parse_community_playlist_tracks,
        parse_playlist_tracks, parse_search_tracks, parse_track,
    },
    validators::{
        is_invalid_album, is_invalid_artist, is_invalid_community_playlist, is_invalid_playlist,
        is_invalid_track,
    },
};
use crate::{
    config::AmazonMusicConfig,
    protocol::tracks::{LoadError, LoadResult, PlaylistData, PlaylistInfo, Track},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
#[derive(serde::Deserialize)]
struct StreamApiResponse {
    #[serde(rename = "streamUrl")]
    stream_url: String,
    #[serde(rename = "decryptionKey")]
    decryption_key: String,
}
const TRACK_RE: &str = r"(?i)^https?://(?:www\.)?music\.amazon\.[a-z.]+/tracks/([A-Z0-9]{10,20})";
const ALBUM_RE: &str = r"(?i)^https?://(?:www\.)?music\.amazon\.[a-z.]+/albums/([A-Z0-9]{10,20})";
const ARTIST_RE: &str = r"(?i)^https?://(?:www\.)?music\.amazon\.[a-z.]+/artists/([A-Z0-9]{10,20})";
const PLAYLIST_RE: &str =
    r"(?i)^https?://(?:www\.)?music\.amazon\.[a-z.]+/playlists/([A-Z0-9]{10,20})";
const USER_PLAYLIST_RE: &str =
    r"(?i)^https?://(?:www\.)?music\.amazon\.[a-z.]+/user-playlists/([a-zA-Z0-9]+)";
const DOMAIN_RE: &str = r"(?i)^https?://(?:www\.)?music\.amazon\.";
pub struct AmazonMusicSource {
    client: Arc<AmazonMusicClient>,
    http: Arc<reqwest::Client>,
    search_limit: usize,
    proxy: Option<crate::config::HttpProxyConfig>,
    api_url: Option<String>,
    local_addr: Option<IpAddr>,
    track_re: Regex,
    album_re: Regex,
    artist_re: Regex,
    playlist_re: Regex,
    user_playlist_re: Regex,
    domain_re: Regex,
}
impl AmazonMusicSource {
    pub fn new(config: AmazonMusicConfig, http: Arc<reqwest::Client>) -> Result<Self, String> {
        Ok(Self {
            client: Arc::new(AmazonMusicClient::new(Arc::clone(&http))),
            http,
            search_limit: config.search_limit.min(5),
            proxy: config.proxy,
            api_url: config.api_url,
            local_addr: None,
            track_re: Regex::new(TRACK_RE).map_err(|e| e.to_string())?,
            album_re: Regex::new(ALBUM_RE).map_err(|e| e.to_string())?,
            artist_re: Regex::new(ARTIST_RE).map_err(|e| e.to_string())?,
            playlist_re: Regex::new(PLAYLIST_RE).map_err(|e| e.to_string())?,
            user_playlist_re: Regex::new(USER_PLAYLIST_RE).map_err(|e| e.to_string())?,
            domain_re: Regex::new(DOMAIN_RE).map_err(|e| e.to_string())?,
        })
    }
    fn capture_id(&self, re: &Regex, url: &str) -> Option<String> {
        re.captures(url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }
    async fn load_track(&self, url: &str) -> LoadResult {
        let id = match self.capture_id(&self.track_re, url) {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let resp = match self.client.fetch_track(&id).await {
            Some(v) => v,
            None => {
                return LoadResult::Error(LoadError {
                    message: Some(format!("Amazon Music: failed to fetch track '{id}'")),
                    severity: crate::common::Severity::Suspicious,
                    cause: "API request failed".to_string(),
                    cause_stack_trace: None,
                });
            }
        };
        if is_invalid_track(&resp) {
            debug!("Amazon Music: track '{id}' not found or no longer available");
            return LoadResult::Empty {};
        }
        match parse_track(&resp, &id) {
            Some(info) => LoadResult::Track(Track::new(info)),
            None => {
                debug!("Amazon Music: failed to parse track '{id}'");
                LoadResult::Empty {}
            }
        }
    }
    async fn load_album(&self, url: &str) -> LoadResult {
        let album_id = match self.capture_id(&self.album_re, url) {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let domain_hint = super::region::extract_domain(url);
        let resp = match self
            .client
            .fetch_album_multi_region(&album_id, domain_hint.as_deref())
            .await
        {
            Some(v) => v,
            None => return LoadResult::Empty {},
        };
        if is_invalid_album(&resp) {
            debug!("Amazon Music: album '{album_id}' not found");
            return LoadResult::Empty {};
        }
        let (album_name, artist_name, track_infos) = match parse_album_tracks(&resp, &album_id) {
            Some(r) => r,
            None => return LoadResult::Empty {},
        };
        let artwork = resp["methods"][0]["template"]["headerImage"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(super::api::clean_image_url);
        let tracks: Vec<Track> = track_infos.into_iter().map(Track::new).collect();
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: album_name.clone(),
                selected_track: -1,
            },
            plugin_info: json!({
                "type": "album",
                "url": format!("https://music.amazon.com/albums/{album_id}"),
                "artworkUrl": artwork,
                "author": artist_name,
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }
    async fn load_artist(&self, url: &str) -> LoadResult {
        let artist_id = match self.capture_id(&self.artist_re, url) {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let resp = match self.client.fetch_artist(&artist_id).await {
            Some(v) => v,
            None => return LoadResult::Empty {},
        };
        if is_invalid_artist(&resp) {
            debug!("Amazon Music: artist '{artist_id}' not found");
            return LoadResult::Empty {};
        }
        let unique_album_ids: Vec<String> = {
            let widgets = match resp["methods"][0]["template"]["widgets"].as_array() {
                Some(w) => w,
                None => return LoadResult::Empty {},
            };
            let top_songs = match widgets.iter().find(|w| {
                w["header"]
                    .as_str()
                    .map(|h| h.to_lowercase().contains("top songs"))
                    .unwrap_or(false)
            }) {
                Some(w) => w,
                None => return LoadResult::Empty {},
            };
            let mut seen = std::collections::HashSet::new();
            top_songs["items"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            let key = item["iconButton"]["observer"]["storageKey"].as_str()?;
                            let album_id = key.split(':').next()?.to_string();
                            if !album_id.is_empty() && seen.insert(album_id.clone()) {
                                Some(album_id)
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let fetch_futures: Vec<_> = unique_album_ids
            .iter()
            .map(|album_id| self.client.fetch_album(album_id))
            .collect();
        let album_responses = futures::future::join_all(fetch_futures).await;
        let mut duration_map: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for (album_id, album_resp) in unique_album_ids.iter().zip(album_responses) {
            let album_resp = match album_resp {
                Some(v) => v,
                None => continue,
            };
            let album_items =
                match album_resp["methods"][0]["template"]["widgets"][0]["items"].as_array() {
                    Some(i) => i.clone(),
                    None => continue,
                };
            for track in &album_items {
                let deeplink = match track["primaryTextLink"]["deeplink"].as_str() {
                    Some(dl) => dl,
                    None => continue,
                };
                let track_id = match deeplink.split("/tracks/").nth(1) {
                    Some(id) => id.split('/').next().unwrap_or("").to_string(),
                    None => continue,
                };
                if track_id.is_empty() {
                    continue;
                }
                let duration_ms =
                    super::api::duration_str_to_ms(track["secondaryText3"].as_str().unwrap_or(""));
                duration_map.insert(format!("{album_id}:{track_id}"), duration_ms);
            }
        }
        let result = match parse_artist_top_songs(&resp, &artist_id, &duration_map) {
            Some(r) => r,
            None => return LoadResult::Empty {},
        };
        let tracks: Vec<Track> = result.tracks.into_iter().map(Track::new).collect();
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: format!("{}'s Top Songs", result.name),
                selected_track: -1,
            },
            plugin_info: json!({
                "type": "artist",
                "url": format!("https://music.amazon.com/artists/{artist_id}"),
                "artworkUrl": result.artwork_url,
                "author": result.name,
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }
    async fn load_playlist(&self, url: &str) -> LoadResult {
        let playlist_id = match self.capture_id(&self.playlist_re, url) {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let domain_hint = super::region::extract_domain(url);
        let resp = match self
            .client
            .fetch_playlist_multi_region(&playlist_id, domain_hint.as_deref())
            .await
        {
            Some(v) => v,
            None => return LoadResult::Empty {},
        };
        if is_invalid_playlist(&resp) {
            debug!("Amazon Music: playlist '{playlist_id}' not found/unavailable");
            return LoadResult::Empty {};
        }
        let result = match parse_playlist_tracks(&resp) {
            Some(r) => r,
            None => return LoadResult::Empty {},
        };
        let tracks: Vec<Track> = result.tracks.into_iter().map(Track::new).collect();
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: result.name.clone(),
                selected_track: -1,
            },
            plugin_info: json!({
                "type": "playlist",
                "url": format!("https://music.amazon.com/playlists/{playlist_id}"),
                "artworkUrl": result.artwork_url,
                "author": "Amazon Music",
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }
    async fn load_community_playlist(&self, url: &str) -> LoadResult {
        let playlist_id = match self.capture_id(&self.user_playlist_re, url) {
            Some(id) => id,
            None => return LoadResult::Empty {},
        };
        let resp = match self.client.fetch_community_playlist(&playlist_id).await {
            Some(v) => v,
            None => return LoadResult::Empty {},
        };
        if is_invalid_community_playlist(&resp) {
            debug!("Amazon Music: community playlist '{playlist_id}' not found");
            return LoadResult::Empty {};
        }
        let result = match parse_community_playlist_tracks(&resp) {
            Some(r) => r,
            None => return LoadResult::Empty {},
        };
        let tracks: Vec<Track> = result.tracks.into_iter().map(Track::new).collect();
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: result.name.clone(),
                selected_track: -1,
            },
            plugin_info: json!({
                "type": "playlist",
                "url": format!("https://music.amazon.com/user-playlists/{playlist_id}"),
                "artworkUrl": result.artwork_url,
                "author": "Community User",
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }
    async fn load_search(&self, query: &str) -> LoadResult {
        let resp = match self.client.search_tracks(query).await {
            Some(v) => v,
            None => return LoadResult::Empty {},
        };
        let items: Vec<serde_json::Value> = match resp["methods"]
            .as_array()
            .and_then(|m| m.first())
            .and_then(|m| m["template"]["widgets"].as_array())
            .and_then(|w| w.first())
            .and_then(|w| w["items"].as_array())
        {
            Some(i) => i.iter().take(self.search_limit).cloned().collect(),
            None => return LoadResult::Empty {},
        };
        let mut unique_albums: std::collections::HashSet<String> = std::collections::HashSet::new();
        for item in &items {
            if let Some(key) = item["iconButton"]["observer"]["storageKey"].as_str()
                && let Some(album_id) = key.split(':').next()
                && !album_id.is_empty()
            {
                unique_albums.insert(album_id.to_string());
            }
        }
        let album_ids: Vec<String> = unique_albums.into_iter().collect();
        let mut duration_map: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for batch in album_ids.chunks(5) {
            let futures: Vec<_> = batch
                .iter()
                .map(|album_id| self.client.fetch_album(album_id))
                .collect();
            let results = futures::future::join_all(futures).await;
            for (album_id, album_resp) in batch.iter().zip(results) {
                let album_resp = match album_resp {
                    Some(v) => v,
                    None => continue,
                };
                let album_items =
                    match album_resp["methods"][0]["template"]["widgets"][0]["items"].as_array() {
                        Some(i) => i.clone(),
                        None => continue,
                    };
                for track in &album_items {
                    let deeplink = match track["primaryTextLink"]["deeplink"].as_str() {
                        Some(dl) => dl,
                        None => continue,
                    };
                    let track_id = match deeplink.split("/tracks/").nth(1) {
                        Some(id) => id.split('/').next().unwrap_or("").to_string(),
                        None => continue,
                    };
                    if track_id.is_empty() {
                        continue;
                    }
                    let duration_ms = super::api::duration_str_to_ms(
                        track["secondaryText3"].as_str().unwrap_or(""),
                    );
                    duration_map.insert(format!("{album_id}:{track_id}"), duration_ms);
                }
            }
        }
        let tracks: Vec<Track> = parse_search_tracks(&resp, self.search_limit, &duration_map)
            .into_iter()
            .map(Track::new)
            .collect();
        if tracks.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Search(tracks)
        }
    }
}
#[async_trait]
impl SourcePlugin for AmazonMusicSource {
    fn name(&self) -> &str {
        "amazonmusic"
    }
    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || self.domain_re.is_match(identifier)
    }
    fn search_prefixes(&self) -> Vec<&str> {
        vec!["azmsearch:", "amznsearch:"]
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
            return self.load_search(&identifier[prefix.len()..]).await;
        }
        if self.track_re.is_match(identifier) {
            return self.load_track(identifier).await;
        }
        if self.album_re.is_match(identifier) {
            return self.load_album(identifier).await;
        }
        if self.artist_re.is_match(identifier) {
            return self.load_artist(identifier).await;
        }
        if self.user_playlist_re.is_match(identifier) {
            return self.load_community_playlist(identifier).await;
        }
        if self.playlist_re.is_match(identifier) {
            return self.load_playlist(identifier).await;
        }
        LoadResult::Empty {}
    }
    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let api_base = match self.api_url.as_ref() {
            Some(url) => url,
            None => {
                tracing::debug!("AmazonMusic: api_url not set, falling back to mirror");
                return None;
            }
        };
        let track_id = self
            .capture_id(&self.track_re, identifier)
            .unwrap_or_else(|| identifier.to_string());
        let api_endpoint = format!("{}/api/track/{}", api_base.trim_end_matches('/'), track_id);
        let response = match self
            .http
            .get(&api_endpoint)
            .header("User-Agent", super::direct::UA)
            .send()
            .await
        {
            Ok(res) => {
                if !res.status().is_success() {
                    tracing::warn!("AmazonMusic API returned error status: {}", res.status());
                    return None;
                }
                match res.json::<StreamApiResponse>().await {
                    Ok(data) => data,
                    Err(e) => {
                        tracing::warn!("AmazonMusic API failed to parse JSON: {}", e);
                        return None;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("AmazonMusic API request failed: {}", e);
                return None;
            }
        };
        if response.stream_url.is_empty() {
            tracing::warn!("AmazonMusic API returned empty stream URL");
            return None;
        }
        let local_addr = routeplanner
            .as_ref()
            .and_then(|rp| rp.get_address())
            .or(self.local_addr);
        tracing::info!(
            "AmazonMusic: Direct playback configured successfully for {}",
            track_id
        );
        Some(Arc::new(AmazonMusicTrack {
            track_id,
            stream_url: response.stream_url,
            decryption_key: response.decryption_key,
            local_addr,
            proxy: self.proxy.clone(),
        }))
    }
    fn get_proxy_config(&self) -> Option<crate::config::HttpProxyConfig> {
        self.proxy.clone()
    }
}
}
pub mod parsers {
use serde_json::Value;
use super::api::{clean_image_url, clean_song_title, duration_str_to_ms, normalize_artist};
use crate::protocol::tracks::TrackInfo;
pub fn parse_track(resp: &Value, track_id: &str) -> Option<TrackInfo> {
    let methods = resp["methods"].as_array()?;
    let template = &methods.first()?["template"];
    if template.is_null() {
        return None;
    }
    let widgets = template["widgets"].as_array()?;
    let tracklist = widgets.iter().find(|w| {
        w["header"]
            .as_str()
            .map(|h| h.to_lowercase().contains("album tracklist"))
            .unwrap_or(false)
    })?;
    let items = tracklist["items"].as_array()?;
    let track_item = items.iter().find(|item| {
        item["primaryTextLink"]["deeplink"]
            .as_str()
            .map(|dl| dl.contains(&format!("/tracks/{track_id}")))
            .unwrap_or(false)
    })?;
    let title = template["headerText"]["text"]
        .as_str()
        .unwrap_or("Unknown Title")
        .to_string();
    let artist = normalize_artist(
        template["headerPrimaryText"]
            .as_str()
            .unwrap_or("Unknown Artist"),
    );
    let duration_ms = duration_str_to_ms(track_item["secondaryText3"].as_str().unwrap_or(""));
    let artwork_url = template["headerImage"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(clean_image_url);
    let isrc = extract_isrc(template);
    Some(TrackInfo {
        identifier: track_id.to_string(),
        is_seekable: true,
        author: artist,
        length: duration_ms,
        is_stream: false,
        position: 0,
        title,
        uri: Some(format!("https://music.amazon.com/tracks/{track_id}")),
        artwork_url,
        isrc,
        source_name: "amazonmusic".to_string(),
    })
}
pub fn parse_album_tracks(
    resp: &Value,
    _album_id: &str,
) -> Option<(String, String, Vec<TrackInfo>)> {
    let template = &resp["methods"][0]["template"];
    let album_name = template["headerText"]["text"]
        .as_str()
        .unwrap_or("Unknown Album")
        .to_string();
    let artist_name = template["headerPrimaryText"]
        .as_str()
        .unwrap_or("Unknown Artist")
        .to_string();
    let artwork = template["headerImage"].as_str().unwrap_or("").to_string();
    let items = template["widgets"][0]["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let tracks = items
        .iter()
        .filter_map(|item| {
            let track_id = item["primaryTextLink"]["deeplink"]
                .as_str()
                .and_then(|dl| dl.split("/tracks/").nth(1))?
                .to_string();
            let title = item["primaryText"]
                .as_str()
                .unwrap_or("Unknown Title")
                .to_string();
            let item_artist = normalize_artist(
                item["secondaryText2"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&artist_name),
            );
            let duration_ms = duration_str_to_ms(item["secondaryText3"].as_str().unwrap_or(""));
            let art = if artwork.is_empty() {
                None
            } else {
                Some(clean_image_url(&artwork))
            };
            Some(TrackInfo {
                identifier: track_id.clone(),
                is_seekable: true,
                author: item_artist,
                length: duration_ms,
                is_stream: false,
                position: 0,
                title,
                uri: Some(format!("https://music.amazon.com/tracks/{track_id}")),
                artwork_url: art,
                isrc: None,
                source_name: "amazonmusic".to_string(),
            })
        })
        .collect();
    Some((album_name, artist_name, tracks))
}
pub struct ArtistResult {
    pub name: String,
    pub artwork_url: Option<String>,
    pub tracks: Vec<TrackInfo>,
}
pub fn parse_artist_top_songs(
    resp: &Value,
    artist_id: &str,
    duration_map: &std::collections::HashMap<String, u64>,
) -> Option<ArtistResult> {
    let template = &resp["methods"][0]["template"];
    let artist_name = template["headerText"]["text"]
        .as_str()
        .unwrap_or("Unknown Artist")
        .to_string();
    let artwork_url = template["backgroundImage"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(clean_image_url);
    let widgets = template["widgets"].as_array()?;
    let top_songs_widget = widgets.iter().find(|w| {
        w["header"]
            .as_str()
            .map(|h| h.to_lowercase().contains("top songs"))
            .unwrap_or(false)
    })?;
    let items = top_songs_widget["items"].as_array()?;
    let tracks = items
        .iter()
        .filter_map(|item| {
            let storage_key = item["iconButton"]["observer"]["storageKey"].as_str()?;
            let mut parts = storage_key.splitn(2, ':');
            let album_id = parts.next()?.to_string();
            let track_id = parts.next()?.to_string();
            if track_id.is_empty() {
                return None;
            }
            let title = clean_song_title(
                item["primaryText"]["text"]
                    .as_str()
                    .unwrap_or("Unknown Title"),
            );
            let artist = normalize_artist(item["secondaryText"].as_str().unwrap_or(&artist_name));
            let item_artwork = item["image"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(clean_image_url)
                .or_else(|| artwork_url.clone());
            let duration_ms = duration_map
                .get(&format!("{album_id}:{track_id}"))
                .copied()
                .unwrap_or(0);
            Some(TrackInfo {
                identifier: track_id.clone(),
                is_seekable: true,
                author: artist,
                length: duration_ms,
                is_stream: false,
                position: 0,
                title,
                uri: Some(format!("https://music.amazon.com/tracks/{track_id}")),
                artwork_url: item_artwork,
                isrc: None,
                source_name: "amazonmusic".to_string(),
            })
        })
        .collect();
    let _ = artist_id;
    Some(ArtistResult {
        name: artist_name,
        artwork_url,
        tracks,
    })
}
pub struct PlaylistResult {
    pub name: String,
    pub artwork_url: Option<String>,
    pub tracks: Vec<TrackInfo>,
}
pub fn parse_playlist_tracks(resp: &Value) -> Option<PlaylistResult> {
    let template = &resp["methods"][0]["template"];
    let name = template["headerText"]["text"]
        .as_str()
        .unwrap_or("Unknown Playlist")
        .to_string();
    let artwork_url = template["headerImage"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(clean_image_url);
    let items = template["widgets"][0]["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let tracks = items
        .iter()
        .filter_map(|item| {
            let storage_key = item["iconButton"]["observer"]["storageKey"].as_str()?;
            let mut parts = storage_key.splitn(2, ':');
            let _album_id = parts.next()?;
            let track_id = parts.next()?.to_string();
            if track_id.is_empty() {
                return None;
            }
            let title = item["primaryText"]
                .as_str()
                .unwrap_or("Unknown Title")
                .to_string();
            let artist =
                normalize_artist(item["secondaryText1"].as_str().unwrap_or("Unknown Artist"));
            let duration_ms = duration_str_to_ms(item["secondaryText3"].as_str().unwrap_or(""));
            let item_art = item["image"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(clean_image_url)
                .or_else(|| artwork_url.clone());
            Some(TrackInfo {
                identifier: track_id.clone(),
                is_seekable: true,
                author: artist,
                length: duration_ms,
                is_stream: false,
                position: 0,
                title,
                uri: Some(format!("https://music.amazon.com/tracks/{track_id}")),
                artwork_url: item_art,
                isrc: None,
                source_name: "amazonmusic".to_string(),
            })
        })
        .collect();
    Some(PlaylistResult {
        name,
        artwork_url,
        tracks,
    })
}
pub fn parse_community_playlist_tracks(resp: &Value) -> Option<PlaylistResult> {
    let template = &resp["methods"][0]["template"];
    let name = template["headerText"]["text"]
        .as_str()
        .unwrap_or("Unknown Playlist")
        .to_string();
    let artwork_url = template["headerImage"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(clean_image_url);
    let items = template["widgets"][0]["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let tracks = items
        .iter()
        .filter_map(|item| {
            let track_id = item["id"].as_str()?.to_string();
            if track_id.is_empty() {
                return None;
            }
            let title = item["primaryText"]
                .as_str()
                .unwrap_or("Unknown Title")
                .to_string();
            let artist =
                normalize_artist(item["secondaryText1"].as_str().unwrap_or("Unknown Artist"));
            let duration_ms = duration_str_to_ms(item["secondaryText3"].as_str().unwrap_or(""));
            let item_art = item["image"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(clean_image_url)
                .or_else(|| artwork_url.clone());
            Some(TrackInfo {
                identifier: track_id.clone(),
                is_seekable: true,
                author: artist,
                length: duration_ms,
                is_stream: false,
                position: 0,
                title,
                uri: Some(format!("https://music.amazon.com/tracks/{track_id}")),
                artwork_url: item_art,
                isrc: None,
                source_name: "amazonmusic".to_string(),
            })
        })
        .collect();
    Some(PlaylistResult {
        name,
        artwork_url,
        tracks,
    })
}
pub fn parse_search_tracks(
    resp: &Value,
    limit: usize,
    duration_map: &std::collections::HashMap<String, u64>,
) -> Vec<TrackInfo> {
    let items = match resp["methods"]
        .as_array()
        .and_then(|m| m.first())
        .and_then(|m| m["template"]["widgets"].as_array())
        .and_then(|w| w.first())
        .and_then(|w| w["items"].as_array())
    {
        Some(i) => i,
        None => return Vec::new(),
    };
    items
        .iter()
        .take(limit)
        .filter_map(|item| {
            let storage_key = item["iconButton"]["observer"]["storageKey"].as_str()?;
            let mut parts = storage_key.splitn(2, ':');
            let album_id = parts.next()?.to_string();
            let track_id = parts.next()?.to_string();
            if track_id.is_empty() {
                return None;
            }
            let title = item["primaryText"]["text"]
                .as_str()
                .unwrap_or("Unknown Title")
                .to_string();
            let artist =
                normalize_artist(item["secondaryText"].as_str().unwrap_or("Unknown Artist"));
            let artwork = item["image"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(clean_image_url);
            let duration_ms = duration_map
                .get(&format!("{album_id}:{track_id}"))
                .copied()
                .unwrap_or(0);
            Some(TrackInfo {
                identifier: track_id.clone(),
                is_seekable: true,
                author: artist,
                length: duration_ms,
                is_stream: false,
                position: 0,
                title,
                uri: Some(format!("https://music.amazon.com/tracks/{track_id}")),
                artwork_url: artwork,
                isrc: None,
                source_name: "amazonmusic".to_string(),
            })
        })
        .collect()
}
fn extract_isrc(template: &Value) -> Option<String> {
    let scripts = template["templateData"]["seoHead"]["script"].as_array()?;
    for script in scripts {
        let inner_html = script["innerHTML"].as_str()?;
        if let Ok(parsed) = serde_json::from_str::<Value>(inner_html)
            && let Some(isrc) = parsed["isrcCode"].as_str()
        {
            return Some(isrc.to_string());
        }
    }
    None
}
}
pub mod reader {
use std::io::{Read, Seek, SeekFrom};
use symphonia::core::io::MediaSource;
use crate::{
    audio::source::{HttpSource, create_client},
    common::types::AnyResult,
};
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";
pub struct AmazonRemoteReader {
    inner: HttpSource,
}
impl AmazonRemoteReader {
    pub async fn open(
        url: &str,
        local_addr: Option<std::net::IpAddr>,
        proxy: Option<crate::config::HttpProxyConfig>,
    ) -> AnyResult<Self> {
        let client = create_client(UA.to_owned(), local_addr, proxy, None)?;
        let inner = HttpSource::new(client, url).await?;
        Ok(Self { inner })
    }
}
impl Read for AmazonRemoteReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}
impl Seek for AmazonRemoteReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}
impl MediaSource for AmazonRemoteReader {
    fn is_seekable(&self) -> bool {
        self.inner.is_seekable()
    }
    fn byte_len(&self) -> Option<u64> {
        self.inner.byte_len()
    }
}
}
pub mod region {
use std::{sync::LazyLock, time::Duration};
use futures::{StreamExt, stream::FuturesUnordered};
use regex::Regex;
use serde_json::{Value, json};
use tracing::{debug, warn};
use super::api::gen_request_id;
const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 \
    (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36";
pub struct RegionConfig {
    pub skill_endpoint: &'static str,
    pub language: &'static str,
    pub currency: &'static str,
    pub device_family: &'static str,
    pub feature_flags: &'static str,
}
const EP_NA: &str = "https://na.web.skill.music.a2z.com";
const EP_EU: &str = "https://eu.web.skill.music.a2z.com";
const EP_FE: &str = "https://fe.web.skill.music.a2z.com";
static REGION_CONFIGS: &[(&str, RegionConfig)] = &[
    (
        "music.amazon.com",
        RegionConfig {
            skill_endpoint: EP_NA,
            language: "en_US",
            currency: "USD",
            device_family: "WebPlayer",
            feature_flags: "",
        },
    ),
    (
        "music.amazon.com.mx",
        RegionConfig {
            skill_endpoint: EP_NA,
            language: "es_MX",
            currency: "MXN",
            device_family: "WebPlayer",
            feature_flags: "",
        },
    ),
    (
        "music.amazon.com.br",
        RegionConfig {
            skill_endpoint: EP_NA,
            language: "pt_BR",
            currency: "BRL",
            device_family: "WebPlayer",
            feature_flags: "",
        },
    ),
    (
        "music.amazon.ca",
        RegionConfig {
            skill_endpoint: EP_NA,
            language: "en_CA",
            currency: "CAD",
            device_family: "WebPlayer",
            feature_flags: "",
        },
    ),
    (
        "music.amazon.co.uk",
        RegionConfig {
            skill_endpoint: EP_EU,
            language: "en_GB",
            currency: "GBP",
            device_family: "WebPlayer",
            feature_flags: "hd-supported,uhd-supported",
        },
    ),
    (
        "music.amazon.de",
        RegionConfig {
            skill_endpoint: EP_EU,
            language: "de_DE",
            currency: "EUR",
            device_family: "WebPlayer",
            feature_flags: "hd-supported,uhd-supported",
        },
    ),
    (
        "music.amazon.fr",
        RegionConfig {
            skill_endpoint: EP_EU,
            language: "fr_FR",
            currency: "EUR",
            device_family: "WebPlayer",
            feature_flags: "hd-supported,uhd-supported",
        },
    ),
    (
        "music.amazon.it",
        RegionConfig {
            skill_endpoint: EP_EU,
            language: "it_IT",
            currency: "EUR",
            device_family: "WebPlayer",
            feature_flags: "hd-supported,uhd-supported",
        },
    ),
    (
        "music.amazon.es",
        RegionConfig {
            skill_endpoint: EP_EU,
            language: "es_ES",
            currency: "EUR",
            device_family: "WebPlayer",
            feature_flags: "hd-supported,uhd-supported",
        },
    ),
    (
        "music.amazon.in",
        RegionConfig {
            skill_endpoint: EP_EU,
            language: "en_IN",
            currency: "INR",
            device_family: "WebPlayer",
            feature_flags: "hd-supported,uhd-supported",
        },
    ),
    (
        "music.amazon.sa",
        RegionConfig {
            skill_endpoint: EP_EU,
            language: "ar_SA",
            currency: "SAR",
            device_family: "WebPlayer",
            feature_flags: "",
        },
    ),
    (
        "music.amazon.ae",
        RegionConfig {
            skill_endpoint: EP_EU,
            language: "ar_AE",
            currency: "AED",
            device_family: "WebPlayer",
            feature_flags: "",
        },
    ),
    (
        "music.amazon.co.jp",
        RegionConfig {
            skill_endpoint: EP_FE,
            language: "ja_JP",
            currency: "JPY",
            device_family: "WebPlayer",
            feature_flags: "hd-supported,uhd-supported",
        },
    ),
    (
        "music.amazon.com.au",
        RegionConfig {
            skill_endpoint: EP_FE,
            language: "en_AU",
            currency: "AUD",
            device_family: "WebPlayer",
            feature_flags: "hd-supported,uhd-supported",
        },
    ),
];
static REGION_FALLBACKS: &[(&str, &str)] = &[
    ("NA", "music.amazon.com"),
    ("EU", "music.amazon.co.uk"),
    ("FE", "music.amazon.co.jp"),
];
fn get_region_config(domain: &str) -> Option<&'static RegionConfig> {
    REGION_CONFIGS
        .iter()
        .find(|(d, _)| *d == domain)
        .map(|(_, cfg)| cfg)
}
static DOMAIN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^https?://(?:www\.)?(music\.amazon\.[a-z.]+)").unwrap());
pub fn extract_domain(url: &str) -> Option<String> {
    let caps = DOMAIN_RE.captures(url)?;
    let domain = caps.get(1)?.as_str().to_lowercase();
    if get_region_config(&domain).is_some() {
        Some(domain)
    } else {
        None
    }
}
fn build_region_headers(
    config: &Value,
    region: &RegionConfig,
    domain: &str,
    page_url: &str,
) -> Value {
    let access_token = config["accessToken"].as_str().unwrap_or("");
    let device_id = config["deviceId"].as_str().unwrap_or("");
    let session_id = config["sessionId"].as_str().unwrap_or("");
    let version = config["version"].as_str().unwrap_or("");
    let csrf_token = config["csrf"]["token"].as_str().unwrap_or("");
    let csrf_ts = config["csrf"]["ts"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| config["csrf"]["ts"].as_u64().map(|n| n.to_string()))
        .unwrap_or_default();
    let csrf_rnd = config["csrf"]["rnd"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| config["csrf"]["rnd"].as_u64().map(|n| n.to_string()))
        .unwrap_or_default();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default();
    json!({
        "x-amzn-authentication": serde_json::to_string(&json!({
            "interface": "ClientAuthenticationInterface.v1_0.ClientTokenElement",
            "accessToken": access_token
        })).unwrap_or_default(),
        "x-amzn-device-model": "WEBPLAYER",
        "x-amzn-device-width": "1920",
        "x-amzn-device-family": region.device_family,
        "x-amzn-device-id": device_id,
        "x-amzn-user-agent": USER_AGENT,
        "x-amzn-session-id": session_id,
        "x-amzn-device-height": "1080",
        "x-amzn-request-id": gen_request_id(),
        "x-amzn-device-language": region.language,
        "x-amzn-currency-of-preference": region.currency,
        "x-amzn-os-version": "1.0",
        "x-amzn-application-version": version,
        "x-amzn-device-time-zone": "Asia/Calcutta",
        "x-amzn-timestamp": ts,
        "x-amzn-csrf": serde_json::to_string(&json!({
            "interface": "CSRFInterface.v1_0.CSRFHeaderElement",
            "token": csrf_token,
            "timestamp": csrf_ts,
            "rndNonce": csrf_rnd
        })).unwrap_or_default(),
        "x-amzn-music-domain": domain,
        "x-amzn-referer": domain,
        "x-amzn-affiliate-tags": "",
        "x-amzn-ref-marker": "",
        "x-amzn-page-url": page_url,
        "x-amzn-weblab-id-overrides": "",
        "x-amzn-video-player-token": "",
        "x-amzn-feature-flags": region.feature_flags,
        "x-amzn-has-profile-id": "",
        "x-amzn-age-band": ""
    })
}
async fn fetch_from_endpoint(
    http: &reqwest::Client,
    id: &str,
    api_path: &str,
    url_path_segment: &str,
    region: &RegionConfig,
    domain: &str,
    base_config: &Value,
) -> Option<Value> {
    let page_url = format!(
        "https://{domain}/{url_path_segment}/{}",
        urlencoding::encode(id)
    );
    let inner_headers = build_region_headers(base_config, region, domain, &page_url);
    let body = json!({
        "id": id,
        "userHash": serde_json::to_string(&json!({"level": "LIBRARY_MEMBER"})).unwrap_or_default(),
        "headers": serde_json::to_string(&inner_headers).unwrap_or_default()
    });
    let endpoint_url = format!("{}/api/{api_path}", region.skill_endpoint);
    let authority = region
        .skill_endpoint
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let resp = http
        .post(&endpoint_url)
        .header("authority", authority)
        .header("accept", "*/*")
        .header("accept-language", "en-US,en;q=0.9")
        .header("content-type", "text/plain;charset=UTF-8")
        .header("origin", format!("https://{domain}"))
        .header("referer", format!("https://{domain}/"))
        .header(
            "sec-ch-ua",
            "\"Chromium\";v=\"125\", \"Not.A/Brand\";v=\"24\"",
        )
        .header("sec-ch-ua-mobile", "?1")
        .header("sec-ch-ua-platform", "\"Android\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "cross-site")
        .header("user-agent", USER_AGENT)
        .timeout(Duration::from_secs(8))
        .body(serde_json::to_string(&body).unwrap_or_default())
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<Value>().await.ok()
}
#[allow(clippy::too_many_arguments)]
pub async fn fetch_multi_region(
    http: &reqwest::Client,
    id: &str,
    api_path: &str,
    url_path_segment: &str,
    entity_name: &str,
    domain_hint: Option<&str>,
    is_error: fn(&Value) -> bool,
    base_config: &Value,
) -> Option<Value> {
    let mut tried_endpoint: Option<&str> = None;
    if let Some(hint) = domain_hint
        && let Some(region) = get_region_config(hint)
    {
        debug!("Amazon Music: trying {entity_name} on hinted domain '{hint}'");
        tried_endpoint = Some(region.skill_endpoint);
        if let Some(data) = fetch_from_endpoint(
            http,
            id,
            api_path,
            url_path_segment,
            region,
            hint,
            base_config,
        )
        .await
            && !is_error(&data)
        {
            debug!("Amazon Music: {entity_name} resolved via hinted domain '{hint}'");
            return Some(data);
        }
        debug!(
            "Amazon Music: hinted domain '{hint}' failed for {entity_name}, trying other regions"
        );
    }
    let mut futs = FuturesUnordered::new();
    for &(label, domain) in REGION_FALLBACKS {
        let region = match get_region_config(domain) {
            Some(r) => r,
            None => continue,
        };
        if tried_endpoint == Some(region.skill_endpoint) {
            continue;
        }
        let base_config = base_config.clone();
        futs.push(async move {
            let result = fetch_from_endpoint(
                http,
                id,
                api_path,
                url_path_segment,
                region,
                domain,
                &base_config,
            )
            .await;
            result.map(|data| (data, label))
        });
    }
    while let Some(result) = futs.next().await {
        if let Some((data, label)) = result
            && !is_error(&data)
        {
            debug!("Amazon Music: {entity_name} resolved via {label} region");
            return Some(data);
        }
    }
    warn!("Amazon Music: {entity_name} not found on any regional endpoint (NA, EU, FE)");
    None
}
}
pub mod streaming_reader {
use std::{
    io::{Read, Seek, SeekFrom},
    sync::Arc,
    time::Duration,
};
use parking_lot::{Condvar, Mutex};
use symphonia::core::io::MediaSource;
use tracing::{debug, warn};
use super::crypt::{CencDecryptor, extract_flac_stream_header};
struct BufferState {
    data: Vec<u8>,
    available: usize,
    finished: bool,
    error: Option<String>,
}
pub struct AmazonStreamingReader {
    cursor: u64,
    shared: Arc<(Mutex<BufferState>, Condvar)>,
}
impl AmazonStreamingReader {
    pub fn new(
        client: reqwest::Client,
        url: &str,
        decryption_key: &str,
        total_len: u64,
    ) -> Result<Self, String> {
        let decryptor = CencDecryptor::from_hex(decryption_key)?;
        let shared = Arc::new((
            Mutex::new(BufferState {
                data: Vec::new(),
                available: 0,
                finished: false,
                error: None,
            }),
            Condvar::new(),
        ));
        let shared_bg = Arc::clone(&shared);
        let url_owned = url.to_string();
        std::thread::Builder::new()
            .name("amz-flac-extract".into())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(download_and_extract_flac(
                        shared_bg, client, url_owned, decryptor, total_len,
                    ));
            })
            .map_err(|e| format!("failed to spawn stream thread: {e}"))?;
        {
            let (lock, cvar) = &*shared;
            let mut state = lock.lock();
            while state.available == 0 && !state.finished && state.error.is_none() {
                cvar.wait_for(&mut state, Duration::from_millis(50));
            }
            if let Some(ref e) = state.error {
                return Err(e.clone());
            }
            if state.available == 0 {
                return Err("stream ended before FLAC header could be extracted".into());
            }
        }
        Ok(Self { cursor: 0, shared })
    }
}
impl Read for AmazonStreamingReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let (lock, cvar) = &*self.shared;
        let mut state = lock.lock();
        loop {
            if let Some(ref err) = state.error {
                return Err(std::io::Error::other(err.clone()));
            }
            let pos = self.cursor as usize;
            if pos < state.available {
                let n = (state.available - pos).min(buf.len());
                buf[..n].copy_from_slice(&state.data[pos..pos + n]);
                self.cursor += n as u64;
                return Ok(n);
            }
            if state.finished {
                return Ok(0);
            }
            cvar.wait_for(&mut state, Duration::from_millis(50));
        }
    }
}
impl Seek for AmazonStreamingReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target: u64 = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::Current(d) => self.cursor.saturating_add_signed(d),
            SeekFrom::End(d) => {
                let (lock, cvar) = &*self.shared;
                let mut state = lock.lock();
                while !state.finished && state.error.is_none() {
                    cvar.wait_for(&mut state, Duration::from_millis(50));
                }
                if let Some(ref e) = state.error {
                    return Err(std::io::Error::other(e.clone()));
                }
                (state.available as i64).saturating_add(d).max(0) as u64
            }
        };
        {
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            while (state.available as u64) < target && !state.finished && state.error.is_none() {
                cvar.wait_for(&mut state, Duration::from_millis(50));
            }
            if let Some(ref e) = state.error {
                return Err(std::io::Error::other(e.clone()));
            }
            self.cursor = target.min(state.available as u64);
        }
        Ok(self.cursor)
    }
}
impl MediaSource for AmazonStreamingReader {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        let (lock, _) = &*self.shared;
        let state = lock.lock();
        if state.finished {
            Some(state.available as u64)
        } else {
            None
        }
    }
}
impl Drop for AmazonStreamingReader {
    fn drop(&mut self) {
        let (lock, _) = &*self.shared;
        lock.lock().finished = true;
    }
}
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";
async fn download_and_extract_flac(
    shared: Arc<(Mutex<BufferState>, Condvar)>,
    client: reqwest::Client,
    url: String,
    decryptor: CencDecryptor,
    _total_len: u64,
) {
    let response = match client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "*/*")
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            signal_error(&shared, format!("HTTP {}", r.status()));
            return;
        }
        Err(e) => {
            signal_error(&shared, format!("request failed: {e}"));
            return;
        }
    };
    let mut staging: Vec<u8> = Vec::with_capacity(256 * 1024);
    let mut total_drained: usize = 0;
    let mut remainder: Vec<u8> = Vec::new();
    let mut stream = response;
    let mut header_written = false;
    loop {
        match stream.chunk().await {
            Ok(Some(chunk)) => {
                staging.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(e) => {
                signal_error(&shared, format!("download error: {e}"));
                return;
            }
        }
        total_drained += process_staging_lowmem(
            &shared,
            &decryptor,
            &mut staging,
            &mut remainder,
            &mut header_written,
        );
        if total_drained > 512 * 1024 {
            staging.shrink_to_fit();
            total_drained = 0;
        }
    }
    process_staging_lowmem(
        &shared,
        &decryptor,
        &mut staging,
        &mut remainder,
        &mut header_written,
    );
    drop(staging);
    if !remainder.is_empty() {
        append_to_buffer(&shared, &remainder);
        remainder.clear();
        remainder.shrink_to_fit();
    }
    let (lock, cvar) = &*shared;
    let mut state = lock.lock();
    state.finished = true;
    cvar.notify_all();
    debug!(
        "Amazon streaming: download complete ({} bytes)",
        state.available
    );
}
fn process_staging_lowmem(
    shared: &Arc<(Mutex<BufferState>, Condvar)>,
    decryptor: &CencDecryptor,
    staging: &mut Vec<u8>,
    remainder: &mut Vec<u8>,
    header_written: &mut bool,
) -> usize {
    let mut total_drained = 0;
    loop {
        if staging.len() < 8 {
            break;
        }
        let box_size =
            u32::from_be_bytes([staging[0], staging[1], staging[2], staging[3]]) as usize;
        if box_size < 8 {
            break;
        }
        let box_type = [staging[4], staging[5], staging[6], staging[7]];
        if &box_type == b"moof" {
            if staging.len() < box_size + 8 {
                break;
            }
            let mdat_size = u32::from_be_bytes([
                staging[box_size],
                staging[box_size + 1],
                staging[box_size + 2],
                staging[box_size + 3],
            ]) as usize;
            let mdat_type = [
                staging[box_size + 4],
                staging[box_size + 5],
                staging[box_size + 6],
                staging[box_size + 7],
            ];
            if &mdat_type != b"mdat" || mdat_size < 8 {
                staging.drain(..box_size);
                total_drained += box_size;
                continue;
            }
            let fragment_total = box_size + mdat_size;
            if staging.len() < fragment_total {
                break;
            }
            let _ = decryptor.decrypt_buffer(&mut staging[..fragment_total]);
            let payload_start = box_size + 8;
            let payload_end = box_size + mdat_size;
            if *header_written && payload_end > payload_start {
                let payload_len = payload_end - payload_start;
                let mut payload_copy = Vec::with_capacity(payload_len);
                payload_copy.extend_from_slice(&staging[payload_start..payload_end]);
                flush_complete_flac_frames(shared, remainder, &payload_copy);
            }
            staging.drain(..fragment_total);
            total_drained += fragment_total;
            continue;
        }
        if staging.len() < box_size {
            break;
        }
        if &box_type == b"moov" && !*header_written {
            match extract_flac_stream_header(&staging[..box_size]) {
                Some(hdr) => {
                    debug!("Amazon FLAC: header extracted ({} bytes)", hdr.len());
                    append_to_buffer(shared, &hdr);
                    *header_written = true;
                }
                None => warn!("Amazon FLAC: failed to extract FLAC header from moov"),
            }
        }
        staging.drain(..box_size);
        total_drained += box_size;
    }
    total_drained
}
fn flush_complete_flac_frames(
    shared: &Arc<(Mutex<BufferState>, Condvar)>,
    remainder: &mut Vec<u8>,
    payload: &[u8],
) {
    remainder.extend_from_slice(payload);
    let mut pos = 0;
    loop {
        if pos >= remainder.len() {
            break;
        }
        match next_flac_frame_end(&remainder[pos..]) {
            Some(frame_len) => {
                append_to_buffer(shared, &remainder[pos..pos + frame_len]);
                pos += frame_len;
            }
            None => {
                break;
            }
        }
    }
    remainder.drain(..pos);
}
fn next_flac_frame_end(data: &[u8]) -> Option<usize> {
    if data.len() < 6 {
        return None;
    }
    if (data[0] != 0xFF) || (data[1] & 0xFE) != 0xF8 {
        if let Some(next) = find_sync(&data[1..]) {
            return next_flac_frame_end(&data[1 + next..]);
        }
        return None;
    }
    let search_from = 6;
    if data.len() <= search_from {
        return None;
    }
    find_sync(&data[search_from..]).map(|offset| search_from + offset)
}
fn find_sync(data: &[u8]) -> Option<usize> {
    data.windows(2)
        .position(|w| w[0] == 0xFF && (w[1] & 0xFE) == 0xF8)
}
fn append_to_buffer(shared: &Arc<(Mutex<BufferState>, Condvar)>, data: &[u8]) {
    let (lock, cvar) = &**shared;
    let mut state = lock.lock();
    state.data.extend_from_slice(data);
    state.available = state.data.len();
    cvar.notify_all();
}
fn signal_error(shared: &Arc<(Mutex<BufferState>, Condvar)>, msg: String) {
    warn!("Amazon FLAC streaming error: {}", msg);
    let (lock, cvar) = &**shared;
    let mut state = lock.lock();
    state.error = Some(msg);
    state.finished = true;
    cvar.notify_all();
}
}
pub mod track {
use crate::protocol::tracks::TrackInfo;
pub struct AmazonTrackData {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub duration_ms: u64,
    pub artwork_url: Option<String>,
    pub isrc: Option<String>,
}
impl AmazonTrackData {
    pub fn into_track_info(self) -> TrackInfo {
        let uri = format!("https://music.amazon.com/tracks/{}", self.track_id);
        TrackInfo {
            identifier: self.track_id,
            is_seekable: true,
            author: self.artist,
            length: self.duration_ms,
            is_stream: false,
            position: 0,
            title: self.title,
            uri: Some(uri),
            artwork_url: self.artwork_url,
            isrc: self.isrc,
            source_name: "amazonmusic".to_string(),
        }
    }
}
}
pub mod validators {
use serde_json::Value;
pub fn is_invalid_track(resp: &Value) -> bool {
    let methods = match resp["methods"].as_array() {
        Some(m) if !m.is_empty() => m,
        _ => return true,
    };
    let has_error_note = methods.iter().any(|m| {
        m["interface"]
            .as_str()
            .map(|i| i.contains("ShowNotificationMethod"))
            .unwrap_or(false)
            && m["notification"]["message"]["text"]
                .as_str()
                .map(|t| t.contains("no longer available"))
                .unwrap_or(false)
    });
    let is_homepage = methods[0]["template"]["interface"]
        .as_str()
        .map(|i| i.contains("GalleryTemplate"))
        .unwrap_or(false)
        && methods[0]["template"]["widgets"]
            .as_array()
            .map(|w| w.is_empty())
            .unwrap_or(false);
    has_error_note || is_homepage
}
pub fn is_invalid_album(resp: &Value) -> bool {
    let methods = match resp["methods"].as_array() {
        Some(m) if !m.is_empty() => m,
        _ => return true,
    };
    let template = &methods[0]["template"];
    template["interface"]
        .as_str()
        .map(|i| i.contains("DialogTemplate"))
        .unwrap_or(false)
        && template["header"]
            .as_str()
            .map(|h| h == "Service error")
            .unwrap_or(false)
}
pub fn is_invalid_artist(resp: &Value) -> bool {
    let methods = match resp["methods"].as_array() {
        Some(m) if !m.is_empty() => m,
        _ => return true,
    };
    let template = &methods[0]["template"];
    template["interface"]
        .as_str()
        .map(|i| i.contains("MessageTemplate"))
        .unwrap_or(false)
        && template["header"]
            .as_str()
            .map(|h| h == "We're Sorry")
            .unwrap_or(false)
        && template["message"]
            .as_str()
            .map(|m| m.contains("unable to complete your action"))
            .unwrap_or(false)
}
pub fn is_invalid_playlist(resp: &Value) -> bool {
    let methods = match resp["methods"].as_array() {
        Some(m) if !m.is_empty() => m,
        _ => return true,
    };
    let first = &methods[0];
    if first["template"]["widgets"]
        .as_array()
        .map(|w| w.is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    if let Some(second) = methods.get(1) {
        let msg = second["notification"]["message"]["text"]
            .as_str()
            .or_else(|| second["notification"]["message"]["innerHTML"].as_str())
            .unwrap_or("");
        if msg
            .to_lowercase()
            .contains("playlist is no longer available")
        {
            return true;
        }
    }
    first["template"]["templateData"]["deeplink"]
        .as_str()
        .map(|d| d == "/")
        .unwrap_or(false)
}
pub fn is_invalid_community_playlist(resp: &Value) -> bool {
    let template = match resp["methods"].as_array().and_then(|m| m.first()) {
        Some(m) => &m["template"],
        None => return false,
    };
    let is_dialog = template["interface"]
        .as_str()
        .map(|i| i == "Web.TemplatesInterface.v1_0.Touch.DialogTemplateInterface.DialogTemplate")
        .unwrap_or(false);
    let is_service_error = template["header"]
        .as_str()
        .map(|h| h.trim().to_lowercase() == "service error")
        .unwrap_or(false);
    let has_error_body = template["body"]["text"]
        .as_str()
        .map(|t| t.to_lowercase().contains("sorry something went wrong"))
        .unwrap_or(false);
    is_dialog && is_service_error && has_error_body
}
}
pub use manager::AmazonMusicSource;