pub mod audio;
pub mod common;
pub mod config;
pub mod events;
pub mod gateway;
pub mod lyrics;
pub mod pipeline;
pub mod player;
pub mod protocol;
pub mod routeplanner;
pub mod sources;
pub mod transport;
pub mod utils;

pub use player::Player;

use config::AppConfig;
use sources::manager::SourceManager;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

static SOURCE_MANAGER: OnceLock<Mutex<Option<Arc<SourceManager>>>> = OnceLock::new();
static CONFIG_PATH: Mutex<Option<String>> = Mutex::new(None);

pub fn set_config_path(path: Option<String>) {
    if let Ok(mut config_path) = CONFIG_PATH.lock() {
        *config_path = path;
    }
}

pub fn get_source_manager() -> &'static Mutex<Option<Arc<SourceManager>>> {
    SOURCE_MANAGER.get_or_init(|| {
        let config_file = CONFIG_PATH
            .lock()
            .ok()
            .and_then(|p| p.clone())
            .unwrap_or_else(|| "source.json".to_string());

        let mut config = AppConfig::default();
        let path = Path::new(&config_file);

        if path.exists()
            && let Ok(raw) = fs::read_to_string(path)
            && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&raw)
        {
            if let Some(sources_val) = json_val.get("sources")
                && let Ok(sources_cfg) = serde_json::from_value(sources_val.clone())
            {
                config.sources = sources_cfg;
            }
            if let Some(rp_val) = json_val.get("route_planner")
                && let Ok(rp_cfg) = serde_json::from_value(rp_val.clone())
            {
                config.route_planner = rp_cfg;
            }
            if let Some(player_val) = json_val.get("player")
                && let Ok(player_cfg) = serde_json::from_value(player_val.clone())
            {
                config.player = player_cfg;
            }
        }

        let logging_cfg = config.logging.clone().unwrap_or_default();
        common::logger::init(&logging_cfg);
        Mutex::new(Some(Arc::new(SourceManager::new(&config))))
    })
}

static LYRICS_MANAGER: OnceLock<Mutex<Option<Arc<crate::lyrics::LyricsManager>>>> = OnceLock::new();

pub fn get_lyrics_manager() -> &'static Mutex<Option<Arc<crate::lyrics::LyricsManager>>> {
    LYRICS_MANAGER.get_or_init(|| {
        let config_file = CONFIG_PATH
            .lock()
            .ok()
            .and_then(|p| p.clone())
            .unwrap_or_else(|| "source.json".to_string());

        let mut config = AppConfig::default();
        let path = Path::new(&config_file);

        if path.exists()
            && let Ok(raw) = fs::read_to_string(path)
            && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&raw)
        {
            if let Some(lyrics_val) = json_val.get("lyrics")
                && let Ok(lyrics_cfg) = serde_json::from_value(lyrics_val.clone())
            {
                config.lyrics = lyrics_cfg;
            }
            if let Some(sources_val) = json_val.get("sources")
                && let Ok(sources_cfg) = serde_json::from_value(sources_val.clone())
            {
                config.sources = sources_cfg;
            }
        }
        Mutex::new(Some(Arc::new(crate::lyrics::LyricsManager::new(&config))))
    })
}

pub async fn resolve_jiosaavn(url_or_token: String) -> Result<String, String> {
    Ok(url_or_token)
}

pub async fn resolve_youtube(url_or_id: String) -> Result<String, String> {
    Ok(url_or_id)
}

pub async fn load(identifier: String) -> Result<String, String> {
    let sm_arc = {
        let sm_guard = get_source_manager().lock().map_err(|e| e.to_string())?;
        sm_guard
            .as_ref()
            .ok_or("SourceManager not initialized")?
            .clone()
    };
    let res = sm_arc.load(&identifier, None).await;
    serde_json::to_string(&res).map_err(|e| e.to_string())
}

pub async fn load_lyrics(encoded_track: String, skip_track_source: bool) -> Result<String, String> {
    let track = match protocol::decode::decode_track(&encoded_track) {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to decode track: {e}")),
    };
    let lm = {
        let lm_guard = get_lyrics_manager().lock().map_err(|e| e.to_string())?;
        lm_guard
            .as_ref()
            .ok_or("LyricsManager not initialized")?
            .clone()
    };
    match lm.load_lyrics_ext(&track.info, skip_track_source).await {
        Some(lyrics) => serde_json::to_string(&lyrics).map_err(|e| e.to_string()),
        None => Err("No lyrics found".to_string()),
    }
}

pub async fn load_lyrics_by_search(title: String, artist: String) -> Result<String, String> {
    let track_info = protocol::tracks::TrackInfo {
        title,
        author: artist,
        identifier: "".to_string(),
        is_seekable: false,
        length: 0,
        is_stream: false,
        position: 0,
        uri: None,
        artwork_url: None,
        isrc: None,
        source_name: "".to_string(),
    };
    let lm = {
        let lm_guard = get_lyrics_manager().lock().map_err(|e| e.to_string())?;
        lm_guard
            .as_ref()
            .ok_or("LyricsManager not initialized")?
            .clone()
    };
    match lm.load_lyrics_ext(&track_info, false).await {
        Some(lyrics) => serde_json::to_string(&lyrics).map_err(|e| e.to_string()),
        None => Err("No lyrics found".to_string()),
    }
}
