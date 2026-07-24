use crate::{
    protocol::tracks::{LoadResult, PlaylistData, PlaylistInfo},
    sources::mixcloud::{MixcloudSource, extractor},
};
use serde_json::{Value, json};
use std::sync::Arc;

const GRAPHQL_URL: &str = "https://app.mixcloud.com/graphql";

pub async fn graphql_request(client: &Arc<reqwest::Client>, query: &str) -> Option<Value> {
    let url = format!("{GRAPHQL_URL}?query={}", urlencoding::encode(query));
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<Value>().await.ok()
}

pub async fn resolve_track(source: &MixcloudSource, username: &str, slug: &str) -> LoadResult {
    let query = format!(
        "{{
            cloudcastLookup(lookup: {{username: \"{username}\", slug: \"{slug}\"}}) {{
              audioLength
              name
              url
              owner {{ displayName username }}
              picture(width: 1024, height: 1024) {{ url }}
              streamInfo {{ hlsUrl url }}
              restrictedReason
            }}
        }}"
    );
    match graphql_request(&source.client, &query).await {
        Some(body) => {
            if let Some(data) = body["data"]["cloudcastLookup"].as_object() {
                if let Some(reason) = data.get("restrictedReason").and_then(|v| v.as_str()) {
                    return LoadResult::Error(crate::protocol::tracks::LoadError {
                        message: Some(format!("Track restricted: {reason}")),
                        severity: crate::common::Severity::Common,
                        cause: reason.to_owned(),
                        cause_stack_trace: None,
                    });
                }
                if let Some(track) = extractor::parse_track_data(&Value::Object(data.clone())) {
                    return LoadResult::Track(track);
                }
            }
            LoadResult::Empty {}
        }
        None => LoadResult::Empty {},
    }
}

pub async fn resolve_playlist(source: &MixcloudSource, user: &str, slug: &str) -> LoadResult {
    let query_template = |cursor: Option<&str>| {
        let cursor_arg = cursor
            .map(|c| format!(", after: \"{c}\""))
            .unwrap_or_default();
        format!(
            "{{
                playlistLookup(lookup: {{username: \"{user}\", slug: \"{slug}\"}}) {{
                  name
                  items(first: 100{cursor_arg}) {{
                    edges {{
                      node {{
                        cloudcast {{
                          audioLength
                          name
                          url
                          owner {{ displayName username }}
                          picture(width: 1024, height: 1024) {{ url }}
                          streamInfo {{ hlsUrl url }}
                        }}
                      }}
                    }}
                    pageInfo {{ endCursor hasNextPage }}
                  }}
                }}
            }}"
        )
    };

    let mut tracks = Vec::new();
    let mut cursor: Option<String> = None;
    let mut playlist_name = "Mixcloud Playlist".to_owned();

    loop {
        let query = query_template(cursor.as_deref());
        let body = match graphql_request(&source.client, &query).await {
            Some(b) => b,
            None => break,
        };

        let lookup = &body["data"]["playlistLookup"];
        if lookup.is_null() {
            break;
        }

        if let Some(name) = lookup["name"].as_str() {
            playlist_name = name.to_owned();
        }

        if let Some(edges) = lookup["items"]["edges"].as_array() {
            for edge in edges {
                if let Some(track) = extractor::parse_track_data(&edge["node"]["cloudcast"]) {
                    tracks.push(track);
                }
            }
        }

        if lookup["items"]["pageInfo"]["hasNextPage"].as_bool() == Some(true) {
            cursor = lookup["items"]["pageInfo"]["endCursor"]
                .as_str()
                .map(|s| s.to_owned());
            if cursor.is_none() || tracks.len() >= 1000 {
                break;
            }
        } else {
            break;
        }
    }

    if tracks.is_empty() {
        return LoadResult::Empty {};
    }

    LoadResult::Playlist(PlaylistData {
        info: PlaylistInfo {
            name: playlist_name,
            selected_track: -1,
        },
        plugin_info: json!({}),
        tracks,
    })
}

pub async fn resolve_user(source: &MixcloudSource, username: &str, list_type: &str) -> LoadResult {
    let (query_type, node_query) = match list_type {
        "stream" => (
            "stream",
            "... on Cloudcast { audioLength name url owner { displayName username } picture(width: 1024, height: 1024) { url } streamInfo { hlsUrl url } }",
        ),
        _ => (
            list_type,
            "audioLength name url owner { displayName username } picture(width: 1024, height: 1024) { url } streamInfo { hlsUrl url }",
        ),
    };

    let query_template = |cursor: Option<&str>| {
        let cursor_arg = cursor
            .map(|c| format!(", after: \"{c}\""))
            .unwrap_or_default();
        format!(
            "{{
                userLookup(lookup: {{username: \"{username}\"}}) {{
                  displayName
                  {query_type}(first: 100{cursor_arg}) {{
                    edges {{
                      node {{
                        {node_query}
                      }}
                    }}
                    pageInfo {{ endCursor hasNextPage }}
                  }}
                }}
            }}"
        )
    };

    let mut tracks = Vec::new();
    let mut cursor: Option<String> = None;
    let mut display_name = username.to_owned();

    loop {
        let query = query_template(cursor.as_deref());
        let body = match graphql_request(&source.client, &query).await {
            Some(b) => b,
            None => break,
        };

        let lookup = &body["data"]["userLookup"];
        if lookup.is_null() {
            break;
        }

        display_name = lookup["displayName"]
            .as_str()
            .unwrap_or(username)
            .to_owned();

        if let Some(edges) = lookup[query_type]["edges"].as_array() {
            for edge in edges {
                if let Some(track) = extractor::parse_track_data(&edge["node"]) {
                    tracks.push(track);
                }
            }
        }

        if lookup[query_type]["pageInfo"]["hasNextPage"].as_bool() == Some(true) {
            cursor = lookup[query_type]["pageInfo"]["endCursor"]
                .as_str()
                .map(|s| s.to_owned());
            if cursor.is_none() || tracks.len() >= 1000 {
                break;
            }
        } else {
            break;
        }
    }

    if tracks.is_empty() {
        return LoadResult::Empty {};
    }

    LoadResult::Playlist(PlaylistData {
        info: PlaylistInfo {
            name: format!("{display_name} ({list_type})"),
            selected_track: -1,
        },
        plugin_info: json!({}),
        tracks,
    })
}

pub async fn fetch_track_stream_info(
    client: &Arc<reqwest::Client>,
    url: &str,
) -> Option<(Option<String>, Option<String>)> {
    let path_parts: Vec<&str> = url
        .split("mixcloud.com/")
        .nth(1)?
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if path_parts.len() < 2 {
        return None;
    }

    let query = format!(
        "{{
            cloudcastLookup(lookup: {{username: \"{}\", slug: \"{}\"}}) {{
              streamInfo {{ hlsUrl url }}
            }}
        }}",
        path_parts[0], path_parts[1]
    );

    let body = graphql_request(client, &query).await?;
    let data = body["data"]["cloudcastLookup"].as_object()?;
    let info = data.get("streamInfo")?;

    let hls = info
        .get("hlsUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());
    let stream = info
        .get("url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned());

    Some((hls, stream))
}
