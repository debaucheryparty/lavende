use crate::protocol::tracks::{Track, TrackInfo};
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

pub fn song_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?audiomack\.com/(?P<artist>[^/]+)/song/(?P<slug>[^/?#]+)")
            .unwrap()
    })
}

pub fn album_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?audiomack\.com/(?P<artist>[^/]+)/album/(?P<slug>[^/?#]+)")
            .unwrap()
    })
}

pub fn playlist_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"https?://(?:www\.)?audiomack\.com/(?P<artist>[^/]+)/playlist/(?P<slug>[^/?#]+)",
        )
        .unwrap()
    })
}

pub fn artist_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?audiomack\.com/(?P<artist>[^/?#]+)(?:/songs)?/?$").unwrap()
    })
}

pub fn likes_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?audiomack\.com/(?P<artist>[^/]+)/likes").unwrap()
    })
}

pub fn parse_track(json: &Value) -> Option<Track> {
    let id_val = json.get("id").or_else(|| json.get("song_id"));
    let id = match id_val {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => {
            tracing::debug!("Audiomack track missing id: {json:?}");
            return None;
        }
    };
    let title = json.get("title")?.as_str()?.to_owned();
    let author = json.get("artist")?.as_str()?.to_owned();
    let duration_sec = json
        .get("duration")
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_i64().map(|i| i as u64))
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or_default();
    let uploader_slug = json
        .pointer("/uploader/url_slug")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("uploader_url_slug").and_then(|v| v.as_str()))
        .unwrap_or("unknown");
    let url_slug = json.get("url_slug")?.as_str()?;
    let uri = Some(format!(
        "https://audiomack.com/{uploader_slug}/song/{url_slug}"
    ));
    let artwork_url = json
        .get("image")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned());
    let isrc = json
        .get("isrc")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned());
    Some(Track::new(TrackInfo {
        identifier: id,
        is_seekable: true,
        author,
        length: duration_sec * 1000,
        is_stream: false,
        position: 0,
        title,
        uri,
        artwork_url,
        isrc,
        source_name: "audiomack".to_owned(),
    }))
}
