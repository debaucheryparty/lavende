use crate::{
    common::Severity,
    protocol::tracks::{LoadError, LoadResult, Track},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use std::{path::Path, sync::Arc};
use tracing::{debug, error, warn};

pub mod api;
pub mod reader;
pub mod track;

pub use track::LocalTrack;

pub struct LocalSource;

impl Default for LocalSource {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalSource {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SourcePlugin for LocalSource {
    fn name(&self) -> &str {
        "local"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        let path = identifier.strip_prefix("file://").unwrap_or(identifier);
        Path::new(path).is_file()
    }

    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        let path = identifier
            .strip_prefix("file://")
            .unwrap_or(identifier)
            .to_owned();
        debug!("Local source probing file: {path}");
        let path_clone = path.clone();
        let result = tokio::task::spawn_blocking(move || api::probe_file(&path_clone)).await;
        match result {
            Ok(Ok(info)) => LoadResult::Track(Track::new(info)),
            Ok(Err(e)) => {
                warn!("Local source: failed to probe '{path}': {e}");
                LoadResult::Error(LoadError {
                    message: Some(format!("Failed to load local file: {e}")),
                    severity: Severity::Suspicious,
                    cause: e.to_string(),
                    cause_stack_trace: None,
                })
            }
            Err(e) => {
                error!("Local source: task join error: {e}");
                LoadResult::Error(LoadError {
                    message: Some("Internal error reading local file".to_owned()),
                    severity: Severity::Fault,
                    cause: e.to_string(),
                    cause_stack_trace: None,
                })
            }
        }
    }

    async fn get_track(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let path = identifier
            .strip_prefix("file://")
            .unwrap_or(identifier)
            .to_owned();
        if !Path::new(&path).is_file() {
            return None;
        }
        Some(Arc::new(LocalTrack { path }))
    }
}
