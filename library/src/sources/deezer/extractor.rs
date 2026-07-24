use crate::protocol::tracks::{Track, TrackInfo};
use base64::{Engine as _, engine::general_purpose};
use serde_json::{Value, json};

const DECRYPTION_KEY: &[u8] = b"IFYOUWANTTHEARTISTSTOGETPAIDDONOTDOWNLOADFROMMIXCLOUD";

pub fn decrypt(ciphertext_b64: &str) -> String {
    let ciphertext: Vec<u8> = match general_purpose::STANDARD.decode(ciphertext_b64) {
        Ok(b) => b,
        Err(_) => return String::new(),
    };
    let mut decrypted = Vec::with_capacity(ciphertext.len());
    for (i, &byte) in ciphertext.iter().enumerate() {
        decrypted.push(byte ^ DECRYPTION_KEY[i % DECRYPTION_KEY.len()]);
    }
    String::from_utf8(decrypted).unwrap_or_default()
}

pub fn parse_track(json: &Value) -> Option<Track> {
    let id = json.get("id").and_then(|v| {
        v.as_str()
            .map(|s| s.to_owned())
            .or_else(|| v.as_i64().map(|n| n.to_string()))
    })?;
    let title = json.get("title")?.as_str()?.to_owned();
    let artist = json.get("artist")?.get("name")?.as_str()?.to_owned();
    let duration = json.get("duration")?.as_u64()? * 1000;
    if let Some(readable) = json.get("readable").and_then(|v| v.as_bool()) {
        if !readable {
            let countries = json
                .get("available_countries")
                .and_then(|v| {
                    v.as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|c| c.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .or_else(|| v.as_str().map(|s| s.to_owned()))
                })
                .unwrap_or_default();
            tracing::debug!(
                "Deezer track {} ({}) is marked as not readable. Available countries: {}. It might fail unless a fallback is found.",
                title,
                id,
                countries
            );
        }
    }
    let isrc = json
        .get("isrc")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned());
    let artwork_url = json
        .get("album")
        .and_then(|a| a.get("cover_xl"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .or_else(|| {
            json.get("md5_image").and_then(|v| v.as_str()).map(|id| {
                format!(
                    "https://cdn-images.dzcdn.net/images/cover/{id}/1000x1000-000000-80-0-0.jpg"
                )
            })
        });
    let uri = json
        .get("link")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned());
    let mut track = Track::new(TrackInfo {
        identifier: id,
        is_seekable: true,
        author: artist,
        length: duration,
        is_stream: false,
        position: 0,
        title,
        uri: uri.clone(),
        artwork_url,
        isrc,
        source_name: "deezer".to_owned(),
    });
    let album_name = json.pointer("/album/title").and_then(|v| v.as_str());
    let album_url = json
        .pointer("/album/id")
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_u64().map(|id| id.to_string()))
        })
        .map(|id| format!("https://www.deezer.com/album/{id}"));
    let artist_url = json
        .pointer("/artist/id")
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_u64().map(|id| id.to_string()))
        })
        .map(|id| format!("https://www.deezer.com/artist/{id}"));
    let artist_artwork_url = json.pointer("/artist/picture_xl").and_then(|v| v.as_str());
    let preview_url = json.get("preview").and_then(|v| v.as_str());
    track.plugin_info = json!({
        "albumName": album_name,
        "albumUrl": album_url,
        "artistUrl": artist_url,
        "artistArtworkUrl": artist_artwork_url,
        "previewUrl": preview_url,
        "isPreview": false
    });
    Some(track)
}

pub fn parse_recommendation_track(json: &Value) -> Option<Track> {
    let id = json.get("SNG_ID").and_then(|v| {
        v.as_str()
            .map(|s| s.to_owned())
            .or_else(|| v.as_i64().map(|n| n.to_string()))
    })?;
    let title = json.get("SNG_TITLE")?.as_str()?.to_owned();
    let artist = json.get("ART_NAME")?.as_str()?.to_owned();
    let duration = json.get("DURATION")?.as_u64()? * 1000;
    if let Some(readable) = json.get("READABLE").and_then(|v| v.as_bool()) {
        if !readable {
            tracing::debug!(
                "Deezer recommendation track {} ({}) is marked as not readable. It might fail unless a fallback is found.",
                title,
                id
            );
        }
    }
    let isrc = json
        .get("ISRC")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    let album_pic = json
        .get("ALB_PICTURE")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let artwork_url = if !album_pic.is_empty() {
        Some(format!(
            "https://cdn-images.dzcdn.net/images/cover/{album_pic}/1000x1000-000000-80-0-0.jpg"
        ))
    } else {
        None
    };
    let uri_val = Some(format!("https://deezer.com/track/{id}"));
    let mut track = Track::new(TrackInfo {
        identifier: id.clone(),
        is_seekable: true,
        author: artist,
        length: duration,
        is_stream: false,
        position: 0,
        title,
        uri: uri_val.clone(),
        artwork_url,
        isrc,
        source_name: "deezer".to_owned(),
    });
    let album_name = json.get("ALB_TITLE").and_then(|v| v.as_str());
    let album_url = json
        .get("ALB_ID")
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_u64().map(|id| id.to_string()))
        })
        .map(|id| format!("https://www.deezer.com/album/{id}"));
    let artist_url = json
        .get("ART_ID")
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_owned())
                .or_else(|| v.as_u64().map(|id| id.to_string()))
        })
        .map(|id| format!("https://www.deezer.com/artist/{id}"));
    let artist_artwork_url = json
        .pointer("/ARTISTS/0/ART_PICTURE")
        .and_then(|v| v.as_str())
        .map(|id| {
            format!("https://cdn-images.dzcdn.net/images/cover/{id}/1000x1000-000000-80-0-0.jpg")
        });
    let preview_url = json.pointer("/MEDIA/0/HREF").and_then(|v| v.as_str());
    track.plugin_info = json!({
        "albumName": album_name,
        "albumUrl": album_url,
        "artistUrl": artist_url,
        "artistArtworkUrl": artist_artwork_url,
        "previewUrl": preview_url,
        "isPreview": false
    });
    Some(track)
}
