use crate::{
    protocol::tracks::{Track, TrackInfo},
    sources::youtube::innertube::extract_thumbnail,
};
use serde_json::Value;

pub fn extract_from_player(body: &Value, source_name: &str) -> Option<Track> {
    let details = body
        .get("videoDetails")
        .or_else(|| body.get("video_details"))?;

    let video_id = details
        .get("videoId")
        .or_else(|| details.get("video_id"))?
        .as_str()?;

    let title = details.get("title")?.as_str()?.to_string();
    let author = details.get("author")?.as_str()?.to_string();

    let is_stream = details
        .get("isLiveContent")
        .or_else(|| details.get("is_live_content"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let parse_u64_val = |v: &Value| -> Option<u64> {
        v.as_u64()
            .or_else(|| v.as_i64().filter(|&i| i >= 0).map(|i| i as u64))
            .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
    };

    let length = if is_stream {
        9223372036854775807
    } else if let Some(ms) = details
        .get("length")
        .or_else(|| details.get("lengthMs"))
        .or_else(|| details.get("length_ms"))
        .and_then(parse_u64_val)
    {
        ms
    } else if let Some(sec) = details
        .get("lengthSeconds")
        .or_else(|| details.get("length_seconds"))
        .and_then(parse_u64_val)
    {
        sec * 1000
    } else if let Some(ms) = details
        .get("approxDurationMs")
        .or_else(|| details.get("approx_duration_ms"))
        .and_then(parse_u64_val)
    {
        ms
    } else {
        0
    };

    let artwork_url = extract_thumbnail(details, Some(video_id));

    let track = Track::new(TrackInfo {
        identifier: video_id.to_string(),
        is_seekable: !is_stream,
        author,
        length,
        is_stream,
        position: 0,
        title,
        uri: Some(format!("https://www.youtube.com/watch?v={}", video_id)),
        artwork_url,
        isrc: None,
        source_name: source_name.to_string(),
    });

    Some(track)
}

pub fn extract_from_next(body: &Value, source_name: &str) -> Option<(Vec<Track>, String)> {
    let contents_root = body.get("contents").and_then(|c| {
        c.get("singleColumnWatchNextResults")
            .or_else(|| c.get("singleColumnMusicWatchNextResultsRenderer"))
            .or_else(|| c.get("twoColumnWatchNextResults"))
    })?;

    let playlist_content = contents_root
        .get("playlist")
        .and_then(|p| p.get("playlist"))
        .and_then(|p| p.get("contents"))
        .and_then(|c| c.as_array())
        .or_else(|| {
            contents_root
                .get("tabbedRenderer")
                .and_then(|t| t.get("watchNextTabbedResultsRenderer"))
                .and_then(|w| w.get("tabs"))
                .and_then(|t| t.get(0))
                .and_then(|t| t.get("tabRenderer"))
                .and_then(|t| t.get("content"))
                .and_then(|c| c.get("musicQueueRenderer"))
                .and_then(|music_queue| {
                    music_queue
                        .get("content")
                        .and_then(|c| c.get("playlistPanelRenderer"))
                        .and_then(|p| p.get("contents"))
                        .or_else(|| music_queue.get("contents"))
                        .and_then(|c| c.as_array())
                })
        })?;

    if playlist_content.is_empty() {
        return None;
    }

    let mut tracks = Vec::new();
    for item in playlist_content {
        if let Some(track) = extract_track(item, source_name) {
            tracks.push(track);
        }
    }

    if tracks.is_empty() {
        return None;
    }

    let title = contents_root
        .get("tabbedRenderer")
        .and_then(|t| t.get("watchNextTabbedResultsRenderer"))
        .and_then(|t| t.get("tabs"))
        .and_then(|t| t.get(0))
        .and_then(|t| t.get("tabRenderer"))
        .and_then(|t| t.get("content"))
        .and_then(|c| c.get("musicQueueRenderer"))
        .and_then(|m| m.get("header"))
        .and_then(|h| h.get("musicQueueHeaderRenderer"))
        .and_then(|m| m.get("subtitle"))
        .and_then(get_text)
        .or_else(|| {
            contents_root
                .get("playlist")
                .and_then(|p| p.get("playlist"))
                .and_then(|p| p.get("title"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Unknown Playlist".to_string());

    Some((tracks, title))
}

pub fn extract_from_browse(body: &Value, source_name: &str) -> Option<(Vec<Track>, String)> {
    let title = body
        .get("header")
        .and_then(|h| {
            h.get("playlistHeaderRenderer")
                .or_else(|| h.get("musicAlbumReleaseHeaderRenderer"))
                .or_else(|| h.get("musicDetailHeaderRenderer"))
                .or_else(|| {
                    h.get("musicEditablePlaylistDetailHeaderRenderer")
                        .and_then(|m| m.get("header"))
                        .and_then(|h| h.get("musicDetailHeaderRenderer"))
                })
        })
        .and_then(|h| h.get("title"))
        .and_then(get_text)
        .unwrap_or_else(|| "Unknown Playlist".to_string());

    let mut tracks = Vec::new();
    if let Some(section_list) = find_section_list(body) {
        if let Some(contents) = section_list.get("contents").and_then(|c| c.as_array()) {
            for section in contents {
                if let Some(list) = section
                    .get("itemSectionRenderer")
                    .and_then(|i| i.get("contents"))
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|first| first.get("playlistVideoListRenderer"))
                    .and_then(|p| p.get("contents"))
                    .and_then(|c| c.as_array())
                {
                    for item in list {
                        if let Some(track) = extract_track(item, source_name) {
                            tracks.push(track);
                        }
                    }
                }
                if let Some(list) = section
                    .get("musicShelfRenderer")
                    .and_then(|s| s.get("contents"))
                    .and_then(|c| c.as_array())
                {
                    for item in list {
                        if let Some(track) = extract_track(item, source_name) {
                            tracks.push(track);
                        }
                    }
                }
                if let Some(shelf) = section.get("musicPlaylistShelfRenderer") {
                    if let Some(list) = shelf.get("contents").and_then(|c| c.as_array()) {
                        for item in list {
                            if let Some(track) = extract_track(item, source_name) {
                                tracks.push(track);
                            }
                        }
                    }
                }
            }
        }
    }

    if tracks.is_empty() {
        if let Some(contents) = body
            .get("contents")
            .and_then(|c| c.get("singleColumnBrowseResultsRenderer"))
            .and_then(|s| s.get("tabs"))
            .and_then(|t| t.as_array())
            .and_then(|t| t.first())
            .and_then(|t| t.get("tabRenderer"))
            .and_then(|t| t.get("content"))
            .and_then(|c| c.get("sectionListRenderer"))
            .and_then(|s| s.get("contents"))
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("musicPlaylistShelfRenderer"))
        {
            if let Some(list) = contents.get("contents").and_then(|c| c.as_array()) {
                for item in list {
                    if let Some(track) = extract_track(item, source_name) {
                        tracks.push(track);
                    }
                }
            }
        }
    }

    if tracks.is_empty() {
        if let Some(list) = find_music_playlist_shelf(body) {
            for item in list {
                if let Some(track) = extract_track(item, source_name) {
                    tracks.push(track);
                }
            }
        }
    }

    if tracks.is_empty() {
        if let Some(continuation_contents) = body
            .get("onResponseReceivedActions")
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .and_then(|a| a.get("appendContinuationItemsAction"))
            .and_then(|a| a.get("continuationItems"))
            .and_then(|c| c.as_array())
        {
            for item in continuation_contents {
                if let Some(track) = extract_track(item, source_name) {
                    tracks.push(track);
                }
            }
        }
    }

    if tracks.is_empty() {
        return None;
    }
    Some((tracks, title))
}

fn find_music_playlist_shelf(value: &Value) -> Option<&Vec<Value>> {
    if let Some(shelf) = value.get("musicPlaylistShelfRenderer") {
        return shelf.get("contents").and_then(|c| c.as_array());
    }
    if let Some(obj) = value.as_object() {
        for (_, val) in obj {
            if let Some(list) = find_music_playlist_shelf(val) {
                return Some(list);
            }
        }
    }
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(list) = find_music_playlist_shelf(item) {
                return Some(list);
            }
        }
    }
    None
}

pub fn find_section_list(value: &Value) -> Option<&Value> {
    if let Some(list) = value.get("sectionListRenderer") {
        return Some(list);
    }
    if let Some(contents) = value.get("contents") {
        if let Some(list) = find_section_list(contents) {
            return Some(list);
        }
    }
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(list) = find_section_list(item) {
                return Some(list);
            }
        }
    }
    if let Some(tabs) = value.get("tabs").and_then(|t| t.as_array()) {
        for tab in tabs {
            if let Some(content) = tab.get("tabRenderer").and_then(|tr| tr.get("content")) {
                if let Some(list) = find_section_list(content) {
                    return Some(list);
                }
            }
        }
    }
    if let Some(primary) = value
        .get("twoColumnSearchResultsRenderer")
        .and_then(|t| t.get("primaryContents"))
    {
        return find_section_list(primary);
    }
    None
}

pub fn extract_track(item: &Value, source_name: &str) -> Option<Track> {
    let renderer = item
        .get("videoRenderer")
        .or_else(|| item.get("compactVideoRenderer"))
        .or_else(|| item.get("playlistVideoRenderer"))
        .or_else(|| item.get("musicResponsiveListItemRenderer"))
        .or_else(|| item.get("musicTwoColumnItemRenderer"))
        .or_else(|| item.get("playlistPanelVideoRenderer"))
        .or_else(|| item.get("gridVideoRenderer"))?;

    let video_id = renderer
        .get("videoId")
        .and_then(|v| v.as_str())
        .or_else(|| {
            renderer
                .get("playlistItemData")
                .and_then(|d| d.get("videoId"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            renderer
                .get("doubleTapCommand")
                .and_then(|c| c.get("watchEndpoint"))
                .and_then(|w| w.get("videoId"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| {
            renderer
                .get("navigationEndpoint")
                .and_then(|n| n.get("watchEndpoint"))
                .and_then(|w| w.get("videoId"))
                .and_then(|v| v.as_str())
        })?;

    let title = get_text(renderer.get("title").or_else(|| {
        renderer
            .get("flexColumns")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("musicResponsiveListItemFlexColumnRenderer"))
            .and_then(|r| r.get("text"))
    })?)
    .unwrap_or_else(|| "Unknown Title".to_string());

    let author = extract_author(renderer).unwrap_or_else(|| "Unknown Artist".to_string());

    let is_stream = renderer
        .get("isLive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || renderer
            .get("badges")
            .and_then(|b| b.as_array())
            .map(|arr| {
                arr.iter().any(|badge| {
                    badge
                        .get("metadataBadgeRenderer")
                        .and_then(|mbr| mbr.get("label"))
                        .and_then(|l| l.as_str())
                        .map(|s| s == "LIVE")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

    let length_ms = if is_stream {
        9223372036854775807
    } else {
        renderer
            .get("lengthText")
            .and_then(get_text)
            .map(|s| parse_duration(&s))
            .or_else(|| {
                renderer
                    .get("lengthSeconds")
                    .and_then(|v| v.as_i64())
                    .map(|s| s * 1000)
            })
            .or_else(|| {
                let check_runs = |runs: &Vec<Value>| -> Option<i64> {
                    for run in runs {
                        if let Some(text) = run.get("text").and_then(|t| t.as_str()) {
                            let text = text.trim();
                            if text.contains(':')
                                && text.chars().all(|c| c.is_ascii_digit() || c == ':')
                            {
                                let dur = parse_duration(text);
                                if dur > 0 {
                                    return Some(dur);
                                }
                            }
                        }
                    }
                    None
                };

                if let Some(cols) = renderer.get("fixedColumns").and_then(|c| c.as_array()) {
                    for col in cols {
                        if let Some(runs) = col
                            .get("musicResponsiveListItemFixedColumnRenderer")
                            .and_then(|r| r.get("text"))
                            .and_then(|t| t.get("runs"))
                            .and_then(|r| r.as_array())
                        {
                            if let Some(dur) = check_runs(runs) {
                                return Some(dur);
                            }
                        }
                    }
                }

                if let Some(cols) = renderer.get("flexColumns").and_then(|c| c.as_array()) {
                    for col in cols {
                        if let Some(runs) = col
                            .get("musicResponsiveListItemFlexColumnRenderer")
                            .and_then(|r| r.get("text"))
                            .and_then(|t| t.get("runs"))
                            .and_then(|r| r.as_array())
                        {
                            if let Some(dur) = check_runs(runs) {
                                return Some(dur);
                            }
                        }
                    }
                }

                if let Some(runs) = renderer
                    .get("subtitle")
                    .and_then(|s| s.get("runs"))
                    .and_then(|r| r.as_array())
                {
                    if let Some(dur) = check_runs(runs) {
                        return Some(dur);
                    }
                }

                None
            })
            .unwrap_or(0)
    };

    Some(Track::new(TrackInfo {
        identifier: video_id.to_string(),
        is_seekable: !is_stream,
        author,
        length: length_ms as u64,
        is_stream,
        position: 0,
        title,
        uri: Some(format!("https://www.youtube.com/watch?v={}", video_id)),
        artwork_url: get_thumbnail(renderer, Some(video_id)),
        isrc: None,
        source_name: source_name.to_string(),
    }))
}

fn extract_author(renderer: &Value) -> Option<String> {
    if let Some(subtitle) = renderer.get("subtitle") {
        if let Some(text) = get_first_subtitle_run(subtitle) {
            let artist = text.split(" • ").next().unwrap_or(&text).trim();
            if !artist.is_empty() {
                return Some(artist.to_string());
            }
        }
    }
    if let Some(author) = renderer
        .get("menu")
        .and_then(|m| m.get("menuRenderer"))
        .and_then(|m| m.get("title"))
        .and_then(|t| t.get("musicMenuTitleRenderer"))
        .and_then(|m| m.get("secondaryText"))
        .and_then(get_first_text)
    {
        return Some(author);
    }
    if let Some(text) = renderer.get("longBylineText").and_then(get_first_text) {
        return Some(text);
    }
    if let Some(text) = renderer.get("shortBylineText").and_then(get_first_text) {
        return Some(text);
    }
    if let Some(text) = renderer.get("ownerText").and_then(get_first_text) {
        return Some(text);
    }
    if let Some(flex) = renderer
        .get("flexColumns")
        .and_then(|c| c.get(1))
        .and_then(|c| c.get("musicResponsiveListItemFlexColumnRenderer"))
        .and_then(|r| r.get("text"))
    {
        if let Some(runs) = flex.get("runs").and_then(|r| r.as_array()) {
            if let Some(text) = runs
                .first()
                .and_then(|r| r.get("text"))
                .and_then(|t| t.as_str())
            {
                return Some(text.to_string());
            }
        }
    }
    None
}

fn get_first_subtitle_run(subtitle: &Value) -> Option<String> {
    if let Some(runs) = subtitle.get("runs").and_then(|r| r.as_array()) {
        return runs
            .first()
            .and_then(|r| r.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
    }
    if let Some(simple_text) = subtitle.get("simpleText").and_then(|v| v.as_str()) {
        return Some(simple_text.to_string());
    }
    if let Some(s) = subtitle.as_str() {
        return Some(s.to_string());
    }
    None
}

pub fn get_text(obj: &Value) -> Option<String> {
    if let Some(s) = obj.as_str() {
        return Some(s.to_string());
    }
    if let Some(simple_text) = obj.get("simpleText").and_then(|v| v.as_str()) {
        return Some(simple_text.to_string());
    }
    if let Some(runs) = obj.get("runs").and_then(|v| v.as_array()) {
        let mut text = String::new();
        for run in runs {
            if let Some(t) = run.get("text").and_then(|v| v.as_str()) {
                text.push_str(t);
            }
        }
        return Some(text);
    }
    None
}

fn get_first_text(obj: &Value) -> Option<String> {
    if let Some(s) = obj.as_str() {
        return Some(s.to_string());
    }
    if let Some(simple_text) = obj.get("simpleText").and_then(|v| v.as_str()) {
        return Some(simple_text.to_string());
    }
    if let Some(runs) = obj.get("runs").and_then(|v| v.as_array()) {
        return runs
            .first()
            .and_then(|run| run.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
    }
    None
}

fn parse_duration(s: &str) -> i64 {
    let parts: Vec<&str> = s.split(':').collect();
    let mut seconds = 0;
    for part in parts {
        seconds = seconds * 60 + part.parse::<i64>().unwrap_or(0);
    }
    seconds * 1000
}

fn get_thumbnail(renderer: &Value, video_id: Option<&str>) -> Option<String> {
    extract_thumbnail(renderer, video_id)
}

fn find_music_search_shelf(content: &Value) -> Option<&Vec<Value>> {
    if let Some(section_list) = content.get("sectionListRenderer") {
        if let Some(sections) = section_list.get("contents").and_then(|c| c.as_array()) {
            for section in sections {
                if let Some(shelf) = section.get("musicShelfRenderer") {
                    return shelf.get("contents").and_then(|c| c.as_array());
                }
            }
        }
    } else if let Some(sections) = content.as_array() {
        for section in sections {
            if let Some(shelf) = section.get("musicShelfRenderer") {
                return shelf.get("contents").and_then(|c| c.as_array());
            }
        }
    }
    None
}

pub fn extract_from_search(body: &Value, source_name: &str) -> Vec<Track> {
    let mut tracks = Vec::new();

    if let Some(tabbed) = body
        .get("contents")
        .and_then(|c| c.get("tabbedSearchResultsRenderer"))
    {
        let tab_content = tabbed
            .get("tabs")
            .and_then(|t| t.get(0))
            .and_then(|t| t.get("tabRenderer"))
            .and_then(|t| t.get("content"));

        let mut items = None;

        if let Some(tab) = tab_content {
            items = find_music_search_shelf(tab);
            if items.is_none() {
                if let Some(split_view) = tab.get("musicSplitViewRenderer") {
                    if let Some(main_content) = split_view.get("mainContent") {
                        items = find_music_search_shelf(main_content);
                    }
                }
            }
        }

        if let Some(items) = items {
            for item in items {
                if let Some(track) = extract_track(item, source_name) {
                    tracks.push(track);
                }
            }
        }
        return tracks;
    }

    if let Some(contents) = body.get("contents") {
        let sections = contents
            .get("sectionListRenderer")
            .and_then(|s| s.get("contents"))
            .and_then(|c| c.as_array())
            .or_else(|| {
                contents
                    .get("twoColumnSearchResultsRenderer")
                    .and_then(|t| t.get("primaryContents"))
                    .and_then(|p| p.get("sectionListRenderer"))
                    .and_then(|s| s.get("contents"))
                    .and_then(|c| c.as_array())
            });

        if let Some(sections) = sections {
            for section in sections {
                let items_opt = section
                    .get("itemSectionRenderer")
                    .and_then(|i| i.get("contents"))
                    .and_then(|c| c.as_array());

                let shelf_items_opt = items_opt
                    .is_none()
                    .then(|| {
                        let shelf = section
                            .get("shelfRenderer")
                            .or_else(|| section.get("richShelfRenderer"))
                            .or_else(|| section.get("reelShelfRenderer"));
                        shelf.and_then(|s| {
                            s.get("content")
                                .and_then(|c| {
                                    c.get("verticalListRenderer")
                                        .or_else(|| c.get("horizontalListRenderer"))
                                })
                                .and_then(|v| v.get("items"))
                                .or_else(|| {
                                    s.get("content")
                                        .and_then(|c| c.get("richGridRenderer"))
                                        .and_then(|r| r.get("contents"))
                                })
                                .and_then(|c| c.as_array())
                        })
                    })
                    .flatten();

                let items = items_opt.or(shelf_items_opt);
                if let Some(items) = items {
                    for item in items {
                        let inner = item
                            .get("richItemRenderer")
                            .and_then(|r| r.get("content"))
                            .unwrap_or(item);
                        if let Some(track) = extract_track(inner, source_name) {
                            tracks.push(track);
                        }
                    }
                }
            }
        } else if let Some(contents) = contents
            .get("twoColumnSearchResultsRenderer")
            .and_then(|t| t.get("primaryContents"))
            .and_then(|p| p.get("richGridRenderer"))
            .and_then(|r| r.get("contents"))
            .and_then(|c| c.as_array())
        {
            for item in contents {
                let inner = item
                    .get("richItemRenderer")
                    .and_then(|r| r.get("content"))
                    .unwrap_or(item);
                if let Some(track) = extract_track(inner, source_name) {
                    tracks.push(track);
                }
            }
        }
    }

    tracks
}
