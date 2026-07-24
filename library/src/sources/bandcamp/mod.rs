use crate::{
    protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track, TrackInfo},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::error;

pub mod api;
pub mod extractor;
pub mod track;

pub struct BandcampSource {
    client: Arc<reqwest::Client>,
    search_limit: usize,
}

impl BandcampSource {
    pub fn new(
        config: Option<crate::config::BandcampConfig>,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        Ok(Self {
            client,
            search_limit: config.map(|c| c.search_limit).unwrap_or(10),
        })
    }

    async fn search(&self, query: &str) -> LoadResult {
        let url = format!(
            "https://bandcamp.com/search?q={}&item_type=t&from=results",
            urlencoding::encode(query)
        );
        let resp = match api::base_request(self.client.get(url)).send().await {
            Ok(r) => r,
            Err(e) => {
                error!("Bandcamp search request failed: {e}");
                return LoadResult::Empty {};
            }
        };
        if !resp.status().is_success() {
            return LoadResult::Empty {};
        }
        let body = match resp.text().await {
            Ok(t) => t,
            Err(_) => return LoadResult::Empty {},
        };

        let result_blocks_re = extractor::result_blocks_pattern();
        let url_re = extractor::art_url_pattern();
        let title_re = extractor::title_pattern();
        let subhead_re = extractor::subhead_pattern();
        let artwork_re = extractor::artwork_pattern();

        let mut tracks = Vec::new();
        for block in result_blocks_re.find_iter(&body) {
            let block_str = block.as_str();
            if let (Some(url_m), Some(title_m), Some(subhead_m)) = (
                url_re.captures(block_str),
                title_re.captures(block_str),
                subhead_re.captures(block_str),
            ) {
                let uri = url_m[1].split('?').next().unwrap_or(&url_m[1]).to_owned();
                let title = title_m[1].trim().to_owned();
                let subhead = subhead_m[1].trim();
                let artist = subhead
                    .split(" de ")
                    .last()
                    .unwrap_or(subhead)
                    .trim()
                    .to_owned();
                let artwork_url = artwork_re.captures(block_str).map(|m| m[1].to_owned());
                tracks.push(Track::new(TrackInfo {
                    identifier: extractor::get_identifier_from_url(&uri),
                    is_seekable: true,
                    author: artist,
                    length: 0,
                    is_stream: false,
                    position: 0,
                    title,
                    uri: Some(uri),
                    artwork_url,
                    isrc: None,
                    source_name: "bandcamp".to_owned(),
                }));
                if tracks.len() >= self.search_limit {
                    break;
                }
            }
        }
        if tracks.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Search(tracks)
        }
    }

    async fn resolve(&self, url: &str) -> LoadResult {
        let (tralbum_data, _) = match api::fetch_track_data(&self.client, url).await {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let artist = tralbum_data["artist"]
            .as_str()
            .unwrap_or("Unknown Artist")
            .to_owned();
        let artwork_url = tralbum_data["art_id"]
            .as_u64()
            .map(|id| format!("https://f4.bcbits.com/img/a{id}_10.jpg"));
        if let Some(trackinfo) = tralbum_data["trackinfo"].as_array() {
            if trackinfo.len() > 1 {
                let mut tracks = Vec::new();
                for item in trackinfo {
                    let title = match item["title"].as_str() {
                        Some(t) => t.to_owned(),
                        None => continue,
                    };
                    if let Some(suffix) = item["title_link"].as_str() {
                        let track_url = if suffix.starts_with("http") {
                            suffix.to_owned()
                        } else {
                            let base = url.split(".bandcamp.com").next().unwrap_or("");
                            format!("{base}.bandcamp.com{suffix}")
                        };
                        let duration = (item["duration"].as_f64().unwrap_or(0.0) * 1000.0) as u64;
                        let identifier = item["track_id"]
                            .as_u64()
                            .or(item["id"].as_u64())
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| extractor::get_identifier_from_url(&track_url));
                        tracks.push(Track::new(TrackInfo {
                            identifier,
                            is_seekable: true,
                            author: artist.clone(),
                            length: duration,
                            is_stream: false,
                            position: 0,
                            title,
                            uri: Some(track_url),
                            artwork_url: artwork_url.clone(),
                            isrc: None,
                            source_name: "bandcamp".to_owned(),
                        }));
                    }
                }
                let playlist_name = tralbum_data["current"]["title"]
                    .as_str()
                    .unwrap_or("Bandcamp Album")
                    .to_owned();
                return LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name: playlist_name,
                        selected_track: -1,
                    },
                    plugin_info: serde_json::json!({}),
                    tracks,
                });
            } else if let Some(track_data) = trackinfo.first() {
                let title = track_data["title"]
                    .as_str()
                    .unwrap_or("Unknown Title")
                    .to_owned();
                let duration = (track_data["duration"].as_f64().unwrap_or(0.0) * 1000.0) as u64;
                let identifier = track_data["track_id"]
                    .as_u64()
                    .or(track_data["id"].as_u64())
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| extractor::get_identifier_from_url(url));
                return LoadResult::Track(Track::new(TrackInfo {
                    identifier,
                    is_seekable: true,
                    author: artist,
                    length: duration,
                    is_stream: false,
                    position: 0,
                    title,
                    uri: Some(url.to_owned()),
                    artwork_url,
                    isrc: None,
                    source_name: "bandcamp".to_owned(),
                }));
            }
        }
        LoadResult::Empty {}
    }
}

#[async_trait]
impl SourcePlugin for BandcampSource {
    fn name(&self) -> &str {
        "bandcamp"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        let url_re = extractor::url_pattern();
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || url_re.is_match(identifier)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["bcsearch:"]
    }

    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        if let Some(prefix) = self
            .search_prefixes()
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            return self.search(&identifier[prefix.len()..]).await;
        }
        let url_re = extractor::url_pattern();
        if url_re.is_match(identifier) {
            return self.resolve(identifier).await;
        }
        LoadResult::Empty {}
    }

    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let id_re = extractor::identifier_pattern();
        let url = if identifier.starts_with("http") {
            identifier.to_owned()
        } else if let Some(caps) = id_re.captures(identifier) {
            format!(
                "https://{}.bandcamp.com/track/{}",
                &caps["subdomain"], &caps["slug"]
            )
        } else {
            return None;
        };
        let (_, stream_url_opt) = api::fetch_track_data(&self.client, &url).await?;
        let stream_url = stream_url_opt?;
        Some(Arc::new(track::BandcampTrack {
            client: self.client.clone(),
            uri: url,
            stream_url: Some(stream_url),
            local_addr: routeplanner.and_then(|rp| rp.get_address()),
        }))
    }
}
