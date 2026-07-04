uniffi::setup_scaffolding!();

use lavende::{LavendeEvent, LavendeManager, LavendePlayer};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

#[uniffi::export(callback_interface)]
pub trait ShardSender: Send + Sync {
    fn send_to_shard(&self, guild_id: String, payload_json: String);
}

#[derive(uniffi::Object)]
pub struct SwiftLavendeManager {
    inner: Arc<LavendeManager>,
    #[allow(dead_code)]
    shard_sender: Arc<Box<dyn ShardSender>>,
}

#[uniffi::export]
impl SwiftLavendeManager {
    #[uniffi::constructor]
    pub fn new(client_id: String, shard_sender: Box<dyn ShardSender>) -> Arc<Self> {
        let sender = Arc::new(shard_sender);
        let sender_clone = sender.clone();
        let inner = LavendeManager::new(client_id, move |guild_id, payload| {
            if let Ok(json_str) = serde_json::to_string(&payload) {
                sender_clone.send_to_shard(guild_id, json_str);
            }
        });

        Arc::new(Self {
            inner: Arc::new(inner),
            shard_sender: sender,
        })
    }

    pub fn get_or_create_player(&self, guild_id: String) -> Arc<SwiftLavendePlayer> {
        let player = self.inner.get_or_create_player(&guild_id);
        Arc::new(SwiftLavendePlayer {
            inner: player,
            manager: self.inner.clone(),
        })
    }

    pub async fn destroy_player(&self, guild_id: String) {
        self.inner.destroy_player(&guild_id).await;
    }

    pub async fn send_raw_data(&self, packet_json: String) {
        if let Ok(val) = serde_json::from_str::<Value>(&packet_json) {
            self.inner.send_raw_data(&val).await;
        }
    }

    pub async fn listen_events(&self, listener: Box<dyn LavendeEventListener>) {
        let mut rx = self.inner.subscribe_events();
        let listener = Arc::new(listener);
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                listener.on_event(json);
            }
        });
    }
}

#[uniffi::export(callback_interface)]
pub trait LavendeEventListener: Send + Sync {
    fn on_event(&self, event_json: String);
}

#[derive(uniffi::Object)]
pub struct SwiftLavendePlayer {
    inner: LavendePlayer,
    #[allow(dead_code)]
    manager: Arc<LavendeManager>,
}

#[derive(uniffi::Error, Debug)]
pub enum LavendeError {
    PlayError { message: String },
}

impl std::fmt::Display for LavendeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LavendeError::PlayError { message } => write!(f, "PlayError: {}", message),
        }
    }
}

#[uniffi::export]
impl SwiftLavendePlayer {
    pub async fn connect(&self, channel_id: Option<String>, self_deaf: bool, self_mute: bool) {
        self.inner.connect(channel_id, self_deaf, self_mute).await;
    }

    pub async fn disconnect(&self) {
        self.inner.disconnect().await;
    }

    pub async fn destroy(&self, reason: Option<String>) {
        self.inner.destroy(reason).await;
    }

    pub async fn search(&self, query: String) -> String {
        match self.inner.search(&query).await {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()),
            Err(e) => format!(r#"{{"error": "{}"}}"#, e),
        }
    }

    pub async fn play(&self) -> Result<(), LavendeError> {
        self.inner
            .play()
            .await
            .map_err(|e| LavendeError::PlayError { message: e })
    }

    pub async fn pause(&self, state: bool) {
        self.inner.pause(state).await;
    }

    pub async fn resume(&self) {
        self.inner.resume().await;
    }

    pub async fn stop(&self) {
        self.inner.stop().await;
    }

    pub async fn skip(&self) {
        self.inner.skip().await;
    }

    pub async fn seek(&self, position_ms: i64) {
        self.inner.seek(position_ms).await;
    }

    pub async fn set_volume(&self, volume: u32) {
        self.inner.set_volume(volume).await;
    }

    pub async fn set_filters(&self, filters_json: String) {
        let _ = self.inner.set_filters(filters_json).await;
    }

    pub fn get_position(&self) -> i64 {
        self.inner.get_position()
    }

    pub fn is_paused(&self) -> bool {
        self.inner.is_paused()
    }
}
