use crate::{
    protocol::tracks::{LoadResult, Track},
    sources::{playable_track::BoxedTrack, plugin::SourcePlugin},
};
use async_trait::async_trait;
use std::sync::Arc;

pub mod api;
pub mod extractor;
pub mod track;

pub use track::RedditTrack;

pub struct RedditSource {
    http: Arc<reqwest::Client>,
}

impl RedditSource {
    pub fn new(
        _config: Option<crate::config::sources::RedditConfig>,
        http: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        Ok(Self { http })
    }
}

#[async_trait]
impl SourcePlugin for RedditSource {
    fn name(&self) -> &str {
        "reddit"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        identifier.contains("reddit.com/") || identifier.contains("v.redd.it/")
    }

    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        match extractor::acquire_metadata_packet(&self.http, identifier).await {
            Some((meta, _)) => LoadResult::Track(Track::new(meta)),
            None => LoadResult::Empty {},
        }
    }

    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let (meta, audio_stream) =
            extractor::acquire_metadata_packet(&self.http, identifier).await?;
        Some(Arc::new(RedditTrack {
            client: self.http.clone(),
            uri: meta.uri.unwrap_or_else(|| identifier.to_owned()),
            audio_url: audio_stream,
            local_addr: routeplanner.and_then(|rp| rp.get_address()),
        }))
    }
}
