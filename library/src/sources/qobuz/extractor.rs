use super::track::QobuzTrack;
use crate::protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo, Track, TrackInfo};
use serde_json::{Value, json};
use tracing::error;

const API_URL: &str = "https://www.qobuz.com/api.json/0.2/";

impl super::QobuzSource {
    pub fn parse_qobuz_track(&self, json: &Value) -> QobuzTrack {
        let identifier = json["id"]
            .as_i64()
            .or_else(|| json["id"].as_str().and_then(|s| s.parse::<i64>().ok()))
            .unwrap_or(0)
            .to_string();
        let title = json["title"].as_str().unwrap_or("Unknown Title").to_owned();
        let (author, artist_url) = if !json["artist"].is_null() && json["artist"].is_object() {
            let name = json["artist"]["name"]["display"]
                .as_str()
                .or_else(|| json["artist"]["name"].as_str())
                .unwrap_or("Unknown Artist")
                .to_owned();
            let url = json["artist"]["id"]
                .as_i64()
                .map(|id| format!("https://open.qobuz.com/artist/{id}"));
            (name, url)
        } else {
            let name = json["album"]["artist"]["name"]
                .as_str()
                .unwrap_or("Unknown Artist")
                .to_owned();
            let url = json["album"]["artist"]["id"]
                .as_i64()
                .map(|id| format!("https://open.qobuz.com/artist/{id}"));
            (name, url)
        };
        let length = json["duration"].as_i64().unwrap_or(0) * 1000;
        let artwork_url = json["album"]["image"]["large"]
            .as_str()
            .map(|s| s.to_owned());
        let isrc = json["isrc"].as_str().map(|s| s.to_owned());
        let uri = format!("https://open.qobuz.com/track/{identifier}");
        let album_name = json["album"]["title"].as_str().map(|s| s.to_owned());
        let album_url = json["album"]["id"]
            .as_i64()
            .map(|id| format!("https://open.qobuz.com/album/{id}"));
        let artist_artwork_url =
            if !json["album"]["artist"].is_null() && !json["album"]["artist"]["image"].is_null() {
                json["album"]["artist"]["image"]
                    .as_str()
                    .map(|s| s.to_owned())
            } else {
                None
            };
        QobuzTrack {
            info: TrackInfo {
                identifier,
                is_seekable: true,
                author,
                length: length as u64,
                is_stream: false,
                position: 0,
                title,
                uri: Some(uri),
                artwork_url,
                isrc,
                source_name: "qobuz".to_owned(),
            },
            album_name,
            album_url,
            artist_url,
            artist_artwork_url,
            token_tracker: self.token_tracker.clone(),
            client: self.client.clone(),
        }
    }

    pub async fn handle_search(&self, query: &str) -> LoadResult {
        match self
            .api_request(
                "catalog/search",
                vec![
                    ("query", query.to_owned()),
                    ("limit", self.search_limit.to_string()),
                    ("type", "tracks".to_owned()),
                ],
            )
            .await
        {
            Ok(json) => {
                let items = json["tracks"]["items"].as_array();
                if items.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
                    return LoadResult::Empty {};
                }
                let tracks: Vec<Track> = items
                    .unwrap()
                    .iter()
                    .map(|item| Track::new(self.parse_qobuz_track(item).info))
                    .collect();
                LoadResult::Search(tracks)
            }
            Err(e) => {
                error!("Qobuz search error: {e}");
                LoadResult::Empty {}
            }
        }
    }

    pub async fn handle_isrc(&self, isrc: &str) -> LoadResult {
        match self
            .api_request(
                "catalog/search",
                vec![
                    ("query", isrc.to_owned()),
                    ("limit", "15".to_owned()),
                    ("type", "tracks".to_owned()),
                ],
            )
            .await
        {
            Ok(json) => {
                let items = json["tracks"]["items"].as_array();
                if items.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
                    return LoadResult::Empty {};
                }
                let track = Track::new(self.parse_qobuz_track(&items.unwrap()[0]).info);
                LoadResult::Track(track)
            }
            Err(e) => {
                error!("Qobuz ISRC search error: {e}");
                LoadResult::Empty {}
            }
        }
    }

    pub async fn handle_recommendations(&self, track_id: &str) -> LoadResult {
        let track_json = match self
            .api_request("track/get", vec![("track_id", track_id.to_owned())])
            .await
        {
            Ok(j) => j,
            Err(_) => return LoadResult::Empty {},
        };
        let artist_id = track_json["performer"]["id"]
            .as_i64()
            .or_else(|| track_json["artist"]["id"].as_i64())
            .unwrap_or(0);
        let track_id_i64 = track_id.parse::<i64>().unwrap_or(0);
        let payload = json!({
            "limit": 50,
            "listened_tracks_ids": [track_id_i64],
            "track_to_analysed": [
                {
                    "track_id": track_id_i64,
                    "artist_id": artist_id
                }
            ]
        });
        let tokens = match self.token_tracker.get_tokens().await {
            Some(t) => t,
            None => return LoadResult::Empty {},
        };
        let mut request = self
            .base_request(self.client.post(format!("{API_URL}dynamic/suggest")))
            .header("Accept", "application/json")
            .header("x-app-id", &tokens.app_id)
            .json(&payload);
        if let Some(user_token) = &tokens.user_token {
            request = request.header("x-user-auth-token", user_token);
        }
        let resp = match request.send().await {
            Ok(r) => r,
            Err(e) => {
                error!("Qobuz recommendations request error: {e}");
                return LoadResult::Empty {};
            }
        };
        if !resp.status().is_success() {
            return LoadResult::Empty {};
        }
        let json: Value = resp.json().await.unwrap_or(json!({}));
        let items = json["tracks"]["items"].as_array();
        if items.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
            return LoadResult::Empty {};
        }
        let tracks: Vec<Track> = items
            .unwrap()
            .iter()
            .map(|item| Track::new(self.parse_qobuz_track(item).info))
            .collect();
        LoadResult::Playlist(PlaylistData {
            info: PlaylistInfo {
                name: "Qobuz Recommendations".to_owned(),
                selected_track: -1,
            },
            plugin_info: json!({
                "type": "recommendations",
                "totalTracks": tracks.len()
            }),
            tracks,
        })
    }

    pub async fn handle_album(&self, id: &str) -> LoadResult {
        match self
            .api_request(
                "album/get",
                vec![
                    ("album_id", id.to_owned()),
                    ("limit", self.album_load_limit.to_string()),
                    ("offset", "0".to_owned()),
                ],
            )
            .await
        {
            Ok(mut json) => {
                let title = json["title"].as_str().unwrap_or("Unknown Album").to_owned();
                let author = json["artist"]["name"]
                    .as_str()
                    .or_else(|| json["artist"]["name"]["display"].as_str())
                    .unwrap_or("Unknown Artist")
                    .to_owned();
                let artwork_url = json["image"]["large"].as_str().map(|s| s.to_owned());
                let uri = format!("https://open.qobuz.com/album/{id}");
                let tracks_json = json["tracks"]["items"].take();
                if tracks_json
                    .as_array()
                    .as_ref()
                    .map(|a| a.is_empty())
                    .unwrap_or(true)
                {
                    return LoadResult::Empty {};
                }
                let tracks: Vec<Track> = tracks_json
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| {
                        let mut item = item.clone();
                        item["album"] = json.clone();
                        Track::new(self.parse_qobuz_track(&item).info)
                    })
                    .collect();
                let track_count = tracks.len();
                LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name: title,
                        selected_track: -1,
                    },
                    plugin_info: json!({
                        "type": "album",
                        "url": uri,
                        "artworkUrl": artwork_url,
                        "author": author,
                        "totalTracks": track_count
                    }),
                    tracks,
                })
            }
            Err(_) => LoadResult::Empty {},
        }
    }

    pub async fn handle_playlist(&self, id: &str) -> LoadResult {
        match self
            .api_request(
                "playlist/get",
                vec![
                    ("playlist_id", id.to_owned()),
                    ("extra", "tracks".to_owned()),
                    ("limit", self.playlist_load_limit.to_string()),
                    ("offset", "0".to_owned()),
                ],
            )
            .await
        {
            Ok(json) => {
                let items = json["tracks"]["items"].as_array();
                if items.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
                    return LoadResult::Empty {};
                }
                let name = json["name"]
                    .as_str()
                    .unwrap_or("Unknown Playlist")
                    .to_owned();
                let author = json["owner"]["name"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_owned();
                let artwork_url = json["images300"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned());
                let url = json["url"]
                    .as_str()
                    .map(|s| s.to_owned())
                    .unwrap_or_else(|| format!("https://open.qobuz.com/playlist/{id}"));
                let tracks: Vec<Track> = items
                    .unwrap()
                    .iter()
                    .map(|item| Track::new(self.parse_qobuz_track(item).info))
                    .collect();
                let track_count = tracks.len();
                LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name,
                        selected_track: -1,
                    },
                    plugin_info: json!({
                        "type": "playlist",
                        "url": url,
                        "artworkUrl": artwork_url,
                        "author": author,
                        "totalTracks": track_count
                    }),
                    tracks,
                })
            }
            Err(_) => LoadResult::Empty {},
        }
    }

    pub async fn handle_artist(&self, id: &str) -> LoadResult {
        match self
            .api_request("artist/page", vec![("artist_id", id.to_owned())])
            .await
        {
            Ok(json) => {
                let top_tracks = json["top_tracks"].as_array();
                if top_tracks.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
                    return LoadResult::Empty {};
                }
                let name = json["name"]["display"]
                    .as_str()
                    .or_else(|| json["name"].as_str())
                    .unwrap_or("Unknown Artist")
                    .to_owned();
                let artwork_url = json["images"]["potrait"]["hash"]
                    .as_str()
                    .filter(|h| !h.is_empty())
                    .map(|h| {
                        format!("https://static.qobuz.com/images/artists/covers/large/{h}.jpg")
                    });
                let uri = format!("https://open.qobuz.com/artist/{id}");
                let tracks: Vec<Track> = top_tracks
                    .unwrap()
                    .iter()
                    .take(self.artist_load_limit)
                    .map(|item| Track::new(self.parse_qobuz_track(item).info))
                    .collect();
                let track_count = tracks.len();
                LoadResult::Playlist(PlaylistData {
                    info: PlaylistInfo {
                        name: format!("{name}'s Top Tracks"),
                        selected_track: -1,
                    },
                    plugin_info: json!({
                        "type": "artist",
                        "url": uri,
                        "artworkUrl": artwork_url,
                        "author": name,
                        "totalTracks": track_count
                    }),
                    tracks,
                })
            }
            Err(_) => LoadResult::Empty {},
        }
    }
}
