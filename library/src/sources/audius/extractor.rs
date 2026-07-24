use crate::protocol::tracks::{Track, TrackInfo};
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

pub fn track_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^https?://(?:www\.)?audius\.co/(?P<artist>[^/]+)/(?P<slug>[^/?#]+)(?:\?.*)?$",
        )
        .unwrap()
    })
}

pub fn playlist_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^https?://(?:www\.)?audius\.co/(?P<artist>[^/]+)/playlist/(?P<slug>[^/?#]+)(?:\?.*)?$").unwrap())
}

pub fn album_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^https?://(?:www\.)?audius\.co/(?P<artist>[^/]+)/album/(?P<slug>[^/?#]+)(?:\?.*)?$").unwrap())
}

pub fn user_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^https?://(?:www\.)?audius\.co/(?P<user>[^/?#]+)(?:\?.*)?$").unwrap()
    })
}

pub fn parse_tracks(data: &Value) -> Vec<Track> {
    data.as_array()
        .map(|arr| arr.iter().filter_map(|item| build_track(item)).collect())
        .unwrap_or_default()
}

pub fn build_track(data: &Value) -> Option<Track> {
    let id = data["id"].as_str()?;
    let title = data["title"].as_str()?.to_owned();
    let author = data["user"]["name"]
        .as_str()
        .unwrap_or("Unknown Artist")
        .to_owned();
    let duration = (data["duration"].as_f64().unwrap_or(0.0) * 1000.0) as u64;
    let uri = data["permalink"].as_str().map(|p| {
        if p.starts_with("http") {
            p.to_owned()
        } else {
            format!("https://audius.co{p}")
        }
    });
    let artwork_url = get_artwork_url(&data["artwork"]);
    Some(Track::new(TrackInfo {
        identifier: id.to_owned(),
        is_seekable: true,
        author,
        length: duration,
        is_stream: false,
        position: 0,
        title,
        uri,
        artwork_url,
        isrc: None,
        source_name: "audius".to_owned(),
    }))
}

pub fn get_artwork_url(artwork: &Value) -> Option<String> {
    if artwork.is_null() {
        return None;
    }
    if let Some(url) = artwork.as_str() {
        return Some(if url.starts_with('/') {
            format!("https://audius.co{url}")
        } else {
            url.to_owned()
        });
    }
    for size in &["480x480", "1000x1000", "150x150"] {
        if let Some(url) = artwork[size].as_str() {
            return Some(if url.starts_with('/') {
                format!("https://audius.co{url}")
            } else {
                url.to_owned()
            });
        }
    }
    None
}
