use regex::Regex;
use serde_json::Value;
use std::sync::{Arc, OnceLock};

pub fn path_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https?://(?:www\.)?last\.fm/(?:[a-z]{2}/)?(music|user)/([^/]+)(?:/([^/]+)(?:/([^/]+))?)?")
            .expect("lastfm path regex is a valid literal")
    })
}

pub fn search_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)<tr[^>]*?>[\s\S]*?<img[^>]*?src="([^"]+)"[\s\S]*?data-track-name="([^"]+)"[\s\S]*?data-track-url="([^"]+)"[\s\S]*?data-artist-name="([^"]+)"#)
            .expect("lastfm search regex is a valid literal")
    })
}

pub fn encode_path_segment(segment: &str) -> String {
    urlencoding::encode(segment).replace("%20", "+")
}

pub fn construct_track_url(artist: &str, track: &str) -> String {
    format!(
        "https://www.last.fm/music/{}/_/{}",
        encode_path_segment(artist),
        encode_path_segment(track)
    )
}

pub async fn get_json(client: &Arc<reqwest::Client>, url: &str) -> Option<Value> {
    let res = match client.get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .send()
        .await {
            Ok(r) => r,
            Err(e) => {
                let redacted = if let Some(pos) = url.find("api_key=") {
                    let end = url[pos..].find('&').map(|e| pos + e).unwrap_or(url.len());
                    let mut s = url.to_owned();
                    s.replace_range(pos + 8..end, "REDACTED");
                    s
                } else {
                    url.to_owned()
                };
                tracing::debug!("Last.fm: API request failed for {}: {}", redacted, e);
                return None;
            }
        };
    if !res.status().is_success() {
        let redacted = if let Some(pos) = url.find("api_key=") {
            let end = url[pos..].find('&').map(|e| pos + e).unwrap_or(url.len());
            let mut s = url.to_owned();
            s.replace_range(pos + 8..end, "REDACTED");
            s
        } else {
            url.to_owned()
        };
        tracing::debug!(
            "Last.fm: API returned error status {} for {}",
            res.status(),
            redacted
        );
        return None;
    }
    res.json().await.ok()
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
            .replace("&apos;", "'")
            .replace("&#x27;", "'");
        if next == result {
            break;
        }
        result = next;
    }
    result
}

pub fn parse_duration_to_ms(duration: &str) -> u64 {
    let parts: Vec<&str> = duration.split(':').collect();
    if parts.len() == 2 {
        let minutes = parts[0].trim().parse::<u64>().unwrap_or(0);
        let seconds = parts[1].trim().parse::<u64>().unwrap_or(0);
        (minutes * 60 + seconds) * 1000
    } else if parts.len() == 3 {
        let hours = parts[0].trim().parse::<u64>().unwrap_or(0);
        let minutes = parts[1].trim().parse::<u64>().unwrap_or(0);
        let seconds = parts[2].trim().parse::<u64>().unwrap_or(0);
        (hours * 3600 + minutes * 60 + seconds) * 1000
    } else {
        0
    }
}
