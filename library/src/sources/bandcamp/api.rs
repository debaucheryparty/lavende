use serde_json::Value;
use std::sync::Arc;

pub fn base_request(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
}

pub async fn fetch_stream_url(client: &Arc<reqwest::Client>, uri: &str) -> Option<String> {
    let resp = base_request(client.get(uri)).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body = resp.text().await.ok()?;
    super::extractor::extract_stream_url(&body)
}

pub async fn fetch_track_data(
    client: &Arc<reqwest::Client>,
    url: &str,
) -> Option<(Value, Option<String>)> {
    let resp = base_request(client.get(url)).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body = resp.text().await.ok()?;
    let tralbum_data = if let Some(match_cap) = super::extractor::tralbum_pattern().captures(&body)
    {
        let decoded = match_cap[1].replace("&quot;", "\"");
        serde_json::from_str(&decoded).ok()?
    } else {
        return None;
    };
    let stream_url = super::extractor::extract_stream_url(&body);
    Some((tralbum_data, stream_url))
}
