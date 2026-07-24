use crate::sources::{
    http::HttpTrack,
    playable_track::{PlayableTrack, ResolvedTrack},
};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

pub struct BandcampTrack {
    pub client: Arc<reqwest::Client>,
    pub uri: String,
    pub stream_url: Option<String>,
    pub local_addr: Option<std::net::IpAddr>,
}

#[async_trait]
impl PlayableTrack for BandcampTrack {
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let url = if let Some(url) = self.stream_url.clone() {
            url
        } else {
            super::api::fetch_stream_url(&self.client, &self.uri)
                .await
                .ok_or_else(|| format!("Failed to fetch Bandcamp stream URL for {}", self.uri))?
        };
        debug!("Bandcamp stream URL: {url}");
        HttpTrack {
            url,
            local_addr: self.local_addr,
            proxy: None,
        }
        .resolve()
        .await
    }
}
