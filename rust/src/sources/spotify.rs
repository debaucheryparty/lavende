pub mod helpers {
use std::sync::Arc;
use futures::future::join_all;
use serde_json::{Value, json};
use tokio::sync::Semaphore;
use tracing::warn;
use crate::sources::spotify::token::SpotifyTokenTracker;
const PARTNER_API_URL: &str = "https://api-partner.spotify.com/pathfinder/v2/query";
pub struct SpotifyHelpers;
impl SpotifyHelpers {
    pub fn base62_to_hex(id: &str) -> String {
        const ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut bn = 0u128;
        for c in id.chars() {
            if let Some(idx) = ALPHABET.find(c) {
                bn = bn.wrapping_mul(62).wrapping_add(idx as u128);
            }
        }
        format!("{:032x}", bn)
    }
    pub async fn partner_api_request(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        operation: &str,
        variables: Value,
        sha256_hash: &str,
    ) -> Option<Value> {
        let token = token_tracker.get_token().await?;
        let body = json!({
            "variables": variables,
            "operationName": operation,
            "extensions": {
                "persistedQuery": {
                    "version": 1,
                    "sha256Hash": sha256_hash
                }
            }
        });
        let resp = client
            .post(PARTNER_API_URL)
            .bearer_auth(token)
            .header("App-Platform", "WebPlayer")
            .header("Spotify-App-Version", "1.2.81.104.g225ec0e6")
            .json(&body)
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            warn!("Partner API returned {} for {}", resp.status(), operation);
            return None;
        }
        resp.json().await.ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn fetch_paginated_items(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        operation: &str,
        sha256_hash: &str,
        base_vars: Value,
        items_pointer: &str,
        total_count: u64,
        page_limit: u64,
        concurrency: usize,
    ) -> Vec<Value> {
        let pages_needed = total_count.saturating_sub(page_limit);
        if pages_needed == 0 {
            return Vec::new();
        }
        let offsets: Vec<u64> = (1..=((total_count - 1) / page_limit)).collect();
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let futs: Vec<_> = offsets
            .into_iter()
            .map(|page_idx| {
                let semaphore = semaphore.clone();
                let mut vars = base_vars.clone();
                if let Some(obj) = vars.as_object_mut() {
                    obj.insert("offset".to_owned(), json!(page_idx * page_limit));
                    obj.insert("limit".to_owned(), json!(page_limit));
                }
                let op = operation.to_owned();
                let h = sha256_hash.to_owned();
                let c = client.clone();
                let tt = token_tracker.clone();
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    Self::partner_api_request(&c, &tt, &op, vars, &h).await
                }
            })
            .collect();
        let results = join_all(futs).await;
        results
            .into_iter()
            .flatten()
            .filter_map(|result| {
                result
                    .pointer(items_pointer)
                    .and_then(|v| v.as_array())
                    .cloned()
            })
            .flatten()
            .collect()
    }
}
}
pub mod metadata {
use std::sync::Arc;
use futures::future::join_all;
use serde_json::{Value, json};
use tokio::sync::Semaphore;
use crate::{
    protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track, TrackInfo},
    sources::spotify::{
        helpers::SpotifyHelpers, parser::SpotifyParser, token::SpotifyTokenTracker,
    },
};
pub struct SpotifyMetadata;
impl SpotifyMetadata {
    pub async fn fetch_metadata_isrc(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        id: &str,
        isrc_binary_regex: &regex::Regex,
    ) -> Option<String> {
        let token = token_tracker.get_token().await?;
        let hex_id = SpotifyHelpers::base62_to_hex(id);
        let url =
            format!("https://spclient.wg.spotify.com/metadata/4/track/{hex_id}?market=from_token");
        let resp = client
            .get(&url)
            .bearer_auth(token)
            .header("App-Platform", "WebPlayer")
            .header("Spotify-App-Version", "1.2.81.104.g225ec0e6")
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body_bytes = resp.bytes().await.ok()?;
        let isrc_marker = b"isrc";
        if let Some(pos) = body_bytes.windows(4).position(|w| w == isrc_marker) {
            let end = std::cmp::min(pos + 64, body_bytes.len());
            let chunk_str = String::from_utf8_lossy(&body_bytes[pos..end]);
            if let Some(mat) = isrc_binary_regex.find(&chunk_str) {
                return Some(mat.as_str().to_owned());
            }
        }
        if let Ok(json_str) = std::str::from_utf8(&body_bytes)
            && let Ok(json) = serde_json::from_str::<Value>(json_str)
            && let Some(isrc) = json
                .get("external_id")
                .and_then(|ids| ids.as_array())
                .and_then(|items| {
                    items
                        .iter()
                        .find(|i| i.get("type").and_then(|v| v.as_str()) == Some("isrc"))
                })
                .and_then(|i| i.get("id"))
                .and_then(|v| v.as_str())
        {
            return Some(isrc.to_owned());
        }
        None
    }
    pub async fn parse_generic_track(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        track_val: &Value,
        artwork_url: Option<String>,
        isrc_binary_regex: &regex::Regex,
    ) -> Option<TrackInfo> {
        let mut track_info = SpotifyParser::parse_track_inner(track_val, artwork_url)?;
        if track_info.isrc.is_none() {
            let isrc = Self::fetch_metadata_isrc(
                client,
                token_tracker,
                &track_info.identifier,
                isrc_binary_regex,
            )
            .await;
            track_info.isrc = isrc;
        }
        Some(track_info)
    }
    pub async fn fetch_track(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        id: &str,
        isrc_binary_regex: &regex::Regex,
    ) -> Option<TrackInfo> {
        let variables = json!({
            "uri": format!("spotify:track:{id}")
        });
        let hash = "612585ae06ba435ad26369870deaae23b5c8800a256cd8a57e08eddc25a37294";
        let data =
            SpotifyHelpers::partner_api_request(client, token_tracker, "getTrack", variables, hash)
                .await?;
        let track = data.pointer("/data/trackUnion")?;
        Self::parse_generic_track(client, token_tracker, track, None, isrc_binary_regex).await
    }
    pub async fn fetch_album(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        id: &str,
        album_load_limit: usize,
        album_page_load_concurrency: usize,
        track_resolve_concurrency: usize,
        isrc_binary_regex: &regex::Regex,
    ) -> LoadResult {
        const HASH: &str = "b9bfabef66ed756e5e13f68a942deb60bd4125ec1f1be8cc42769dc0259b4b10";
        const PAGE_LIMIT: u64 = 50;
        let base_vars = json!({
            "uri": format!("spotify:album:{id}"),
            "locale": "en",
            "offset": 0,
            "limit": PAGE_LIMIT
        });
        let data = match SpotifyHelpers::partner_api_request(
            client,
            token_tracker,
            "getAlbum",
            base_vars.clone(),
            HASH,
        )
        .await
        {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let album = match data.pointer("/data/albumUnion") {
            Some(a) => a,
            None => return LoadResult::Empty {},
        };
        let name = album
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Album")
            .to_owned();
        let total_count = album
            .pointer("/tracksV2/totalCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let album_artwork = album
            .pointer("/coverArt/sources")
            .and_then(|s| s.as_array())
            .and_then(|s| s.first())
            .and_then(|i| i.get("url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let mut all_items: Vec<Value> = album
            .pointer("/tracksV2/items")
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default();
        if total_count > PAGE_LIMIT {
            let max_tracks = if album_load_limit == 0 {
                u64::MAX
            } else {
                album_load_limit as u64 * PAGE_LIMIT
            };
            let effective_total = total_count.min(max_tracks);
            if effective_total > PAGE_LIMIT {
                let extra = SpotifyHelpers::fetch_paginated_items(
                    client,
                    token_tracker,
                    "getAlbum",
                    HASH,
                    base_vars,
                    "/data/albumUnion/tracksV2/items",
                    effective_total,
                    PAGE_LIMIT,
                    album_page_load_concurrency,
                )
                .await;
                all_items.extend(extra);
            }
        }
        let semaphore = Arc::new(Semaphore::new(track_resolve_concurrency));
        let futs: Vec<_> = all_items
            .into_iter()
            .take(if album_load_limit > 0 {
                (PAGE_LIMIT * album_load_limit as u64) as usize
            } else {
                usize::MAX
            })
            .filter_map(|item| {
                let track_data = item.get("track")?.clone();
                let semaphore = semaphore.clone();
                let artwork = album_artwork.clone();
                let c = client.clone();
                let tt = token_tracker.clone();
                let re = isrc_binary_regex.clone();
                Some(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    Self::parse_generic_track(&c, &tt, &track_data, artwork, &re).await
                })
            })
            .collect();
        let results = join_all(futs).await;
        let tracks: Vec<Track> = results.into_iter().flatten().map(Track::new).collect();
        if tracks.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name,
                    selected_track: -1,
                },
                plugin_info: json!({ "type": "album", "url": format!("https://open.spotify.com/album/{id}"), "artworkUrl": album_artwork, "author": album.pointer("/artists/items/0/profile/name").and_then(|v| v.as_str()), "totalTracks": total_count }),
                tracks,
            })
        }
    }
    pub async fn fetch_playlist(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        id: &str,
        playlist_load_limit: usize,
        playlist_page_load_concurrency: usize,
        track_resolve_concurrency: usize,
        isrc_binary_regex: &regex::Regex,
    ) -> LoadResult {
        const HASH: &str = "bb67e0af06e8d6f52b531f97468ee4acd44cd0f82b988e15c2ea47b1148efc77";
        const PAGE_LIMIT: u64 = 100;
        let base_vars = json!({
            "uri": format!("spotify:playlist:{id}"),
            "offset": 0,
            "limit": PAGE_LIMIT,
            "enableWatchFeedEntrypoint": false
        });
        let data = match SpotifyHelpers::partner_api_request(
            client,
            token_tracker,
            "fetchPlaylist",
            base_vars.clone(),
            HASH,
        )
        .await
        {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let playlist = match data.pointer("/data/playlistV2") {
            Some(p) => p,
            None => return LoadResult::Empty {},
        };
        let name = playlist
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Playlist")
            .to_owned();
        let total_count = playlist
            .pointer("/content/totalCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let mut all_items: Vec<Value> = playlist
            .pointer("/content/items")
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default();
        if total_count > PAGE_LIMIT {
            let max_tracks = if playlist_load_limit == 0 {
                u64::MAX
            } else {
                playlist_load_limit as u64 * PAGE_LIMIT
            };
            let effective_total = total_count.min(max_tracks);
            if effective_total > PAGE_LIMIT {
                let extra = SpotifyHelpers::fetch_paginated_items(
                    client,
                    token_tracker,
                    "fetchPlaylist",
                    HASH,
                    base_vars,
                    "/data/playlistV2/content/items",
                    effective_total,
                    PAGE_LIMIT,
                    playlist_page_load_concurrency,
                )
                .await;
                all_items.extend(extra);
            }
        }
        let semaphore = Arc::new(Semaphore::new(track_resolve_concurrency));
        let futs: Vec<_> = all_items
            .into_iter()
            .take(if playlist_load_limit > 0 {
                (PAGE_LIMIT * playlist_load_limit as u64) as usize
            } else {
                usize::MAX
            })
            .filter_map(|item| {
                let track_data = item
                    .pointer("/item/data")
                    .or_else(|| item.pointer("/itemV2/data"))?
                    .clone();
                let semaphore = semaphore.clone();
                let c = client.clone();
                let tt = token_tracker.clone();
                let re = isrc_binary_regex.clone();
                Some(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    Self::parse_generic_track(&c, &tt, &track_data, None, &re).await
                })
            })
            .collect();
        let results = join_all(futs).await;
        let tracks: Vec<Track> = results.into_iter().flatten().map(Track::new).collect();
        if tracks.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: name.clone(),
                    selected_track: -1,
                },
                plugin_info: json!({
                  "type": "playlist",
                  "url": format!("https://open.spotify.com/playlist/{id}"),
                  "artworkUrl": playlist.pointer("/images/items/0/sources/0/url").and_then(|v| v.as_str()),
                  "author": playlist.get("ownerV2").and_then(|v| v.get("name")).and_then(|v| v.as_str()).or_else(|| (id.starts_with("37i9dQZ")).then_some("Spotify")),
                  "totalTracks": total_count
                }),
                tracks,
            })
        }
    }
    pub async fn fetch_artist(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        id: &str,
        isrc_binary_regex: &regex::Regex,
    ) -> LoadResult {
        let variables = json!({
            "uri": format!("spotify:artist:{id}"),
            "locale": "en",
            "includePrerelease": true
        });
        let hash = "35648a112beb1794e39ab931365f6ae4a8d45e65396d641eeda94e4003d41497";
        let data = match SpotifyHelpers::partner_api_request(
            client,
            token_tracker,
            "queryArtistOverview",
            variables,
            hash,
        )
        .await
        {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let artist = match data.pointer("/data/artistUnion") {
            Some(a) => a,
            None => return LoadResult::Empty {},
        };
        let name = artist
            .get("profile")
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Artist")
            .to_owned();
        let mut tracks = Vec::new();
        if let Some(items) = artist
            .pointer("/discography/topTracks/items")
            .and_then(|i| i.as_array())
        {
            for item in items {
                if let Some(track_data) = item.get("track") {
                    let c = client.clone();
                    let tt = token_tracker.clone();
                    let re = isrc_binary_regex.to_owned();
                    if let Some(track_info) =
                        Self::parse_generic_track(&c, &tt, track_data, None, &re).await
                    {
                        tracks.push(Track::new(track_info));
                    }
                }
            }
        }
        if tracks.is_empty() {
            LoadResult::Empty {}
        } else {
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: name.clone(),
                    selected_track: -1,
                },
                plugin_info: json!({
                  "type": "artist",
                  "url": format!("https://open.spotify.com/artist/{id}"),
                  "artworkUrl": artist.pointer("/visuals/avatar/sources/0/url").and_then(|v| v.as_str()),
                  "author": name,
                  "totalTracks": tracks.len()
                }),
                tracks,
            })
        }
    }
}
}
pub mod parser {
use serde_json::Value;
use tracing::debug;
use crate::protocol::tracks::TrackInfo;
pub struct SpotifyParser;
impl SpotifyParser {
    pub fn parse_track_inner(track_val: &Value, artwork_url: Option<String>) -> Option<TrackInfo> {
        let track = if track_val.get("uri").is_some() {
            track_val
        } else if let Some(inner) = track_val.get("track") {
            inner
        } else if let Some(inner) = track_val.get("item") {
            inner
        } else if let Some(inner) = track_val.get("data") {
            inner
        } else {
            debug!(
                "Track data missing uri and no nested track property: {:?}",
                track_val
            );
            return None;
        };
        let uri = track.get("uri").and_then(|v| v.as_str())?;
        let id = uri.split(':').next_back()?.to_owned();
        let title = track.get("name").and_then(|v| v.as_str())?.to_owned();
        let author = Self::extract_author(track);
        let length = track
            .get("duration_ms")
            .or_else(|| {
                track
                    .get("duration")
                    .or_else(|| track.get("trackDuration"))
                    .and_then(|d| d.get("totalMilliseconds"))
            })
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let final_artwork = artwork_url.or_else(|| {
            track
                .get("albumOfTrack")
                .and_then(|a| a.get("coverArt"))
                .and_then(|c| c.get("sources"))
                .and_then(|s| s.as_array())
                .and_then(|s| s.first())
                .and_then(|i| i.get("url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned())
                .or_else(|| {
                    track
                        .get("album")
                        .and_then(|a| a.get("images"))
                        .and_then(|i| i.as_array())
                        .and_then(|i| i.first())
                        .and_then(|i| i.get("url"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_owned())
                })
        });
        let isrc = Self::extract_isrc_inline(track);
        Some(TrackInfo {
            title,
            author,
            length,
            identifier: id.clone(),
            is_stream: false,
            uri: Some(format!("https://open.spotify.com/track/{id}")),
            artwork_url: final_artwork,
            isrc,
            source_name: "spotify".to_owned(),
            is_seekable: true,
            position: 0,
        })
    }
    pub fn extract_author(track: &Value) -> String {
        if let Some(artists) = track
            .get("artists")
            .and_then(|a| a.get("items"))
            .and_then(|i| i.as_array())
        {
            let names: Vec<_> = artists
                .iter()
                .filter_map(|a| {
                    a.get("profile")
                        .and_then(|p| p.get("name"))
                        .or_else(|| a.get("name"))
                        .and_then(|v| v.as_str())
                })
                .collect();
            if !names.is_empty() {
                return names.join(", ");
            }
        }
        if let Some(first_artist) = track
            .get("firstArtist")
            .and_then(|a| a.get("items"))
            .and_then(|i| i.as_array())
            .and_then(|i| i.first())
        {
            let first_name = first_artist
                .get("profile")
                .and_then(|p| p.get("name"))
                .or_else(|| first_artist.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let mut all_artists = vec![first_name];
            if let Some(others) = track
                .get("otherArtists")
                .and_then(|a| a.get("items"))
                .and_then(|i| i.as_array())
            {
                for artist in others {
                    if let Some(name) = artist
                        .get("profile")
                        .and_then(|p| p.get("name"))
                        .or_else(|| artist.get("name"))
                        .and_then(|v| v.as_str())
                    {
                        all_artists.push(name);
                    }
                }
            }
            return all_artists.join(", ");
        }
        if let Some(artists) = track.get("artists").and_then(|a| a.as_array()) {
            let names: Vec<_> = artists
                .iter()
                .filter_map(|a| {
                    a.get("name")
                        .or_else(|| a.get("profile").and_then(|p| p.get("name")))
                        .and_then(|v| v.as_str())
                })
                .collect();
            if !names.is_empty() {
                return names.join(", ");
            }
        }
        track
            .get("artist")
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Artist")
            .to_owned()
    }
    pub fn extract_isrc_inline(track: &Value) -> Option<String> {
        track
            .get("externalIds")
            .or_else(|| track.get("external_ids"))
            .and_then(|ids| {
                if let Some(isrc) = ids
                    .get("isrc")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    return Some(isrc.to_owned());
                }
                ids.get("items")
                    .and_then(|items| items.as_array())
                    .and_then(|items| {
                        items
                            .iter()
                            .find(|i| i.get("type").and_then(|v| v.as_str()) == Some("isrc"))
                    })
                    .and_then(|i| i.get("id"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_owned())
            })
    }
}
}
pub mod recommendations {
use std::sync::Arc;
use futures::future::join_all;
use serde_json::{Value, json};
use crate::{
    protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track},
    sources::spotify::{
        helpers::SpotifyHelpers, parser::SpotifyParser, search::SpotifySearch,
        token::SpotifyTokenTracker,
    },
};
pub struct SpotifyRecommendations;
impl SpotifyRecommendations {
    pub async fn fetch_recommendations(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        query: &str,
        mix_regex: &regex::Regex,
        recommendations_limit: usize,
        search_limit: usize,
        isrc_binary_regex: &regex::Regex,
    ) -> Result<LoadResult, String> {
        let mut seed = query.to_owned();
        if let Some(caps) = mix_regex.captures(query) {
            let mut seed_type = caps.get(1).unwrap().as_str().to_owned();
            seed = caps.get(2).unwrap().as_str().to_owned();
            if seed_type == "isrc" {
                if let Some(res) = SpotifySearch::search_full(
                    client,
                    token_tracker,
                    &format!("isrc:{seed}"),
                    &["track".to_owned()],
                    search_limit,
                    isrc_binary_regex,
                )
                .await
                {
                    if let Some(track) = res.tracks.first() {
                        seed = track.info.identifier.clone();
                        seed_type = "track".to_string();
                    } else {
                        return Ok(LoadResult::Empty {});
                    }
                } else {
                    return Ok(LoadResult::Empty {});
                }
            }
            let token = match token_tracker.get_token().await {
                Some(t) => t,
                None => return Ok(LoadResult::Empty {}),
            };
            let url = format!(
                "https://spclient.wg.spotify.com/inspiredby-mix/v2/seed_to_playlist/spotify:{seed_type}:{seed}?response-format=json"
            );
            let resp = client
                .get(&url)
                .bearer_auth(token)
                .header("App-Platform", "WebPlayer")
                .header("Spotify-App-Version", "1.2.81.104.g225ec0e6")
                .send()
                .await
                .ok();
            if let Some(resp) = resp
                && resp.status().is_success()
                && let Ok(json) = resp.json::<Value>().await
                && let Some(playlist_uri) =
                    json.pointer("/mediaItems/0/uri").and_then(|v| v.as_str())
                && let Some(id) = playlist_uri.split(':').next_back()
            {
                return Err(id.to_owned());
            }
        }
        let track_id = seed.strip_prefix("track:").unwrap_or(&seed);
        Ok(Self::fetch_pathfinder_recommendations(
            client,
            token_tracker,
            track_id,
            recommendations_limit,
        )
        .await)
    }
    pub async fn fetch_pathfinder_recommendations(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        id: &str,
        recommendations_limit: usize,
    ) -> LoadResult {
        let variables = json!({
            "uri": format!("spotify:track:{id}"),
            "limit": recommendations_limit
        });
        let hash = "c77098ee9d6ee8ad3eb844938722db60570d040b49f41f5ec6e7be9160a7c86b";
        let data = match SpotifyHelpers::partner_api_request(
            client,
            token_tracker,
            "internalLinkRecommenderTrack",
            variables,
            hash,
        )
        .await
        {
            Some(d) => d,
            None => return LoadResult::Empty {},
        };
        let items = data
            .pointer("/data/internalLinkRecommenderTrack/relatedTracks/items")
            .or_else(|| data.pointer("/data/seoRecommendedTrack/items"))
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            return LoadResult::Empty {};
        }
        let mut tracks = Vec::new();
        let futs: Vec<_> = items
            .into_iter()
            .map(|item| async move { SpotifyParser::parse_track_inner(&item, None) })
            .collect();
        let results = join_all(futs).await;
        for track_info in results.into_iter().flatten() {
            tracks.push(Track::new(track_info));
        }
        if tracks.is_empty() {
            return LoadResult::Empty {};
        }
        tracks.truncate(recommendations_limit);
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: "Spotify Recommendations".to_owned(),
                selected_track: -1,
            },
            plugin_info: json!({
              "type": "recommendations",
              "totalTracks": tracks.len()
            }),
            tracks,
        })
    }
}
}
pub mod search {
use std::{sync::Arc, time::Duration};
use serde_json::json;
use tokio::time::timeout;
use crate::{
    protocol::tracks::{PlaylistData, PlaylistInfo, SearchResult, Track},
    sources::spotify::{
        helpers::SpotifyHelpers, metadata::SpotifyMetadata, parser::SpotifyParser,
        token::SpotifyTokenTracker,
    },
};
pub struct SpotifySearch;
impl SpotifySearch {
    pub async fn get_autocomplete(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        query: &str,
        types: &[String],
        search_limit: usize,
        isrc_binary_regex: &regex::Regex,
    ) -> Option<SearchResult> {
        Self::search_full(
            client,
            token_tracker,
            query,
            types,
            search_limit,
            isrc_binary_regex,
        )
        .await
    }
    pub async fn search_full(
        client: &reqwest::Client,
        token_tracker: &Arc<SpotifyTokenTracker>,
        query: &str,
        types: &[String],
        search_limit: usize,
        isrc_binary_regex: &regex::Regex,
    ) -> Option<SearchResult> {
        let variables = json!({
            "searchTerm": query,
            "offset": 0,
            "limit": search_limit,
            "numberOfTopResults": 5,
            "includeAudiobooks": false,
            "includeArtistHasConcertsField": false,
            "includePreReleases": false
        });
        let hash = "fcad5a3e0d5af727fb76966f06971c19cfa2275e6ff7671196753e008611873c";
        let data = match SpotifyHelpers::partner_api_request(
            client,
            token_tracker,
            "searchDesktop",
            variables,
            hash,
        )
        .await
        {
            Some(d) => d,
            None => {
                return None;
            }
        };
        let mut tracks = Vec::new();
        let mut albums = Vec::new();
        let mut artists = Vec::new();
        let mut playlists = Vec::new();
        let all_types = types.is_empty();
        if (all_types || types.contains(&"track".to_owned()))
            && let Some(items) = data
                .pointer("/data/searchV2/tracksV2/items")
                .or_else(|| data.pointer("/data/searchV2/tracks/items"))
                .and_then(|v| v.as_array())
        {
            for item in items {
                if let Some(track_data) = item
                    .get("item")
                    .or_else(|| item.get("itemV2"))
                    .and_then(|v| v.get("data"))
                    .or_else(|| item.get("data"))
                    && let Some(track_info) = SpotifyParser::parse_track_inner(track_data, None)
                {
                    let mut track = Track::new(track_info);
                    let album_name = track_data
                        .pointer("/albumOfTrack/name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_owned());
                    let album_url = track_data
                        .pointer("/albumOfTrack/uri")
                        .and_then(|v| v.as_str())
                        .map(|s| {
                            let id = s.split(':').next_back().unwrap_or("");
                            format!("https://open.spotify.com/album/{id}")
                        });
                    let artist_url = track_data
                        .pointer("/artists/items/0/uri")
                        .and_then(|v| v.as_str())
                        .map(|s| {
                            let id = s.split(':').next_back().unwrap_or("");
                            format!("https://open.spotify.com/artist/{id}")
                        });
                    track.plugin_info = json!({
                        "albumName": album_name,
                        "albumUrl": album_url,
                        "artistUrl": artist_url,
                        "artistArtworkUrl": null,
                        "previewUrl": null,
                        "isPreview": false
                    });
                    if track.info.isrc.is_none()
                        && let Ok(Some(isrc)) = timeout(
                            Duration::from_secs(2),
                            SpotifyMetadata::fetch_metadata_isrc(
                                client,
                                token_tracker,
                                &track.info.identifier,
                                isrc_binary_regex,
                            ),
                        )
                        .await
                    {
                        track.info.isrc = Some(isrc);
                    }
                    tracks.push(track);
                }
            }
        }
        if (all_types || types.contains(&"album".to_owned()))
            && let Some(items) = data
                .pointer("/data/searchV2/albumsV2/items")
                .or_else(|| data.pointer("/data/searchV2/albums/items"))
                .and_then(|v| v.as_array())
        {
            for item in items {
                if let Some(album_data) = item
                    .get("item")
                    .or_else(|| item.get("itemV2"))
                    .and_then(|v| v.get("data"))
                    .or_else(|| item.get("data"))
                {
                    let name = album_data
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Album");
                    let uri = album_data.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                    let id = uri.split(':').next_back().unwrap_or("");
                    let artwork = album_data
                        .pointer("/coverArt/sources/0/url")
                        .and_then(|v| v.as_str());
                    let author = album_data
                        .pointer("/artists/items/0/profile/name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Artist");
                    albums.push(PlaylistData {
                        info: PlaylistInfo {
                            name: name.to_owned(),
                            selected_track: -1,
                        },
                        plugin_info: json!({
                          "type": "album",
                          "url": format!("https://open.spotify.com/album/{id}"),
                          "artworkUrl": artwork,
                          "author": author,
                          "totalTracks": 0
                        }),
                        tracks: Vec::new(),
                    });
                }
            }
        }
        if (all_types || types.contains(&"artist".to_owned()))
            && let Some(items) = data
                .pointer("/data/searchV2/artistsV2/items")
                .or_else(|| data.pointer("/data/searchV2/artists/items"))
                .or_else(|| data.pointer("/data/searchV2/profilesV2/items"))
                .or_else(|| data.pointer("/data/searchV2/profiles/items"))
                .and_then(|v| v.as_array())
        {
            for item in items {
                if let Some(artist_data) = item
                    .get("item")
                    .or_else(|| item.get("itemV2"))
                    .and_then(|v| v.get("data"))
                    .or_else(|| item.get("data"))
                {
                    let name = artist_data
                        .pointer("/profile/name")
                        .or_else(|| artist_data.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Artist");
                    let uri = artist_data
                        .get("uri")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let id = uri.split(':').next_back().unwrap_or("");
                    let artwork = artist_data
                        .pointer("/visuals/avatarImage/sources/0/url")
                        .or_else(|| artist_data.pointer("/images/items/0/sources/0/url"))
                        .and_then(|v| v.as_str());
                    artists.push(PlaylistData {
                        info: PlaylistInfo {
                            name: format!("{name}'s Top Tracks"),
                            selected_track: -1,
                        },
                        plugin_info: json!({
                          "type": "artist",
                          "url": format!("https://open.spotify.com/artist/{id}"),
                          "artworkUrl": artwork,
                          "author": name,
                          "totalTracks": 0
                        }),
                        tracks: Vec::new(),
                    });
                }
            }
        }
        if all_types || types.contains(&"playlist".to_owned()) {
            let playlist_paths = [
                "/data/searchV2/playlistsV2/items",
                "/data/searchV2/playlists/items",
            ];
            for path in playlist_paths {
                if let Some(items) = data.pointer(path).and_then(|v| v.as_array()) {
                    for item in items {
                        if let Some(playlist_data) = item
                            .get("item")
                            .or_else(|| item.get("itemV2"))
                            .and_then(|v| v.get("data"))
                            .or_else(|| item.get("data"))
                        {
                            let name = playlist_data
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown");
                            let uri = playlist_data
                                .get("uri")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let parts: Vec<&str> = uri.split(':').collect();
                            let type_str = parts.get(1).unwrap_or(&"playlist");
                            let id = parts.last().unwrap_or(&"");
                            let artwork = playlist_data
                                .pointer("/images/items/0/sources/0/url")
                                .or_else(|| playlist_data.pointer("/coverArt/sources/0/url"))
                                .and_then(|v| v.as_str());
                            let author = playlist_data
                                .pointer("/ownerV2/data/name")
                                .or_else(|| playlist_data.pointer("/ownerV2/name"))
                                .and_then(|v| v.as_str());
                            playlists.push(PlaylistData {
                                info: PlaylistInfo {
                                    name: name.to_owned(),
                                    selected_track: -1,
                                },
                                plugin_info: json!({
                                  "type": type_str,
                                  "url": format!("https://open.spotify.com/{type_str}/{id}"),
                                  "artworkUrl": artwork,
                                  "author": author,
                                  "totalTracks": 0
                                }),
                                tracks: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
        Some(SearchResult {
            tracks,
            albums,
            artists,
            playlists,
            texts: Vec::new(),
            plugin: json!({}),
        })
    }
}
}
pub mod token {
use std::sync::{Arc, OnceLock};
use regex::Regex;
use tokio::sync::RwLock;
use tracing::{debug, error};
use crate::common::types::SharedRw;
const EMBED_URL: &str = "https://open.spotify.com/embed/track/4cOdK2wGLETKBW3PvgPWqT";
fn token_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#""accessToken":"([^"]+)""#).expect("spotify token regex is a valid literal")
    })
}
fn expiry_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#""accessTokenExpirationTimestampMs":(\d+)"#)
            .expect("spotify expiry regex is a valid literal")
    })
}
#[derive(Clone, Debug)]
pub struct SpotifyToken {
    pub access_token: String,
    pub expiry_ms: u64,
}
pub struct SpotifyTokenTracker {
    client: Arc<reqwest::Client>,
    token: SharedRw<Option<SpotifyToken>>,
}
impl SpotifyTokenTracker {
    pub fn new(client: Arc<reqwest::Client>) -> Self {
        Self {
            client,
            token: Arc::new(RwLock::new(None)),
        }
    }
    pub async fn get_token(&self) -> Option<String> {
        {
            let token_lock = self.token.read().await;
            if let Some(token) = &*token_lock {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                if token.expiry_ms > now + 5_000 {
                    return Some(token.access_token.clone());
                }
            }
        }
        self.refresh_token().await
    }
    async fn refresh_token(&self) -> Option<String> {
        debug!("Refreshing Spotify token from embed...");
        let request = self
            .client
            .get(EMBED_URL)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.6998.178 Spotify/1.2.65.255 Safari/537.36")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Sec-Fetch-Dest", "iframe")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "cross-site");
        let resp = match request.send().await {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to fetch Spotify embed page: {e}");
                return None;
            }
        };
        if !resp.status().is_success() {
            error!("Embed page returned status {}", resp.status());
            return None;
        }
        let html = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to read Spotify embed HTML: {e}");
                return None;
            }
        };
        let token_caps = token_regex().captures(&html);
        let expiry_caps = expiry_regex().captures(&html);
        if token_caps.is_none() || expiry_caps.is_none() {
            error!("Token or expiry not found in embed page");
            return None;
        }
        let token = match token_caps.and_then(|c| c.get(1)) {
            Some(m) => m.as_str().to_owned(),
            None => {
                error!("Successfully found token caps but group 1 was missing");
                return None;
            }
        };
        let expiry_ms = match expiry_caps.and_then(|c| c.get(1)) {
            Some(m) => m.as_str().parse::<u64>().ok()?,
            None => {
                error!("Successfully found expiry caps but group 1 was missing");
                return None;
            }
        };
        let mut token_lock = self.token.write().await;
        *token_lock = Some(SpotifyToken {
            access_token: token.clone(),
            expiry_ms,
        });
        debug!("Successfully refreshed Spotify token. Expiry: {expiry_ms}");
        Some(token)
    }
    pub fn init(self: Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            this.get_token().await;
        });
    }
}
}
use std::sync::{Arc, OnceLock};
use async_trait::async_trait;
use regex::Regex;
use crate::{
    protocol::tracks::{LoadResult, Track},
    sources::{SourcePlugin, playable_track::BoxedTrack, spotify::token::SpotifyTokenTracker},
};
fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"https?://(?:open\.)?spotify\.com/(?:intl-[a-z]{2}/)?(track|album|playlist|artist)/([a-zA-Z0-9]+)",
        ).expect("spotify URL regex is a valid literal")
    })
}
fn mix_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"mix:(album|artist|track|isrc):([a-zA-Z0-9\-_]+)")
            .expect("spotify mix regex is a valid literal")
    })
}
fn isrc_binary_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"[A-Z0-9]{12}").expect("spotify ISRC binary regex is a valid literal")
    })
}
pub struct SpotifySource {
    client: Arc<reqwest::Client>,
    token_tracker: Arc<SpotifyTokenTracker>,
    playlist_load_limit: usize,
    album_load_limit: usize,
    search_limit: usize,
    recommendations_limit: usize,
    playlist_page_load_concurrency: usize,
    album_page_load_concurrency: usize,
    track_resolve_concurrency: usize,
}
impl SpotifySource {
    pub fn new(
        config: Option<crate::config::SpotifyConfig>,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let (
            playlist_load_limit,
            album_load_limit,
            search_limit,
            recommendations_limit,
            playlist_page_load_concurrency,
            album_page_load_concurrency,
            track_resolve_concurrency,
        ) = if let Some(c) = config {
            (
                c.playlist_load_limit,
                c.album_load_limit,
                c.search_limit,
                c.recommendations_limit,
                c.playlist_page_load_concurrency,
                c.album_page_load_concurrency,
                c.track_resolve_concurrency,
            )
        } else {
            (6, 6, 10, 10, 10, 5, 50)
        };
        let token_tracker = Arc::new(SpotifyTokenTracker::new(client.clone()));
        token_tracker.clone().init();
        Ok(Self {
            client,
            token_tracker,
            playlist_load_limit,
            album_load_limit,
            search_limit,
            recommendations_limit,
            playlist_page_load_concurrency,
            album_page_load_concurrency,
            track_resolve_concurrency,
        })
    }
    pub async fn get_autocomplete(
        &self,
        query: &str,
        types: &[String],
    ) -> Option<crate::protocol::tracks::SearchResult> {
        search::SpotifySearch::get_autocomplete(
            &self.client,
            &self.token_tracker,
            query,
            types,
            self.search_limit,
            isrc_binary_regex(),
        )
        .await
    }
    pub fn base_request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.6998.178 Spotify/1.2.65.255 Safari/537.36")
    }
}
#[async_trait]
impl SourcePlugin for SpotifySource {
    fn name(&self) -> &str {
        "spotify"
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
        vec!["spsearch:"]
    }
    fn is_mirror(&self) -> bool {
        true
    }
    fn rec_prefixes(&self) -> Vec<&str> {
        vec!["sprec:"]
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
            let query = &identifier[prefix.len()..];
            return match self.get_autocomplete(query, &["track".to_owned()]).await {
                Some(res) => {
                    if res.tracks.is_empty() {
                        LoadResult::Empty {}
                    } else {
                        LoadResult::Search(res.tracks)
                    }
                }
                None => LoadResult::Empty {},
            };
        }
        if let Some(prefix) = self
            .rec_prefixes()
            .into_iter()
            .find(|p| identifier.starts_with(p))
        {
            let query = &identifier[prefix.len()..];
            return match recommendations::SpotifyRecommendations::fetch_recommendations(
                &self.client,
                &self.token_tracker,
                query,
                mix_regex(),
                self.recommendations_limit,
                self.search_limit,
                isrc_binary_regex(),
            )
            .await
            {
                Ok(res) => res,
                Err(playlist_id) => {
                    metadata::SpotifyMetadata::fetch_playlist(
                        &self.client,
                        &self.token_tracker,
                        &playlist_id,
                        self.playlist_load_limit,
                        self.playlist_page_load_concurrency,
                        self.track_resolve_concurrency,
                        isrc_binary_regex(),
                    )
                    .await
                }
            };
        }
        if let Some(caps) = url_regex().captures(identifier) {
            let type_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let id = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            match type_str {
                "track" => {
                    if let Some(track_info) = metadata::SpotifyMetadata::fetch_track(
                        &self.client,
                        &self.token_tracker,
                        id,
                        isrc_binary_regex(),
                    )
                    .await
                    {
                        return LoadResult::Track(Track::new(track_info));
                    }
                }
                "album" => {
                    return metadata::SpotifyMetadata::fetch_album(
                        &self.client,
                        &self.token_tracker,
                        id,
                        self.album_load_limit,
                        self.album_page_load_concurrency,
                        self.track_resolve_concurrency,
                        isrc_binary_regex(),
                    )
                    .await;
                }
                "playlist" => {
                    return metadata::SpotifyMetadata::fetch_playlist(
                        &self.client,
                        &self.token_tracker,
                        id,
                        self.playlist_load_limit,
                        self.playlist_page_load_concurrency,
                        self.track_resolve_concurrency,
                        isrc_binary_regex(),
                    )
                    .await;
                }
                "artist" => {
                    return metadata::SpotifyMetadata::fetch_artist(
                        &self.client,
                        &self.token_tracker,
                        id,
                        isrc_binary_regex(),
                    )
                    .await;
                }
                _ => {}
            }
        }
        LoadResult::Empty {}
    }
    async fn load_search(
        &self,
        query: &str,
        types: &[String],
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<crate::protocol::tracks::SearchResult> {
        let mut q = query;
        for prefix in self.search_prefixes() {
            if let Some(stripped) = q.strip_prefix(prefix) {
                q = stripped;
                break;
            }
        }
        self.get_autocomplete(q, types).await
    }
    async fn get_track(
        &self,
        _identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        None
    }
}