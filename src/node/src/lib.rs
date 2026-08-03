pub mod player;

use napi_derive::napi;

#[napi]
pub fn set_config_path(path: Option<String>) {
    lavende::set_config_path(path);
}

#[napi]
pub async fn load(identifier: String) -> napi::Result<String> {
    let result = lavende::load(identifier)
        .await
        .map_err(|e| napi::Error::from_reason(e))?;
    serde_json::to_string(&result).map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub async fn load_lyrics(encoded_track: String, skip_track_source: bool) -> napi::Result<String> {
    lavende::load_lyrics(encoded_track, skip_track_source)
        .await
        .map_err(|e| napi::Error::from_reason(e))
}

#[napi]
pub async fn load_lyrics_by_search(title: String, artist: String) -> napi::Result<String> {
    lavende::load_lyrics_by_search(title, artist)
        .await
        .map_err(|e| napi::Error::from_reason(e))
}
