use crate::sources::{
    http::HttpTrack,
    playable_track::{PlayableTrack, ResolvedTrack},
};
use async_trait::async_trait;
use rand::{Rng, distributions::Alphanumeric, thread_rng};
use std::{collections::BTreeMap, sync::Arc};
use tracing::debug;

pub struct AudiomackTrack {
    pub stream_url: String,
    pub local_addr: Option<std::net::IpAddr>,
}

#[async_trait]
impl PlayableTrack for AudiomackTrack {
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let url = self.stream_url.clone();
        debug!("Audiomack playback URL: {url}");
        HttpTrack {
            url,
            local_addr: self.local_addr,
            proxy: None,
        }
        .resolve()
        .await
    }
}

pub async fn fetch_stream_url(client: &Arc<reqwest::Client>, identifier: &str) -> Option<String> {
    let nonce = generate_nonce();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    let post_url = format!("https://api.audiomack.com/v1/music/{identifier}/play");
    let mut body = BTreeMap::new();
    body.insert("environment".to_owned(), "desktop-web".to_owned());
    body.insert("session".to_owned(), "backend-session".to_owned());
    body.insert("hq".to_owned(), "true".to_owned());
    let auth_post = super::utils::build_auth_header("POST", &post_url, &body, &nonce, &timestamp);

    if let Ok(resp) = client
        .post(&post_url)
        .header("Authorization", auth_post)
        .form(&body)
        .send()
        .await
    {
        if let Some(url) = parse_response(resp).await {
            return Some(url);
        }
    }

    let get_url = format!("https://api.audiomack.com/v1/music/play/{identifier}");
    let mut query = BTreeMap::new();
    query.insert("environment".to_owned(), "desktop-web".to_owned());
    query.insert("hq".to_owned(), "true".to_owned());
    let auth_get = super::utils::build_auth_header("GET", &get_url, &query, &nonce, &timestamp);

    if let Ok(resp) = client
        .get(&get_url)
        .header("Authorization", auth_get)
        .query(&query)
        .send()
        .await
    {
        if let Some(url) = parse_response(resp).await {
            return Some(url);
        }
    }
    None
}

async fn parse_response(resp: reqwest::Response) -> Option<String> {
    if !resp.status().is_success() {
        return None;
    }
    let text = resp.text().await.ok()?;
    let is_stream = |url: &str| {
        url.contains("music.audiomack.com")
            || url.ends_with(".m4a")
            || url.ends_with(".mp3")
            || url.contains(".m4a?")
            || url.contains(".mp3?")
    };
    if text.starts_with("http") && is_stream(&text) {
        return Some(text);
    }
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    if let Some(s) = json.as_str() {
        if is_stream(s) {
            return Some(s.to_owned());
        }
    }
    let results = json.get("results").unwrap_or(&json);
    let potential_url = results
        .get("signedUrl")
        .or_else(|| results.get("signed_url"))
        .or_else(|| results.get("streamUrl"))
        .or_else(|| results.get("stream_url"))
        .or_else(|| results.get("url"))
        .and_then(|v| v.as_str());
    if let Some(url) = potential_url {
        if is_stream(url) {
            return Some(url.to_owned());
        }
    }
    None
}

fn generate_nonce() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}
