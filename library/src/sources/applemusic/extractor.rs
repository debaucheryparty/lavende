use crate::protocol::tracks::{PlaylistData, PlaylistInfo, Track, TrackInfo};
use serde_json::{Value, json};

pub fn build_track(item: &Value, artwork_override: Option<String>) -> Option<Track> {
    let attributes = item.get("attributes")?;
    let id = item.get("id")?.as_str()?.to_owned();
    let title = attributes
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Title")
        .to_owned();
    let author = attributes
        .get("artistName")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Artist")
        .to_owned();
    let length = attributes
        .get("durationInMillis")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let isrc = attributes
        .get("isrc")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    let artwork_url = artwork_override.or_else(|| {
        attributes
            .get("artwork")
            .and_then(|a| a.get("url"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.replace("{w}", "1000").replace("{h}", "1000"))
    });
    let url = attributes
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned());

    let mut track = Track::new(TrackInfo {
        title,
        author,
        length,
        identifier: id,
        is_stream: false,
        uri: url,
        artwork_url,
        isrc,
        source_name: "applemusic".to_owned(),
        is_seekable: true,
        position: 0,
    });

    let album_name = attributes.get("albumName").and_then(|v| v.as_str());
    let artist_url = attributes.get("artistUrl").and_then(|v| v.as_str());
    let preview_url = attributes
        .pointer("/previews/0/url")
        .and_then(|v| v.as_str());
    let album_url = track
        .info
        .uri
        .as_ref()
        .and_then(|u| u.split('?').next().map(|s| s.to_owned()));

    track.plugin_info = json!({
        "albumName": album_name,
        "albumUrl": album_url,
        "artistUrl": artist_url,
        "artistArtworkUrl": null,
        "previewUrl": preview_url,
        "isPreview": false
    });

    Some(track)
}

pub fn build_collection(content: &Value, kind: &str) -> Option<PlaylistData> {
    let attributes = content.get("attributes")?;
    let url = attributes.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let name = attributes
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let artwork = attributes
        .pointer("/artwork/url")
        .and_then(|v| v.as_str())
        .map(|s| s.replace("{w}", "500").replace("{h}", "500"));

    let (author, track_count, display_name) = match kind {
        "album" => (
            attributes
                .get("artistName")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Artist")
                .to_owned(),
            attributes
                .get("trackCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            name.to_owned(),
        ),
        "artist" => (name.to_owned(), 0, format!("{}'s Top Tracks", name)),
        "playlist" => (
            attributes
                .get("curatorName")
                .and_then(|v| v.as_str())
                .unwrap_or("Apple Music")
                .to_owned(),
            attributes
                .get("trackCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            name.to_owned(),
        ),
        _ => return None,
    };

    Some(PlaylistData {
        info: PlaylistInfo {
            name: display_name,
            selected_track: -1,
        },
        plugin_info: json!({
            "type": kind,
            "url": url,
            "author": author,
            "artworkUrl": artwork,
            "totalTracks": track_count
        }),
        tracks: Vec::new(),
    })
}
