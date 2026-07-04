use dashmap::DashMap;
use rand::seq::SliceRandom;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

pub use lavende_core::player::{
    ChannelMixFilter, EqBand, Filters, RotationFilter, TimescaleFilter, TremoloFilter,
    VibratoFilter,
};
pub use lavende_core::protocol::events::{TrackEndReason, TrackException};
pub use lavende_core::protocol::tracks::{
    LoadResult, PlaylistData, SearchResult, Track, TrackInfo,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LavendeEvent {
    TrackStart {
        guild_id: String,
        track: Track,
    },
    TrackEnd {
        guild_id: String,
        track: Track,
        reason: TrackEndReason,
    },
    TrackException {
        guild_id: String,
        track: Track,
        exception: TrackException,
    },
    TrackStuck {
        guild_id: String,
        track: Track,
        threshold_ms: u64,
    },
    Position {
        guild_id: String,
        position: i64,
    },
    Paused {
        guild_id: String,
    },
    Resumed {
        guild_id: String,
    },
    VolumeChanged {
        guild_id: String,
        volume: f64,
    },
    QueueEnd {
        guild_id: String,
    },
    Error {
        guild_id: String,
        message: String,
    },
    PlayerDestroy {
        guild_id: String,
        reason: Option<String>,
    },
}

#[derive(Debug, Default, Clone)]
pub struct Queue {
    pub tracks: VecDeque<Track>,
    pub current: Option<Track>,
    pub previous: Vec<Track>,
    pub guild_id: String,
}

impl Queue {
    pub fn new(guild_id: String) -> Self {
        Self {
            guild_id,
            tracks: VecDeque::new(),
            current: None,
            previous: Vec::new(),
        }
    }
    pub fn size(&self) -> usize {
        self.tracks.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
    pub fn add(&mut self, track: Track) {
        self.tracks.push_back(track);
    }
    pub fn add_multiple(&mut self, tracks: impl IntoIterator<Item = Track>) {
        self.tracks.extend(tracks);
    }
    pub fn remove(&mut self, index: usize) -> Option<Track> {
        if index < self.tracks.len() {
            self.tracks.remove(index)
        } else {
            None
        }
    }
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current = None;
        self.previous.clear();
    }
    pub fn shuffle(&mut self) {
        let mut vec: Vec<Track> = self.tracks.drain(..).collect();
        vec.shuffle(&mut rand::rng());
        self.tracks = vec.into();
    }
    pub fn next(&mut self) -> Option<Track> {
        self.tracks.pop_front()
    }
    pub fn total_duration(&self) -> u64 {
        let mut total = self.current.as_ref().map(|t| t.info.length).unwrap_or(0);
        for t in &self.tracks {
            total += t.info.length;
        }
        total
    }
    pub fn filter_tracks<F>(&self, mut f: F) -> Vec<(usize, Track)>
    where
        F: FnMut(&Track, usize) -> bool,
    {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(i, t)| f(t, *i))
            .map(|(i, t)| (i, t.clone()))
            .collect()
    }
    pub fn find_track<F>(&self, f: F) -> Option<(usize, Track)>
    where
        F: FnMut(&Track, usize) -> bool,
    {
        self.filter_tracks(f).into_iter().next()
    }
}

#[derive(Debug, Default, Clone)]
pub struct FilterManager {
    pub filters: Filters,
}

impl FilterManager {
    pub fn new() -> Self {
        Self {
            filters: Filters::default(),
        }
    }
    pub fn set_volume(&mut self, volume: f32) -> &mut Self {
        self.filters.volume = Some(volume);
        self
    }
    pub fn set_equalizer(&mut self, bands: Vec<EqBand>) -> &mut Self {
        self.filters.equalizer = if bands.is_empty() { None } else { Some(bands) };
        self
    }
    pub fn set_timescale(&mut self, speed: f64, pitch: f64, rate: f64) -> &mut Self {
        self.filters.timescale = Some(TimescaleFilter {
            speed: Some(speed),
            pitch: Some(pitch),
            rate: Some(rate),
        });
        self
    }
    pub fn set_speed(&mut self, speed: f64) -> &mut Self {
        let mut ts = self.filters.timescale.clone().unwrap_or(TimescaleFilter {
            speed: None,
            pitch: None,
            rate: None,
        });
        ts.speed = Some(speed);
        self.filters.timescale = Some(ts);
        self
    }
    pub fn set_pitch(&mut self, pitch: f64) -> &mut Self {
        let mut ts = self.filters.timescale.clone().unwrap_or(TimescaleFilter {
            speed: None,
            pitch: None,
            rate: None,
        });
        ts.pitch = Some(pitch);
        self.filters.timescale = Some(ts);
        self
    }
    pub fn set_rate(&mut self, rate: f64) -> &mut Self {
        let mut ts = self.filters.timescale.clone().unwrap_or(TimescaleFilter {
            speed: None,
            pitch: None,
            rate: None,
        });
        ts.rate = Some(rate);
        self.filters.timescale = Some(ts);
        self
    }
    pub fn toggle_tremolo(&mut self, frequency: f32, depth: f32) -> &mut Self {
        if self.filters.tremolo.is_some() {
            self.filters.tremolo = None;
        } else {
            self.filters.tremolo = Some(TremoloFilter {
                frequency: Some(frequency),
                depth: Some(depth),
            });
        }
        self
    }
    pub fn toggle_vibrato(&mut self, frequency: f32, depth: f32) -> &mut Self {
        if self.filters.vibrato.is_some() {
            self.filters.vibrato = None;
        } else {
            self.filters.vibrato = Some(VibratoFilter {
                frequency: Some(frequency),
                depth: Some(depth),
            });
        }
        self
    }
    pub fn toggle_rotation(&mut self, rotation_hz: f64) -> &mut Self {
        if self.filters.rotation.is_some() {
            self.filters.rotation = None;
        } else {
            self.filters.rotation = Some(RotationFilter {
                rotation_hz: Some(rotation_hz),
            });
        }
        self
    }
    pub fn set_audio_output(&mut self, mode: &str) -> &mut Self {
        let mix = match mode {
            "mono" => ChannelMixFilter {
                left_to_left: Some(0.5),
                left_to_right: Some(0.5),
                right_to_left: Some(0.5),
                right_to_right: Some(0.5),
            },
            "left" => ChannelMixFilter {
                left_to_left: Some(1.0),
                left_to_right: Some(0.0),
                right_to_left: Some(1.0),
                right_to_right: Some(0.0),
            },
            "right" => ChannelMixFilter {
                left_to_left: Some(0.0),
                left_to_right: Some(1.0),
                right_to_left: Some(0.0),
                right_to_right: Some(1.0),
            },
            _ => ChannelMixFilter {
                left_to_left: Some(1.0),
                left_to_right: Some(0.0),
                right_to_left: Some(0.0),
                right_to_right: Some(1.0),
            },
        };
        self.filters.channel_mix = Some(mix);
        self
    }
    pub fn reset_filters(&mut self) -> &mut Self {
        self.filters = Filters::default();
        self
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.filters).unwrap_or_else(|_| "{}".to_string())
    }
}

#[derive(Clone)]
pub struct VoiceState {
    pub session_id: Option<String>,
    pub token: Option<String>,
    pub endpoint: Option<String>,
    pub voice_channel_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    Track,
    Queue,
}

pub struct LavendePlayer {
    pub guild_id: String,
    pub queue: Arc<RwLock<Queue>>,
    pub filter_manager: Arc<RwLock<FilterManager>>,
    pub voice_state: Arc<RwLock<VoiceState>>,
    pub repeat_mode: Arc<RwLock<RepeatMode>>,
    pub volume: Arc<RwLock<u32>>,
    pub paused: Arc<RwLock<bool>>,
    pub data: Arc<DashMap<String, serde_json::Value>>,

    native_player: Arc<lavende_core::Player>,
    event_sender: broadcast::Sender<LavendeEvent>,
    play_on_connect: Arc<RwLock<bool>>,
    manager_client_id: String,
    send_to_shard: Arc<dyn Fn(String, serde_json::Value) + Send + Sync>,
}

impl LavendePlayer {
    pub fn new(
        guild_id: String,
        client_id: String,
        event_sender: broadcast::Sender<LavendeEvent>,
        send_to_shard: Arc<dyn Fn(String, serde_json::Value) + Send + Sync>,
    ) -> Self {
        Self {
            guild_id: guild_id.clone(),
            queue: Arc::new(RwLock::new(Queue::new(guild_id.clone()))),
            filter_manager: Arc::new(RwLock::new(FilterManager::new())),
            voice_state: Arc::new(RwLock::new(VoiceState {
                session_id: None,
                token: None,
                endpoint: None,
                voice_channel_id: None,
            })),
            repeat_mode: Arc::new(RwLock::new(RepeatMode::Off)),
            volume: Arc::new(RwLock::new(100)),
            paused: Arc::new(RwLock::new(false)),
            data: Arc::new(DashMap::new()),
            native_player: Arc::new(lavende_core::Player::new(guild_id)),
            event_sender,
            play_on_connect: Arc::new(RwLock::new(false)),
            manager_client_id: client_id,
            send_to_shard,
        }
    }

    pub fn set_data(&self, key: &str, value: serde_json::Value) {
        self.data.insert(key.to_string(), value);
    }
    pub fn get_data(&self, key: &str) -> Option<serde_json::Value> {
        self.data.get(key).map(|v| v.clone())
    }
    pub fn delete_data(&self, key: &str) {
        self.data.remove(key);
    }
    pub fn clear_data(&self) {
        self.data.clear();
    }
    pub fn get_all_data(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        for kv in self.data.iter() {
            map.insert(kv.key().clone(), kv.value().clone());
        }
        map
    }

    pub async fn subscribe(&self) -> broadcast::Receiver<LavendeEvent> {
        self.event_sender.subscribe()
    }

    pub async fn connect(&self, channel_id: Option<String>, self_deaf: bool, self_mute: bool) {
        if let Some(ref c) = channel_id {
            self.voice_state.write().await.voice_channel_id = Some(c.clone());
        }
        let payload = serde_json::json!({
            "op": 4,
            "d": {
                "guild_id": self.guild_id,
                "channel_id": channel_id,
                "self_mute": self_mute,
                "self_deaf": self_deaf
            }
        });
        (self.send_to_shard)(self.guild_id.clone(), payload);
    }

    pub async fn disconnect(&self) {
        self.voice_state.write().await.voice_channel_id = None;
        let payload = serde_json::json!({
            "op": 4,
            "d": {
                "guild_id": self.guild_id,
                "channel_id": serde_json::Value::Null,
                "self_mute": false,
                "self_deaf": false
            }
        });
        (self.send_to_shard)(self.guild_id.clone(), payload);
        self.stop().await;
    }

    pub async fn destroy(&self, reason: Option<String>) {
        self.disconnect().await;
        let _ = self.event_sender.send(LavendeEvent::PlayerDestroy {
            guild_id: self.guild_id.clone(),
            reason,
        });
    }

    pub async fn search(&self, query: &str) -> Result<LoadResult, String> {
        load(query.to_string()).await
    }

    pub async fn skip(&self) {
        self.stop().await;
    }

    pub async fn update_voice_state(
        &self,
        session_id: Option<String>,
        token: Option<String>,
        endpoint: Option<String>,
        channel_id: Option<String>,
    ) {
        {
            let mut vs = self.voice_state.write().await;
            if let Some(s) = session_id {
                vs.session_id = Some(s);
            }
            if let Some(t) = token {
                vs.token = Some(t);
            }
            if let Some(e) = endpoint {
                vs.endpoint = Some(e);
            }
            if let Some(c) = channel_id {
                vs.voice_channel_id = Some(c);
            }
        }
        self.check_play_on_connect().await;
    }

    async fn check_play_on_connect(&self) {
        let vs = self.voice_state.read().await;
        if vs.session_id.is_some() && vs.token.is_some() && vs.endpoint.is_some() {
            let mut poc = self.play_on_connect.write().await;
            if *poc {
                *poc = false;
                let self_clone = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = self_clone.play().await {
                        let _ = self_clone.event_sender.send(LavendeEvent::Error {
                            guild_id: self_clone.guild_id.clone(),
                            message: e,
                        });
                    }
                });
            }
        }
    }

    pub async fn play(&self) -> Result<(), String> {
        let current_track = {
            let mut q = self.queue.write().await;
            if q.current.is_none() {
                if let Some(next) = q.next() {
                    q.current = Some(next);
                } else {
                    let _ = self.event_sender.send(LavendeEvent::QueueEnd {
                        guild_id: self.guild_id.clone(),
                    });
                    return Ok(());
                }
            }
            q.current.clone().unwrap()
        };

        let vs = self.voice_state.read().await.clone();
        if vs.session_id.is_none() || vs.token.is_none() || vs.endpoint.is_none() {
            *self.play_on_connect.write().await = true;
            return Ok(());
        }

        let float_volume = *self.volume.read().await as f64 / 100.0;
        self.native_player.set_volume(float_volume).await;

        let guild_id = self.guild_id.clone();
        let tx = self.event_sender.clone();
        let current_track_clone = current_track.clone();
        let self_clone = self.clone();

        self.native_player
            .play(
                self.manager_client_id.clone(),
                vs.voice_channel_id.unwrap_or_default(),
                vs.session_id.unwrap(),
                vs.token.unwrap(),
                vs.endpoint.unwrap(),
                current_track
                    .info
                    .uri
                    .unwrap_or_else(|| current_track.info.identifier.clone()),
                move |event_name, payload| match event_name {
                    "trackStart" => {
                        let _ = tx.send(LavendeEvent::TrackStart {
                            guild_id: guild_id.clone(),
                            track: current_track_clone.clone(),
                        });
                    }
                    "trackEnd" => {
                        let reason_str = payload
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("finished");
                        let reason = serde_json::from_value(serde_json::json!(reason_str))
                            .unwrap_or(TrackEndReason::Finished);
                        let _ = tx.send(LavendeEvent::TrackEnd {
                            guild_id: guild_id.clone(),
                            track: current_track_clone.clone(),
                            reason,
                        });
                        let sc = self_clone.clone();
                        tokio::spawn(sc.handle_track_end());
                    }
                    "position" => {
                        if let Some(pos) = payload.get("position").and_then(|v| v.as_i64()) {
                            let _ = tx.send(LavendeEvent::Position {
                                guild_id: guild_id.clone(),
                                position: pos,
                            });
                        }
                    }
                    "error" => {
                        let msg = payload
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error")
                            .to_string();
                        let _ = tx.send(LavendeEvent::Error {
                            guild_id: guild_id.clone(),
                            message: msg,
                        });
                    }
                    _ => {}
                },
            )
            .await
    }

    fn handle_track_end(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let self_clone = self.clone();
        Box::pin(async move {
            let (repeat_mode, finished_track_opt) = {
                let mode = self_clone.repeat_mode.read().await.clone();
                let mut q = self_clone.queue.write().await;
                (mode, q.current.take())
            };
            if let Some(finished_track) = finished_track_opt {
                match repeat_mode {
                    RepeatMode::Track => {
                        self_clone.queue.write().await.current = Some(finished_track);
                    }
                    RepeatMode::Queue => {
                        self_clone.queue.write().await.add(finished_track);
                    }
                    RepeatMode::Off => {
                        self_clone.queue.write().await.previous.push(finished_track);
                    }
                }
            }
            let _ = self_clone.play().await;
        })
    }

    pub async fn pause(&self, pause_state: bool) {
        *self.paused.write().await = pause_state;
        if pause_state {
            self.native_player.pause().await;
        } else {
            self.native_player.resume().await;
        }
    }
    pub async fn resume(&self) {
        self.pause(false).await;
    }
    pub async fn stop(&self) {
        self.queue.write().await.current = None;
        self.native_player.stop().await;
    }
    pub async fn seek(&self, position_ms: i64) {
        self.native_player.seek(position_ms).await;
    }
    pub async fn set_volume(&self, volume: u32) {
        *self.volume.write().await = volume;
        self.native_player.set_volume(volume as f64 / 100.0).await;
    }
    pub async fn set_repeat_mode(&self, mode: RepeatMode) {
        *self.repeat_mode.write().await = mode;
    }
    pub async fn apply_filters(&self) {
        let json_str = self.filter_manager.read().await.to_json();
        let _ = self.native_player.set_filters(json_str).await;
    }
    pub async fn set_filters(&self, json_str: String) {
        let _ = self.native_player.set_filters(json_str).await;
    }
    pub fn get_position(&self) -> i64 {
        self.native_player.get_position()
    }
    pub fn is_paused(&self) -> bool {
        self.native_player.is_paused()
    }
}

impl Clone for LavendePlayer {
    fn clone(&self) -> Self {
        Self {
            guild_id: self.guild_id.clone(),
            queue: self.queue.clone(),
            filter_manager: self.filter_manager.clone(),
            voice_state: self.voice_state.clone(),
            repeat_mode: self.repeat_mode.clone(),
            volume: self.volume.clone(),
            paused: self.paused.clone(),
            data: self.data.clone(),
            native_player: self.native_player.clone(),
            event_sender: self.event_sender.clone(),
            play_on_connect: self.play_on_connect.clone(),
            manager_client_id: self.manager_client_id.clone(),
            send_to_shard: self.send_to_shard.clone(),
        }
    }
}

pub struct LavendeManager {
    pub players: DashMap<String, LavendePlayer>,
    pub client_id: String,
    event_tx: broadcast::Sender<LavendeEvent>,
    send_to_shard: Arc<dyn Fn(String, serde_json::Value) + Send + Sync>,
}

impl LavendeManager {
    pub fn new<F>(client_id: String, send_to_shard_fn: F) -> Self
    where
        F: Fn(String, serde_json::Value) + Send + Sync + 'static,
    {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            players: DashMap::new(),
            client_id,
            event_tx,
            send_to_shard: Arc::new(send_to_shard_fn),
        }
    }

    pub fn create_player(&self, guild_id: &str) -> LavendePlayer {
        self.get_or_create_player(guild_id)
    }
    pub fn get_or_create_player(&self, guild_id: &str) -> LavendePlayer {
        if let Some(p) = self.players.get(guild_id) {
            return p.clone();
        }
        let player = LavendePlayer::new(
            guild_id.to_string(),
            self.client_id.clone(),
            self.event_tx.clone(),
            self.send_to_shard.clone(),
        );
        self.players.insert(guild_id.to_string(), player.clone());
        player
    }

    pub fn get_player(&self, guild_id: &str) -> Option<LavendePlayer> {
        self.players.get(guild_id).map(|p| p.clone())
    }

    pub async fn destroy_player(&self, guild_id: &str) {
        if let Some((_, player)) = self.players.remove(guild_id) {
            player.destroy(None).await;
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<LavendeEvent> {
        self.event_tx.subscribe()
    }

    pub async fn send_raw_data(&self, packet: &serde_json::Value) {
        let t = packet.get("t").and_then(|v| v.as_str());
        let d = packet.get("d");
        if let (Some(t_str), Some(data)) = (t, d) {
            if t_str == "VOICE_STATE_UPDATE" {
                if data.get("user_id").and_then(|v| v.as_str()) == Some(&self.client_id) {
                    if let Some(guild_id) = data.get("guild_id").and_then(|v| v.as_str()) {
                        if let Some(player) = self.get_player(guild_id) {
                            let session_id = data
                                .get("session_id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let channel_id = data
                                .get("channel_id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            player
                                .update_voice_state(session_id, None, None, channel_id)
                                .await;
                        }
                    }
                }
            } else if t_str == "VOICE_SERVER_UPDATE" {
                if let Some(guild_id) = data.get("guild_id").and_then(|v| v.as_str()) {
                    if let Some(player) = self.get_player(guild_id) {
                        let token = data
                            .get("token")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let endpoint = data
                            .get("endpoint")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        player.update_voice_state(None, token, endpoint, None).await;
                    }
                }
            }
        }
    }
}

pub async fn load(identifier: String) -> Result<LoadResult, String> {
    let json_str = lavende_core::load(identifier).await?;
    serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse load result: {}", e))
}

pub const VALID_SPONSOR_BLOCKS: &[&str] = &[
    "sponsor",
    "selfpromo",
    "interaction",
    "intro",
    "outro",
    "preview",
    "music_offtopic",
    "filler",
];

pub fn get_eq_list_bassboost_earrape() -> Vec<EqBand> {
    vec![
        EqBand {
            band: 0,
            gain: 0.225,
        },
        EqBand {
            band: 1,
            gain: 0.25125,
        },
        EqBand {
            band: 2,
            gain: 0.25125,
        },
        EqBand {
            band: 3,
            gain: 0.15,
        },
        EqBand {
            band: 4,
            gain: -0.1875,
        },
        EqBand {
            band: 5,
            gain: 0.05625,
        },
        EqBand {
            band: 6,
            gain: -0.16875,
        },
        EqBand {
            band: 7,
            gain: 0.08625,
        },
        EqBand {
            band: 8,
            gain: 0.13125,
        },
        EqBand {
            band: 9,
            gain: 0.16875,
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugEvents {
    SetSponsorBlock,
    DeleteSponsorBlock,
    TrackEndReplaced,
    AutoplayExecution,
    AutoplayNoSongsAdded,
    AutoplayThresholdSpamLimiter,
    TriggerQueueEmptyInterval,
    QueueEnded,
    TrackStartNewSongsOnly,
    TrackStartNoTrack,
    ResumingFetchingError,
    PlayerUpdateNoPlayer,
    PlayerUpdateFilterFixApply,
    PlayerUpdateSuccess,
    HeartBeatTriggered,
    NoSocketOnDestroy,
    SocketCleanupError,
    SocketTerminateHeartBeatTimeout,
    TryingConnectWhileConnected,
    LavaSearchNothingFound,
    SearchNothingFound,
    ValidatingBlacklistLinks,
    ValidatingWhitelistLinks,
    TrackErrorMaxTracksErroredPerTime,
    TrackStuckMaxTracksErroredPerTime,
    PlayerDestroyingSomewhereElse,
    PlayerCreateNodeNotFound,
    PlayerPlayQueueEmptyTimeoutClear,
    PlayerPlayWithTrackReplace,
    PlayerPlayUnresolvedTrack,
    PlayerPlayUnresolvedTrackFailed,
    PlayerVolumeAsFilter,
    BandcampSearchLokalEngine,
    PlayerChangeNode,
    BuildTrackError,
    TransformRequesterFunctionFailed,
    GetClosestTrackFailed,
    PlayerDeleteInsteadOfDestroy,
    FailedToConnectToNodes,
    NoAudioDebug,
    PlayerAutoReconnect,
    PlayerDestroyFail,
    PlayerChangeNodeFailNoEligibleNode,
    PlayerChangeNodeFail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestroyReasons {
    QueueEmpty,
    NodeDestroy,
    NodeDeleted,
    LavalinkNoVoice,
    NodeReconnectFail,
    Disconnected,
    PlayerReconnectFail,
    PlayerChangeNodeFail,
    PlayerChangeNodeFailNoEligibleNode,
    ChannelDeleted,
    DisconnectAllNodes,
    ReconnectAllNodes,
    TrackErrorMaxTracksErroredPerTime,
    TrackStuckMaxTracksErroredPerTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReasons {
    Disconnected,
    DisconnectAllNodes,
}

pub struct MiniMap<K, V> {
    pub map: HashMap<K, V>,
}
impl<K: Eq + std::hash::Hash, V> MiniMap<K, V> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn filter<F>(&self, mut f: F) -> Self
    where
        F: FnMut(&V, &K) -> bool,
        V: Clone,
        K: Clone,
    {
        let mut results = HashMap::new();
        for (k, v) in &self.map {
            if f(v, k) {
                results.insert(k.clone(), v.clone());
            }
        }
        Self { map: results }
    }
    pub fn map<T, F>(&self, mut f: F) -> Vec<T>
    where
        F: FnMut(&V, &K) -> T,
    {
        let mut results = Vec::new();
        for (k, v) in &self.map {
            results.push(f(v, k));
        }
        results
    }
}

pub const DEFAULT_SOURCES: &[(&str, &str)] = &[
    ("youtube music", "ytmsearch"),
    ("youtubemusic", "ytmsearch"),
    ("ytmsearch", "ytmsearch"),
    ("ytm", "ytmsearch"),
    ("musicyoutube", "ytmsearch"),
    ("music youtube", "ytmsearch"),
    ("youtube", "ytsearch"),
    ("yt", "ytsearch"),
    ("ytsearch", "ytsearch"),
    ("soundcloud", "scsearch"),
    ("scsearch", "scsearch"),
    ("sc", "scsearch"),
    ("apple music", "amsearch"),
    ("apple", "amsearch"),
    ("applemusic", "amsearch"),
    ("amsearch", "amsearch"),
    ("am", "amsearch"),
    ("musicapple", "amsearch"),
    ("music apple", "amsearch"),
    ("spotify", "spsearch"),
    ("spsearch", "spsearch"),
    ("sp", "spsearch"),
    ("spotify.com", "spsearch"),
    ("spotifycom", "spsearch"),
    ("sprec", "sprec"),
    ("spsuggestion", "sprec"),
    ("deezer", "dzsearch"),
    ("dz", "dzsearch"),
    ("dzsearch", "dzsearch"),
    ("dzisrc", "dzisrc"),
    ("dzrec", "dzrec"),
    ("yandex music", "ymsearch"),
    ("yandexmusic", "ymsearch"),
    ("yandex", "ymsearch"),
    ("ymsearch", "ymsearch"),
    ("ymrec", "ymrec"),
    ("vksearch", "vksearch"),
    ("vkmusic", "vksearch"),
    ("vk music", "vksearch"),
    ("vkrec", "vkrec"),
    ("vk", "vksearch"),
    ("qbsearch", "qbsearch"),
    ("qobuz", "qbsearch"),
    ("qbisrc", "qbisrc"),
    ("qbrec", "qbrec"),
    ("pandora", "pdsearch"),
    ("pd", "pdsearch"),
    ("pdsearch", "pdsearch"),
    ("pandora music", "pdsearch"),
    ("pandoramusic", "pdsearch"),
    ("speak", "speak"),
    ("tts", "tts"),
    ("ftts", "ftts"),
    ("flowery", "ftts"),
    ("flowery.tts", "ftts"),
    ("flowerytts", "ftts"),
    ("bandcamp", "bcsearch"),
    ("bc", "bcsearch"),
    ("bcsearch", "bcsearch"),
    ("phsearch", "phsearch"),
    ("pornhub", "phsearch"),
    ("porn", "phsearch"),
    ("local", "local"),
    ("http", "http"),
    ("https", "https"),
    ("link", "link"),
    ("uri", "uri"),
    ("tidal", "tdsearch"),
    ("td", "tdsearch"),
    ("tidal music", "tdsearch"),
    ("tdrec", "tdrec"),
    ("jiosaavn", "jssearch"),
    ("js", "jssearch"),
    ("jssearch", "jssearch"),
    ("jsrec", "jsrec"),
    ("amzsearch", "amzsearch"),
    ("admsearch", "admsearch"),
    ("gnsearch", "gnsearch"),
    ("szsearch", "szsearch"),
];

pub const SOURCE_LINKS_REGEXES: &[(&str, &str)] = &[
    (
        "YoutubeRegex",
        r"https?:\/\/?(?:www\.)?(?:(m|www)\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|shorts|playlist\?|watch\?v=|watch\?.+(?:&|&#38;);v=))([a-zA-Z0-9\-_]{11})?(?:(?:\?|&|&#38;)index=((?:\d){1,3}))?(?:(?:\?|&|&#38;)?list=([a-zA-Z\-_0-9]{34}))?(?:\S+)?",
    ),
    (
        "YoutubeMusicRegex",
        r"https?:\/\/?(?:www\.)?(?:(music|m|www)\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|shorts|playlist\?|watch\?v=|watch\?.+(?:&|&#38;);v=))([a-zA-Z0-9\-_]{11})?(?:(?:\?|&|&#38;)index=((?:\d){1,3}))?(?:(?:\?|&|&#38;)?list=([a-zA-Z\-_0-9]{34}))?(?:\S+)?",
    ),
    ("SoundCloudRegex", r"https?:\/\/(?:on\.)?soundcloud\.com\/"),
    (
        "SoundCloudMobileRegex",
        r"https?:\/\/(soundcloud\.app\.goo\.gl)\/(\S+)",
    ),
    (
        "bandcamp",
        r"https?:\/\/?(?:www\.)?([\d|\w]+)\.bandcamp\.com\/(\S+)",
    ),
    ("TwitchTv", r"https?:\/\/?(?:www\.)?twitch\.tv\/\w+"),
    (
        "vimeo",
        r"https?:\/\/(www\.)?vimeo.com\/(?:channels\/(?:\w+\/)?|groups\/([^/]*)\/videos\/|)(\d+)(?:|\/\?)",
    ),
    ("mp3Url", r"(https?|ftp|file):\/\/(www.)?(.*?)\.(mp3)$"),
    ("m3uUrl", r"(https?|ftp|file):\/\/(www.)?(.*?)\.(m3u)$"),
    ("m3u8Url", r"(https?|ftp|file):\/\/(www.)?(.*?)\.(m3u8)$"),
    ("mp4Url", r"(https?|ftp|file):\/\/(www.)?(.*?)\.(mp4)$"),
    ("m4aUrl", r"(https?|ftp|file):\/\/(www.)?(.*?)\.(m4a)$"),
    ("wavUrl", r"(https?|ftp|file):\/\/(www.)?(.*?)\.(wav)$"),
    ("aacpUrl", r"(https?|ftp|file):\/\/(www.)?(.*?)\.(aacp)$"),
    (
        "DeezerTrackRegex",
        r"(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?track\/(\d+)",
    ),
    (
        "DeezerPageLinkRegex",
        r"(https?:\/\/|)?(?:www\.)?deezer\.page\.link\/(\S+)",
    ),
    (
        "DeezerPlaylistRegex",
        r"(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?playlist\/(\d+)",
    ),
    (
        "DeezerAlbumRegex",
        r"(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?album\/(\d+)",
    ),
    (
        "DeezerArtistRegex",
        r"(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?artist\/(\d+)",
    ),
    (
        "DeezerMixesRegex",
        r"(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?mixes\/genre\/(\d+)",
    ),
    (
        "DeezerEpisodeRegex",
        r"(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?episode\/(\d+)",
    ),
    (
        "AllDeezerRegexWithoutPageLink",
        r"(https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?(track|playlist|album|artist|mixes\/genre|episode)\/(\d+)",
    ),
    (
        "AllDeezerRegex",
        r"((https?:\/\/|)?(?:www\.)?deezer\.com\/(?:\w{2}\/)?(track|playlist|album|artist|mixes\/genre|episode)\/(\d+)|(https?:\/\/|)?(?:www\.)?deezer\.page\.link\/(\S+))",
    ),
    (
        "SpotifySongRegex",
        r"(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?track\/(?<identifier>[a-zA-Z0-9-_]+)",
    ),
    (
        "SpotifyPlaylistRegex",
        r"(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?playlist\/(?<identifier>[a-zA-Z0-9-_]+)",
    ),
    (
        "SpotifyArtistRegex",
        r"(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?artist\/(?<identifier>[a-zA-Z0-9-_]+)",
    ),
    (
        "SpotifyEpisodeRegex",
        r"(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?episode\/(?<identifier>[a-zA-Z0-9-_]+)",
    ),
    (
        "SpotifyShowRegex",
        r"(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?show\/(?<identifier>[a-zA-Z0-9-_]+)",
    ),
    (
        "SpotifyAlbumRegex",
        r"(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?album\/(?<identifier>[a-zA-Z0-9-_]+)",
    ),
    (
        "AllSpotifyRegex",
        r"(https?:\/\/)(www\.)?open\.spotify\.com\/((?<region>[a-zA-Z-]+)\/)?(user\/(?<user>[a-zA-Z0-9-_]+)\/)?(?<type>track|album|playlist|artist|episode|show)\/(?<identifier>[a-zA-Z0-9-_]+)",
    ),
    (
        "appleMusic",
        r"https?:\/\/?(?:www\.)?music\.apple\.com\/(\S+)",
    ),
    (
        "tidal",
        r"https?:\/\/?(?:www\.)?(?:tidal|listen)\.tidal\.com\/(?<type>track|album|playlist|artist)\/(?<identifier>[a-zA-Z0-9-_]+)",
    ),
    (
        "jiosaavn",
        r"(https?:\/\/)(www\.)?jiosaavn\.com\/(?<type>song|album|featured|artist)\/([a-zA-Z0-9-_/,]+)",
    ),
    (
        "PandoraTrackRegex",
        r"^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/artist\/[\w-]+(?:\/[\w-]+)*\/(?<identifier>TR[A-Za-z0-9]+)(?:[?#].*)?$",
    ),
    (
        "PandoraAlbumRegex",
        r"^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/artist\/[\w-]+(?:\/[\w-]+)*\/(?<identifier>AL[A-Za-z0-9]+)(?:[?#].*)?$",
    ),
    (
        "PandoraArtistRegex",
        r"^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/artist\/[\w-]+\/(?<identifier>AR[A-Za-z0-9]+)(?:[?#].*)?$",
    ),
    (
        "PandoraPlaylistRegex",
        r"^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/playlist\/(?<identifier>PL:[\d:]+)(?:[?#].*)?$",
    ),
    (
        "AllPandoraRegex",
        r"^@?(?:https?:\/\/)?(?:www\.)?pandora\.com\/(?:playlist\/(?<playlistId>PL:[\d:]+)|artist\/[\w-]+(?:\/[\w-]+)*\/(?<identifier>(?:TR|AL|AR)[A-Za-z0-9]+))(?:[?#].*)?$",
    ),
    ("tiktok", r"https:\/\/www\.tiktok\.com\/"),
    ("mixcloud", r"https:\/\/www\.mixcloud\.com\/"),
    ("musicYandex", r"https:\/\/music\.yandex\.ru\/"),
    ("radiohost", r"https?:\/\/[^.\s]+\.radiohost\.de\/(\S+)"),
];
