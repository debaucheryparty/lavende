use crate::{
    common::types::AudioFormat,
    config::HttpProxyConfig,
    sources::playable_track::{PlayableTrack, ResolvedTrack},
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct GaanaTrack {
    pub client: Arc<reqwest::Client>,
    pub track_id: String,
    pub stream_quality: String,
    pub local_addr: Option<std::net::IpAddr>,
    pub proxy: Option<HttpProxyConfig>,
}

#[async_trait]
impl PlayableTrack for GaanaTrack {
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let url = super::api::fetch_stream_url_internal(
            &self.client,
            &self.track_id,
            &self.stream_quality,
        )
        .await
        .ok_or_else(|| {
            format!(
                "GaanaTrack: Failed to fetch stream URL for {}",
                self.track_id
            )
        })?;
        let local_addr = self.local_addr;
        let proxy = self.proxy.clone();
        let is_hls = url.contains(".m3u8") || url.contains("/api/manifest/hls_");
        if is_hls {
            crate::sources::youtube::hls::HlsReader::new(&url, local_addr, None, None, proxy)
                .await
                .map(|r| {
                    ResolvedTrack::new(
                        Box::new(r) as Box<dyn symphonia::core::io::MediaSource>,
                        Some(AudioFormat::Aac),
                    )
                })
                .map_err(|e| format!("Failed to init HLS reader: {e}"))
        } else {
            let hint = std::path::Path::new(&url)
                .extension()
                .and_then(|s| s.to_str())
                .map(AudioFormat::from_ext);
            super::reader::GaanaReader::new(&url, local_addr, proxy)
                .await
                .map(|r| {
                    ResolvedTrack::new(
                        Box::new(r) as Box<dyn symphonia::core::io::MediaSource>,
                        hint,
                    )
                })
                .map_err(|e| format!("Failed to init reader: {e}"))
        }
    }
}
