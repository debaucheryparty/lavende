use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::audio::{
    Mixer,
    filters::FilterChain,
    playback::{TrackHandle, handle::PlaybackState as PlayState},
};
use crate::common::types::{ChannelId, GuildId, SessionId, Shared, UserId};
use crate::events::EventSender;
use crate::gateway::{VoiceGateway, VoiceGatewayConfig};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub band: u8,
    pub gain: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KaraokeFilter {
    pub level: Option<f32>,
    pub mono_level: Option<f32>,
    pub filter_band: Option<f32>,
    pub filter_width: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimescaleFilter {
    pub speed: Option<f64>,
    pub pitch: Option<f64>,
    pub rate: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TremoloFilter {
    pub frequency: Option<f32>,
    pub depth: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VibratoFilter {
    pub frequency: Option<f32>,
    pub depth: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistortionFilter {
    pub sin_offset: Option<f32>,
    pub sin_scale: Option<f32>,
    pub cos_offset: Option<f32>,
    pub cos_scale: Option<f32>,
    pub tan_offset: Option<f32>,
    pub tan_scale: Option<f32>,
    pub offset: Option<f32>,
    pub scale: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationFilter {
    pub rotation_hz: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMixFilter {
    pub left_to_left: Option<f32>,
    pub left_to_right: Option<f32>,
    pub right_to_left: Option<f32>,
    pub right_to_right: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LowPassFilter {
    pub smoothing: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EchoFilter {
    pub echo_length: Option<f32>,
    pub decay: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighPassFilter {
    pub cutoff_frequency: Option<i32>,
    pub boost_factor: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizationFilter {
    pub max_amplitude: Option<f32>,
    pub adaptive: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChorusFilter {
    pub rate: Option<f32>,
    pub depth: Option<f32>,
    pub delay: Option<f32>,
    pub mix: Option<f32>,
    pub feedback: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressorFilter {
    pub threshold: Option<f32>,
    pub ratio: Option<f32>,
    pub attack: Option<f32>,
    pub release: Option<f32>,
    pub makeup_gain: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlangerFilter {
    pub rate: Option<f32>,
    pub depth: Option<f32>,
    pub feedback: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaserFilter {
    pub stages: Option<i32>,
    pub rate: Option<f32>,
    pub depth: Option<f32>,
    pub feedback: Option<f32>,
    pub mix: Option<f32>,
    pub min_frequency: Option<f32>,
    pub max_frequency: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhonographFilter {
    pub frequency: Option<f32>,
    pub depth: Option<f32>,
    pub crackle: Option<f32>,
    pub flutter: Option<f32>,
    pub room: Option<f32>,
    pub mic_agc: Option<f32>,
    pub drive: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverbFilter {
    pub mix: Option<f32>,
    pub room_size: Option<f32>,
    pub damping: Option<f32>,
    pub width: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialFilter {
    pub depth: Option<f32>,
    pub rate: Option<f32>,
}
macro_rules! define_filters {
    ($($field:ident : $type:ty => $name:expr),* $(,)?) => {
        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Filters {
            $(
                #[serde(skip_serializing_if = "Option::is_none")]
                pub $field: Option<$type>,
            )*
        }
        impl Filters {
            pub fn names() -> Vec<String> {
                vec![
                    $($name.into()),*
                ]
            }
            pub fn merge_from(&mut self, incoming: Filters) {
                $(
                    if incoming.$field.is_some() {
                        self.$field = incoming.$field;
                    }
                )*
            }
            pub fn is_all_none(&self) -> bool {
                $(
                    self.$field.is_none() &&
                )* true
            }
        }
    };
}
define_filters! {
    volume: f32 => "volume",
    equalizer: Vec<EqBand> => "equalizer",
    karaoke: KaraokeFilter => "karaoke",
    timescale: TimescaleFilter => "timescale",
    tremolo: TremoloFilter => "tremolo",
    vibrato: VibratoFilter => "vibrato",
    distortion: DistortionFilter => "distortion",
    rotation: RotationFilter => "rotation",
    channel_mix: ChannelMixFilter => "channelMix",
    low_pass: LowPassFilter => "lowPass",
    echo: EchoFilter => "echo",
    high_pass: HighPassFilter => "highPass",
    normalization: NormalizationFilter => "normalization",
    chorus: ChorusFilter => "chorus",
    compressor: CompressorFilter => "compressor",
    flanger: FlangerFilter => "flanger",
    phaser: PhaserFilter => "phaser",
    phonograph: PhonographFilter => "phonograph",
    reverb: ReverbFilter => "reverb",
    spatial: SpatialFilter => "spatial",
    plugin_filters: std::collections::HashMap<String, serde_json::Value> => "pluginFilters",
}
pub struct Player {
    pub guild_id: String,
    pub paused: Arc<AtomicBool>,
    pub volume: Arc<AtomicU32>,
    pub mixer: Shared<Mixer>,
    pub filter_chain: Shared<FilterChain>,
    pub voice_gateway_cancel: Arc<tokio::sync::Mutex<Option<CancellationToken>>>,
    pub position_tracking_cancel: Arc<tokio::sync::Mutex<Option<CancellationToken>>>,
    pub track_handle: Arc<tokio::sync::Mutex<Option<TrackHandle>>>,
    pub event_sender: Arc<tokio::sync::Mutex<Option<EventSender>>>,
    pub stop_signal: Arc<AtomicBool>,
    pub track_task: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl Player {
    pub fn new(guild_id: String) -> Self {
        Self {
            guild_id,
            paused: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            mixer: Shared::new(tokio::sync::Mutex::new(Mixer::new(48000))),
            filter_chain: Shared::new(tokio::sync::Mutex::new(FilterChain::from_config(
                &Filters::default(),
            ))),
            voice_gateway_cancel: Arc::new(tokio::sync::Mutex::new(None)),
            position_tracking_cancel: Arc::new(tokio::sync::Mutex::new(None)),
            track_handle: Arc::new(tokio::sync::Mutex::new(None)),
            event_sender: Arc::new(tokio::sync::Mutex::new(None)),
            stop_signal: Arc::new(AtomicBool::new(false)),
            track_task: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
    pub async fn play<F>(
        &self,
        user_id: String,
        channel_id: String,
        session_id: String,
        token: String,
        endpoint: String,
        url: String,
        callback: F,
    ) -> Result<(), String>
    where
        F: Fn(&str, serde_json::Value) + Send + Sync + 'static,
    {
        let events = EventSender::new(callback);
        {
            *self.event_sender.lock().await = Some(events.clone());
        }
        self.stop_signal.store(false, Ordering::Release);
        fn get_source_manager_arc() -> Arc<crate::sources::manager::SourceManager> {
            let guard = crate::get_source_manager().lock().unwrap();
            guard.as_ref().unwrap().clone()
        }
        let sm_arc = get_source_manager_arc();
        let player_config = sm_arc.player_config.clone();

        {
            let mut task_guard = self.track_task.lock().await;
            if let Some(task) = task_guard.take() {
                if !player_config.transitions.gapless && !player_config.transitions.crossfade {
                    task.abort();
                }
            }
        }
        {
            let mut cancel_guard = self.position_tracking_cancel.lock().await;
            if let Some(cancel) = cancel_guard.take() {
                cancel.cancel();
            }
        }
        {
            let mut mixer_guard = self.mixer.lock().await;
            if !player_config.transitions.gapless && !player_config.transitions.crossfade {
                mixer_guard.stop_all();
            }
        }
        self.paused.store(false, Ordering::Release);

        let playable_track = match load_and_resolve_first_result(&sm_arc, &url).await {
            Ok(pt) => pt,
            Err(e) => {
                events.send("error", json!({ "message": e }));
                return Ok(());
            }
        };
        let (frame_rx, cmd_tx, err_rx) = playable_track.start_decoding(player_config.clone());
        let (handle, audio_state, vol, pos, is_buffering) =
            TrackHandle::new(cmd_tx, Arc::new(AtomicBool::new(false)));
        {
            let mut mixer_guard = self.mixer.lock().await;
            mixer_guard.add_track(
                frame_rx,
                audio_state,
                vol,
                pos,
                is_buffering,
                player_config.clone(),
            );
        }
        {
            let mut handle_guard = self.track_handle.lock().await;
            *handle_guard = Some(handle.clone());
        }
        events.send("trackStart", json!({}));
        let tracking_token = CancellationToken::new();
        {
            *self.position_tracking_cancel.lock().await = Some(tracking_token.clone());
        }
        let events_err = events.clone();
        let err_cancel = tracking_token.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = err_cancel.cancelled() => {},
                err = err_rx.recv_async() => {
                    if let Ok(msg) = err {
                        events_err.send("error", json!({ "message": msg }));
                    }
                },
            }
        });
        let tracking_token_clone = tracking_token.clone();
        let track_handle_clone = self.track_handle.clone();
        let events_position = events.clone();
        let stop_signal_clone = self.stop_signal.clone();
        let track_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
            let mut ticks = 0;
            while !tracking_token_clone.is_cancelled() {
                interval.tick().await;
                ticks += 1;
                if stop_signal_clone.load(Ordering::Acquire) {
                    break;
                }
                if let Some(handle) = &*track_handle_clone.lock().await {
                    if handle.get_state() == PlayState::Stopped {
                        events_position.send("trackEnd", json!({ "reason": "FINISHED" }));
                        break;
                    }
                    if handle.get_state() == PlayState::Playing && ticks >= 10 {
                        events_position
                            .send("position", json!({ "position": handle.get_position() }));
                        ticks = 0;
                    }
                }
            }
        });
        {
            *self.track_task.lock().await = Some(track_task);
        }
        if self.voice_gateway_cancel.lock().await.is_none() {
            let gateway_token = CancellationToken::new();
            *self.voice_gateway_cancel.lock().await = Some(gateway_token.clone());
            let gateway_config = VoiceGatewayConfig {
                guild_id: GuildId(self.guild_id.clone()),
                user_id: UserId(user_id.parse().unwrap_or(0)),
                channel_id: ChannelId(channel_id.parse().unwrap_or(0)),
                session_id: SessionId(session_id.clone()),
                token: token.clone(),
                endpoint: endpoint.clone(),
                mixer: self.mixer.clone(),
                filter_chain: self.filter_chain.clone(),
                ping: Arc::new(std::sync::atomic::AtomicI64::new(-1)),
                event_tx: None,
                frames_sent: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                frames_nulled: Arc::new(std::sync::atomic::AtomicU64::new(0)),
                cancel_token: gateway_token.clone(),
            };
            let voice_gateway = VoiceGateway::new(gateway_config);
            let events_gw = events.clone();
            tokio::spawn(async move {
                if let Err(e) = voice_gateway.run().await {
                    events_gw.send(
                        "error",
                        json!({ "message": format!("VoiceGateway error: {e}") }),
                    );
                }
            });
        }
        Ok(())
    }
    pub async fn pause(&self) {
        self.paused.store(true, Ordering::Release);
        if let Some(handle) = &*self.track_handle.lock().await {
            handle.pause();
        }
        let guard = self.event_sender.lock().await;
        if let Some(ref e) = *guard {
            e.send("paused", json!({}));
        }
    }
    pub async fn resume(&self) {
        self.paused.store(false, Ordering::Release);
        if let Some(handle) = &*self.track_handle.lock().await {
            handle.play();
        }
        let guard = self.event_sender.lock().await;
        if let Some(ref e) = *guard {
            e.send("resumed", json!({}));
        }
    }
    pub async fn stop(&self) {
        self.stop_signal.store(true, Ordering::Release);
        {
            let mut task_guard = self.track_task.lock().await;
            if let Some(task) = task_guard.take() {
                task.abort();
            }
        }
        {
            let mut cancel_guard = self.voice_gateway_cancel.lock().await;
            if let Some(cancel) = cancel_guard.take() {
                cancel.cancel();
            }
        }
        {
            let mut cancel_guard = self.position_tracking_cancel.lock().await;
            if let Some(cancel) = cancel_guard.take() {
                cancel.cancel();
            }
        }
        if let Some(handle) = &*self.track_handle.lock().await {
            handle.stop();
        }
        let mut mixer_guard = self.mixer.lock().await;
        mixer_guard.stop_all();
    }
    pub async fn seek(&self, position_ms: i64) {
        if let Some(handle) = &*self.track_handle.lock().await {
            handle.seek(position_ms.max(0) as u64);
        }
    }
    pub async fn set_volume(&self, volume: f64) {
        let vol_f = volume as f32;
        self.volume.store(vol_f.to_bits(), Ordering::Relaxed);
        if let Some(handle) = &*self.track_handle.lock().await {
            handle.set_volume(vol_f);
        }
        let guard = self.event_sender.lock().await;
        if let Some(ref e) = *guard {
            e.send("volume", json!({ "volume": volume }));
        }
    }
    pub fn get_position(&self) -> i64 {
        let handle_guard = futures::executor::block_on(self.track_handle.lock());
        if let Some(handle) = &*handle_guard {
            handle.get_position() as i64
        } else {
            0
        }
    }
    pub fn is_paused(&self) -> bool {
        let handle_guard = futures::executor::block_on(self.track_handle.lock());
        if let Some(handle) = &*handle_guard {
            matches!(handle.get_state(), PlayState::Paused)
        } else {
            self.paused.load(Ordering::Acquire)
        }
    }
    pub async fn set_filters(&self, filters_json: String) -> Result<(), String> {
        let filters: Filters = serde_json::from_str(&filters_json)
            .map_err(|e| format!("Invalid filters JSON: {e}"))?;
        let new_chain = FilterChain::from_config(&filters);
        {
            let mut filter_chain_guard = self.filter_chain.lock().await;
            *filter_chain_guard = new_chain;
        }
        Ok(())
    }
}
async fn load_and_resolve_first_result(
    sm: &crate::sources::manager::SourceManager,
    url: &str,
) -> Result<crate::sources::playable_track::BoxedTrack, String> {
    match sm.load(url, None).await {
        crate::protocol::tracks::LoadResult::Track(track) => {
            sm.resolve_track(&track.info, None).await
        }
        crate::protocol::tracks::LoadResult::Search(tracks) => {
            if tracks.is_empty() {
                return Err("Search returned no results".to_string());
            }
            sm.resolve_track(&tracks[0].info, None).await
        }
        crate::protocol::tracks::LoadResult::Playlist(playlist) => {
            if playlist.tracks.is_empty() {
                return Err("Playlist is empty".to_string());
            }
            sm.resolve_track(&playlist.tracks[0].info, None).await
        }
        _ => Err(format!("Failed to load track or query: {url}")),
    }
}
