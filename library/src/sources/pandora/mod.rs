use crate::{
    protocol::tracks::{LoadResult, SearchResult},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
use async_trait::async_trait;
use regex::Regex;
use reqwest::header::{ACCEPT, CONTENT_TYPE, USER_AGENT};
use serde_json::Value;
use std::sync::{Arc, OnceLock};
use tracing::{debug, warn};

pub mod extractor;
pub mod token;

const BASE_URL: &str = "https://www.pandora.com";
const BASE_URL_API: &str = "https://www.pandora.com";

fn url_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?pandora\.com/(?:playlist/(?P<id>PL:[\d:]+)|artist/(?:[\w\-]+/)*(?P<id2>(?:TR|AL|AR)[A-Za-z0-9]+))").unwrap()
    })
}

pub struct PandoraSource {
    client: Arc<reqwest::Client>,
    token_tracker: Arc<token::PandoraTokenTracker>,
    search_limit: usize,
}

impl PandoraSource {
    pub fn new(
        config: Option<crate::config::PandoraConfig>,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let (s_limit, csrf_override) = config
            .map(|c| (c.search_limit, c.csrf_token))
            .unwrap_or((10, None));
        let token_tracker = Arc::new(token::PandoraTokenTracker::new(
            client.clone(),
            csrf_override,
        ));
        token_tracker.clone().init();
        Ok(Self {
            client,
            token_tracker,
            search_limit: s_limit,
        })
    }

    pub fn base_request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
    }

    pub async fn api_request(&self, path: &str, body: Value) -> Option<Value> {
        for is_retry in [false, true] {
            let tokens = self.token_tracker.get_tokens().await?;
            let url = format!("{BASE_URL_API}{path}");
            let resp = match self
                .base_request(self.client.post(&url))
                .header(ACCEPT, "application/json, text/plain, */*")
                .header(CONTENT_TYPE, "application/json")
                .header("origin", BASE_URL)
                .header("X-Csrftoken", &tokens.csrf_token_parsed)
                .header("X-Authtoken", &tokens.auth_token)
                .header("Cookie", &tokens.csrf_token_raw)
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("Pandora API request error for {path}: {e}");
                    return None;
                }
            };
            let status = resp.status();
            let body_res: Value = resp.json::<Value>().await.ok()?;
            let error_code = body_res
                .get("errorCode")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1);
            let error_string = body_res
                .get("errorString")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let is_error = !status.is_success()
                || (body_res.get("errorCode").is_some()
                    && !body_res.get("errorCode").unwrap().is_null());
            if is_error {
                if !is_retry
                    && (error_code == 1001 || error_string.contains("could not be validated"))
                {
                    debug!(
                        "Auth token error (code: {error_code}, message: {error_string}), refreshing..."
                    );
                    self.token_tracker.force_refresh().await;
                    continue;
                }
                warn!(
                    "Pandora API error for {path}: status {status}, code {error_code}, message {error_string}"
                );
                return None;
            }
            return Some(body_res);
        }
        None
    }
}

#[async_trait]
impl SourcePlugin for PandoraSource {
    fn name(&self) -> &str {
        "pandora"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        self.search_prefixes()
            .iter()
            .any(|p| identifier.starts_with(p))
            || self
                .rec_prefixes()
                .iter()
                .any(|p| identifier.starts_with(p))
            || url_regex().is_match(identifier)
    }

    fn search_prefixes(&self) -> Vec<&str> {
        vec!["pdsearch:"]
    }

    fn is_mirror(&self) -> bool {
        true
    }

    fn rec_prefixes(&self) -> Vec<&str> {
        vec!["pdrec:"]
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
            let query = identifier.strip_prefix(prefix).unwrap();
            if query.is_empty() {
                return LoadResult::Empty {};
            }
            return self.get_search(query).await;
        }
        if let Some(prefix) = self
            .rec_prefixes()
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            let id = identifier.strip_prefix(prefix).unwrap();
            if id.is_empty() {
                return LoadResult::Empty {};
            }
            return self.get_recommendations(id).await;
        }
        let input = identifier.trim();
        if let Some(caps) = url_regex().captures(input) {
            if let Some(id_match) = caps.name("id").or_else(|| caps.name("id2")) {
                let id = id_match.as_str();
                if id.is_empty() {
                    return LoadResult::Empty {};
                }
                if let Some(tr_id) = id.strip_prefix("TR") {
                    if !tr_id.is_empty() {
                        return self.fetch_track(id).await;
                    }
                }
                if let Some(al_id) = id.strip_prefix("AL") {
                    if !al_id.is_empty() {
                        return self.get_album(id).await;
                    }
                }
                if let Some(ar_id) = id.strip_prefix("AR") {
                    if !ar_id.is_empty() {
                        if input.contains("/artist/all-songs/") {
                            return self.get_artist_all_songs(id).await;
                        }
                        return self.get_artist(id).await;
                    }
                }
                if let Some(pl_id) = id.strip_prefix("PL:") {
                    if !pl_id.is_empty() {
                        return self.get_playlist(id).await;
                    }
                }
            }
        }
        LoadResult::Empty {}
    }

    async fn load_search(
        &self,
        query: &str,
        types: &[String],
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<SearchResult> {
        self.get_autocomplete(query, types).await
    }

    async fn get_track(
        &self,
        _identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        None
    }
}
