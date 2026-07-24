use crate::protocol::tracks::TrackInfo;
use regex::Regex;
use std::sync::{Arc, OnceLock};

static PATH_EXTRACTOR: OnceLock<Regex> = OnceLock::new();

pub fn identify_resource(link: &str) -> String {
    let pattern =
        PATH_EXTRACTOR.get_or_init(|| Regex::new(r"/(?:comments|video|s)/([^/?#]+)").unwrap());
    if let Some(hits) = pattern.captures(link) {
        return hits[1].to_owned();
    }
    link.to_owned()
}

pub fn unescape_html(input: &str) -> String {
    let mut result = input.to_owned();
    loop {
        let next = result
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'");
        if next == result {
            break;
        }
        result = next;
    }
    result
}

pub async fn acquire_metadata_packet(
    client: &Arc<reqwest::Client>,
    resource_url: &str,
) -> Option<(TrackInfo, Option<String>)> {
    let mut target_endpoint = resource_url.to_owned();
    if resource_url.contains("/s/") || resource_url.contains("/video/") {
        target_endpoint = super::api::resolve_canonical_link(client, resource_url).await?;
    }
    let metadata_url = if target_endpoint.ends_with('/') {
        format!("{}.json", &target_endpoint[..target_endpoint.len() - 1])
    } else {
        format!("{}.json", target_endpoint)
    };
    let raw_json = super::api::fetch_payload(client, &metadata_url).await?;
    let post_listing = match raw_json.as_array().and_then(|a| a.first()) {
        Some(l) => l,
        None => return None,
    };
    let post_data = match post_listing["data"]["children"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|c| c["data"].as_object())
    {
        Some(d) => d,
        None => return None,
    };
    let entry_name = match post_data.get("title").and_then(|v| v.as_str()) {
        Some(t) => unescape_html(t),
        None => return None,
    };
    let creator = format!(
        "u/{}",
        post_data
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
    );
    let preview_img = post_data
        .get("preview")
        .and_then(|p| p.get("images"))
        .and_then(|i| i.as_array())
        .and_then(|i| i.first())
        .and_then(|i| i.get("source"))
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())
        .or_else(|| post_data.get("thumbnail").and_then(|v| v.as_str()))
        .filter(|s| s.starts_with("http"))
        .map(|s| unescape_html(s));
    let media_spec = match post_data
        .get("secure_media")
        .and_then(|sm| sm.get("reddit_video"))
        .or_else(|| post_data.get("media").and_then(|m| m.get("reddit_video")))
    {
        Some(ms) => ms,
        None => return None,
    };
    let duration = match media_spec.get("duration").and_then(|v| v.as_f64()) {
        Some(d) => (d * 1000.0) as u64,
        None => 0,
    };
    let base_stream = match media_spec.get("fallback_url").and_then(|v| v.as_str()) {
        Some(u) => u.split('?').next().unwrap_or(u),
        None => return None,
    };
    let rid = identify_resource(resource_url);
    let audio_stream = super::api::discover_audio_asset(client, base_stream).await;
    Some((
        TrackInfo {
            identifier: rid,
            is_seekable: true,
            author: creator,
            length: duration,
            is_stream: false,
            position: 0,
            title: entry_name,
            uri: Some(resource_url.to_owned()),
            artwork_url: preview_img,
            isrc: None,
            source_name: "reddit".to_owned(),
        },
        audio_stream,
    ))
}
