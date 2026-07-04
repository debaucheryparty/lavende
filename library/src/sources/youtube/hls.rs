pub mod fetcher {
    use super::types::Resource;
    use crate::common::types::AnyResult;
    pub async fn fetch_segment_into(
        client: &reqwest::Client,
        resource: &Resource,
        out: &mut Vec<u8>,
    ) -> AnyResult<()> {
        let mut req = client.get(&resource.url).header("Accept", "*/*");
        if let Some(range) = &resource.range {
            let end = range.offset + range.length - 1;
            req = req.header("Range", format!("bytes={}-{}", range.offset, end));
        }
        let res = req.send().await?;
        if !res.status().is_success() {
            return Err(format!("HLS fetch failed {}: {}", res.status(), resource.url).into());
        }
        let bytes = res.bytes().await?;
        out.extend_from_slice(&bytes);
        Ok(())
    }
}
pub mod parser {
    use super::{
        types::{ByteRange, M3u8Playlist, Media, Resource, Variant},
        utils::{extract_attr_str, extract_attr_u64, parse_byte_range, resolve_url},
    };
    use std::collections::HashMap;
    pub fn parse_m3u8(text: &str, base_url: &str) -> M3u8Playlist {
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let is_master = lines.iter().any(|l| l.starts_with("#EXT-X-STREAM-INF"));
        if is_master {
            let mut variants = Vec::new();
            let mut audio_groups: HashMap<String, Vec<Media>> = HashMap::new();
            let mut i = 0;
            while i < lines.len() {
                let line = lines[i];
                if line.starts_with("#EXT-X-MEDIA") {
                    let type_ = extract_attr_str(line, "TYPE").unwrap_or_default();
                    let group_id = extract_attr_str(line, "GROUP-ID").unwrap_or_default();
                    let uri = extract_attr_str(line, "URI").map(|u| resolve_url(base_url, &u));
                    let is_default = extract_attr_str(line, "DEFAULT").as_deref() == Some("YES");
                    if type_ == "AUDIO" && !group_id.is_empty() {
                        audio_groups
                            .entry(group_id.clone())
                            .or_default()
                            .push(Media {
                                _type: type_,
                                _group_id: group_id,
                                uri,
                                is_default,
                            });
                    }
                    i += 1;
                } else if line.starts_with("#EXT-X-STREAM-INF") {
                    let bandwidth = extract_attr_u64(line, "BANDWIDTH").unwrap_or(0);
                    let codecs = extract_attr_str(line, "CODECS").unwrap_or_default();
                    let audio_group = extract_attr_str(line, "AUDIO");
                    let has_audio = codecs.contains("mp4a")
                        || codecs.contains("opus")
                        || codecs.contains("aac");
                    let has_video = codecs.contains("avc1")
                        || codecs.contains("hvc1")
                        || codecs.contains("hev1")
                        || codecs.contains("dvh1")
                        || codecs.contains("vp09")
                        || codecs.contains("av01")
                        || codecs.contains("vp9")
                        || codecs.contains("av1")
                        || codecs.contains("vp8")
                        || codecs.contains("h264")
                        || codecs.contains("h265")
                        || codecs.contains("mp4v");
                    let mut j = i + 1;
                    while j < lines.len() && lines[j].starts_with('#') {
                        j += 1;
                    }
                    if j < lines.len() && !lines[j].is_empty() {
                        variants.push(Variant {
                            url: resolve_url(base_url, lines[j]),
                            bandwidth,
                            codecs: codecs.clone(),
                            is_audio_only: has_audio && !has_video,
                            audio_group,
                        });
                    }
                    i = j + 1;
                } else {
                    i += 1;
                }
            }
            return M3u8Playlist::Master {
                variants,
                audio_groups,
            };
        }
        let mut segments = Vec::new();
        let mut map = None;
        let mut next_offset = 0u64;
        let mut pending_range: Option<ByteRange> = None;
        for i in 0..lines.len() {
            let line = lines[i];
            if line.starts_with("#EXT-X-MAP") {
                if let Some(url) = extract_attr_str(line, "URI").map(|u| resolve_url(base_url, &u))
                {
                    let range =
                        extract_attr_str(line, "BYTERANGE").map(|r| parse_byte_range(&r, 0));
                    map = Some(Resource {
                        url,
                        range,
                        duration: None,
                    });
                }
            } else if let Some(stripped) = line.strip_prefix("#EXT-X-BYTERANGE:") {
                let r = parse_byte_range(stripped, next_offset);
                next_offset = r.offset + r.length;
                pending_range = Some(r);
            } else if line.starts_with("#EXTINF:") {
                let seg_duration = line
                    .strip_prefix("#EXTINF:")
                    .and_then(|rest| rest.split(',').next())
                    .and_then(|d| d.trim().parse::<f64>().ok());
                let mut j = i + 1;
                while j < lines.len() && lines[j].starts_with('#') {
                    if let Some(stripped) = lines[j].strip_prefix("#EXT-X-BYTERANGE:") {
                        let r = parse_byte_range(stripped, next_offset);
                        next_offset = r.offset + r.length;
                        pending_range = Some(r);
                    }
                    j += 1;
                }
                if j < lines.len() {
                    segments.push(Resource {
                        url: resolve_url(base_url, lines[j]),
                        range: pending_range.take(),
                        duration: seg_duration,
                    });
                }
            }
        }
        M3u8Playlist::Media { segments, map }
    }
}
pub mod resolver {
    use super::{
        parser::parse_m3u8,
        types::{M3u8Playlist, Resource},
    };
    use crate::{common::types::AnyResult, sources::youtube::cipher::YouTubeCipherManager};
    use std::sync::Arc;
    pub async fn resolve_playlist(
        client: &reqwest::Client,
        url: &str,
    ) -> AnyResult<(Vec<Resource>, Option<Resource>)> {
        let text = fetch_text(client, url).await?;
        let playlist = parse_m3u8(&text, url);
        match playlist {
            M3u8Playlist::Master {
                variants,
                audio_groups,
            } => {
                let best = variants
                    .iter()
                    .filter(|v| v.is_audio_only)
                    .max_by_key(|v| v.bandwidth)
                    .or_else(|| {
                        variants
                            .iter()
                            .filter(|v| v.audio_group.is_some())
                            .max_by_key(|v| v.bandwidth)
                    })
                    .or_else(|| variants.iter().max_by_key(|v| v.bandwidth));
                match best {
                    Some(v) => {
                        if let Some(group_id) = &v.audio_group
                            && let Some(group) = audio_groups.get(group_id)
                        {
                            let rendition = group
                                .iter()
                                .find(|m| m.is_default)
                                .or_else(|| group.iter().find(|m| m.uri.is_some()))
                                .and_then(|m| m.uri.as_ref());
                            if let Some(uri) = rendition {
                                tracing::debug!(
                                    "HLS: selected audio group {} -> {}",
                                    group_id,
                                    uri
                                );
                                return Box::pin(resolve_playlist(client, uri)).await;
                            }
                        }
                        tracing::debug!(
                            "HLS: selected variant bw={} codecs={:?} audio_only={} audio_group={:?} url={}",
                            v.bandwidth,
                            v.codecs,
                            v.is_audio_only,
                            v.audio_group,
                            v.url
                        );
                        Box::pin(resolve_playlist(client, &v.url)).await
                    }
                    None => Err("HLS master playlist has no variants".into()),
                }
            }
            M3u8Playlist::Media { segments, map } => Ok((segments, map)),
        }
    }
    pub async fn fetch_text(client: &reqwest::Client, url: &str) -> AnyResult<String> {
        let res = client
            .get(url)
            .header("Accept", "application/x-mpegURL, */*")
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(format!("HLS playlist fetch failed {}: {}", res.status(), url).into());
        }
        let text = res.text().await?;
        Ok(text)
    }
    pub async fn resolve_url_string(
        url: &str,
        cipher_manager: &Option<Arc<YouTubeCipherManager>>,
        player_url: &Option<String>,
    ) -> AnyResult<String> {
        let (cipher, p_url) = match (cipher_manager, player_url) {
            (Some(c), Some(p)) => (c, p),
            _ => return Ok(url.to_string()),
        };
        let n_token = if let Some(pos) = url.find("/n/") {
            let rest = &url[pos + 3..];
            rest.split('/').next()
        } else {
            url.split("&n=")
                .nth(1)
                .or_else(|| url.split("?n=").nth(1))
                .and_then(|s| s.split('&').next())
        };
        if let Some(n) = n_token {
            let cipher = cipher.clone();
            let url_str = url.to_string();
            let p_url_str = p_url.clone();
            let n_str = n.to_string();
            Ok(cipher
                .resolve_url(&url_str, &p_url_str, Some(&n_str), None)
                .await?)
        } else {
            Ok(url.to_string())
        }
    }
}
pub mod ts_demux {
    const TS_PACKET_SIZE: usize = 188;
    const TS_SYNC_BYTE: u8 = 0x47;
    const PAT_PID: u16 = 0x0000;
    const STREAM_TYPE_AAC: u8 = 0x0F;
    const STREAM_TYPE_AAC_LATM: u8 = 0x11;
    pub fn extract_adts_from_ts(ts_data: &[u8]) -> Vec<u8> {
        let mut adts_out = Vec::with_capacity(ts_data.len() / 2);
        let mut pmt_pid: Option<u16> = None;
        let mut audio_pid: Option<u16> = None;
        let mut offset = 0;
        while offset < ts_data.len() && ts_data[offset] != TS_SYNC_BYTE {
            offset += 1;
        }
        while offset + TS_PACKET_SIZE <= ts_data.len() {
            let packet = &ts_data[offset..offset + TS_PACKET_SIZE];
            offset += TS_PACKET_SIZE;
            if packet[0] != TS_SYNC_BYTE {
                let remaining = &ts_data[offset..];
                if let Some(sync_pos) = remaining.iter().position(|&b| b == TS_SYNC_BYTE) {
                    offset += sync_pos;
                } else {
                    break;
                }
                continue;
            }
            let _transport_error = (packet[1] & 0x80) != 0;
            let payload_start = (packet[1] & 0x40) != 0;
            let pid = ((packet[1] as u16 & 0x1F) << 8) | packet[2] as u16;
            let adaptation_field_control = (packet[3] >> 4) & 0x03;
            if _transport_error {
                continue;
            }
            let mut payload_offset: usize = 4;
            if (adaptation_field_control == 2 || adaptation_field_control == 3)
                && payload_offset < TS_PACKET_SIZE
            {
                let adaptation_length = packet[payload_offset] as usize;
                payload_offset += 1 + adaptation_length;
            }
            if adaptation_field_control == 0 || adaptation_field_control == 2 {
                continue;
            }
            if payload_offset >= TS_PACKET_SIZE {
                continue;
            }
            let payload = &packet[payload_offset..];
            if pid == PAT_PID {
                if let Some(pid) = parse_pat(payload, payload_start) {
                    pmt_pid = Some(pid);
                }
                continue;
            }
            if Some(pid) == pmt_pid {
                if let Some(pid) = parse_pmt(payload, payload_start) {
                    audio_pid = Some(pid);
                }
                continue;
            }
            if Some(pid) == audio_pid {
                extract_pes_payload(payload, payload_start, &mut adts_out);
            }
        }
        adts_out
    }
    fn parse_pat(payload: &[u8], payload_start: bool) -> Option<u16> {
        let data = if payload_start && !payload.is_empty() {
            let pointer = payload[0] as usize;
            if 1 + pointer >= payload.len() {
                return None;
            }
            &payload[1 + pointer..]
        } else {
            payload
        };
        if data.len() < 8 {
            return None;
        }
        let _table_id = data[0];
        let section_length = ((data[1] as usize & 0x0F) << 8) | data[2] as usize;
        let header_size = 8;
        let entries_end = std::cmp::min(header_size + section_length.saturating_sub(5), data.len());
        let mut pos = header_size;
        while pos + 4 <= entries_end {
            let program_number = ((data[pos] as u16) << 8) | data[pos + 1] as u16;
            let pid = ((data[pos + 2] as u16 & 0x1F) << 8) | data[pos + 3] as u16;
            if program_number != 0 {
                return Some(pid);
            }
            pos += 4;
        }
        None
    }
    fn parse_pmt(payload: &[u8], payload_start: bool) -> Option<u16> {
        let data = if payload_start && !payload.is_empty() {
            let pointer = payload[0] as usize;
            if 1 + pointer >= payload.len() {
                return None;
            }
            &payload[1 + pointer..]
        } else {
            payload
        };
        if data.len() < 12 {
            return None;
        }
        let section_length = ((data[1] as usize & 0x0F) << 8) | data[2] as usize;
        let program_info_length = ((data[10] as usize & 0x0F) << 8) | data[11] as usize;
        let mut pos = 12 + program_info_length;
        let section_end = std::cmp::min(3 + section_length.saturating_sub(4), data.len());
        while pos + 5 <= section_end {
            let stream_type = data[pos];
            let elementary_pid = ((data[pos + 1] as u16 & 0x1F) << 8) | data[pos + 2] as u16;
            let es_info_length = ((data[pos + 3] as usize & 0x0F) << 8) | data[pos + 4] as usize;
            if stream_type == STREAM_TYPE_AAC || stream_type == STREAM_TYPE_AAC_LATM {
                return Some(elementary_pid);
            }
            if stream_type == 0x03 || stream_type == 0x04 {
                return Some(elementary_pid);
            }
            pos += 5 + es_info_length;
        }
        None
    }
    fn extract_pes_payload(payload: &[u8], payload_start: bool, out: &mut Vec<u8>) {
        if payload_start {
            if payload.len() < 9 {
                return;
            }
            if payload[0] != 0x00 || payload[1] != 0x00 || payload[2] != 0x01 {
                out.extend_from_slice(payload);
                return;
            }
            let header_data_length = payload[8] as usize;
            let pes_header_size = 9 + header_data_length;
            if pes_header_size < payload.len() {
                out.extend_from_slice(&payload[pes_header_size..]);
            }
        } else {
            out.extend_from_slice(payload);
        }
    }
}
pub mod types {
    use std::collections::HashMap;
    #[derive(Clone, Debug)]
    pub struct ByteRange {
        pub length: u64,
        pub offset: u64,
    }
    #[derive(Clone, Debug)]
    pub struct Resource {
        pub url: String,
        pub range: Option<ByteRange>,
        pub duration: Option<f64>,
    }
    pub struct Variant {
        pub url: String,
        pub bandwidth: u64,
        pub codecs: String,
        pub is_audio_only: bool,
        pub audio_group: Option<String>,
    }
    pub struct Media {
        pub _type: String,
        pub _group_id: String,
        pub uri: Option<String>,
        pub is_default: bool,
    }
    pub enum M3u8Playlist {
        Master {
            variants: Vec<Variant>,
            audio_groups: HashMap<String, Vec<Media>>,
        },
        Media {
            segments: Vec<Resource>,
            map: Option<Resource>,
        },
    }
}
pub mod utils {
    use super::types::ByteRange;
    pub fn extract_attr_u64(line: &str, key: &str) -> Option<u64> {
        extract_attr_str(line, key)?.parse().ok()
    }
    pub fn extract_attr_str(line: &str, key: &str) -> Option<String> {
        let key_eq = format!("{}=", key);
        let pos = line
            .find(&format!(":{}", key_eq))
            .map(|p| p + 1)
            .or_else(|| line.find(&format!(",{}", key_eq)).map(|p| p + 1))?;
        let rest = &line[pos + key_eq.len()..];
        if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"')?;
            Some(stripped[..end].to_string())
        } else {
            let end = rest.find(',').unwrap_or(rest.len());
            Some(rest[..end].trim().to_string())
        }
    }
    pub fn resolve_url(base: &str, maybe_relative: &str) -> String {
        if maybe_relative.starts_with("http://") || maybe_relative.starts_with("https://") {
            return maybe_relative.to_string();
        }
        let base_clean = base.split('?').next().unwrap_or(base);
        let base_clean = base_clean.split('#').next().unwrap_or(base_clean);
        if maybe_relative.starts_with('/')
            && let Some(scheme_end) = base_clean.find("://")
        {
            let host_start = scheme_end + 3;
            let host_end = base_clean[host_start..]
                .find('/')
                .map(|p| host_start + p)
                .unwrap_or(base_clean.len());
            return format!("{}{}", &base_clean[..host_end], maybe_relative);
        }
        let base_dir = base_clean
            .rfind('/')
            .map(|i| &base_clean[..=i])
            .unwrap_or(base_clean);
        format!("{}{}", base_dir, maybe_relative)
    }
    pub fn parse_byte_range(attr: &str, last_end_offset: u64) -> ByteRange {
        let attr = attr.trim().trim_matches('"');
        let parts: Vec<&str> = attr.split('@').collect();
        let length = parts[0].trim().parse::<u64>().unwrap_or(0);
        let offset = if parts.len() > 1 {
            parts[1].trim().parse::<u64>().unwrap_or(0)
        } else {
            last_end_offset
        };
        ByteRange { length, offset }
    }
}
use self::{
    fetcher::fetch_segment_into,
    resolver::{resolve_playlist, resolve_url_string},
    ts_demux::extract_adts_from_ts,
    types::Resource,
};
use crate::common::types::AnyResult;
use crate::{config::HttpProxyConfig, sources::youtube::cipher::YouTubeCipherManager};
use parking_lot::{Condvar, Mutex};
use std::{
    io::{self, Read, Seek, SeekFrom},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use symphonia::core::io::MediaSource;
const PREFETCH_SEGMENTS: usize = 4;
const LOW_WATER_BYTES: usize = 512 * 1024;
enum PrefetchCommand {
    Continue,
    Seek(usize),
    Stop,
}
struct SharedState {
    next_buf: Vec<u8>,
    need_data: bool,
    pending: Vec<Resource>,
    current_segment_index: usize,
    command: PrefetchCommand,
    seek_done: bool,
    eos: bool,
}
pub struct HlsReader {
    buf: Vec<u8>,
    pos: usize,
    shared: Arc<(Mutex<SharedState>, Condvar)>,
    abort_flag: Arc<AtomicBool>,
    bg_thread: Option<std::thread::JoinHandle<()>>,
    all_segments: Vec<Resource>,
    segment_durations: Vec<f64>,
    has_durations: bool,
}
impl Drop for HlsReader {
    fn drop(&mut self) {
        self.abort_flag.store(true, Ordering::Relaxed);
        {
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            state.command = PrefetchCommand::Stop;
            state.need_data = true;
            cvar.notify_one();
        }
        if let Some(handle) = self.bg_thread.take() {
            let _ = handle.join();
        }
    }
}
impl HlsReader {
    pub async fn new(
        manifest_url: &str,
        local_addr: Option<std::net::IpAddr>,
        cipher_manager: Option<Arc<YouTubeCipherManager>>,
        player_url: Option<String>,
        proxy: Option<HttpProxyConfig>,
    ) -> AnyResult<Self> {
        let mut builder = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15));
        if let Some(ip) = local_addr {
            builder = builder.local_address(ip);
        }
        if let Some(ref proxy_cfg) = proxy
            && let Some(ref proxy_url) = proxy_cfg.url
        {
            tracing::debug!("HLS: configuring proxy: {}", proxy_url);
            let mut proxy_obj = reqwest::Proxy::all(proxy_url)?;
            if let (Some(user), Some(pass)) = (&proxy_cfg.username, &proxy_cfg.password) {
                proxy_obj = proxy_obj.basic_auth(user, pass);
            }
            builder = builder.proxy(proxy_obj);
        }
        let client: reqwest::Client = builder.build()?;
        let (segment_urls, map_url) = resolve_playlist(&client, manifest_url).await?;
        if segment_urls.is_empty() {
            return Err("HLS playlist contained no segments".into());
        }
        let segment_durations: Vec<f64> = segment_urls
            .iter()
            .map(|r| r.duration.unwrap_or(0.0))
            .collect();
        let has_durations = segment_durations.iter().any(|&d| d > 0.0);
        let all_segments = segment_urls.clone();
        let mut initial_buf = Vec::with_capacity(512 * 1024);
        let mut cached_map_data = None;
        if let Some(map_res) = &map_url {
            let resolved = resolve_resource_static(map_res, &cipher_manager, &player_url).await?;
            let mut map_data = Vec::new();
            fetch_segment_into(&client, &resolved, &mut map_data).await?;
            initial_buf.extend_from_slice(&map_data);
            cached_map_data = Some(map_data);
        }
        let first_batch_count = 1_usize.min(segment_urls.len());
        let mut pending = segment_urls;
        let first_batch: Vec<Resource> = pending.drain(..first_batch_count).collect();
        for res in &first_batch {
            let resolved = resolve_resource_static(res, &cipher_manager, &player_url).await?;
            fetch_and_demux_into(&client, &resolved, &mut initial_buf).await?;
        }
        let current_segment_index = first_batch.len();
        let shared_state = SharedState {
            next_buf: Vec::with_capacity(512 * 1024),
            need_data: true,
            pending,
            current_segment_index,
            command: PrefetchCommand::Continue,
            seek_done: false,
            eos: false,
        };
        let shared = Arc::new((Mutex::new(shared_state), Condvar::new()));
        let shared_bg = Arc::clone(&shared);
        let bg_client = client;
        let bg_cipher = cipher_manager;
        let bg_player_url = player_url;
        let bg_cached_map = cached_map_data;
        let bg_all_segments = all_segments.clone();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let abort_flag_bg = Arc::clone(&abort_flag);
        let bg_thread = std::thread::Builder::new()
            .name("hls-prefetch".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(prefetch_loop(
                    shared_bg,
                    abort_flag_bg,
                    bg_client,
                    bg_cipher,
                    bg_player_url,
                    bg_cached_map,
                    bg_all_segments,
                ));
            })
            .expect("failed to spawn HLS prefetch thread");
        Ok(Self {
            buf: initial_buf,
            pos: 0,
            shared,
            abort_flag,
            bg_thread: Some(bg_thread),
            all_segments,
            segment_durations,
            has_durations,
        })
    }
    fn seek_to_ms(&mut self, position_ms: u64) -> io::Result<u64> {
        if !self.has_durations {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "HLS streams without segment durations are not seekable",
            ));
        }
        let target_secs = position_ms as f64 / 1000.0;
        let mut elapsed = 0.0;
        let mut target_index = 0;
        for (i, &dur) in self.segment_durations.iter().enumerate() {
            if elapsed + dur <= target_secs {
                elapsed += dur;
                target_index = i + 1;
            } else {
                break;
            }
        }
        if target_index >= self.all_segments.len() {
            target_index = self.all_segments.len().saturating_sub(1);
        }
        tracing::debug!(
            "HLS seek to {}ms -> segment {} (elapsed {:.1}s)",
            position_ms,
            target_index,
            elapsed
        );
        self.buf.clear();
        self.pos = 0;
        {
            let (lock, cvar) = &*self.shared;
            self.abort_flag.store(true, Ordering::Relaxed);
            let mut state = lock.lock();
            state.command = PrefetchCommand::Seek(target_index);
            state.need_data = true;
            state.seek_done = false;
            cvar.notify_one();
            while !state.seek_done {
                cvar.wait(&mut state);
            }
            state.seek_done = false;
            self.abort_flag.store(false, Ordering::Relaxed);
            std::mem::swap(&mut self.buf, &mut state.next_buf);
            state.next_buf.clear();
            state.need_data = true;
            cvar.notify_one();
        }
        Ok(0)
    }
}
impl Read for HlsReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.pos < self.buf.len() {
            let remaining = self.buf.len() - self.pos;
            if remaining <= LOW_WATER_BYTES {
                let (lock, cvar) = &*self.shared;
                if let Some(mut state) = lock.try_lock()
                    && !state.need_data
                    && !state.eos
                {
                    state.need_data = true;
                    cvar.notify_one();
                }
            }
            let n = out.len().min(remaining);
            out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
            self.pos += n;
            return Ok(n);
        }
        let (lock, cvar) = &*self.shared;
        let mut state = lock.lock();
        if !state.need_data && state.next_buf.is_empty() && !state.eos {
            state.need_data = true;
            cvar.notify_one();
        }
        while state.next_buf.is_empty() && !state.eos {
            cvar.wait(&mut state);
        }
        if state.next_buf.is_empty() && state.eos {
            return Ok(0);
        }
        self.buf.clear();
        self.pos = 0;
        std::mem::swap(&mut self.buf, &mut state.next_buf);
        state.next_buf.clear();
        state.need_data = true;
        cvar.notify_one();
        drop(state);
        let available = &self.buf[self.pos..];
        let n = out.len().min(available.len());
        out[..n].copy_from_slice(&available[..n]);
        self.pos += n;
        Ok(n)
    }
}
impl Seek for HlsReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(ms) => self.seek_to_ms(ms),
            _ => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "HLS seek only supports SeekFrom::Start (milliseconds)",
            )),
        }
    }
}
impl MediaSource for HlsReader {
    fn is_seekable(&self) -> bool {
        self.has_durations
    }
    fn byte_len(&self) -> Option<u64> {
        None
    }
}
#[allow(clippy::too_many_arguments)]
async fn prefetch_loop(
    shared: Arc<(Mutex<SharedState>, Condvar)>,
    abort_flag: Arc<AtomicBool>,
    client: reqwest::Client,
    cipher_manager: Option<Arc<YouTubeCipherManager>>,
    player_url: Option<String>,
    cached_map_data: Option<Vec<u8>>,
    all_segments: Vec<Resource>,
) {
    let (lock, cvar) = &*shared;
    loop {
        enum Action {
            Stop,
            Seek {
                batch: Vec<Resource>,
            },
            Fetch {
                batch: Vec<Resource>,
                seg_idx: usize,
            },
            Eos,
        }
        let action = {
            let mut state = lock.lock();
            while !state.need_data {
                cvar.wait(&mut state);
            }
            match std::mem::replace(&mut state.command, PrefetchCommand::Continue) {
                PrefetchCommand::Stop => Action::Stop,
                PrefetchCommand::Seek(target_index) => {
                    state.next_buf.clear();
                    state.eos = false;
                    state.current_segment_index = target_index;
                    state.pending = all_segments[target_index..].to_vec();
                    if let Some(map_data) = &cached_map_data {
                        state.next_buf.extend_from_slice(map_data);
                    }
                    let count = if !state.pending.is_empty() { 1 } else { 0 };
                    let batch = state.pending.drain(..count).collect();
                    Action::Seek { batch }
                }
                PrefetchCommand::Continue => {
                    if state.pending.is_empty() {
                        state.eos = true;
                        state.need_data = false;
                        cvar.notify_one();
                        Action::Eos
                    } else {
                        let count = PREFETCH_SEGMENTS.min(state.pending.len());
                        let batch = state.pending.drain(..count).collect();
                        let seg_idx = state.current_segment_index;
                        Action::Fetch { batch, seg_idx }
                    }
                }
            }
        };
        match action {
            Action::Stop => return,
            Action::Eos => continue,
            Action::Seek { batch } => {
                let mut tmp_buf = Vec::with_capacity(256 * 1024);
                for res in &batch {
                    if let Ok(resolved) =
                        resolve_resource_static(res, &cipher_manager, &player_url).await
                        && let Err(e) = fetch_and_demux_into(&client, &resolved, &mut tmp_buf).await
                    {
                        tracing::warn!("HLS prefetch: segment fetch error during seek: {}", e);
                    }
                }
                let mut state = lock.lock();
                state.next_buf.extend_from_slice(&tmp_buf);
                state.current_segment_index += batch.len();
                state.need_data = false;
                state.seek_done = true;
                state.eos = state.pending.is_empty();
                cvar.notify_one();
            }
            Action::Fetch { batch, seg_idx } => {
                let mut tmp_buf = Vec::with_capacity(256 * 1024);
                for res in &batch {
                    if abort_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(resolved) =
                        resolve_resource_static(res, &cipher_manager, &player_url).await
                        && let Err(e) = fetch_and_demux_into(&client, &resolved, &mut tmp_buf).await
                    {
                        tracing::warn!("HLS prefetch: segment fetch error: {}", e);
                    }
                }
                let mut state = lock.lock();
                if !matches!(state.command, PrefetchCommand::Continue) {
                    continue;
                }
                state.next_buf.extend_from_slice(&tmp_buf);
                state.current_segment_index = seg_idx + batch.len();
                state.eos = state.pending.is_empty();
                state.need_data = false;
                cvar.notify_one();
            }
        }
    }
}
async fn resolve_resource_static(
    res: &Resource,
    cipher_manager: &Option<Arc<YouTubeCipherManager>>,
    player_url: &Option<String>,
) -> AnyResult<Resource> {
    let mut resolved = res.clone();
    resolved.url = resolve_url_string(&res.url, cipher_manager, player_url).await?;
    Ok(resolved)
}
async fn fetch_and_demux_into(
    client: &reqwest::Client,
    res: &Resource,
    out: &mut Vec<u8>,
) -> AnyResult<()> {
    let mut raw = Vec::new();
    fetch_segment_into(client, res, &mut raw).await?;
    let is_ts = raw.first() == Some(&0x47);
    if is_ts {
        let adts = extract_adts_from_ts(&raw);
        if !adts.is_empty() {
            out.extend_from_slice(&adts);
        } else {
            tracing::warn!("HLS: TS demux produced no output, using raw segment");
            out.extend_from_slice(&raw);
        }
    } else {
        out.extend_from_slice(&raw);
    }
    Ok(())
}
