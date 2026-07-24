use crate::protocol::tracks::{Track, TrackInfo};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct JioSaavnArtistDto {
    pub name: Option<String>,
    pub perma_url: Option<String>,
    pub image: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct JioSaavnArtistMapDto {
    pub primary_artists: Option<Vec<JioSaavnArtistDto>>,
    pub artists: Option<Vec<JioSaavnArtistDto>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct JioSaavnMoreInfoDto {
    pub duration: Option<serde_json::Value>,
    #[serde(rename = "artistMap")]
    pub artist_map: Option<JioSaavnArtistMapDto>,
    pub album: Option<String>,
    pub album_url: Option<String>,
    pub media_preview_url: Option<String>,
    pub vlink: Option<String>,
    pub primary_artists: Option<String>,
    pub singers: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct JioSaavnTrackDto {
    pub id: serde_json::Value,
    pub title: Option<String>,
    pub song: Option<String>,
    pub duration: Option<serde_json::Value>,
    pub image: Option<String>,
    pub perma_url: Option<String>,
    pub more_info: Option<JioSaavnMoreInfoDto>,
    pub subtitle: Option<String>,
    pub header_desc: Option<String>,
    pub album: Option<String>,
    pub album_url: Option<String>,
    pub media_preview_url: Option<String>,
    pub vlink: Option<String>,
}

pub fn clean_string(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&amp;", "&")
}

pub fn parse_track(dto: &JioSaavnTrackDto) -> Option<Track> {
    let id = match &dto.id {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => return None,
    };

    let title = clean_string(dto.title.as_deref().or(dto.song.as_deref())?);

    let duration = if let Some(more) = &dto.more_info {
        if let Some(dur_val) = &more.duration {
            match dur_val {
                serde_json::Value::Number(n) => n.as_u64().unwrap_or(0),
                serde_json::Value::String(s) => s.parse::<u64>().unwrap_or(0),
                _ => 0,
            }
        } else {
            0
        }
    } else if let Some(dur_val) = &dto.duration {
        match dur_val {
            serde_json::Value::Number(n) => n.as_u64().unwrap_or(0),
            serde_json::Value::String(s) => s.parse::<u64>().unwrap_or(0),
            _ => 0,
        }
    } else {
        0
    };

    let artwork_url = dto
        .image
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|s| s.replace("150x150", "500x500").replace("50x50", "500x500"));

    let mut artist_names = Vec::new();
    if let Some(more) = &dto.more_info {
        if let Some(artist_map) = &more.artist_map {
            let primary = artist_map
                .primary_artists
                .as_ref()
                .or(artist_map.artists.as_ref());
            if let Some(arr) = primary {
                for a in arr {
                    if let Some(name) = &a.name {
                        artist_names.push(name.clone());
                    }
                }
            }
        }
    }

    let artists_str = if !artist_names.is_empty() {
        artist_names
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    } else if let Some(more) = &dto.more_info {
        let raw = more
            .primary_artists
            .as_ref()
            .or(more.singers.as_ref())
            .or(dto.subtitle.as_ref())
            .or(dto.header_desc.as_ref());
        match raw {
            Some(s) => s
                .split(',')
                .map(|part| part.trim())
                .take(3)
                .collect::<Vec<_>>()
                .join(", "),
            None => "Unknown Artist".to_owned(),
        }
    } else {
        let raw = dto.subtitle.as_ref().or(dto.header_desc.as_ref());
        match raw {
            Some(s) => s
                .split(',')
                .map(|part| part.trim())
                .take(3)
                .collect::<Vec<_>>()
                .join(", "),
            None => "Unknown Artist".to_owned(),
        }
    };

    let author = clean_string(&artists_str);

    let album_name = dto
        .album
        .as_ref()
        .or_else(|| dto.more_info.as_ref().and_then(|m| m.album.as_ref()))
        .cloned();

    let album_url = dto
        .album_url
        .as_ref()
        .or_else(|| dto.more_info.as_ref().and_then(|m| m.album_url.as_ref()))
        .cloned();

    let primary_artist_url = dto
        .more_info
        .as_ref()
        .and_then(|m| m.artist_map.as_ref())
        .and_then(|am| am.primary_artists.as_ref())
        .and_then(|pa| pa.first())
        .and_then(|a| a.perma_url.clone());

    let primary_artist_img = dto
        .more_info
        .as_ref()
        .and_then(|m| m.artist_map.as_ref())
        .and_then(|am| am.primary_artists.as_ref())
        .and_then(|pa| pa.first())
        .and_then(|a| a.image.clone())
        .map(|s| s.replace("150x150", "500x500").replace("50x50", "500x500"));

    let preview_url = dto
        .media_preview_url
        .as_ref()
        .or(dto.vlink.as_ref())
        .or_else(|| {
            dto.more_info
                .as_ref()
                .and_then(|m| m.media_preview_url.as_ref())
        })
        .or_else(|| dto.more_info.as_ref().and_then(|m| m.vlink.as_ref()))
        .cloned();

    let mut track = Track::new(TrackInfo {
        title,
        author,
        length: duration * 1000,
        identifier: id,
        source_name: "jiosaavn".to_owned(),
        uri: dto.perma_url.clone().filter(|s| !s.is_empty()),
        artwork_url,
        is_stream: false,
        is_seekable: true,
        ..Default::default()
    });

    track.plugin_info = serde_json::json!({
        "albumName": album_name,
        "albumUrl": album_url,
        "artistUrl": primary_artist_url,
        "artistArtworkUrl": primary_artist_img,
        "previewUrl": preview_url,
        "isPreview": false
    });

    Some(track)
}
