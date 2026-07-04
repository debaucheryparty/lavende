use std::sync::Arc;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use serde_json::Value;

#[napi]
pub struct Player {
    inner: Arc<lavende_core::Player>,
}

#[napi]
impl Player {
    #[napi(constructor)]
    pub fn new(guild_id: String) -> Self {
        Self {
            inner: Arc::new(lavende_core::Player::new(guild_id)),
        }
    }

    #[napi]
    pub async fn play(
        &self,
        user_id: String,
        channel_id: String,
        session_id: String,
        token: String,
        endpoint: String,
        url: String,
        #[napi(ts_arg_type = "(err: null | Error, event: string) => void")]
        callback: ThreadsafeFunction<String>,
    ) -> napi::Result<()> {
        let cb = move |_: &str, payload: Value| {
            if let Ok(json_str) = serde_json::to_string(&payload) {
                let _ = callback.call(Ok(json_str), ThreadsafeFunctionCallMode::NonBlocking);
            }
        };

        self.inner.play(user_id, channel_id, session_id, token, endpoint, url, cb)
            .await
            .map_err(|e| napi::Error::from_reason(e))
    }

    #[napi]
    pub async fn pause(&self) {
        self.inner.pause().await;
    }

    #[napi]
    pub async fn resume(&self) {
        self.inner.resume().await;
    }

    #[napi]
    pub async fn stop(&self) {
        self.inner.stop().await;
    }

    #[napi]
    pub async fn seek(&self, position_ms: i64) {
        self.inner.seek(position_ms).await;
    }

    #[napi]
    pub async fn set_volume(&self, volume: f64) {
        self.inner.set_volume(volume).await;
    }

    #[napi]
    pub fn get_position(&self) -> i64 {
        self.inner.get_position()
    }

    #[napi]
    pub fn is_paused(&self) -> bool {
        self.inner.is_paused()
    }

    #[napi]
    pub async fn set_filters(&self, filters_json: String) -> napi::Result<()> {
        self.inner.set_filters(filters_json).await.map_err(|e| napi::Error::from_reason(e))
    }
}
