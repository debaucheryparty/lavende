pub mod codec {
    pub mod io {
        use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
        use std::io::{self, Cursor, Read, Write};
        pub const V1: u8 = 1;
        pub const V2: u8 = 2;
        pub const V3: u8 = 3;
        pub struct BinaryBuffer<T>(Cursor<T>);
        impl BinaryBuffer<Vec<u8>> {
            pub fn new() -> Self {
                Self(Cursor::new(Vec::new()))
            }
            pub fn with_capacity(capacity: usize) -> Self {
                Self(Cursor::new(Vec::with_capacity(capacity)))
            }
            pub fn into_inner(self) -> Vec<u8> {
                self.0.into_inner()
            }
        }
        impl Default for BinaryBuffer<Vec<u8>> {
            fn default() -> Self {
                Self::new()
            }
        }
        impl<T: AsRef<[u8]>> BinaryBuffer<T> {
            pub fn from_data(data: T) -> Self {
                Self(Cursor::new(data))
            }
            pub fn read_string(&mut self) -> io::Result<String> {
                let size = self.0.read_u16::<BigEndian>()? as usize;
                let mut content = vec![0u8; size];
                self.0.read_exact(&mut content)?;
                String::from_utf8(content)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            }
            pub fn read_nullable_string(&mut self) -> io::Result<Option<String>> {
                if self.0.read_u8()? != 0 {
                    self.read_string().map(Some)
                } else {
                    Ok(None)
                }
            }
            pub fn read_json<V: serde::de::DeserializeOwned>(&mut self) -> io::Result<V> {
                let s = self.read_string()?;
                serde_json::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            }
        }
        impl BinaryBuffer<Vec<u8>> {
            pub fn write_string(&mut self, text: &str) -> io::Result<()> {
                let raw = text.as_bytes();
                self.0.write_u16::<BigEndian>(raw.len() as u16)?;
                self.0.write_all(raw)
            }
            pub fn write_nullable_string(&mut self, text: Option<&str>) -> io::Result<()> {
                match text {
                    Some(s) => {
                        self.0.write_u8(1)?;
                        self.write_string(s)
                    }
                    None => self.0.write_u8(0),
                }
            }
            pub fn write_json<V: serde::Serialize>(&mut self, value: &V) -> io::Result<()> {
                let s = serde_json::to_string(value).map_err(io::Error::other)?;
                self.write_string(&s)
            }
        }
        impl<T> std::ops::Deref for BinaryBuffer<T> {
            type Target = Cursor<T>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl<T> std::ops::DerefMut for BinaryBuffer<T> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    }
    pub mod decode {
        use crate::protocol::{
            CodecError, PlaylistInfo, Track, TrackInfo,
            codec::io::{BinaryBuffer, V1, V2, V3},
        };
        use base64::{Engine, prelude::BASE64_STANDARD};
        use byteorder::{BigEndian, ReadBytesExt};
        pub fn decode_track(encoded: &str) -> Result<Track, CodecError> {
            if encoded.is_empty() {
                return Err(CodecError::EmptyInput);
            }
            let raw_payload = BASE64_STANDARD.decode(encoded)?;
            if raw_payload.len() < 4 {
                return Err(CodecError::TruncatedBuffer("header".into()));
            }
            let mut stream = BinaryBuffer::from_data(raw_payload);
            let envelope = stream.read_u32::<BigEndian>()?;
            let flags = (envelope >> 30) & 0x03;
            let ver = if flags & 1 != 0 {
                stream.read_u8()?
            } else {
                V1
            };
            if !(V1..=V3).contains(&ver) {
                return Err(CodecError::UnknownVersion(ver));
            }
            let title = stream.read_string()?;
            let author = stream.read_string()?;
            let length = stream.read_u64::<BigEndian>()?;
            let identifier = stream.read_string()?;
            let is_stream = stream.read_u8()? != 0;
            let (uri, artwork_url, isrc) = match ver {
                V2 => (stream.read_nullable_string()?, None, None),
                V3 => (
                    stream.read_nullable_string()?,
                    stream.read_nullable_string()?,
                    stream.read_nullable_string()?,
                ),
                _ => (None, None, None),
            };
            let source_name = stream.read_string()?;
            let position = stream.read_u64::<BigEndian>()?;
            let user_data = stream.read_json().unwrap_or_else(|_| serde_json::json!({}));
            Ok(Track {
                encoded: encoded.to_owned(),
                info: TrackInfo {
                    identifier,
                    is_seekable: !is_stream,
                    author,
                    length,
                    is_stream,
                    position,
                    title,
                    uri,
                    artwork_url,
                    isrc,
                    source_name,
                },
                plugin_info: serde_json::json!({}),
                user_data,
            })
        }
        pub fn decode_playlist_info(encoded: &str) -> Result<PlaylistInfo, CodecError> {
            let raw = BASE64_STANDARD.decode(encoded)?;
            let mut stream = BinaryBuffer::from_data(raw);
            Ok(PlaylistInfo {
                name: stream.read_string()?,
                selected_track: stream.read_i32::<BigEndian>()?,
            })
        }
    }
    pub mod encode {
        use crate::protocol::{
            CodecError, PlaylistInfo, TrackInfo,
            codec::io::{BinaryBuffer, V3},
        };
        use base64::{Engine, prelude::BASE64_STANDARD};
        use byteorder::{BigEndian, WriteBytesExt};
        use std::io::Write;
        pub fn encode_track(
            metadata: &TrackInfo,
            user_data: &serde_json::Value,
        ) -> Result<String, CodecError> {
            let mut blob = BinaryBuffer::with_capacity(128);
            blob.write_u8(V3)?;
            blob.write_string(&metadata.title)?;
            blob.write_string(&metadata.author)?;
            blob.write_u64::<BigEndian>(metadata.length)?;
            blob.write_string(&metadata.identifier)?;
            blob.write_u8(u8::from(metadata.is_stream))?;
            blob.write_nullable_string(metadata.uri.as_deref())?;
            blob.write_nullable_string(metadata.artwork_url.as_deref())?;
            blob.write_nullable_string(metadata.isrc.as_deref())?;
            blob.write_string(&metadata.source_name)?;
            blob.write_u64::<BigEndian>(metadata.position)?;
            if let Some(obj) = user_data.as_object() {
                if !obj.is_empty() {
                    blob.write_json(user_data)?;
                }
            } else if !user_data.is_null() {
                blob.write_json(user_data)?;
            }
            let inner = blob.into_inner();
            let header = (inner.len() as u32) | (1u32 << 30);
            let mut out = Vec::with_capacity(4 + inner.len());
            out.write_u32::<BigEndian>(header)?;
            out.write_all(&inner)?;
            Ok(BASE64_STANDARD.encode(&out))
        }
        pub fn encode_playlist_info(info: &PlaylistInfo) -> Result<String, CodecError> {
            let mut blob = BinaryBuffer::with_capacity(64);
            blob.write_string(&info.name)?;
            blob.write_i32::<BigEndian>(info.selected_track)?;
            Ok(BASE64_STANDARD.encode(blob.into_inner()))
        }
    }
    pub use decode::{decode_playlist_info, decode_track};
    pub use encode::{encode_playlist_info, encode_track};
}
pub mod info {
    use serde::Serialize;
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Info {
        pub version: Version,
        pub build_time: u64,
        pub git: GitInfo,
        pub jvm: String,
        pub lavaplayer: String,
        pub source_managers: Vec<String>,
        pub filters: Vec<String>,
        pub plugins: Vec<Plugin>,
    }
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Version {
        pub semver: String,
        pub major: u32,
        pub minor: u32,
        pub patch: u32,
        pub pre_release: Option<String>,
    }
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GitInfo {
        pub branch: String,
        pub commit: String,
        pub commit_time: u64,
    }
    #[derive(Debug, Serialize)]
    pub struct Plugin {
        pub name: String,
        pub version: String,
    }
}
pub mod stats {
    use serde::Serialize;
    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Stats {
        pub players: u64,
        pub playing_players: u64,
        pub uptime: u64,
        pub memory: Memory,
        pub cpu: Cpu,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub frame_stats: Option<FrameStats>,
    }
    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Memory {
        pub free: u64,
        pub used: u64,
        pub allocated: u64,
        pub reservable: u64,
    }
    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Cpu {
        pub cores: i32,
        pub system_load: f64,
        pub lavalink_load: f64,
    }
    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FrameStats {
        pub sent: i32,
        pub nulled: i32,
        pub deficit: i32,
    }
}
pub mod routeplanner {
    use serde::Serialize;
    #[derive(Debug, Serialize, Clone)]
    #[serde(tag = "class", content = "details")]
    pub enum RoutePlannerStatus {
        RotatingIpRoutePlanner(RotatingIpDetails),
        NanoIpRoutePlanner(NanoIpDetails),
        RotatingNanoIpRoutePlanner(RotatingNanoIpDetails),
        BalancingIpRoutePlanner(BalancingIpDetails),
    }
    #[derive(Debug, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct RotatingIpDetails {
        pub ip_block: IpBlock,
        pub failing_addresses: Vec<FailingAddress>,
        pub rotate_index: String,
        pub ip_index: String,
        pub current_address: String,
    }
    #[derive(Debug, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct NanoIpDetails {
        pub ip_block: IpBlock,
        pub failing_addresses: Vec<FailingAddress>,
        pub current_address: String,
    }
    #[derive(Debug, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct RotatingNanoIpDetails {
        pub ip_block: IpBlock,
        pub failing_addresses: Vec<FailingAddress>,
        pub block_index: String,
        pub current_address_index: String,
    }
    #[derive(Debug, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct BalancingIpDetails {
        pub ip_block: IpBlock,
        pub failing_addresses: Vec<FailingAddress>,
    }
    #[derive(Debug, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct IpBlock {
        #[serde(rename = "type")]
        pub block_type: String,
        pub size: String,
    }
    #[derive(Debug, Serialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct FailingAddress {
        pub failing_address: String,
        pub failing_timestamp: u64,
        pub failing_time: String,
    }
    #[derive(Debug, serde::Deserialize)]
    pub struct FreeAddressRequest {
        pub address: String,
    }
}
pub mod tracks {
    use crate::protocol::codec::{decode_track, encode_track};
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Track {
        pub encoded: String,
        pub info: TrackInfo,
        #[serde(default = "serde_json::Value::default")]
        pub plugin_info: serde_json::Value,
        #[serde(default = "serde_json::Value::default")]
        pub user_data: serde_json::Value,
    }
    impl Track {
        pub fn new(info: TrackInfo) -> Self {
            let mut track = Self {
                encoded: String::new(),
                info,
                plugin_info: serde_json::json!({}),
                user_data: serde_json::json!({}),
            };
            track.encoded = track.encode();
            track
        }
        pub fn encode(&self) -> String {
            encode_track(&self.info, &self.user_data).unwrap_or_else(|_| self.encoded.clone())
        }
        pub fn decode(encoded: &str) -> Option<Self> {
            decode_track(encoded).ok()
        }
    }
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct TrackInfo {
        pub identifier: String,
        pub is_seekable: bool,
        pub author: String,
        pub length: u64,
        pub is_stream: bool,
        pub position: u64,
        pub title: String,
        pub uri: Option<String>,
        pub artwork_url: Option<String>,
        pub isrc: Option<String>,
        pub source_name: String,
    }
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "loadType", content = "data", rename_all = "camelCase")]
    pub enum LoadResult {
        Track(Track),
        Playlist(PlaylistData),
        Search(Vec<Track>),
        Empty {},
        Error(LoadError),
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PlaylistData {
        pub info: PlaylistInfo,
        pub plugin_info: serde_json::Value,
        pub tracks: Vec<Track>,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TextData {
        pub text: String,
        pub plugin: serde_json::Value,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SearchResult {
        pub tracks: Vec<Track>,
        pub albums: Vec<PlaylistData>,
        pub artists: Vec<PlaylistData>,
        pub playlists: Vec<PlaylistData>,
        pub texts: Vec<TextData>,
        pub plugin: serde_json::Value,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PlaylistInfo {
        pub name: String,
        pub selected_track: i32,
    }
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LoadError {
        pub message: Option<String>,
        pub severity: crate::common::Severity,
        pub cause: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub cause_stack_trace: Option<String>,
    }
}
pub mod events {
    use crate::protocol::tracks::Track;
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PlayerState {
        pub time: u64,
        pub position: u64,
        pub connected: bool,
        pub ping: i64,
    }
    #[derive(Debug, Serialize)]
    #[serde(tag = "op", rename_all = "camelCase")]
    pub enum OutgoingMessage {
        Ready {
            resumed: bool,
            #[serde(rename = "sessionId")]
            session_id: crate::common::types::SessionId,
        },
        #[serde(rename = "playerUpdate")]
        PlayerUpdate {
            #[serde(rename = "guildId")]
            guild_id: crate::common::types::GuildId,
            state: PlayerState,
        },
        #[serde(rename = "stats")]
        Stats {
            #[serde(flatten)]
            stats: super::stats::Stats,
        },
        #[serde(rename = "event")]
        Event {
            #[serde(flatten)]
            event: Box<RustalinkEvent>,
        },
    }
    #[derive(Debug, Serialize)]
    #[serde(tag = "type", rename_all = "camelCase")]
    pub enum RustalinkEvent {
        #[serde(rename = "TrackStartEvent")]
        TrackStart {
            #[serde(rename = "guildId")]
            guild_id: crate::common::types::GuildId,
            track: Track,
        },
        #[serde(rename = "TrackEndEvent")]
        TrackEnd {
            #[serde(rename = "guildId")]
            guild_id: crate::common::types::GuildId,
            track: Track,
            reason: TrackEndReason,
        },
        #[serde(rename = "TrackExceptionEvent")]
        TrackException {
            #[serde(rename = "guildId")]
            guild_id: crate::common::types::GuildId,
            track: Track,
            exception: TrackException,
        },
        #[serde(rename = "TrackStuckEvent")]
        TrackStuck {
            #[serde(rename = "guildId")]
            guild_id: crate::common::types::GuildId,
            track: Track,
            #[serde(rename = "thresholdMs")]
            threshold_ms: u64,
        },
        #[serde(rename = "LyricsFoundEvent")]
        LyricsFound {
            #[serde(rename = "guildId")]
            guild_id: crate::common::types::GuildId,
            lyrics: super::models::RustalinkLyrics,
        },
        #[serde(rename = "LyricsNotFoundEvent")]
        LyricsNotFound {
            #[serde(rename = "guildId")]
            guild_id: crate::common::types::GuildId,
        },
        #[serde(rename = "LyricsLineEvent")]
        LyricsLine {
            #[serde(rename = "guildId")]
            guild_id: crate::common::types::GuildId,
            line_index: i32,
            line: super::models::RustalinkLyricsLine,
            skipped: bool,
        },
        #[serde(rename = "WebSocketClosedEvent")]
        WebSocketClosed {
            #[serde(rename = "guildId")]
            guild_id: crate::common::types::GuildId,
            code: u16,
            reason: String,
            #[serde(rename = "byRemote")]
            by_remote: bool,
        },
    }
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum TrackEndReason {
        #[serde(rename = "finished")]
        Finished,
        #[serde(rename = "loadFailed")]
        LoadFailed,
        #[serde(rename = "stopped")]
        Stopped,
        #[serde(rename = "replaced")]
        Replaced,
        #[serde(rename = "cleanup")]
        Cleanup,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TrackException {
        pub message: Option<String>,
        pub severity: crate::common::Severity,
        pub cause: String,
        pub cause_stack_trace: Option<String>,
    }
}
pub mod models {
    use serde::{Deserialize, Serialize};
    #[derive(Deserialize)]
    pub struct LoadTracksQuery {
        pub identifier: String,
    }
    #[derive(Deserialize)]
    pub struct LoadSearchQuery {
        pub query: String,
        pub types: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DecodeTrackQuery {
        pub encoded_track: Option<String>,
        pub track: Option<String>,
    }
    #[derive(Deserialize)]
    pub struct EncodedTracks(pub Vec<String>);
    #[derive(Serialize)]
    pub struct Tracks {
        pub tracks: Vec<crate::protocol::tracks::Track>,
    }
    #[derive(Serialize)]
    pub struct Exception {
        pub message: String,
        pub severity: String,
        pub cause: String,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct LyricsLine {
        pub text: String,
        pub timestamp: u64,
        pub duration: u64,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct LyricsData {
        pub name: String,
        pub author: String,
        pub provider: String,
        pub text: String,
        pub lines: Option<Vec<LyricsLine>>,
    }
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "loadType", content = "data", rename_all = "camelCase")]
    pub enum LyricsLoadResult {
        Lyrics(LyricsResultData),
        Text(LyricsTextData),
        Empty {},
        Error(LyricsLoadError),
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LyricsResultData {
        pub name: String,
        pub synced: bool,
        pub lines: Vec<LyricsLine>,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LyricsTextData {
        pub text: String,
    }
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LyricsLoadError {
        pub message: String,
        pub severity: crate::common::Severity,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct RustalinkLyrics {
        pub source_name: String,
        pub provider: Option<String>,
        pub text: Option<String>,
        pub lines: Option<Vec<RustalinkLyricsLine>>,
        pub plugin: serde_json::Value,
    }
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct RustalinkLyricsLine {
        pub timestamp: u64,
        pub duration: Option<u64>,
        pub line: String,
        pub plugin: serde_json::Value,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetLyricsQuery {
        pub track: String,
        #[serde(default)]
        pub skip_track_source: bool,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GetPlayerLyricsQuery {
        #[serde(default)]
        pub skip_track_source: bool,
    }
}
use thiserror::Error;
#[derive(Debug, Error)]
pub enum CodecError {
    #[error("Empty input")]
    EmptyInput,
    #[error("Corrupt buffer: {0}")]
    CorruptBuffer(String),
    #[error("Truncated buffer: {0}")]
    TruncatedBuffer(String),
    #[error("Unknown track version: {0}")]
    UnknownVersion(u8),
    #[error("Base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}
pub use codec::*;
pub use events::*;
pub use info::*;
pub use models::*;
pub use routeplanner::*;
pub use stats::*;
pub use tracks::*;
