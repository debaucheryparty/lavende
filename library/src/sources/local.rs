pub mod track {
    use crate::{
        common::AudioFormat,
        sources::playable_track::{PlayableTrack, ResolvedTrack},
    };
    use async_trait::async_trait;
    use std::{
        io::{Read, Seek, SeekFrom},
        path::Path,
    };
    use tracing::error;
    pub struct LocalTrack {
        pub path: String,
    }
    struct LocalFileSource {
        file: std::fs::File,
        len: u64,
    }
    impl LocalFileSource {
        fn open(path: &str) -> std::io::Result<Self> {
            let file = std::fs::File::open(path)?;
            let len = file.metadata()?.len();
            Ok(Self { file, len })
        }
    }
    impl Read for LocalFileSource {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.file.read(buf)
        }
    }
    impl Seek for LocalFileSource {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            self.file.seek(pos)
        }
    }
    impl symphonia::core::io::MediaSource for LocalFileSource {
        fn is_seekable(&self) -> bool {
            true
        }
        fn byte_len(&self) -> Option<u64> {
            Some(self.len)
        }
    }
    #[async_trait]
    impl PlayableTrack for LocalTrack {
        fn supports_seek(&self) -> bool {
            true
        }
        async fn resolve(&self) -> Result<ResolvedTrack, String> {
            let path = self.path.clone();
            let hint = Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .map(AudioFormat::from_ext);
            let reader = tokio::task::spawn_blocking(move || {
                LocalFileSource::open(&path)
                    .map(|s| Box::new(s) as Box<dyn symphonia::core::io::MediaSource>)
                    .map_err(|e| {
                        error!("LocalTrack: failed to open '{path}': {e}");
                        format!("Failed to open file: {e}")
                    })
            })
            .await
            .map_err(|e| format!("spawn_blocking failed: {e}"))??;
            Ok(ResolvedTrack::new(reader, hint))
        }
    }
}
use crate::{
    common::Severity,
    protocol::tracks::{LoadError, LoadResult, Track, TrackInfo},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use std::{path::Path, sync::Arc};
use symphonia::core::{
    codecs::CODEC_TYPE_NULL,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, StandardTagKey},
    probe::Hint,
};
use tracing::{debug, error, warn};
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
    fn probe_file(path: &str) -> Result<TrackInfo, Box<dyn std::error::Error + Send + Sync>> {
        let file = std::fs::File::open(path)?;
        let path_obj = Path::new(path);
        let ext = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        let mut hint = Hint::new();
        if let Some(ref e) = ext {
            hint.with_extension(e);
        }
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;
        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("no audio track found")?;
        let duration = track
            .codec_params
            .n_frames
            .and_then(|n| {
                track
                    .codec_params
                    .sample_rate
                    .map(|r| (n as f64 / r as f64 * 1000.0) as u64)
            })
            .unwrap_or(0);
        let mut title = String::new();
        let mut author = String::new();
        if let Some(meta) = format.metadata().current() {
            for tag in meta.tags() {
                match tag.std_key {
                    Some(StandardTagKey::TrackTitle) => title = tag.value.to_string(),
                    Some(StandardTagKey::Artist) | Some(StandardTagKey::AlbumArtist)
                        if author.is_empty() =>
                    {
                        author = tag.value.to_string();
                    }
                    _ => {}
                }
            }
        }
        if title.is_empty() {
            title = path_obj
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_owned();
        }
        if author.is_empty() {
            author = "Unknown Artist".to_owned();
        }
        Ok(TrackInfo {
            identifier: path.to_owned(),
            is_seekable: true,
            author,
            length: duration,
            is_stream: false,
            position: 0,
            title,
            uri: Some(format!("file://{path}")),
            source_name: "local".to_owned(),
            artwork_url: None,
            isrc: None,
        })
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
        let result =
            tokio::task::spawn_blocking(move || LocalSource::probe_file(&path_clone)).await;
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
