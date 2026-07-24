use crate::protocol::tracks::TrackInfo;
use serde_json::Value;

pub fn parse_track_inner(track_val: &Value, artwork_url: Option<String>) -> Option<TrackInfo> {
    let track = if track_val.get("uri").is_some() {
        track_val
    } else {
        track_val
            .get("track")
            .or_else(|| track_val.get("item"))
            .or_else(|| track_val.get("data"))?
    };

    let uri = track.get("uri").and_then(|v| v.as_str())?;
    let id = uri.split(':').next_back()?.to_owned();
    let title = track.get("name").and_then(|v| v.as_str())?.to_owned();
    let author = extract_author(track);
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

    let isrc = extract_isrc_inline(track);

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
