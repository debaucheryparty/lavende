pub mod utils;
pub mod gateway;
pub mod transport;
pub mod pipeline;
pub mod events;
pub mod player;
pub mod common;
pub mod config;
pub mod protocol;
pub mod routeplanner;
pub mod sources;
pub mod audio;

pub use player::Player;

use std::sync::OnceLock;
use sources::manager::SourceManager;
use config::AppConfig;
use std::fs;
use std::path::Path;

static SOURCE_MANAGER: OnceLock<SourceManager> = OnceLock::new();

pub fn get_source_manager() -> &'static SourceManager {
    SOURCE_MANAGER.get_or_init(|| {
        let mut config = AppConfig::default();
        let path = Path::new("source.json");
        if path.exists() {
            if let Ok(raw) = fs::read_to_string(path) {
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(sources_val) = json_val.get("sources") {
                        if let Ok(sources_cfg) = serde_json::from_value(sources_val.clone()) {
                            config.sources = sources_cfg;
                        }
                    }
                    if let Some(rp_val) = json_val.get("route_planner") {
                        if let Ok(rp_cfg) = serde_json::from_value(rp_val.clone()) {
                            config.route_planner = rp_cfg;
                        }
                    }
                    if let Some(player_val) = json_val.get("player") {
                        if let Ok(player_cfg) = serde_json::from_value(player_val.clone()) {
                            config.player = player_cfg;
                        }
                    }
                }
            }
        } else {
            println!("Warning: source.json not found in current directory. Using default source configuration.");
        }
        let logging_cfg = config.logging.clone().unwrap_or_default();
        common::logger::init(&logging_cfg);
        SourceManager::new(&config)
    })
}

#[napi_derive::napi]
pub async fn resolve_jiosaavn(url_or_token: String) -> napi::Result<String> {
    Ok(url_or_token)
}

#[napi_derive::napi]
pub async fn resolve_youtube(url_or_id: String) -> napi::Result<String> {
    Ok(url_or_id)
}

#[napi_derive::napi]
pub async fn load(identifier: String) -> napi::Result<String> {
    let sm = get_source_manager();
    let res = sm.load(&identifier, None).await;
    serde_json::to_string(&res).map_err(|e| napi::Error::from_reason(e.to_string()))
}
