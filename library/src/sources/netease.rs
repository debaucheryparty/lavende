pub mod api {
use aes::{
    Aes128,
    cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray},
};
use md5::{Digest, Md5};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use tracing::{debug, error};
#[derive(Debug, Deserialize)]
pub struct NeteaseResponse<T> {
    pub code: i64,
    #[serde(flatten)]
    pub data: T,
}
#[derive(Debug, Deserialize)]
pub struct NeteaseArtist {
    pub id: i64,
    pub name: String,
}
#[derive(Debug, Deserialize)]
pub struct NeteaseAlbum {
    pub id: i64,
    pub name: String,
    #[serde(alias = "picUrl")]
    pub pic_url: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct NeteaseSong {
    pub id: i64,
    pub name: String,
    #[serde(alias = "ar", alias = "artists")]
    pub artists: Vec<NeteaseArtist>,
    #[serde(alias = "al", alias = "album")]
    pub album: Option<NeteaseAlbum>,
    #[serde(alias = "dt", alias = "duration")]
    pub duration: u64,
}
#[derive(Debug, Deserialize)]
pub struct SearchResultData {
    pub result: SearchResultInner,
}
#[derive(Debug, Deserialize)]
pub struct SearchResultInner {
    #[serde(default)]
    pub songs: Vec<NeteaseSong>,
    #[serde(default)]
    pub albums: Vec<NeteaseAlbum>,
    #[serde(default)]
    pub artists: Vec<NeteaseArtist>,
    #[serde(default)]
    pub playlists: Vec<NeteasePlaylist>,
}
#[derive(Debug, Deserialize)]
pub struct NeteasePlaylist {
    pub id: i64,
    pub name: String,
    #[serde(alias = "coverImgUrl")]
    pub cover_img_url: Option<String>,
    pub creator: Option<NeteaseCreator>,
    #[serde(alias = "trackCount")]
    pub track_count: Option<u64>,
}
#[derive(Debug, Deserialize)]
pub struct NeteaseCreator {
    pub nickname: String,
}
#[derive(Debug, Deserialize)]
pub struct SongDetailData {
    pub songs: Vec<NeteaseSong>,
}
#[derive(Debug, Deserialize)]
pub struct TrackUrlData {
    pub data: Vec<TrackUrlItem>,
}
#[derive(Debug, Deserialize)]
pub struct TrackUrlItem {
    pub id: i64,
    pub url: Option<String>,
    pub br: i64,
    pub code: i64,
    #[serde(rename = "freeTrialInfo")]
    pub free_trial_info: Option<serde_json::Value>,
}
#[derive(Debug, Deserialize)]
pub struct SimilarSongsData {
    pub songs: Vec<NeteaseSong>,
}
#[derive(Debug, Deserialize)]
pub struct AlbumDetailData {
    pub album: NeteaseAlbum,
    pub songs: Vec<NeteaseSong>,
}
#[derive(Debug, Deserialize)]
pub struct PlaylistDetailData {
    pub playlist: PlaylistInfo,
}
#[derive(Debug, Deserialize)]
pub struct PlaylistInfo {
    pub id: i64,
    pub name: String,
    pub tracks: Vec<NeteaseSong>,
}
#[derive(Debug, Deserialize)]
pub struct ArtistDetailData {
    pub artist: NeteaseArtist,
    #[serde(rename = "hotSongs")]
    pub hot_songs: Vec<NeteaseSong>,
}
const EAPI_KEY: &[u8] = b"e82ckenh8dichen8";
const EAPI_URLS: &[&str] = &[
    "https://interface3.music.163.com/eapi",
    "https://interface.music.163.com/eapi",
];
pub fn aes_encrypt_ecb(data: &[u8], key: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut padded_data = pkcs7_pad(data, 16);
    for chunk in padded_data.chunks_mut(16) {
        let block = GenericArray::from_mut_slice(chunk);
        cipher.encrypt_block(block);
    }
    padded_data
}
fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let padding_len = block_size - (data.len() % block_size);
    let mut padded = data.to_vec();
    padded.extend(std::iter::repeat_n(padding_len as u8, padding_len));
    padded
}
pub fn md5_hex(data: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}
pub fn eapi_encrypt(url: &str, obj: &Value) -> String {
    let text = serde_json::to_string(obj).unwrap_or_default();
    let message = format!("nobody{}use{}md5forencrypt", url, text);
    let digest = md5_hex(&message);
    let data = format!("{}-36cd479b6b5-{}-36cd479b6b5-{}", url, text, digest);
    let encrypted = aes_encrypt_ecb(data.as_bytes(), EAPI_KEY);
    hex::encode(encrypted).to_uppercase()
}
pub async fn get_eapi_json<T: DeserializeOwned>(
    client: &reqwest::Client,
    path: &str,
    obj: Value,
    nuid: &str,
    device_id: &str,
) -> Option<T> {
    let params = eapi_encrypt(path, &obj);
    let network_path = path.strip_prefix("/api").unwrap_or(path);
    for &base_url in EAPI_URLS {
        let url = format!("{}{}", base_url, network_path);
        let resp = client.post(&url)
            .header("User-Agent", "NeteaseMusic/2.5.1 (iPhone; iOS 16.6; Scale/3.00)")
            .header("Referer", "https://music.163.com/")
            .header("Origin", "https://music.163.com")
            .header("X-Real-IP", "118.88.88.88")
            .header("X-Forwarded-For", "118.88.88.88")
            .header("X-Netease-PC-IP", "118.88.88.88")
            .header("Cookie", format!(
                "os=iOS; appver=2.5.1; _ntes_nuid={}; deviceId={}; channel=AppStore; mobilename=iPhone15,3", 
                nuid, device_id
            ))
            .form(&[("params", params.clone())])
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => {
                let text = r.text().await.ok()?;
                match serde_json::from_str::<NeteaseResponse<T>>(&text) {
                    Ok(res) => {
                        if res.code == 200 || res.code == 0 {
                            return Some(res.data);
                        } else {
                            debug!(
                                "Netease API {} returned application code {}: {}",
                                path, res.code, text
                            );
                        }
                    }
                    Err(e) => {
                        error!(
                            "Netease API {} failed to parse JSON: {}. Text: {}",
                            path, e, text
                        );
                    }
                }
            }
            _ => continue,
        }
    }
    None
}
}
pub mod manager {
use serde_json::json;
use super::api::*;
use crate::protocol::tracks::{
    LoadResult, PlaylistData, PlaylistInfo, SearchResult, Track, TrackInfo,
};
pub fn parse_track(song: &NeteaseSong) -> Option<Track> {
    let id = song.id.to_string();
    let artist = song
        .artists
        .first()
        .map(|a| a.name.as_str())
        .unwrap_or("Unknown Artist");
    let artwork_url = song.album.as_ref().and_then(|al| al.pic_url.clone());
    Some(Track::new(TrackInfo {
        identifier: id.clone(),
        is_seekable: true,
        author: artist.to_string(),
        length: song.duration,
        is_stream: false,
        position: 0,
        title: song.name.clone(),
        uri: Some(format!("https://music.163.com/song?id={}", id)),
        artwork_url,
        isrc: None,
        source_name: "netease".to_string(),
    }))
}
pub fn parse_album(album: &NeteaseAlbum) -> PlaylistData {
    PlaylistData {
        info: PlaylistInfo {
            name: album.name.clone(),
            selected_track: -1,
        },
        plugin_info: json!({
            "type": "album",
            "url": format!("https://music.163.com/album?id={}", album.id),
            "artworkUrl": album.pic_url,
            "author": "Netease Music",
            "totalTracks": 0
        }),
        tracks: Vec::new(),
    }
}
pub fn parse_artist(artist: &NeteaseArtist) -> PlaylistData {
    PlaylistData {
        info: PlaylistInfo {
            name: format!("{}'s Top Tracks", artist.name),
            selected_track: -1,
        },
        plugin_info: json!({
            "type": "artist",
            "url": format!("https://music.163.com/artist?id={}", artist.id),
            "artworkUrl": None::<String>,
            "author": artist.name,
            "totalTracks": 0
        }),
        tracks: Vec::new(),
    }
}
pub fn parse_playlist(playlist: &NeteasePlaylist) -> PlaylistData {
    PlaylistData {
        info: PlaylistInfo {
            name: playlist.name.clone(),
            selected_track: -1,
        },
        plugin_info: json!({
            "type": "playlist",
            "url": format!("https://music.163.com/playlist?id={}", playlist.id),
            "artworkUrl": playlist.cover_img_url,
            "author": playlist.creator.as_ref().map(|c| c.nickname.clone()).unwrap_or_else(|| "Netease Music".to_string()),
            "totalTracks": playlist.track_count.unwrap_or(0)
        }),
        tracks: Vec::new(),
    }
}
#[derive(Debug, PartialEq)]
pub enum TrackUrlResult {
    Success(String),
    Code(i64),
    Trial,
    None,
}
pub async fn fetch_track_detail(
    client: &reqwest::Client,
    nuid: &str,
    device_id: &str,
    id: &str,
) -> Option<SongDetailData> {
    get_eapi_json(
        client,
        "/api/v3/song/detail",
        json!({
            "c": serde_json::to_string(&[json!({"id": id})]).unwrap(),
            "header": {
                "os": "iOS",
                "appver": "2.5.1",
                "deviceId": device_id
            }
        }),
        nuid,
        device_id,
    )
    .await
}
pub async fn fetch_album(
    client: &reqwest::Client,
    nuid: &str,
    device_id: &str,
    id: &str,
) -> LoadResult {
    match get_eapi_json::<AlbumDetailData>(
        client,
        &format!("/api/v1/album/{}", id),
        json!({}),
        nuid,
        device_id,
    )
    .await
    {
        Some(resp) if !resp.songs.is_empty() => {
            let tracks = resp.songs.iter().filter_map(parse_track).collect();
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: resp.album.name,
                    selected_track: -1,
                },
                plugin_info: json!({ "type": "album", "id": id }),
                tracks,
            })
        }
        _ => LoadResult::Empty {},
    }
}
pub async fn fetch_playlist(
    client: &reqwest::Client,
    nuid: &str,
    device_id: &str,
    id: &str,
) -> LoadResult {
    match get_eapi_json::<PlaylistDetailData>(
        client,
        "/api/v6/playlist/detail",
        json!({ "id": id, "n": 1000 }),
        nuid,
        device_id,
    )
    .await
    {
        Some(resp) if !resp.playlist.tracks.is_empty() => {
            let tracks = resp
                .playlist
                .tracks
                .iter()
                .filter_map(parse_track)
                .collect();
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: resp.playlist.name,
                    selected_track: -1,
                },
                plugin_info: json!({ "type": "playlist", "id": id }),
                tracks,
            })
        }
        _ => LoadResult::Empty {},
    }
}
pub async fn fetch_artist(
    client: &reqwest::Client,
    nuid: &str,
    device_id: &str,
    id: &str,
) -> LoadResult {
    match get_eapi_json::<ArtistDetailData>(
        client,
        &format!("/api/v1/artist/{}", id),
        json!({}),
        nuid,
        device_id,
    )
    .await
    {
        Some(resp) if !resp.hot_songs.is_empty() => {
            let tracks = resp.hot_songs.iter().filter_map(parse_track).collect();
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: format!("{}'s Top Tracks", resp.artist.name),
                    selected_track: -1,
                },
                plugin_info: json!({ "type": "artist", "id": id }),
                tracks,
            })
        }
        _ => LoadResult::Empty {},
    }
}
pub async fn fetch_track_url(
    client: &reqwest::Client,
    nuid: &str,
    device_id: &str,
    id: &str,
    level: &str,
    encode_type: &str,
) -> TrackUrlResult {
    let resp: Option<TrackUrlData> = get_eapi_json(
        client,
        "/api/song/enhance/player/url/v1",
        json!({
            "ids": [id],
            "level": level,
            "encodeType": encode_type,
            "header": {
                "os": "iOS",
                "appver": "2.5.1",
                "deviceId": device_id
            }
        }),
        nuid,
        device_id,
    )
    .await;
    let data_wrap = match resp {
        Some(v) => v,
        None => return TrackUrlResult::None,
    };
    let data = match data_wrap.data.first() {
        Some(d) => d,
        None => return TrackUrlResult::None,
    };
    if data.code == -110 {
        return TrackUrlResult::Code(-110);
    }
    if let Some(trial_info) = &data.free_trial_info
        && trial_info.as_object().is_some_and(|o| !o.is_empty())
    {
        return TrackUrlResult::Trial;
    }
    if let Some(url) = &data.url
        && !url.is_empty()
    {
        return TrackUrlResult::Success(url.clone());
    }
    if data.code != 200 && data.code != 0 {
        TrackUrlResult::Code(data.code)
    } else {
        TrackUrlResult::None
    }
}
pub async fn fetch_track_url_legacy(
    client: &reqwest::Client,
    device_id: &str,
    id: &str,
    br: &str,
) -> Option<String> {
    let url = format!(
        "https://music.163.com/api/song/enhance/player/url?id={}&ids=[{}]&br={}",
        id, id, br
    );
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36")
        .header("Referer", "https://music.163.com/")
        .header("X-Real-IP", "118.88.88.88")
        .header("Cookie", format!("os=pc; deviceId={}", device_id))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let res: NeteaseResponse<TrackUrlData> = resp.json().await.ok()?;
    let data = res.data.data.first()?;
    if let Some(url) = &data.url
        && !url.is_empty()
    {
        return Some(url.clone());
    }
    None
}
pub async fn check_url(client: &reqwest::Client, url: &str) -> bool {
    let resp = match client.head(url)
        .header("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 CloudMusic/2.5.1")
        .header("Referer", "https://music.163.com/")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return false,
    };
    if !resp.status().is_success() {
        return false;
    }
    let content_length = resp
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    content_length >= 100 * 1024
}
pub async fn search_tracks(
    client: &reqwest::Client,
    nuid: &str,
    device_id: &str,
    query: &str,
    limit: usize,
) -> LoadResult {
    match get_eapi_json::<SearchResultData>(
        client,
        "/api/cloudsearch/pc",
        json!({
            "s": query,
            "type": 1,
            "limit": limit,
            "offset": 0,
            "total": true
        }),
        nuid,
        device_id,
    )
    .await
    {
        Some(resp) if !resp.result.songs.is_empty() => {
            let tracks = resp.result.songs.iter().filter_map(parse_track).collect();
            LoadResult::Search(tracks)
        }
        _ => LoadResult::Empty {},
    }
}
pub async fn search_full(
    client: &reqwest::Client,
    nuid: &str,
    device_id: &str,
    query: &str,
    types: &[String],
    limit: usize,
) -> Option<SearchResult> {
    let mut tracks = Vec::new();
    let mut albums = Vec::new();
    let mut artists = Vec::new();
    let mut playlists = Vec::new();
    let all_types = types.is_empty();
    if (all_types || types.contains(&"track".to_owned()))
        && let Some(res) = fetch_search(client, nuid, device_id, query, 1, limit).await
    {
        tracks = res.songs.iter().filter_map(parse_track).collect();
    }
    if (all_types || types.contains(&"album".to_owned()))
        && let Some(res) = fetch_search(client, nuid, device_id, query, 10, limit).await
    {
        albums = res.albums.iter().map(parse_album).collect();
    }
    if (all_types || types.contains(&"artist".to_owned()))
        && let Some(res) = fetch_search(client, nuid, device_id, query, 100, limit).await
    {
        artists = res.artists.iter().map(parse_artist).collect();
    }
    if (all_types || types.contains(&"playlist".to_owned()))
        && let Some(res) = fetch_search(client, nuid, device_id, query, 1000, limit).await
    {
        playlists = res.playlists.iter().map(parse_playlist).collect();
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
async fn fetch_search(
    client: &reqwest::Client,
    nuid: &str,
    device_id: &str,
    query: &str,
    type_id: i32,
    limit: usize,
) -> Option<SearchResultInner> {
    get_eapi_json::<SearchResultData>(
        client,
        "/api/cloudsearch/pc",
        json!({
            "s": query,
            "type": type_id,
            "limit": limit,
            "offset": 0,
            "total": true
        }),
        nuid,
        device_id,
    )
    .await
    .map(|d| d.result)
}
pub async fn fetch_recommendations(
    client: &reqwest::Client,
    nuid: &str,
    device_id: &str,
    identifier: &str,
) -> LoadResult {
    let id = identifier.trim();
    if !id.chars().all(|c| c.is_ascii_digit()) || id.is_empty() {
        return LoadResult::Empty {};
    }
    match get_eapi_json::<SimilarSongsData>(
        client,
        "/api/v1/discovery/simiSong",
        json!({ "songid": id }),
        nuid,
        device_id,
    )
    .await
    {
        Some(resp) if !resp.songs.is_empty() => {
            let tracks = resp.songs.iter().filter_map(parse_track).collect();
            LoadResult::Playlist(PlaylistData {
                info: PlaylistInfo {
                    name: format!("Similar to Track {}", id),
                    selected_track: -1,
                },
                plugin_info: json!({ "type": "recommendations", "seed_id": id }),
                tracks,
            })
        }
        _ => LoadResult::Empty {},
    }
}
}
pub mod track {
use std::net::IpAddr;
use async_trait::async_trait;
use tracing::debug;
use crate::{
    config::HttpProxyConfig,
    sources::{
        http::HttpTrack,
        playable_track::{PlayableTrack, ResolvedTrack},
    },
};
pub struct NeteaseTrack {
    pub stream_url: String,
    pub local_addr: Option<IpAddr>,
    pub proxy: Option<HttpProxyConfig>,
}
#[async_trait]
impl PlayableTrack for NeteaseTrack {
    async fn resolve(&self) -> Result<ResolvedTrack, String> {
        let url = self.stream_url.clone();
        debug!("Netease playback URL: {url}");
        HttpTrack {
            url,
            local_addr: self.local_addr,
            proxy: None,
        }
        .resolve()
        .await
    }
}
}
use std::sync::{Arc, OnceLock};
use async_trait::async_trait;
use rand::Rng;
use regex::Regex;
use tracing::debug;
use crate::{
    protocol::tracks::{LoadResult, SearchResult},
    sources::{SourcePlugin, playable_track::BoxedTrack},
};
fn url_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"https?://music\.163\.com/(?:(?:#|m)/)?(?P<type>song|album|playlist|artist)(?:\?id=|\/)(?P<id>\d+)").unwrap()
    })
}
pub struct NeteaseSource {
    pub(crate) client: Arc<reqwest::Client>,
    pub(crate) proxy: Option<crate::config::HttpProxyConfig>,
    pub(crate) search_limit: usize,
    pub(crate) nuid: String,
    pub(crate) device_id: String,
}
impl NeteaseSource {
    pub fn new(
        config: Option<crate::config::NeteaseMusicConfig>,
        client: Arc<reqwest::Client>,
    ) -> Result<Self, String> {
        let cfg = config.ok_or("Netease Music configuration is missing")?;
        let mut rng = rand::thread_rng();
        let nuid: String = (0..16)
            .map(|_| format!("{:02x}", rng.r#gen::<u8>()))
            .collect::<Vec<String>>()
            .join("");
        let device_id: String = (0..8)
            .map(|_| format!("{:02X}", rng.r#gen::<u8>()))
            .collect::<Vec<String>>()
            .join("");
        Ok(Self {
            client,
            proxy: cfg.proxy,
            search_limit: cfg.search_limit,
            nuid,
            device_id,
        })
    }
}
#[async_trait]
impl SourcePlugin for NeteaseSource {
    fn name(&self) -> &str {
        "netease"
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
        vec!["nmsearch:", "ncsearch:"]
    }
    fn rec_prefixes(&self) -> Vec<&str> {
        vec!["nmrec:", "ncrec:"]
    }
    async fn load(
        &self,
        identifier: &str,
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> LoadResult {
        for prefix in self.search_prefixes() {
            if let Some(query) = identifier.strip_prefix(prefix) {
                return manager::search_tracks(
                    &self.client,
                    &self.nuid,
                    &self.device_id,
                    query,
                    self.search_limit,
                )
                .await;
            }
        }
        for prefix in self.rec_prefixes() {
            if let Some(query) = identifier.strip_prefix(prefix) {
                return manager::fetch_recommendations(
                    &self.client,
                    &self.nuid,
                    &self.device_id,
                    query,
                )
                .await;
            }
        }
        if let Some(caps) = url_regex().captures(identifier) {
            let type_ = caps.name("type").map(|m| m.as_str()).unwrap_or("");
            let id = caps.name("id").map(|m| m.as_str()).unwrap_or("");
            match type_ {
                "song" => {
                    if let Some(detail) =
                        manager::fetch_track_detail(&self.client, &self.nuid, &self.device_id, id)
                            .await
                        && let Some(song) = detail.songs.first()
                        && let Some(track) = manager::parse_track(song)
                    {
                        return LoadResult::Track(track);
                    }
                }
                "album" => {
                    return manager::fetch_album(&self.client, &self.nuid, &self.device_id, id)
                        .await;
                }
                "playlist" => {
                    return manager::fetch_playlist(&self.client, &self.nuid, &self.device_id, id)
                        .await;
                }
                "artist" => {
                    return manager::fetch_artist(&self.client, &self.nuid, &self.device_id, id)
                        .await;
                }
                _ => {}
            }
            return LoadResult::Empty {};
        }
        if identifier.chars().all(|c| c.is_ascii_digit())
            && !identifier.is_empty()
            && let Some(detail) =
                manager::fetch_track_detail(&self.client, &self.nuid, &self.device_id, identifier)
                    .await
            && let Some(song) = detail.songs.first()
            && let Some(track) = manager::parse_track(song)
        {
            return LoadResult::Track(track);
        }
        manager::search_tracks(
            &self.client,
            &self.nuid,
            &self.device_id,
            identifier,
            self.search_limit,
        )
        .await
    }
    async fn load_search(
        &self,
        query: &str,
        _types: &[String],
        _routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<SearchResult> {
        let mut q = query;
        for prefix in self.search_prefixes() {
            if let Some(stripped) = query.strip_prefix(prefix) {
                q = stripped;
                break;
            }
        }
        manager::search_full(
            &self.client,
            &self.nuid,
            &self.device_id,
            q,
            _types,
            self.search_limit,
        )
        .await
    }
    async fn get_track(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<BoxedTrack> {
        let id = url_regex()
            .captures(identifier)
            .and_then(|caps| caps.name("id"))
            .map(|m| m.as_str())
            .unwrap_or(identifier);
        debug!("Netease: Resolving track ID: {}", id);
        let mut stream_url = None;
        let mut fallback_early = false;
        let qualities = [
            ("aac", "standard"),
            ("aac", "higher"),
            ("aac", "exhigh"),
            ("aac", "lossless"),
            ("aac", "hires"),
            ("aac", "jymaster"),
            ("aac", "sky"),
            ("aac", "jyeffect"),
            ("aac", "jylive"),
            ("mp3", "standard"),
            ("mp3", "higher"),
            ("mp3", "exhigh"),
            ("mp3", "lossless"),
            ("mp3", "hires"),
            ("mp3", "jymaster"),
            ("mp3", "sky"),
            ("mp3", "jyeffect"),
            ("mp3", "jylive"),
        ];
        let mut first_code: Option<i64> = None;
        for (format, level) in qualities {
            match manager::fetch_track_url(
                &self.client,
                &self.nuid,
                &self.device_id,
                id,
                level,
                format,
            )
            .await
            {
                manager::TrackUrlResult::Success(url)
                    if manager::check_url(&self.client, &url).await =>
                {
                    stream_url = Some(url);
                    break;
                }
                manager::TrackUrlResult::Code(-110) => {
                    fallback_early = true;
                    break;
                }
                manager::TrackUrlResult::Trial => {
                    debug!("Netease: Track {} is trial-only, skipping quality loop", id);
                    fallback_early = true;
                    break;
                }
                manager::TrackUrlResult::Code(c) => {
                    first_code = first_code.or(Some(c));
                    continue;
                }
                _ => continue,
            }
        }
        if stream_url.is_none() || fallback_early {
            for br in ["320000", "128000"] {
                if let Some(url) =
                    manager::fetch_track_url_legacy(&self.client, &self.device_id, id, br).await
                    && !url.is_empty()
                    && manager::check_url(&self.client, &url).await
                {
                    stream_url = Some(url);
                    break;
                }
            }
        }
        if stream_url.is_none() {
            debug!(
                "Netease: Failed to resolve playback URL for track ID: {}",
                id
            );
        }
        stream_url.map(|url| {
            Arc::new(track::NeteaseTrack {
                stream_url: url,
                proxy: self.proxy.clone(),
                local_addr: routeplanner.and_then(|rp| rp.get_address()),
            }) as BoxedTrack
        })
    }
    fn get_proxy_config(&self) -> Option<crate::config::HttpProxyConfig> {
        self.proxy.clone()
    }
}