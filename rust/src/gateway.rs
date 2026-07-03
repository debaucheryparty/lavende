pub mod constants {
pub const VOICE_GATEWAY_VERSION: u8 = 8;
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;
pub const DAVE_INITIAL_VERSION: u16 = 1;
pub const DEFAULT_VOICE_MODE: &str = "xsalsa20_poly1305";
pub const MAX_RECONNECT_ATTEMPTS: u32 = 15;
pub const BACKOFF_BASE_MS: u64 = 1_000;
pub const RECONNECT_DELAY_FRESH_MS: u64 = 3000;
pub const UDP_KEEPALIVE_GAP_MS: u64 = 5000;
pub const WRITE_TASK_SHUTDOWN_MS: u64 = 500;
pub const RTP_VERSION_BYTE: u8 = 0x80;
pub const RTP_OPUS_PAYLOAD_TYPE: u8 = 0x78;
pub const RTP_TIMESTAMP_STEP: u32 = 960;
pub const FRAME_DURATION_MS: u64 = 20;
pub const PCM_FRAME_SAMPLES: usize = 960;
pub const MAX_OPUS_FRAME_SIZE: usize = 4000;
pub const SILENCE_FRAME: [u8; 3] = [0xf8, 0xff, 0xfe];
pub const MAX_SILENCE_FRAMES: u32 = 5;
pub const UDP_PACKET_BUF_CAPACITY: usize = 1500;
pub const DISCOVERY_PACKET_SIZE: usize = 74;
pub const IP_DISCOVERY_TIMEOUT_SECS: u64 = 2;
pub const IP_DISCOVERY_RETRIES: u32 = 10;
pub const IP_DISCOVERY_RETRY_INTERVAL_MS: u64 = 1000;
pub const OP_HEARTBEAT: u8 = 3;
pub const MAX_PENDING_PROPOSALS: usize = 64;
}
pub mod protocol {
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GatewayPayload {
    pub op: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u32>,
    pub d: Value,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Identify = 0,
    SelectProtocol = 1,
    Ready = 2,
    Heartbeat = 3,
    SessionDescription = 4,
    Speaking = 5,
    HeartbeatAck = 6,
    Resume = 7,
    Hello = 8,
    Resumed = 9,
    ClientConnect = 11,
    Video = 12,
    ClientDisconnect = 13,
    Codecs = 14,
    MediaSinkWants = 15,
    VoiceBackendVersion = 16,
    UserFlags = 18,
    VoicePlatform = 20,
    Unknown = 255,
}
impl From<u8> for OpCode {
    fn from(op: u8) -> Self {
        match op {
            0 => Self::Identify,
            1 => Self::SelectProtocol,
            2 => Self::Ready,
            3 => Self::Heartbeat,
            4 => Self::SessionDescription,
            5 => Self::Speaking,
            6 => Self::HeartbeatAck,
            7 => Self::Resume,
            8 => Self::Hello,
            9 => Self::Resumed,
            11 => Self::ClientConnect,
            12 => Self::Video,
            13 => Self::ClientDisconnect,
            14 => Self::Codecs,
            15 => Self::MediaSinkWants,
            16 => Self::VoiceBackendVersion,
            18 => Self::UserFlags,
            20 => Self::VoicePlatform,
            _ => Self::Unknown,
        }
    }
}
pub mod builders {
    use serde_json::json;
    use super::*;
    pub fn identify(
        guild_id: String,
        user_id: String,
        session_id: String,
        token: String,
    ) -> GatewayPayload {
        GatewayPayload {
            op: OpCode::Identify as u8,
            seq: None,
            d: json!({
                "server_id": guild_id,
                "user_id": user_id,
                "session_id": session_id,
                "token": token,
                "video": true,
            }),
        }
    }
    pub fn resume(
        guild_id: String,
        session_id: String,
        token: String,
        seq_ack: i64,
    ) -> GatewayPayload {
        let seq_ack = seq_ack.max(0);
        GatewayPayload {
            op: OpCode::Resume as u8,
            seq: None,
            d: json!({
                "server_id": guild_id,
                "session_id": session_id,
                "token": token,
                "video": true,
                "seq_ack": seq_ack,
            }),
        }
    }
}
}
pub mod session {
pub mod types {
use serde::{Deserialize, Serialize};
use thiserror::Error;
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Discovery failed: {0}")]
    Discovery(String),
    #[error("Encoding error: {0}")]
    Encoding(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Other error: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
pub fn map_boxed_err<E: std::fmt::Display>(e: E) -> crate::common::types::AnyError {
    Box::new(std::io::Error::other(e.to_string()))
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOutcome {
    Reconnect,
    Identify,
    Shutdown,
}
#[derive(Default, Debug)]
pub struct PersistentSessionState {
    pub ssrc: u32,
    pub udp_addr: Option<std::net::SocketAddr>,
    pub session_key: Option<[u8; 32]>,
    pub rtp_state: Option<crate::gateway::udp_link::RtpState>,
    pub selected_mode: Option<String>,
}
}
pub mod protocol {
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GatewayPayload {
    pub op: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u32>,
    pub d: Value,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Identify = 0,
    SelectProtocol = 1,
    Ready = 2,
    Heartbeat = 3,
    SessionDescription = 4,
    Speaking = 5,
    HeartbeatAck = 6,
    Resume = 7,
    Hello = 8,
    Resumed = 9,
    ClientConnect = 11,
    Video = 12,
    ClientDisconnect = 13,
    Codecs = 14,
    MediaSinkWants = 15,
    VoiceBackendVersion = 16,
    UserFlags = 18, 
    VoicePlatform = 20,
    DavePrepareTransition = 21,
    DaveExecuteTransition = 22,
    DaveTransitionReady = 23,
    DavePrepareEpoch = 24,
    MlsExternalSender = 25,
    MlsProposals = 27,
    MlsAnnounceCommitTransition = 29,
    MlsWelcome = 30,
    MlsInvalidCommitWelcome = 31,
    NoRoute = 32,
    Unknown = 255,
}
impl From<u8> for OpCode {
    fn from(op: u8) -> Self {
        match op {
            0 => Self::Identify,
            1 => Self::SelectProtocol,
            2 => Self::Ready,
            3 => Self::Heartbeat,
            4 => Self::SessionDescription,
            5 => Self::Speaking,
            6 => Self::HeartbeatAck,
            7 => Self::Resume,
            8 => Self::Hello,
            9 => Self::Resumed,
            11 => Self::ClientConnect,
            12 => Self::Video,
            13 => Self::ClientDisconnect,
            14 => Self::Codecs,
            15 => Self::MediaSinkWants,
            16 => Self::VoiceBackendVersion,
            18 => Self::UserFlags,
            20 => Self::VoicePlatform,
            21 => Self::DavePrepareTransition,
            22 => Self::DaveExecuteTransition,
            23 => Self::DaveTransitionReady,
            24 => Self::DavePrepareEpoch,
            25 => Self::MlsExternalSender,
            27 => Self::MlsProposals,
            29 => Self::MlsAnnounceCommitTransition,
            30 => Self::MlsWelcome,
            31 => Self::MlsInvalidCommitWelcome,
            32 => Self::NoRoute,
            _ => Self::Unknown,
        }
    }
}
pub mod builders {
    use serde_json::json;
    use super::*;
    pub fn identify(
        guild_id: String,
        user_id: String,
        session_id: String,
        token: String,
        dave_version: u16,
    ) -> GatewayPayload {
        GatewayPayload {
            op: OpCode::Identify as u8,
            seq: None,
            d: json!({
                "server_id": guild_id,
                "user_id": user_id,
                "session_id": session_id,
                "token": token,
                "video": true,
                "max_dave_protocol_version": dave_version,
            }),
        }
    }
    pub fn resume(
        guild_id: String,
        session_id: String,
        token: String,
        seq_ack: i64,
    ) -> GatewayPayload {
        let seq_ack = seq_ack.max(0);
        GatewayPayload {
            op: OpCode::Resume as u8,
            seq: None,
            d: json!({
                "server_id": guild_id,
                "session_id": session_id,
                "token": token,
                "video": true,
                "seq_ack": seq_ack,
            }),
        }
    }
}
}
pub mod backoff {
use std::time::Duration;
use crate::gateway::constants::{BACKOFF_BASE_MS, MAX_RECONNECT_ATTEMPTS};
#[derive(Debug, Clone, Default)]
pub struct Backoff {
    attempt: u32,
}
impl Backoff {
    pub const fn new() -> Self {
        Self { attempt: 0 }
    }
    pub fn next_delay(&mut self) -> Duration {
        let exponent = self.attempt.min(3);
        let ms = BACKOFF_BASE_MS * 2u64.pow(exponent);
        self.attempt += 1;
        Duration::from_millis(ms)
    }
    #[inline]
    pub const fn is_exhausted(&self) -> bool {
        self.attempt >= MAX_RECONNECT_ATTEMPTS
    }
    #[inline]
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
    #[inline]
    pub const fn attempt(&self) -> u32 {
        self.attempt
    }
}
}
pub mod policy {
use super::types::SessionOutcome;
pub struct FailurePolicy {
    max_retries: u32,
}
impl FailurePolicy {
    pub const fn new(max_retries: u32) -> Self {
        Self { max_retries }
    }
    pub const fn is_retryable(&self, code: u16, attempt: u32) -> bool {
        if attempt >= self.max_retries {
            return false;
        }
        matches!(
            code,
            1000 | 
            1001 | 
            1006 | 
            4000 | 
            4001 | 
            4002 | 
            4003 | 
            4005 | 
            4006 | 
            4009 | 
            4012 | 
            4015 | 
            4016 | 
            4020 | 
            4900 
        )
    }
    pub fn classify(&self, code: u16) -> SessionOutcome {
        match code {
            4004 | 4011 | 4014 | 4021 | 4022 => SessionOutcome::Shutdown,
            4006 | 4009 => SessionOutcome::Identify,
            _ => SessionOutcome::Reconnect,
        }
    }
}
}
pub mod heartbeat {
use std::sync::{
    Arc,
    atomic::{AtomicI64, AtomicU32, AtomicU64, Ordering},
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_util::sync::CancellationToken;
use tracing::warn;
use super::protocol::{GatewayPayload, OpCode};
use crate::common::utils::now_ms;
pub struct HeartbeatTracker {
    pub last_nonce: Arc<AtomicU64>,
    pub sent_at: Arc<AtomicU64>,
    pub missed_acks: Arc<AtomicU32>,
}
impl Default for HeartbeatTracker {
    fn default() -> Self {
        Self {
            last_nonce: Arc::new(AtomicU64::new(0)),
            sent_at: Arc::new(AtomicU64::new(0)),
            missed_acks: Arc::new(AtomicU32::new(0)),
        }
    }
}
impl HeartbeatTracker {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn validate_ack(&self, acked_nonce: u64) -> Option<u64> {
        let expected = self.last_nonce.load(Ordering::Relaxed);
        if expected != acked_nonce {
            warn!("Heartbeat mismatch: sent={expected} got={acked_nonce}");
            return None;
        }
        Some(now_ms().saturating_sub(self.sent_at.load(Ordering::Relaxed)))
    }
    pub fn spawn(
        &self,
        tx: UnboundedSender<Message>,
        seq_ack: Arc<AtomicI64>,
        conn_token: CancellationToken,
        interval_ms: u64,
    ) -> tokio::task::JoinHandle<()> {
        let last_nonce = self.last_nonce.clone();
        let sent_at = self.sent_at.clone();
        let missed_acks = self.missed_acks.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                let missed = missed_acks.fetch_add(1, Ordering::Relaxed);
                if missed >= 2 {
                    warn!("Heartbeat timeout: {missed} missed ACKs.");
                    conn_token.cancel();
                    break;
                }
                let nonce = now_ms();
                last_nonce.store(nonce, Ordering::Relaxed);
                sent_at.store(nonce, Ordering::Relaxed);
                let hb = GatewayPayload {
                    op: OpCode::Heartbeat as u8,
                    seq: None,
                    d: serde_json::json!({
                        "t": nonce,
                        "seq_ack": seq_ack.load(Ordering::Relaxed)
                    }),
                };
                if let Ok(json) = serde_json::to_string(&hb)
                    && tx.send(Message::Text(json.into())).is_err()
                {
                    break;
                }
            }
        })
    }
}
}
pub mod voice {
use std::{
    net::SocketAddr,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tracing::error;
use super::types::GatewayError;
use crate::{
    audio::{Mixer, engine::Encoder, filters::FilterChain},
    common::types::Shared,
    gateway::{
        DaveHandler,
        constants::{
            DISCOVERY_PACKET_SIZE, FRAME_DURATION_MS, IP_DISCOVERY_RETRIES,
            IP_DISCOVERY_RETRY_INTERVAL_MS, IP_DISCOVERY_TIMEOUT_SECS, MAX_OPUS_FRAME_SIZE,
            MAX_SILENCE_FRAMES, PCM_FRAME_SAMPLES, SILENCE_FRAME, UDP_KEEPALIVE_GAP_MS,
        },
        udp_link::UDPVoiceTransport,
    },
};
pub async fn discover_ip(
    socket: &tokio::net::UdpSocket,
    addr: SocketAddr,
    ssrc: u32,
) -> Result<(String, u16), GatewayError> {
    let mut packet = [0u8; DISCOVERY_PACKET_SIZE];
    packet[0..2].copy_from_slice(&1u16.to_be_bytes());
    packet[2..4].copy_from_slice(&70u16.to_be_bytes());
    packet[4..8].copy_from_slice(&ssrc.to_be_bytes());
    for attempt in 1..=IP_DISCOVERY_RETRIES {
        if attempt > 1 {
            tokio::time::sleep(Duration::from_millis(IP_DISCOVERY_RETRY_INTERVAL_MS)).await;
        }
        if let Err(e) = socket.send_to(&packet, addr).await {
            if attempt == IP_DISCOVERY_RETRIES {
                return Err(GatewayError::Discovery(e.to_string()));
            }
            continue;
        }
        let mut client_buf = [0u8; DISCOVERY_PACKET_SIZE];
        match tokio::time::timeout(
            Duration::from_secs(IP_DISCOVERY_TIMEOUT_SECS),
            socket.recv_from(&mut client_buf),
        )
        .await
        {
            Ok(Ok((n, peer))) if n >= DISCOVERY_PACKET_SIZE => {
                if peer != addr {
                    continue;
                }
                let ip = std::str::from_utf8(&client_buf[8..72])
                    .map_err(|e| GatewayError::Discovery(e.to_string()))?
                    .trim_end_matches('\0')
                    .to_owned();
                let port = u16::from_be_bytes([client_buf[72], client_buf[73]]);
                return Ok((ip, port));
            }
            _ => {
                if attempt == IP_DISCOVERY_RETRIES {
                    return Err(GatewayError::Discovery("Timed out".into()));
                }
            }
        }
    }
    Err(GatewayError::Discovery("Exhausted".into()))
}
pub struct SpeakConfig {
    pub mixer: Shared<Mixer>,
    pub socket: Arc<tokio::net::UdpSocket>,
    pub addr: SocketAddr,
    pub ssrc: u32,
    pub key: [u8; 32],
    pub mode: String,
    pub dave: Shared<DaveHandler>,
    pub filter_chain: Shared<FilterChain>,
    pub frames_sent: Arc<std::sync::atomic::AtomicU64>,
    pub frames_nulled: Arc<std::sync::atomic::AtomicU64>,
    pub cancel_token: CancellationToken,
    pub speaking_tx: UnboundedSender<bool>,
    pub persistent_state: Arc<tokio::sync::Mutex<super::types::PersistentSessionState>>,
}
pub async fn speak_loop(config: SpeakConfig) -> Result<(), GatewayError> {
    let rtp_state = { config.persistent_state.lock().await.rtp_state };
    let transport = UDPVoiceTransport::new(
        config.socket.clone(),
        config.addr,
        config.ssrc,
        config.key,
        &config.mode,
        rtp_state,
    )?;
    let mut encoder = Encoder::new().map_err(|e| GatewayError::Encoding(e.to_string()))?;
    let mut session = VoiceSession::new(config, transport);
    session.run(&mut encoder).await
}
struct VoiceSession {
    config: SpeakConfig,
    transport: UDPVoiceTransport,
    is_speaking: bool,
    speaking_holdoff: bool,
    last_tx_time: Instant,
    active_silence: u32,
}
impl VoiceSession {
    fn new(config: SpeakConfig, transport: UDPVoiceTransport) -> Self {
        Self {
            config,
            transport,
            is_speaking: false,
            speaking_holdoff: false,
            last_tx_time: Instant::now(),
            active_silence: 0,
        }
    }
    async fn run(&mut self, encoder: &mut Encoder) -> Result<(), GatewayError> {
        let mut interval = tokio::time::interval(Duration::from_millis(FRAME_DURATION_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut pcm = vec![0i16; PCM_FRAME_SAMPLES * 2];
        let mut opus = vec![0u8; MAX_OPUS_FRAME_SIZE];
        let mut ts_pcm = vec![0i16; PCM_FRAME_SAMPLES * 2];
        while !self.config.cancel_token.is_cancelled() {
            interval.tick().await;
            self.tick(encoder, &mut pcm, &mut opus, &mut ts_pcm).await?;
            if self
                .config
                .frames_sent
                .load(Ordering::Relaxed)
                .is_multiple_of(100)
            {
                self.config.persistent_state.lock().await.rtp_state = Some(self.transport.rtp);
            }
        }
        self.config.persistent_state.lock().await.rtp_state = Some(self.transport.rtp);
        Ok(())
    }
    async fn tick(
        &mut self,
        encoder: &mut Encoder,
        pcm: &mut [i16],
        opus: &mut [u8],
        ts_pcm: &mut [i16],
    ) -> Result<(), GatewayError> {
        macro_rules! try_lock_yield {
            ($mutex:expr) => {{
                let mut guard = None;
                for _ in 0..10 {
                    if let Ok(g) = $mutex.try_lock() {
                        guard = Some(g);
                        break;
                    }
                    tokio::task::yield_now().await;
                }
                guard
            }};
        }
        let mut loop_count = 0;
        while loop_count < 10 {
            loop_count += 1;
            let ready_from_ts = {
                if let Some(mut filters) = try_lock_yield!(self.config.filter_chain) {
                    filters.has_timescale() && filters.fill_frame(ts_pcm)
                } else {
                    false
                }
            };
            if ready_from_ts {
                self.set_speaking(true);
                self.config.frames_sent.fetch_add(1, Ordering::Relaxed);
                if self.speaking_holdoff {
                    self.speaking_holdoff = false;
                    self.send_silence().await?;
                }
                return self.send_pcm(encoder, ts_pcm, opus).await;
            }
            let mut has_input = false;
            let mut opus_data = None;
            if let Some(mut mixer) = try_lock_yield!(self.config.mixer) {
                if let Some(data) = mixer.take_opus_frame() {
                    opus_data = Some(data);
                } else {
                    has_input = mixer.mix(pcm);
                }
            }
            if let Some(data) = opus_data {
                self.reset_timers();
                self.set_speaking(true);
                self.config.frames_sent.fetch_add(1, Ordering::Relaxed);
                if self.speaking_holdoff {
                    self.speaking_holdoff = false;
                    self.send_silence().await?;
                }
                return self.send_raw(&data).await;
            }
            if has_input {
                self.reset_timers();
                self.set_speaking(true);
            } else if self.active_silence > 0 {
                self.active_silence -= 1;
                pcm.fill(0);
                self.set_speaking(true);
            } else {
                self.set_speaking(false);
                if self.last_tx_time.elapsed() >= Duration::from_millis(UDP_KEEPALIVE_GAP_MS) {
                    return self.send_silence().await;
                }
                return Ok(());
            }
            let has_ts = {
                if let Some(mut filters) = try_lock_yield!(self.config.filter_chain) {
                    filters.process(pcm);
                    filters.has_timescale()
                } else {
                    false
                }
            };
            if !has_ts {
                if has_input {
                    self.config.frames_sent.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.config.frames_nulled.fetch_add(1, Ordering::Relaxed);
                }
                if self.speaking_holdoff {
                    self.speaking_holdoff = false;
                    self.send_silence().await?;
                }
                return self.send_pcm(encoder, pcm, opus).await;
            }
            let filled_on_silence = {
                if let Some(mut filters) = try_lock_yield!(self.config.filter_chain) {
                    !has_input && filters.fill_frame(ts_pcm)
                } else {
                    false
                }
            };
            if !has_input && !filled_on_silence {
                break;
            }
        }
        Ok(())
    }
    fn set_speaking(&mut self, speaking: bool) {
        if speaking != self.is_speaking {
            self.is_speaking = speaking;
            let _ = self.config.speaking_tx.send(speaking);
            if speaking {
                self.speaking_holdoff = true;
            }
        }
    }
    async fn send_pcm(
        &mut self,
        encoder: &mut Encoder,
        pcm: &[i16],
        opus: &mut [u8],
    ) -> Result<(), GatewayError> {
        let size = encoder.encode(pcm, opus).unwrap_or_else(|e| {
            error!("Opus encode failed: {e}");
            0
        });
        if size > 0 {
            self.send_raw(&opus[..size]).await?;
        } else {
            self.send_silence().await?;
        }
        Ok(())
    }
    async fn send_silence(&mut self) -> Result<(), GatewayError> {
        self.config.frames_nulled.fetch_add(1, Ordering::Relaxed);
        self.send_raw(&SILENCE_FRAME).await
    }
    async fn send_raw(&mut self, data: &[u8]) -> Result<(), GatewayError> {
        let mut dave = self.config.dave.lock().await;
        let encrypted = dave
            .encrypt_opus(data)
            .map_err(|e| GatewayError::Encryption(e.to_string()))?;
        drop(dave);
        self.transport.transmit_opus(&encrypted).await?;
        self.last_tx_time = Instant::now();
        Ok(())
    }
    fn reset_timers(&mut self) {
        self.active_silence = MAX_SILENCE_FRAMES;
    }
}
}
pub mod handler {
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace, warn};
use uuid::Uuid;
use super::{
    VoiceGateway,
    backoff::Backoff,
    heartbeat::HeartbeatTracker,
    protocol::{GatewayPayload, OpCode},
    types::{GatewayError, PersistentSessionState, SessionOutcome},
    voice::{SpeakConfig, discover_ip, speak_loop},
};
use crate::{
    common::types::{Shared, UserId},
    gateway::{
        DaveHandler,
        constants::{DAVE_INITIAL_VERSION, DEFAULT_VOICE_MODE},
    },
};
pub struct SessionState<'a> {
    gateway: &'a VoiceGateway,
    tx: UnboundedSender<Message>,
    seq_ack: Arc<AtomicI64>,
    ssrc: u32,
    udp_addr: Option<SocketAddr>,
    selected_mode: String,
    connected_users: HashSet<UserId>,
    udp_socket: Arc<tokio::net::UdpSocket>,
    dave: Shared<DaveHandler>,
    heartbeat: HeartbeatTracker,
    heartbeat_handle: Option<tokio::task::JoinHandle<()>>,
    conn_token: CancellationToken,
    speaking_tx: Option<UnboundedSender<bool>>,
    session_key: Option<[u8; 32]>,
    speak_task: Option<tokio::task::JoinHandle<()>>,
    persistent_state: Arc<tokio::sync::Mutex<PersistentSessionState>>,
    backoff: &'a mut Backoff,
}
impl<'a> SessionState<'a> {
    pub async fn new(
        gateway: &'a VoiceGateway,
        tx: UnboundedSender<Message>,
        seq_ack: Arc<AtomicI64>,
        conn_token: CancellationToken,
        persistent_state: Arc<tokio::sync::Mutex<PersistentSessionState>>,
        backoff: &'a mut Backoff,
    ) -> Result<Self, GatewayError> {
        let mut socket_guard = gateway.udp_socket.lock().await;
        let udp_socket = if let Some(existing) = &*socket_guard {
            existing.clone()
        } else {
            let udp = std::net::UdpSocket::bind("0.0.0.0:0")?;
            udp.set_nonblocking(true)?;
            let socket = Arc::new(tokio::net::UdpSocket::from_std(udp)?);
            *socket_guard = Some(socket.clone());
            socket
        };
        Ok(Self {
            gateway,
            tx,
            seq_ack,
            ssrc: 0,
            udp_addr: None,
            selected_mode: DEFAULT_VOICE_MODE.to_string(),
            connected_users: HashSet::from([gateway.user_id]),
            udp_socket,
            dave: gateway.dave.clone(),
            heartbeat: HeartbeatTracker::new(),
            heartbeat_handle: None,
            conn_token,
            speaking_tx: None,
            session_key: None,
            speak_task: None,
            persistent_state,
            backoff,
        })
    }
    pub fn set_speaking_tx(&mut self, tx: UnboundedSender<bool>) {
        self.speaking_tx = Some(tx);
    }
    pub fn ssrc(&self) -> u32 {
        self.ssrc
    }
    pub fn tx(&self) -> &UnboundedSender<Message> {
        &self.tx
    }
    pub fn attempt(&self) -> u32 {
        self.backoff.attempt()
    }
    pub fn has_heartbeat(&self) -> bool {
        self.heartbeat_handle.is_some()
    }
    pub async fn handle_text(&mut self, text: String) -> Option<SessionOutcome> {
        let payload: GatewayPayload = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                warn!("[{}] JSON Parse error: {e}", self.gateway.guild_id);
                return None;
            }
        };
        if let Some(seq) = payload.seq {
            self.seq_ack.store(seq as i64, Ordering::Relaxed);
        }
        let op = OpCode::from(payload.op);
        trace!(
            "[{}] RX OP: {:?} (op={})",
            self.gateway.guild_id, op, payload.op
        );
        match op {
            OpCode::Hello => self.on_hello(payload.d),
            OpCode::Ready => self.on_ready(payload.d).await,
            OpCode::SessionDescription => self.on_session_description(payload.d).await,
            OpCode::HeartbeatAck => self.on_heartbeat_ack(payload.d),
            OpCode::Resumed => self.on_resumed().await,
            OpCode::ClientConnect => self.on_user_connect(payload.d).await,
            OpCode::ClientDisconnect => self.on_user_disconnect(payload.d).await,
            OpCode::VoiceBackendVersion => {
                debug!(
                    "[{}] Voice Backend Version: {:?}",
                    self.gateway.guild_id, payload.d
                );
                None
            }
            OpCode::MediaSinkWants => {
                debug!(
                    "[{}] Media Sink Wants: {:?}",
                    self.gateway.guild_id, payload.d
                );
                None
            }
            OpCode::DavePrepareTransition => self.on_dave_prepare_transition(payload.d).await,
            OpCode::DaveExecuteTransition => self.on_dave_execute_transition(payload.d).await,
            OpCode::DavePrepareEpoch => self.on_dave_prepare_epoch(payload.d).await,
            OpCode::MlsAnnounceCommitTransition => self.on_mls_transition(payload.d).await,
            OpCode::MlsInvalidCommitWelcome => {
                warn!(
                    "[{}] DAVE MLS Invalid Commit Welcome received, resetting session",
                    self.gateway.guild_id
                );
                self.reset_dave(0).await;
                None
            }
            OpCode::NoRoute => {
                warn!(
                    "[{}] No Route received: {:?}",
                    self.gateway.guild_id, payload.d
                );
                None
            }
            OpCode::Speaking
            | OpCode::Video
            | OpCode::Codecs
            | OpCode::UserFlags
            | OpCode::VoicePlatform => None,
            _ => None,
        }
    }
    pub async fn handle_binary(&mut self, bin: Vec<u8>) {
        if bin.len() < 3 {
            return;
        }
        let seq = u16::from_be_bytes([bin[0], bin[1]]);
        let op = bin[2];
        let data = &bin[3..];
        self.seq_ack.store(seq as i64, Ordering::Relaxed);
        let mut dave = self.dave.lock().await;
        match op {
            25 => {
                if let Ok(res) = dave.process_external_sender(data) {
                    for r in res {
                        self.send_binary(28, &r);
                    }
                }
            }
            27 => {
                match dave.process_proposals(data) {
                    Ok(Some(cw)) => self.send_binary(28, &cw),
                    Err(e) => {
                        warn!("[{}] DAVE proposals failed: {e}", self.gateway.guild_id);
                        self.reset_dave_locked(&mut dave, 0).await;
                    }
                    _ => {}
                }
            }
            29 | 30 => {
                let res = if op == 30 {
                    dave.process_welcome(data)
                } else {
                    dave.process_commit(data)
                };
                match res {
                    Ok(tid) if tid != 0 => {
                        self.send_json(23, serde_json::json!({ "transition_id": tid }))
                    }
                    Err(e) => {
                        let tid = if data.len() >= 2 {
                            u16::from_be_bytes([data[0], data[1]])
                        } else {
                            0
                        };
                        warn!(
                            "[{}] DAVE {} failed (tid {tid}): {e}",
                            self.gateway.guild_id,
                            if op == 30 { "welcome" } else { "commit" }
                        );
                        self.reset_dave_locked(&mut dave, tid).await;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn on_hello(&mut self, d: Value) -> Option<SessionOutcome> {
        let interval = d["heartbeat_interval"].as_u64().unwrap_or(30_000);
        if let Some(h) = self.heartbeat_handle.take() {
            debug!(
                "[{}] Restarting heartbeat on HELLO (was already running)",
                self.gateway.guild_id
            );
            h.abort();
        }
        trace!(
            "[{}] Heartbeat interval: {interval}ms",
            self.gateway.guild_id
        );
        self.heartbeat_handle = Some(self.heartbeat.spawn(
            self.tx.clone(),
            self.seq_ack.clone(),
            self.conn_token.clone(),
            interval,
        ));
        None
    }
    async fn on_ready(&mut self, d: Value) -> Option<SessionOutcome> {
        let ssrc = d["ssrc"].as_u64();
        let ip = d["ip"].as_str();
        let port = d["port"].as_u64();
        match (ssrc, ip, port) {
            (Some(ssrc), Some(ip), Some(port)) if port <= 65535 => {
                self.ssrc = ssrc as u32;
                let addr_str = format!("{ip}:{port}");
                match addr_str.parse::<SocketAddr>() {
                    Ok(addr) => self.udp_addr = Some(addr),
                    Err(_) => {
                        error!(
                            "[{}] Invalid READY address: {addr_str}",
                            self.gateway.guild_id
                        );
                        return Some(SessionOutcome::Reconnect);
                    }
                }
            }
            _ => {
                error!("[{}] Malformed READY payload", self.gateway.guild_id);
                return Some(SessionOutcome::Reconnect);
            }
        }
        if let Some(modes) = d["modes"].as_array() {
            let pref = ["aead_aes256_gcm_rtpsize", "xsalsa20_poly1305"];
            if let Some(m) = pref
                .iter()
                .find(|&&p| modes.iter().any(|m| m.as_str() == Some(p)))
            {
                self.selected_mode = (*m).to_owned();
            }
        }
        debug!(
            "[{}] Ready: ssrc={}, mode={}",
            self.gateway.guild_id, self.ssrc, self.selected_mode
        );
        {
            let mut state = self.persistent_state.lock().await;
            state.ssrc = self.ssrc;
            state.selected_mode = Some(self.selected_mode.clone());
        }
        if self.gateway.channel_id.0 > 0 {
            let ver = d["dave_protocol_version"]
                .as_u64()
                .unwrap_or(DAVE_INITIAL_VERSION as u64) as u16;
            let mut dave = self.dave.lock().await;
            if ver == 0 {
                dave.reset();
            } else {
                dave.set_protocol_version(ver);
                if let Ok(kp) = dave.setup_session(ver) {
                    self.send_binary(26, &kp);
                }
            }
        }
        let target_addr = match self.udp_addr {
            Some(a) => a,
            None => return Some(SessionOutcome::Reconnect),
        };
        match discover_ip(&self.udp_socket, target_addr, self.ssrc).await {
            Ok((my_ip, my_port)) => {
                self.send_json(OpCode::SelectProtocol as u8, serde_json::json!({
                    "protocol": "udp",
                    "rtc_connection_id": Uuid::new_v4().to_string(),
                    "codecs": [{"name": "opus", "type": "audio", "priority": 1000, "payload_type": 120}],
                    "data": { "address": my_ip, "port": my_port, "mode": self.selected_mode },
                    "address": my_ip,
                    "port": my_port,
                    "mode": self.selected_mode
                }));
                self.send_json(
                    OpCode::Video as u8,
                    serde_json::json!({"audio_ssrc": self.ssrc, "video_ssrc": 0, "rtx_ssrc": 0}),
                );
                self.send_json(
                    OpCode::Speaking as u8,
                    serde_json::json!({"speaking": 0, "delay": 0, "ssrc": self.ssrc}),
                );
            }
            Err(e) => {
                error!("[{}] IP discovery failed: {e}", self.gateway.guild_id);
                return Some(SessionOutcome::Reconnect);
            }
        }
        self.backoff.reset();
        None
    }
    async fn on_session_description(&mut self, d: Value) -> Option<SessionOutcome> {
        let ka = match d["secret_key"].as_array() {
            Some(a) if a.len() == 32 => a,
            _ => {
                error!(
                    "[{}] Invalid or missing secret_key in VOICE_READY",
                    self.gateway.guild_id
                );
                return Some(SessionOutcome::Reconnect);
            }
        };
        let mut key = [0u8; 32];
        for (i, v) in ka.iter().enumerate() {
            if let Some(val) = v.as_u64()
                && val <= 255
            {
                key[i] = val as u8;
                continue;
            }
            error!(
                "[{}] Invalid secret_key byte at index {i}",
                self.gateway.guild_id
            );
            return Some(SessionOutcome::Reconnect);
        }
        self.session_key = Some(key);
        let addr = match self.udp_addr {
            Some(a) => a,
            None => return Some(SessionOutcome::Reconnect),
        };
        {
            let mut state = self.persistent_state.lock().await;
            state.udp_addr = Some(addr);
            state.session_key = Some(key);
            state.ssrc = self.ssrc;
            state.selected_mode = Some(self.selected_mode.clone());
        }
        self.start_voice(addr, key).await;
        if self.gateway.channel_id.0 > 0 {
            let protocol_version = d["dave_protocol_version"]
                .as_u64()
                .unwrap_or(DAVE_INITIAL_VERSION as u64) as u16;
            let mls_group_id = d["mls_group_id"].as_u64().unwrap_or(0);
            let mut dave = self.dave.lock().await;
            if protocol_version > 0 {
                dave.set_protocol_version(protocol_version);
                if let Ok(kp) = dave.setup_session(protocol_version) {
                    self.send_binary(26, &kp);
                }
            } else {
                dave.reset();
            }
            debug!(
                "DAVE setup context: protocol_version={}, mls_group_id={}",
                protocol_version, mls_group_id
            );
        }
        self.backoff.reset();
        None
    }
    async fn on_resumed(&mut self) -> Option<SessionOutcome> {
        debug!("[{}] Resumed", self.gateway.guild_id);
        let (addr, key, ssrc, mode) = {
            let state = self.persistent_state.lock().await;
            (
                state.udp_addr,
                state.session_key,
                state.ssrc,
                state.selected_mode.clone(),
            )
        };
        match (addr, key) {
            (Some(addr), Some(key)) => {
                self.udp_addr = Some(addr);
                self.session_key = Some(key);
                self.ssrc = ssrc;
                if let Some(m) = mode {
                    self.selected_mode = m;
                }
                if let Some(task) = &self.speak_task
                    && task.is_finished()
                {
                    self.speak_task = None;
                }
                if self.speak_task.is_some() {
                    debug!(
                        "[{}] Keeping existing voice loop alive across resume",
                        self.gateway.guild_id
                    );
                } else {
                    debug!(
                        "[{}] Starting voice loop after resume (task was dead or missing)",
                        self.gateway.guild_id
                    );
                    self.start_voice(addr, key).await;
                }
            }
            _ => {
                warn!(
                    "[{}] Resume failed: missing persistent state",
                    self.gateway.guild_id
                );
                return Some(SessionOutcome::Identify);
            }
        }
        None
    }
    fn on_heartbeat_ack(&self, d: Value) -> Option<SessionOutcome> {
        let nonce = d["t"].as_u64().unwrap_or(0);
        if let Some(rtt) = self.heartbeat.validate_ack(nonce) {
            self.gateway.ping.store(rtt as i64, Ordering::Relaxed);
            self.heartbeat.missed_acks.store(0, Ordering::Relaxed);
        }
        None
    }
    async fn on_user_connect(&mut self, d: Value) -> Option<SessionOutcome> {
        if let Some(ids) = d["user_ids"].as_array() {
            let mut uids = Vec::new();
            for id in ids {
                if let Some(uid) = id.as_str().and_then(|s| s.parse::<u64>().ok()) {
                    self.connected_users.insert(UserId(uid));
                    uids.push(uid);
                }
            }
            if !uids.is_empty() {
                self.dave.lock().await.add_users(&uids);
            }
        }
        None
    }
    async fn on_user_disconnect(&mut self, d: Value) -> Option<SessionOutcome> {
        if let Some(uid) = d["user_id"].as_str().and_then(|s| s.parse::<u64>().ok()) {
            self.connected_users.remove(&UserId(uid));
            self.dave.lock().await.remove_user(uid);
        }
        None
    }
    async fn on_dave_prepare_transition(&mut self, d: Value) -> Option<SessionOutcome> {
        let tid = d["transition_id"].as_u64().unwrap_or(0) as u16;
        let ver = d["protocol_version"].as_u64().unwrap_or(0) as u16;
        debug!(
            "[{}] DAVE Prepare Transition: id={}, version={}",
            self.gateway.guild_id, tid, ver
        );
        if self.dave.lock().await.prepare_transition(tid, ver) {
            debug!(
                "[{}] DAVE Transition Ready (tid={})",
                self.gateway.guild_id, tid
            );
            self.send_json(23, serde_json::json!({ "transition_id": tid }));
        }
        None
    }
    async fn on_dave_execute_transition(&mut self, d: Value) -> Option<SessionOutcome> {
        let tid = d["transition_id"].as_u64().unwrap_or(0) as u16;
        debug!(
            "[{}] DAVE Execute Transition: id={}",
            self.gateway.guild_id, tid
        );
        self.dave.lock().await.execute_transition(tid);
        None
    }
    async fn on_dave_prepare_epoch(&mut self, d: Value) -> Option<SessionOutcome> {
        let epoch = d["epoch"].as_u64().unwrap_or(0);
        let ver = d["protocol_version"].as_u64().unwrap_or(0) as u16;
        debug!(
            "[{}] DAVE Prepare Epoch: epoch={}, version={}",
            self.gateway.guild_id, epoch, ver
        );
        if let Some(kp) = self.dave.lock().await.prepare_epoch(epoch, ver) {
            self.send_binary(26, &kp);
        }
        None
    }
    async fn on_mls_transition(&mut self, d: Value) -> Option<SessionOutcome> {
        let tid = d["transition_id"].as_u64().unwrap_or(0) as u16;
        debug!(
            "[{}] DAVE MLS Announce Commit Transition: tid={}",
            self.gateway.guild_id, tid
        );
        let ver = d["protocol_version"].as_u64().map(|v| v as u16);
        if let Some(v) = ver {
            let mut dave = self.dave.lock().await;
            if dave.prepare_transition(tid, v) && tid != 0 {
                self.send_json(23, serde_json::json!({ "transition_id": tid }));
            }
        }
        None
    }
    async fn start_voice(&mut self, addr: SocketAddr, key: [u8; 32]) {
        if let Some(t) = self.speak_task.take() {
            t.abort();
        }
        let speaking_tx = if let Some(tx) = &self.speaking_tx {
            tx.clone()
        } else {
            error!(
                "[{}] speaking_tx is missing, cannot start voice",
                self.gateway.guild_id
            );
            return;
        };
        let config = SpeakConfig {
            mixer: self.gateway.mixer.clone(),
            socket: self.udp_socket.clone(),
            addr,
            ssrc: self.ssrc,
            key,
            mode: self.selected_mode.clone(),
            dave: self.dave.clone(),
            filter_chain: self.gateway.filter_chain.clone(),
            frames_sent: self.gateway.frames_sent.clone(),
            frames_nulled: self.gateway.frames_nulled.clone(),
            cancel_token: self.conn_token.clone(),
            speaking_tx,
            persistent_state: self.persistent_state.clone(),
        };
        let guild_id = self.gateway.guild_id.clone();
        let conn_token = self.conn_token.clone();
        self.speak_task = Some(tokio::spawn(async move {
            if let Err(e) = speak_loop(config).await {
                error!("[{guild_id}] speak_loop failed: {e}");
                conn_token.cancel();
            }
        }));
        self.send_json(
            OpCode::Video as u8,
            serde_json::json!({"audio_ssrc": self.ssrc, "video_ssrc": 0, "rtx_ssrc": 0}),
        );
        self.send_json(
            OpCode::Speaking as u8,
            serde_json::json!({"speaking": 0, "delay": 0, "ssrc": self.ssrc}),
        );
    }
    async fn reset_dave(&self, tid: u16) {
        let mut dave = self.dave.lock().await;
        self.reset_dave_locked(&mut dave, tid).await;
    }
    async fn reset_dave_locked(&self, dave: &mut DaveHandler, tid: u16) {
        dave.reset();
        self.send_json(31, serde_json::json!({ "transition_id": tid }));
        if let Ok(kp) = dave.setup_session(DAVE_INITIAL_VERSION) {
            self.send_binary(26, &kp);
        }
    }
    fn send_json(&self, op: u8, d: Value) {
        match serde_json::to_string(&GatewayPayload { op, seq: None, d }) {
            Ok(json) => {
                let _ = self.tx.send(Message::Text(json.into()));
            }
            Err(e) => {
                warn!("[{}] JSON serialization failed: {e}", self.gateway.guild_id);
            }
        }
    }
    fn send_binary(&self, op: u8, payload: &[u8]) {
        let mut b = vec![op];
        b.extend_from_slice(payload);
        let _ = self.tx.send(Message::Binary(b.into()));
    }
}
impl<'a> Drop for SessionState<'a> {
    fn drop(&mut self) {
        if let Some(h) = self.heartbeat_handle.take() {
            h.abort();
        }
        if let Some(t) = self.speak_task.take() {
            t.abort();
        }
    }
}
}
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio_tungstenite::tungstenite::protocol::{Message, WebSocketConfig};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};
use crate::{
    audio::{Mixer, filters::FilterChain},
    common::types::{ChannelId, GuildId, SessionId, Shared, UserId},
    gateway::constants::VOICE_GATEWAY_VERSION,
    protocol::LavendeEvent,
};
use self::{
    backoff::Backoff,
    policy::FailurePolicy,
    types::{GatewayError, PersistentSessionState, SessionOutcome},
};
pub struct VoiceGateway {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub channel_id: ChannelId,
    session_id: SessionId,
    token: String,
    endpoint: String,
    pub mixer: Shared<Mixer>,
    pub filter_chain: Shared<FilterChain>,
    pub ping: Arc<AtomicI64>,
    event_tx: Option<UnboundedSender<LavendeEvent>>,
    pub frames_sent: Arc<std::sync::atomic::AtomicU64>,
    pub frames_nulled: Arc<std::sync::atomic::AtomicU64>,
    pub udp_socket: Shared<Option<Arc<tokio::net::UdpSocket>>>,
    pub dave: Shared<crate::gateway::DaveHandler>,
    outer_token: CancellationToken,
    policy: FailurePolicy,
}
pub struct VoiceGatewayConfig {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub session_id: SessionId,
    pub token: String,
    pub endpoint: String,
    pub mixer: Shared<Mixer>,
    pub filter_chain: Shared<FilterChain>,
    pub ping: Arc<AtomicI64>,
    pub event_tx: Option<UnboundedSender<LavendeEvent>>,
    pub frames_sent: Arc<std::sync::atomic::AtomicU64>,
    pub frames_nulled: Arc<std::sync::atomic::AtomicU64>,
}
impl VoiceGateway {
    pub fn new(config: VoiceGatewayConfig) -> Self {
        Self {
            guild_id: config.guild_id,
            user_id: config.user_id,
            channel_id: config.channel_id,
            session_id: config.session_id,
            token: config.token,
            endpoint: config.endpoint,
            mixer: config.mixer,
            filter_chain: config.filter_chain,
            ping: config.ping,
            event_tx: config.event_tx,
            frames_sent: config.frames_sent,
            frames_nulled: config.frames_nulled,
            udp_socket: Arc::new(tokio::sync::Mutex::new(None)),
            dave: Arc::new(tokio::sync::Mutex::new(crate::gateway::DaveHandler::new(
                config.user_id,
                config.channel_id,
            ))),
            outer_token: CancellationToken::new(),
            policy: FailurePolicy::new(3),
        }
    }
    pub async fn run(self) -> Result<(), GatewayError> {
        let mut backoff = Backoff::new();
        let mut is_resume = false;
        let seq_ack = Arc::new(AtomicI64::new(-1));
        let persistent_state = Arc::new(tokio::sync::Mutex::new(PersistentSessionState::default()));
        while !self.outer_token.is_cancelled() {
            let attempt = backoff.attempt();
            match self
                .connect(
                    is_resume,
                    seq_ack.clone(),
                    persistent_state.clone(),
                    &mut backoff,
                )
                .await
            {
                Ok(SessionOutcome::Shutdown) => break,
                Ok(outcome) => {
                    if backoff.is_exhausted() {
                        warn!("[{}] Max attempts reached ({})", self.guild_id, attempt);
                        break;
                    }
                    let delay = backoff.next_delay();
                    is_resume = matches!(outcome, SessionOutcome::Reconnect);
                    if !is_resume {
                        seq_ack.store(-1, Ordering::Relaxed);
                        *persistent_state.lock().await = PersistentSessionState::default();
                        *self.udp_socket.lock().await = None;
                    }
                    debug!(
                        "[{}] Retrying ({:?}) in {:?}",
                        self.guild_id, outcome, delay
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    if backoff.is_exhausted() {
                        error!("[{}] Fatal connection error: {e}", self.guild_id);
                        break;
                    }
                    let delay = backoff.next_delay();
                    warn!(
                        "[{}] Connection error: {e}. Retrying in {:?}",
                        self.guild_id, delay
                    );
                    tokio::time::sleep(delay).await;
                    is_resume = false;
                }
            }
        }
        Ok(())
    }
    async fn connect(
        &self,
        is_resume: bool,
        seq_ack: Arc<AtomicI64>,
        persistent_state: Arc<tokio::sync::Mutex<PersistentSessionState>>,
        backoff: &mut Backoff,
    ) -> Result<SessionOutcome, GatewayError> {
        let endpoint = if self.endpoint.ends_with(":80") {
            &self.endpoint[..self.endpoint.len() - 3]
        } else {
            &self.endpoint
        };
        let url = format!("wss://{}/?v={}", endpoint, VOICE_GATEWAY_VERSION);
        let mut config = WebSocketConfig::default();
        config.max_message_size = None;
        config.max_frame_size = None;
        let (ws_stream, _) =
            tokio_tungstenite::connect_async_with_config(&url, Some(config), true).await?;
        let (mut write, mut read) = ws_stream.split();
        let conn_token = CancellationToken::new();
        let write_token = conn_token.clone();
        let (ws_tx, mut ws_rx) = unbounded_channel::<Message>();
        let writer_handle = tokio::spawn(async move {
            while let Some(msg) = tokio::select! {
                biased;
                _ = write_token.cancelled() => None,
                msg = ws_rx.recv() => msg,
            } {
                if write.send(msg).await.is_err() {
                    break;
                }
            }
        });
        let mut state = handler::SessionState::new(
            self,
            ws_tx.clone(),
            seq_ack.clone(),
            conn_token.clone(),
            persistent_state,
            backoff,
        )
        .await
        .inspect_err(|_e| {
            conn_token.cancel();
        })?;
        let outcome = match read.next().await {
            Some(Ok(m)) => self.handle_message(&mut state, m).await,
            _ => Some(SessionOutcome::Reconnect),
        };
        if let Some(out) = outcome {
            conn_token.cancel();
            writer_handle.abort();
            let _ = writer_handle.await;
            return Ok(out);
        }
        if !state.has_heartbeat() {
            conn_token.cancel();
            writer_handle.abort();
            let _ = writer_handle.await;
            return Ok(SessionOutcome::Reconnect);
        }
        let handshake = if is_resume {
            debug!(
                "[{}] Sending Resume with seq_ack={}",
                self.guild_id,
                seq_ack.load(Ordering::Relaxed)
            );
            protocol::builders::resume(
                self.guild_id.to_string(),
                self.session_id.to_string(),
                self.token.clone(),
                seq_ack.load(Ordering::Relaxed),
            )
        } else {
            protocol::builders::identify(
                self.guild_id.to_string(),
                self.user_id.0.to_string(),
                self.session_id.to_string(),
                self.token.clone(),
                1,
            )
        };
        let _ = ws_tx.send(Message::Text(
            serde_json::to_string(&handshake).unwrap().into(),
        ));
        let (speaking_tx, mut speaking_rx) = unbounded_channel::<bool>();
        state.set_speaking_tx(speaking_tx);
        let outcome = loop {
            tokio::select! {
                biased;
                _ = self.outer_token.cancelled() => break SessionOutcome::Shutdown,
                _ = conn_token.cancelled() => break SessionOutcome::Reconnect,
                Some(speaking) = speaking_rx.recv() => {
                    self.notify_speaking(&ws_tx, state.ssrc(), speaking);
                }
                msg = read.next() => match msg {
                    Some(Ok(m)) => if let Some(out) = self.handle_message(&mut state, m).await {
                        break out;
                    },
                    Some(Err(_)) => break SessionOutcome::Reconnect,
                    None => break SessionOutcome::Reconnect,
                }
            }
        };
        conn_token.cancel();
        writer_handle.abort();
        let _ = writer_handle.await;
        Ok(outcome)
    }
    async fn handle_message(
        &self,
        state: &mut handler::SessionState<'_>,
        msg: Message,
    ) -> Option<SessionOutcome> {
        match msg {
            Message::Text(text) => state.handle_text(text.to_string()).await,
            Message::Binary(bin) => {
                state.handle_binary(bin.to_vec()).await;
                None
            }
            Message::Close(frame) => {
                let code = frame.as_ref().map(|f| f.code.into()).unwrap_or(1000u16);
                let reason = frame.map(|f| f.reason.to_string()).unwrap_or_default();
                let attempt = state.attempt();
                debug!("[{}] Gateway closed: {} ({})", self.guild_id, code, reason);
                if !self.policy.is_retryable(code, attempt) {
                    self.emit_close(code, reason);
                }
                Some(self.policy.classify(code))
            }
            Message::Ping(p) => {
                let _ = state.tx().send(Message::Pong(p));
                None
            }
            _ => None,
        }
    }
    fn notify_speaking(&self, tx: &UnboundedSender<Message>, ssrc: u32, speaking: bool) {
        let msg = protocol::GatewayPayload {
            op: protocol::OpCode::Speaking as u8,
            seq: None,
            d: serde_json::json!({
                "speaking": if speaking { 1 } else { 0 },
                "delay": 0,
                "ssrc": ssrc
            }),
        };
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = tx.send(Message::Text(json.into()));
        }
    }
    fn emit_close(&self, code: u16, reason: String) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(LavendeEvent::WebSocketClosed {
                guild_id: self.guild_id.clone(),
                code,
                reason,
                by_remote: true,
            });
        }
    }
}
}
pub mod udp_link {
use std::{net::SocketAddr, sync::Arc};
use davey::{AeadInPlace, Aes256Gcm, KeyInit};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use xsalsa20poly1305::XSalsa20Poly1305;
use crate::{
    common::types::AnyResult,
    gateway::{
        constants::{
            RTP_OPUS_PAYLOAD_TYPE, RTP_TIMESTAMP_STEP, RTP_VERSION_BYTE, UDP_PACKET_BUF_CAPACITY,
        },
        session::types::map_boxed_err,
    },
};
pub struct UDPVoiceTransport {
    socket: Arc<UdpSocket>,
    address: SocketAddr,
    pub ssrc: u32,
    pub crypto: CryptoBackend,
    pub rtp: RtpState,
    pub buffer: Vec<u8>,
}
pub enum CryptoBackend {
    XSalsa20Poly1305(Box<XSalsa20Poly1305>),
    Aes256Gcm(Box<Aes256Gcm>),
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RtpState {
    pub sequence: u16,
    pub timestamp: u32,
    pub nonce: u32,
}
impl UDPVoiceTransport {
    pub fn new(
        socket: Arc<UdpSocket>,
        address: SocketAddr,
        ssrc: u32,
        secret_key: [u8; 32],
        mode: &str,
        rtp_state: Option<RtpState>,
    ) -> AnyResult<Self> {
        let crypto = match mode {
            "aead_aes256_gcm_rtpsize" => {
                CryptoBackend::Aes256Gcm(Box::new(Aes256Gcm::new(&secret_key.into())))
            }
            _ => {
                CryptoBackend::XSalsa20Poly1305(Box::new(XSalsa20Poly1305::new(&secret_key.into())))
            }
        };
        Ok(Self {
            socket,
            address,
            ssrc,
            crypto,
            rtp: rtp_state.unwrap_or_else(RtpState::randomize),
            buffer: Vec::with_capacity(UDP_PACKET_BUF_CAPACITY),
        })
    }
    pub async fn send_keepalive(&self, counter: u32) -> AnyResult<()> {
        let payload = counter.to_be_bytes();
        self.socket.send_to(&payload, self.address).await?;
        Ok(())
    }
    pub async fn transmit_opus(&mut self, opus_data: &[u8]) -> AnyResult<()> {
        let (seq, ts, nonce_val) = self.rtp.next();
        let mut header = [0u8; 12];
        header[0] = RTP_VERSION_BYTE;
        header[1] = RTP_OPUS_PAYLOAD_TYPE;
        header[2..4].copy_from_slice(&seq.to_be_bytes());
        header[4..8].copy_from_slice(&ts.to_be_bytes());
        header[8..12].copy_from_slice(&self.ssrc.to_be_bytes());
        self.buffer.clear();
        self.buffer.extend_from_slice(&header);
        self.buffer.extend_from_slice(opus_data);
        match &self.crypto {
            CryptoBackend::XSalsa20Poly1305(cipher) => {
                let mut nonce = [0u8; 24];
                nonce[0..12].copy_from_slice(&header);
                let tag = cipher
                    .encrypt_in_place_detached(&nonce.into(), &header, &mut self.buffer[12..])
                    .map_err(|e| map_boxed_err(format!("XSalsa20 error: {e:?}")))?;
                self.buffer.extend_from_slice(&tag);
            }
            CryptoBackend::Aes256Gcm(cipher) => {
                let mut nonce = [0u8; 12];
                nonce[0..4].copy_from_slice(&nonce_val.to_be_bytes());
                let tag = cipher
                    .encrypt_in_place_detached(&nonce.into(), &header, &mut self.buffer[12..])
                    .map_err(|e| map_boxed_err(format!("AES-GCM error: {e:?}")))?;
                self.buffer.extend_from_slice(&tag);
                self.buffer.extend_from_slice(&nonce_val.to_be_bytes());
            }
        }
        self.socket.send_to(&self.buffer, self.address).await?;
        Ok(())
    }
}
impl RtpState {
    fn randomize() -> Self {
        Self {
            sequence: rand::random(),
            timestamp: rand::random(),
            nonce: rand::random(),
        }
    }
    fn next(&mut self) -> (u16, u32, u32) {
        let seq = self.sequence;
        let ts = self.timestamp;
        let n = self.nonce;
        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(RTP_TIMESTAMP_STEP);
        self.nonce = self.nonce.wrapping_add(1);
        (seq, ts, n)
    }
}
}
pub mod encryption {
use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU16,
};
use davey::{DaveSession, ProposalsOperationType};
use tracing::{debug, trace, warn};
use crate::{
    common::types::{AnyError, AnyResult, ChannelId, UserId},
    gateway::{
        constants::{DAVE_INITIAL_VERSION, MAX_PENDING_PROPOSALS, SILENCE_FRAME},
        session::types::map_boxed_err,
    },
};
const DAVE_MIN_VERSION: NonZeroU16 = match NonZeroU16::new(DAVE_INITIAL_VERSION) {
    Some(v) => v,
    None => unreachable!(),
};
pub struct DaveHandler {
    session: Option<DaveSession>,
    user_id: UserId,
    channel_id: ChannelId,
    protocol_version: u16,
    pending_transitions: HashMap<u16, u16>,
    external_sender_set: bool,
    saved_external_sender: Option<Vec<u8>>,
    pending_proposals: Vec<Vec<u8>>,
    pending_handshake: Vec<(Vec<u8>, bool)>,
    was_ready: bool,
    recognized_users: HashSet<UserId>,
    cached_user_ids: Vec<u64>,
}
impl DaveHandler {
    pub fn new(user_id: UserId, channel_id: ChannelId) -> Self {
        let mut recognized_users = HashSet::new();
        recognized_users.insert(user_id);
        Self {
            session: None,
            user_id,
            channel_id,
            protocol_version: 0,
            pending_transitions: HashMap::new(),
            external_sender_set: false,
            saved_external_sender: None,
            pending_proposals: Vec::new(),
            pending_handshake: Vec::new(),
            was_ready: false,
            recognized_users,
            cached_user_ids: vec![user_id.0],
        }
    }
    pub fn add_users(&mut self, uids: &[u64]) {
        for &uid in uids {
            self.recognized_users.insert(UserId(uid));
        }
        self.update_user_cache();
        debug!("DAVE adding users: {:?}", uids);
    }
    pub fn remove_user(&mut self, uid: u64) {
        if self.recognized_users.remove(&UserId(uid)) {
            self.update_user_cache();
        }
        debug!("DAVE removing user: {}", uid);
    }
    fn update_user_cache(&mut self) {
        self.cached_user_ids.clear();
        self.cached_user_ids
            .extend(self.recognized_users.iter().map(|u| u.0));
        self.cached_user_ids.sort_unstable();
    }
    pub fn protocol_version(&self) -> u16 {
        self.protocol_version
    }
    pub fn set_protocol_version(&mut self, version: u16) {
        self.protocol_version = version;
    }
    pub fn setup_session(&mut self, version: u16) -> AnyResult<Vec<u8>> {
        if version == 0 {
            self.reset();
            return Ok(Vec::new());
        }
        let nz_version = NonZeroU16::new(version).unwrap_or(DAVE_MIN_VERSION);
        if let Some(s) = &mut self.session {
            s.reinit(nz_version, self.user_id.0, self.channel_id.0, None)
                .map_err(map_boxed_err)?;
        } else {
            let session = DaveSession::new(nz_version, self.user_id.0, self.channel_id.0, None)
                .map_err(map_boxed_err)?;
            self.session = Some(session);
        }
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| map_boxed_err("DAVE session initialization failed"))?;
        self.protocol_version = version;
        self.external_sender_set = false;
        self.pending_proposals.clear();
        self.pending_handshake.clear();
        self.was_ready = false;
        debug!("DAVE session setup (v{})", version);
        let key_package = session.create_key_package().map_err(map_boxed_err)?;
        if let Some(saved) = self.saved_external_sender.as_deref()
            && let Some(sess) = &mut self.session
        {
            match sess.set_external_sender(saved) {
                Ok(()) => {
                    self.external_sender_set = true;
                    debug!("DAVE re-applied saved external sender after epoch reset");
                }
                Err(e) => {
                    warn!("DAVE failed to re-apply saved external sender: {e}");
                    self.saved_external_sender = None;
                }
            }
        }
        Ok(key_package)
    }
    pub fn reset(&mut self) {
        self.protocol_version = 0;
        self.pending_transitions.clear();
        self.external_sender_set = false;
        self.saved_external_sender = None;
        self.pending_proposals.clear();
        self.pending_handshake.clear();
        self.was_ready = false;
        self.session = None;
        debug!("DAVE session reset to plaintext");
    }
    pub fn prepare_transition(&mut self, transition_id: u16, protocol_version: u16) -> bool {
        self.pending_transitions
            .insert(transition_id, protocol_version);
        if transition_id == 0 {
            self.execute_transition(0);
            return false;
        }
        true
    }
    pub fn execute_transition(&mut self, transition_id: u16) {
        if let Some(next_version) = self.pending_transitions.remove(&transition_id) {
            self.protocol_version = next_version;
            trace!(
                "DAVE transition {} executed (v{})",
                transition_id, next_version
            );
        }
    }
    pub fn prepare_epoch(&mut self, epoch: u64, protocol_version: u16) -> Option<Vec<u8>> {
        if epoch == 1 {
            match self.setup_session(protocol_version) {
                Ok(kp) => return Some(kp),
                Err(e) => warn!("DAVE prepare_epoch setup failed: {e}"),
            }
        }
        None
    }
    pub fn process_external_sender(&mut self, data: &[u8]) -> AnyResult<Vec<Vec<u8>>> {
        let mut responses = Vec::new();
        if let Some(session) = &mut self.session {
            session.set_external_sender(data).map_err(map_boxed_err)?;
            self.external_sender_set = true;
            self.saved_external_sender = Some(data.to_vec());
            if !self.pending_proposals.is_empty() {
                debug!(
                    "DAVE processing {} buffered proposals",
                    self.pending_proposals.len()
                );
                for prop_data in std::mem::take(&mut self.pending_proposals) {
                    if let Ok(Some(res)) =
                        Self::do_process_proposals(session, &prop_data, &self.cached_user_ids)
                    {
                        responses.push(res);
                    }
                }
            }
            if !self.pending_handshake.is_empty() {
                debug!(
                    "DAVE processing {} buffered handshake messages",
                    self.pending_handshake.len()
                );
                for (handshake_data, is_welcome) in std::mem::take(&mut self.pending_handshake) {
                    if let Err(e) = self.do_process_handshake(&handshake_data, is_welcome) {
                        warn!("DAVE buffered handshake processing failed: {e}");
                    }
                }
            }
        }
        Ok(responses)
    }
    pub fn process_welcome(&mut self, data: &[u8]) -> AnyResult<u16> {
        self.process_handshake_message(data, true)
    }
    pub fn process_commit(&mut self, data: &[u8]) -> AnyResult<u16> {
        self.process_handshake_message(data, false)
    }
    fn process_handshake_message(&mut self, data: &[u8], is_welcome: bool) -> AnyResult<u16> {
        let tag = if is_welcome { "welcome" } else { "commit" };
        if data.len() < 2 {
            let msg = if is_welcome {
                "DAVE welcome"
            } else {
                "DAVE commit"
            };
            return Err(short_payload_err(msg));
        }
        let transition_id = u16::from_be_bytes([data[0], data[1]]);
        if !self.external_sender_set {
            if self.pending_handshake.len() < MAX_PENDING_PROPOSALS {
                debug!("DAVE buffering {tag} — external sender not set");
                self.pending_handshake.push((data.to_vec(), is_welcome));
            } else {
                warn!("DAVE handshake buffer full, dropping {tag}");
            }
            return Ok(transition_id);
        }
        self.do_process_handshake(data, is_welcome)?;
        Ok(transition_id)
    }
    fn do_process_handshake(&mut self, data: &[u8], is_welcome: bool) -> AnyResult<()> {
        let transition_id = u16::from_be_bytes([data[0], data[1]]);
        if let Some(session) = &mut self.session {
            if is_welcome {
                session.process_welcome(&data[2..]).map_err(map_boxed_err)?;
            } else {
                session.process_commit(&data[2..]).map_err(map_boxed_err)?;
            }
            if transition_id != 0 {
                self.pending_transitions
                    .insert(transition_id, self.protocol_version);
            }
            debug!(
                "DAVE {} processed (tid {})",
                if is_welcome { "welcome" } else { "commit" },
                transition_id
            );
        }
        Ok(())
    }
    pub fn process_proposals(&mut self, data: &[u8]) -> AnyResult<Option<Vec<u8>>> {
        if data.is_empty() {
            return Err(short_payload_err("DAVE proposals"));
        }
        if !self.external_sender_set {
            if self.pending_proposals.len() < MAX_PENDING_PROPOSALS {
                debug!("DAVE buffering proposal — external sender not set");
                self.pending_proposals.push(data.to_vec());
            } else {
                warn!("DAVE proposal buffer full, dropping proposal");
            }
            return Ok(None);
        }
        let session = match &mut self.session {
            Some(s) => s,
            None => return Ok(None),
        };
        Self::do_process_proposals(session, data, &self.cached_user_ids)
    }
    fn do_process_proposals(
        session: &mut DaveSession,
        data: &[u8],
        user_ids: &[u64],
    ) -> AnyResult<Option<Vec<u8>>> {
        let op_type = match data[0] {
            0 => ProposalsOperationType::APPEND,
            1 => ProposalsOperationType::REVOKE,
            raw => return Err(map_boxed_err(format!("Unknown DAVE proposals op: {raw}"))),
        };
        let result = session
            .process_proposals(op_type, &data[1..], Some(user_ids))
            .map_err(map_boxed_err)?;
        if let Some(cw) = result {
            let mut out = cw.commit;
            if let Some(w) = cw.welcome {
                out.extend_from_slice(&w);
            }
            return Ok(Some(out));
        }
        Ok(None)
    }
    pub fn encrypt_opus(&mut self, packet: &[u8]) -> AnyResult<Vec<u8>> {
        if packet == SILENCE_FRAME || self.protocol_version == 0 {
            return Ok(packet.to_vec());
        }
        if let Some(session) = &mut self.session {
            let is_ready = session.is_ready();
            if is_ready != self.was_ready {
                if is_ready {
                    debug!("DAVE session (v{}) is READY", self.protocol_version);
                } else {
                    warn!("DAVE session (v{}) LOST readiness", self.protocol_version);
                }
                self.was_ready = is_ready;
            }
            if is_ready {
                return session
                    .encrypt_opus(packet)
                    .map(|c| c.into_owned())
                    .map_err(map_boxed_err);
            }
        }
        Ok(packet.to_vec())
    }
    pub fn voice_privacy_code(&self) -> Option<String> {
        self.session
            .as_ref()
            .and_then(|s| s.voice_privacy_code().map(|c| c.to_string()))
    }
}
#[inline]
fn short_payload_err(context: &str) -> AnyError {
    map_boxed_err(format!("Invalid {context} payload: too short"))
}
}
pub mod engine {
use tokio::sync::Mutex;
use crate::{audio::Mixer, common::types::Shared, gateway::constants::DEFAULT_SAMPLE_RATE};
pub struct VoiceEngine {
    pub mixer: Shared<Mixer>,
    pub dave: Option<Shared<crate::gateway::DaveHandler>>,
}
impl VoiceEngine {
    pub fn new() -> Self {
        Self {
            mixer: Shared::new(Mutex::new(Mixer::new(DEFAULT_SAMPLE_RATE))),
            dave: None,
        }
    }
}
impl Default for VoiceEngine {
    fn default() -> Self {
        Self::new()
    }
}
}
pub use encryption::DaveHandler;
pub use engine::VoiceEngine;
pub use session::{VoiceGateway, VoiceGatewayConfig};