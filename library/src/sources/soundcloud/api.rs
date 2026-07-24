use crate::common::types::AnyResult;
use serde_json::Value;

const BASE_URL: &str = "https://api-v2.soundcloud.com";

pub async fn api_resolve(client: &reqwest::Client, url: &str, client_id: &str) -> AnyResult<Value> {
    let req_url = format!(
        "{}/resolve?url={}&client_id={}",
        BASE_URL,
        urlencoding::encode(url),
        client_id
    );
    let resp = client.get(&req_url).send().await?;
    if resp.status().as_u16() == 401 {
        return Err("401 Unauthorized".into());
    }
    if !resp.status().is_success() {
        return Err(format!("Resolve failed: status={}", resp.status()).into());
    }
    Ok(resp.json().await?)
}

pub async fn resolve_stream_url(
    client: &reqwest::Client,
    lookup_url: &str,
    client_id: &str,
) -> AnyResult<String> {
    let url = format!("{}?client_id={}", lookup_url, client_id);
    let resp = client.get(&url).send().await?;
    if resp.status().as_u16() == 401 {
        return Err("401 Unauthorized".into());
    }
    if !resp.status().is_success() {
        return Err(format!("Resolve stream URL failed: status={}", resp.status()).into());
    }
    let json: Value = resp.json().await?;
    let stream_url = json
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing stream URL in response")?;
    Ok(stream_url.to_string())
}

pub async fn search_tracks_api(
    client: &reqwest::Client,
    query: &str,
    client_id: &str,
    limit: usize,
) -> AnyResult<Value> {
    let req_url = format!(
        "{}/search/tracks?q={}&client_id={}&limit={}&offset=0",
        BASE_URL,
        urlencoding::encode(query),
        client_id,
        limit
    );
    let resp = client.get(&req_url).send().await?;
    if !resp.status().is_success() {
        return Err(format!("Search request failed: status={}", resp.status()).into());
    }
    Ok(resp.json().await?)
}

pub async fn load_liked_tracks_api(
    client: &reqwest::Client,
    user_id: &str,
    client_id: &str,
) -> AnyResult<Value> {
    let liked_url =
        format!("{BASE_URL}/users/{user_id}/likes?limit=200&offset=0&client_id={client_id}");
    let resp = client.get(&liked_url).send().await?;
    if !resp.status().is_success() {
        return Err(format!("Likes request failed: status={}", resp.status()).into());
    }
    Ok(resp.json().await?)
}

pub async fn load_collection_tracks_api(
    client: &reqwest::Client,
    user_id: u64,
    endpoint: &str,
    client_id: &str,
) -> AnyResult<Value> {
    let req_url = format!(
        "{BASE_URL}/users/{user_id}/{endpoint}?client_id={client_id}&limit=200&offset=0&linked_partitioning=1"
    );
    let resp = client.get(&req_url).send().await?;
    if !resp.status().is_success() {
        return Err(format!("Collection request failed: status={}", resp.status()).into());
    }
    Ok(resp.json().await?)
}

pub async fn load_tracks_batch_api(
    client: &reqwest::Client,
    ids: &str,
    client_id: &str,
) -> AnyResult<Value> {
    let batch_url = format!("{BASE_URL}/tracks?ids={ids}&client_id={client_id}");
    let resp = client.get(&batch_url).send().await?;
    if !resp.status().is_success() {
        return Err(format!("Batch tracks request failed: status={}", resp.status()).into());
    }
    Ok(resp.json().await?)
}
