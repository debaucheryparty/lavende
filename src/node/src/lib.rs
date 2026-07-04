pub mod player;

use lavende_core;
use napi_derive::napi;

#[napi]
pub async fn resolve_jiosaavn(url_or_token: String) -> napi::Result<String> {
    lavende_core::resolve_jiosaavn(url_or_token)
        .await
        .map_err(|e| napi::Error::from_reason(e))
}

#[napi]
pub async fn resolve_youtube(url_or_id: String) -> napi::Result<String> {
    lavende_core::resolve_youtube(url_or_id)
        .await
        .map_err(|e| napi::Error::from_reason(e))
}

#[napi]
pub async fn load(identifier: String) -> napi::Result<String> {
    lavende_core::load(identifier)
        .await
        .map_err(|e| napi::Error::from_reason(e))
}
