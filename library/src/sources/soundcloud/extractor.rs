use crate::protocol::tracks::{Track, TrackInfo};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundCloudStreamKind {
    ProgressiveMp3,
    ProgressiveAac,
    HlsOpus,
    HlsMp3,
    HlsAac,
}

#[derive(Deserialize, Debug)]
pub struct UserDto {
    pub username: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PublisherMetadataDto {
    pub isrc: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FormatDto {
    pub protocol: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TranscodingDto {
    pub url: Option<String>,
    pub snipped: Option<bool>,
    pub format: Option<FormatDto>,
}

#[derive(Deserialize, Debug)]
pub struct FormatWrapperDto {
    pub protocol: String,
    pub mime_type: String,
}

#[derive(Deserialize, Debug)]
pub struct MediaDto {
    pub transcodings: Option<Vec<TranscodingDto>>,
}

#[derive(Deserialize, Debug)]
pub struct TrackDto {
    pub id: serde_json::Value,
    pub title: Option<String>,
    pub permalink_url: Option<String>,
    pub artwork_url: Option<String>,
    pub full_duration: Option<u64>,
    pub duration: Option<u64>,
    pub user: Option<UserDto>,
    pub publisher_metadata: Option<PublisherMetadataDto>,
    pub media: Option<MediaDto>,
    pub monetization_model: Option<String>,
    pub policy: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct SearchResponseDto {
    pub collection: Option<Vec<TrackDto>>,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistDto {
    pub title: Option<String>,
    pub kind: Option<String>,
    pub tracks: Option<Vec<serde_json::Value>>,
    pub artwork_url: Option<String>,
    pub user: Option<UserDto>,
    pub track_count: Option<u64>,
}

#[derive(Deserialize, Debug)]
pub struct LikedItemDto {
    pub track: Option<TrackDto>,
}

#[derive(Deserialize, Debug)]
pub struct LikedResponseDto {
    pub collection: Option<Vec<LikedItemDto>>,
}

#[derive(Deserialize, Debug)]
pub struct UserResponseDto {
    pub id: Option<u64>,
    pub username: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CollectionItemDto {
    pub track: Option<TrackDto>,
    pub playlist: Option<serde_json::Value>,
    pub kind: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CollectionResponseDto {
    pub collection: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize, Debug)]
pub struct ResolveResponseDto {
    pub url: Option<String>,
}

pub fn parse_track(dto: &TrackDto) -> Result<Track, String> {
    let id = match &dto.id {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => return Err("Invalid ID type".to_owned()),
    };
    let title = dto.title.as_deref().unwrap_or("Unknown").to_owned();
    let author = dto
        .user
        .as_ref()
        .and_then(|u| u.username.clone())
        .unwrap_or_else(|| "Unknown".to_owned());
    let duration = dto.full_duration.or(dto.duration).unwrap_or(0);
    let uri = dto.permalink_url.clone();
    let artwork_url = dto
        .artwork_url
        .as_ref()
        .map(|s| s.replace("-large", "-t500x500"));
    let isrc = dto.publisher_metadata.as_ref().and_then(|m| m.isrc.clone());

    Ok(Track::new(TrackInfo {
        identifier: id,
        is_seekable: true,
        author,
        length: duration,
        is_stream: false,
        position: 0,
        title,
        uri,
        artwork_url,
        isrc,
        source_name: "soundcloud".to_owned(),
    }))
}

pub fn select_format(transcodings: &[TranscodingDto]) -> Option<(SoundCloudStreamKind, String)> {
    if transcodings.is_empty() {
        return None;
    }
    let find_transcoding = |protocol: &str, mime_contains: &str| -> Option<TranscodingDto> {
        transcodings
            .iter()
            .find(|t| {
                let fmt = match &t.format {
                    Some(f) => f,
                    None => return false,
                };
                let proto = fmt.protocol.as_deref().unwrap_or("");
                let mime = fmt.mime_type.as_deref().unwrap_or("");
                let snipped = t.snipped.unwrap_or(false);
                let url = t.url.as_deref().unwrap_or("");
                !snipped
                    && !url.contains("/preview/")
                    && !url.contains("cf-preview-media.sndcdn.com")
                    && proto == protocol
                    && mime.contains(mime_contains)
            })
            .cloned()
    };

    let selected = find_transcoding("progressive", "mpeg")
        .or_else(|| find_transcoding("progressive", "aac"))
        .or_else(|| find_transcoding("hls", "mpeg"))
        .or_else(|| find_transcoding("hls", "aac"))
        .or_else(|| find_transcoding("hls", "mp4"))
        .or_else(|| find_transcoding("hls", "m4a"))
        .or_else(|| find_transcoding("hls", "ogg"))
        .or_else(|| {
            transcodings
                .iter()
                .find(|t| {
                    t.format.as_ref().and_then(|f| f.protocol.as_deref()) == Some("progressive")
                })
                .cloned()
        })
        .or_else(|| {
            transcodings
                .iter()
                .find(|t| t.format.as_ref().and_then(|f| f.protocol.as_deref()) == Some("hls"))
                .cloned()
        })
        .or_else(|| transcodings.first().cloned())?;

    let lookup_url = selected.url?;
    let proto = selected
        .format
        .as_ref()
        .and_then(|f| f.protocol.as_deref())
        .unwrap_or("");
    let mime = selected
        .format
        .as_ref()
        .and_then(|f| f.mime_type.as_deref())
        .unwrap_or("");

    let kind = if proto == "progressive" {
        if mime.contains("aac") {
            SoundCloudStreamKind::ProgressiveAac
        } else {
            SoundCloudStreamKind::ProgressiveMp3
        }
    } else if mime.contains("ogg") {
        SoundCloudStreamKind::HlsOpus
    } else if mime.contains("aac") {
        SoundCloudStreamKind::HlsAac
    } else {
        SoundCloudStreamKind::HlsMp3
    };

    Some((kind, lookup_url))
}
