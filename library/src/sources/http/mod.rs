use crate::{
    protocol::tracks::{LoadResult, Track},
    sources::{SourcePlugin, playable_track::PlayableTrack},
};
use async_trait::async_trait;
use regex::Regex;
use std::sync::{Arc, OnceLock};
use tracing::{debug, warn};

pub mod api;
pub mod reader;
pub mod track;

pub use track::HttpTrack;

fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^(?:https?|icy)://").unwrap())
}

pub struct HttpSource;

impl Default for HttpSource {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpSource {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SourcePlugin for HttpSource {
    fn name(&self) -> &str {
        "http"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        url_regex().is_match(identifier)
    }

    async fn load(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        debug!("Probing HTTP source: {identifier}");
        let identifier = identifier.to_owned();
        let local_addr = routeplanner.as_ref().and_then(|rp| rp.get_address());
        match api::probe_metadata(identifier, local_addr).await {
            Ok(info) => LoadResult::Track(Track::new(info)),
            Err(e) => {
                warn!("Probing failed: {e}");
                LoadResult::Empty {}
            }
        }
    }

    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<Arc<dyn PlayableTrack>> {
        let clean = identifier
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>');
        if self.can_handle(clean) {
            Some(Arc::new(HttpTrack {
                url: clean.to_owned(),
                local_addr: routeplanner.and_then(|rp| rp.get_address()),
                proxy: None,
            }))
        } else {
            None
        }
    }
}
