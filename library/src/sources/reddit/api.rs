use serde_json::Value;
use std::sync::Arc;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

pub async fn resolve_canonical_link(client: &Arc<reqwest::Client>, url: &str) -> Option<String> {
    let outcome = client
        .head(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .ok()?;
    if outcome.status().is_redirection() {
        outcome
            .headers()
            .get(reqwest::header::LOCATION)?
            .to_str()
            .ok()
            .map(|l| l.split('?').next().unwrap_or(l).to_owned())
    } else {
        Some(url.to_owned())
    }
}

pub async fn fetch_payload(client: &Arc<reqwest::Client>, url: &str) -> Option<Value> {
    client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()
}

pub async fn discover_audio_asset(
    client: &Arc<reqwest::Client>,
    primary_url: &str,
) -> Option<String> {
    let anchor = primary_url.split('_').next()?;
    let pool = vec![
        format!("{}_audio.mp4", anchor),
        format!("{}_AUDIO_128.mp4", anchor),
        format!("{}_audio.mp3", anchor),
        primary_url.replace("DASH_", "DASH_audio"),
    ];
    for probe_url in pool {
        let resp = client
            .head(&probe_url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await;
        if let Ok(res) = resp {
            if res.status().is_success() {
                return Some(probe_url);
            }
        }
    }
    None
}
