use crate::{
    common::types::AnyResult,
    config::AppConfig,
    protocol::tracks::LoadResult,
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

pub mod extractor;
pub mod token;
pub mod track;

const API_URL: &str = "https://www.qobuz.com/api.json/0.2/";

fn url_regex() -> &'static regex::Regex {
    static REGEX: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| {
        regex::Regex::new(
            r"https?://(?:www\.|play\.|open\.)?qobuz\.com/(?:(?:[a-z]{2}-[a-z]{2}/)?(?P<type>album|playlist|track|artist)/(?:.+?/)?(?P<id>[a-zA-Z0-9]+)|(?P<type2>playlist)/(?P<id2>\d+))"
        ).unwrap()
    })
}

pub struct QobuzSource {
    client: Arc<reqwest::Client>,
    token_tracker: Arc<token::QobuzTokenTracker>,
    search_limit: usize,
    album_load_limit: usize,
    playlist_load_limit: usize,
    artist_load_limit: usize,
}

impl QobuzSource {
    pub fn new(config: &AppConfig, client: Arc<reqwest::Client>) -> Result<Self, String> {
        let qobuz_config = config.sources.qobuz.clone().unwrap_or_default();
        let tracker = Arc::new(token::QobuzTokenTracker::new(
            client.clone(),
            qobuz_config.user_token,
            qobuz_config.app_id,
            qobuz_config.app_secret,
        ));
        tracker.clone().init();
        Ok(Self {
            client,
            token_tracker: tracker,
            search_limit: qobuz_config.search_limit,
            album_load_limit: qobuz_config.album_load_limit,
            playlist_load_limit: qobuz_config.playlist_load_limit,
            artist_load_limit: qobuz_config.artist_load_limit,
        })
    }

    pub fn base_request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36")
    }

    pub async fn api_request(&self, path: &str, params: Vec<(&str, String)>) -> AnyResult<Value> {
        let tokens = self
            .token_tracker
            .get_tokens()
            .await
            .ok_or("Failed to get Qobuz tokens")?;
        let mut url = reqwest::Url::parse(&format!("{API_URL}{path}"))?;
        {
            let mut query = url.query_pairs_mut();
            for (k, v) in params {
                query.append_pair(k, &v);
            }
        }
        let mut request = self
            .base_request(self.client.get(url))
            .header("Accept", "application/json")
            .header("x-app-id", &tokens.app_id);
        if let Some(user_token) = &tokens.user_token {
            request = request.header("x-user-auth-token", user_token);
        }
        let resp = request.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Qobuz API error ({status}): {body}").into());
        }
        let json: Value = resp.json().await?;
        Ok(json)
    }
}

#[async_trait]
impl SourcePlugin for QobuzSource {
    fn name(&self) -> &str {
        "qobuz"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || self
                .isrc_prefixes()
                .iter()
                .any(|p| identifier.starts_with(p))
            || self
                .rec_prefixes()
                .iter()
                .any(|p| identifier.starts_with(p))
            || url_regex().is_match(identifier)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["qbsearch:"]
    }

    fn isrc_prefixes(&self) -> Vec<&str> {
        vec!["qbisrc:"]
    }

    fn rec_prefixes(&self) -> Vec<&str> {
        vec!["qbrec:"]
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
            return self.handle_search(&identifier[prefix.len()..]).await;
        }
        if let Some(prefix) = self
            .isrc_prefixes()
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            return self.handle_isrc(&identifier[prefix.len()..]).await;
        }
        if let Some(prefix) = self
            .rec_prefixes()
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            return self
                .handle_recommendations(&identifier[prefix.len()..])
                .await;
        }
        if let Some(caps) = url_regex().captures(identifier) {
            let type_ = caps
                .name("type")
                .or_else(|| caps.name("type2"))
                .map(|m| m.as_str())
                .unwrap_or("");
            let id = caps
                .name("id")
                .or_else(|| caps.name("id2"))
                .map(|m| m.as_str())
                .unwrap_or("");
            return match type_ {
                "track" => {
                    match self
                        .api_request("track/get", vec![("track_id", id.to_owned())])
                        .await
                    {
                        Ok(json) => LoadResult::Track(crate::protocol::tracks::Track::new(
                            self.parse_qobuz_track(&json).info,
                        )),
                        Err(_) => LoadResult::Empty {},
                    }
                }
                "album" => self.handle_album(id).await,
                "playlist" => self.handle_playlist(id).await,
                "artist" => self.handle_artist(id).await,
                _ => LoadResult::Empty {},
            };
        }
        LoadResult::Empty {}
    }

    async fn get_track(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let id = if identifier.contains("qobuz.com/track/") {
            identifier
                .split("track/")
                .nth(1)?
                .split('/')
                .next()?
                .split('?')
                .next()?
        } else {
            identifier
        };
        let tokens = self.token_tracker.get_tokens().await?;
        if tokens.user_token.is_none() {
            debug!("Qobuz: No user token, returning None to trigger mirroring");
            return None;
        }
        match self
            .api_request("track/get", vec![("track_id", id.to_owned())])
            .await
        {
            Ok(json) => Some(Arc::new(self.parse_qobuz_track(&json))),
            Err(_) => None,
        }
    }
}
