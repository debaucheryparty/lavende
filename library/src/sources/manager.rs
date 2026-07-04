pub mod resolver {
use std::sync::Arc;
use crate::sources::{manager::SourceManager, playable_track::BoxedTrack};
pub async fn resolve_with_mirrors(
    manager: &SourceManager,
    track_info: &crate::protocol::tracks::TrackInfo,
    identifier: &str,
    mirrors: &crate::config::server::MirrorsConfig,
    routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
) -> Result<BoxedTrack, String> {
    if mirrors.best_match.scoring {
        return super::best_match::resolve_scored(
            manager,
            track_info,
            identifier,
            mirrors,
            routeplanner,
        )
        .await;
    }
    let isrc = track_info.isrc.as_deref().unwrap_or("");
    let query = format!("{} - {}", track_info.title, track_info.author);
    let original_source_name = manager
        .sources
        .iter()
        .find(|s| s.can_handle(identifier))
        .map(|s| s.name());
    for provider in &mirrors.providers {
        if isrc.is_empty() && provider.contains("%ISRC%") {
            tracing::debug!("Skipping mirror provider '{}': track has no ISRC", provider);
            continue;
        }
        let resolved = provider.replace("%ISRC%", isrc).replace("%QUERY%", &query);
        if let Some(handling_source) = manager.sources.iter().find(|s| s.can_handle(&resolved)) {
            if handling_source.is_mirror() {
                tracing::warn!(
                    "Skipping mirror provider '{}': '{}' is a Mirror-type source",
                    resolved,
                    handling_source.name()
                );
                continue;
            }
            if Some(handling_source.name()) == original_source_name {
                tracing::debug!(
                    "Skipping mirror provider '{}': would loop back to '{}'",
                    resolved,
                    handling_source.name()
                );
                continue;
            }
        }
        let res = match manager.load(&resolved, routeplanner.clone()).await {
            crate::protocol::tracks::LoadResult::Track(t) => {
                let id = t.info.uri.as_deref().unwrap_or(&t.info.identifier);
                resolve_nested_track(manager, id, routeplanner.clone()).await
            }
            crate::protocol::tracks::LoadResult::Search(tracks) => {
                if let Some(first) = tracks.first() {
                    let id = first.info.uri.as_deref().unwrap_or(&first.info.identifier);
                    resolve_nested_track(manager, id, routeplanner.clone()).await
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(track) = res {
            return Ok(track);
        }
    }
    tracing::warn!(
        "[Mirror] no valid mirror found for track: {} - {}",
        track_info.title,
        track_info.author
    );
    Err(format!(
        "No mirror found for track: {} - {}",
        track_info.title, track_info.author
    ))
}
pub async fn resolve_nested_track(
    manager: &SourceManager,
    identifier: &str,
    routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
) -> Option<BoxedTrack> {
    for source in &manager.sources {
        if source.can_handle(identifier) {
            if let Some(track) = source.get_track(identifier, routeplanner.clone()).await {
                return Some(track);
            }
            if source.name() != "http" {
                return None;
            }
        }
    }
    None
}
}
pub mod best_match {
use std::sync::Arc;
use futures::stream::{FuturesOrdered, FuturesUnordered, StreamExt};
use crate::sources::{manager::SourceManager, playable_track::BoxedTrack};
pub struct MirrorResult {
    pub track: BoxedTrack,
    pub score: f64,
    pub provider: String,
}
fn normalize(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut stripped = String::with_capacity(lower.len());
    let mut depth: usize = 0;
    for ch in lower.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => stripped.push(ch),
            _ => {}
        }
    }
    let stripped = stripped
        .replace("feat.", " ")
        .replace("feat ", " ")
        .replace("ft.", " ")
        .replace("ft ", " ");
    let clean: String = stripped
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect();
    clean.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}
fn string_similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let na = normalize(a);
    let nb = normalize(b);
    if na == nb {
        return 1.0;
    }
    if na.contains(&nb) || nb.contains(&na) {
        let shorter = na.len().min(nb.len()) as f64;
        let longer = na.len().max(nb.len()) as f64;
        return 0.80 + (shorter / longer) * 0.15;
    }
    let max_len = na.len().max(nb.len());
    if max_len == 0 {
        return 1.0;
    }
    1.0 - levenshtein(&na, &nb) as f64 / max_len as f64
}
fn duration_similarity(d1: u64, d2: u64, tolerance_ms: u64) -> f64 {
    if d1 == 0 || d2 == 0 {
        return 0.5;
    }
    let diff = d1.abs_diff(d2);
    if diff <= tolerance_ms {
        1.0
    } else {
        (1.0 - diff as f64 / d1.max(d2) as f64).max(0.0)
    }
}
fn score_match(
    orig_title: &str,
    orig_author: &str,
    orig_length: u64,
    cand_title: &str,
    cand_author: &str,
    cand_length: u64,
    cfg: &crate::config::server::BestMatchConfig,
) -> f64 {
    let nt = normalize(orig_title);
    let nc = normalize(cand_title);
    let title_score = if nt == nc {
        1.0
    } else if nc.starts_with(&nt) {
        0.95
    } else if nc.contains(&nt) || nt.contains(&nc) {
        let shorter = nt.len().min(nc.len()) as f64;
        let longer = nt.len().max(nc.len()) as f64;
        0.82 + (shorter / longer) * 0.10
    } else {
        string_similarity(&nt, &nc)
    };
    title_score * cfg.weight_title
        + string_similarity(orig_author, cand_author) * cfg.weight_artist
        + duration_similarity(orig_length, cand_length, cfg.duration_tolerance_ms)
            * cfg.weight_duration
}
fn fmt_ms(ms: u64) -> String {
    let s = ms / 1_000;
    format!("{}:{:02}", s / 60, s % 60)
}
pub async fn resolve_scored(
    manager: &SourceManager,
    track_info: &crate::protocol::tracks::TrackInfo,
    identifier: &str,
    mirrors: &crate::config::server::MirrorsConfig,
    routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
) -> Result<BoxedTrack, String> {
    let isrc = track_info.isrc.as_deref().unwrap_or("");
    let query = format!("{} {}", track_info.title, track_info.author);
    let cfg = &mirrors.best_match;
    let original_source_name = manager
        .sources
        .iter()
        .find(|s| s.can_handle(identifier))
        .map(|s| s.name().to_string());
    let mut isrc_providers: Vec<String> = Vec::new();
    let mut free_providers: Vec<String> = Vec::new();
    let mut throttled_providers: Vec<String> = Vec::new();
    for provider in &mirrors.providers {
        let is_isrc_provider = provider.contains("%ISRC%");
        if is_isrc_provider && isrc.is_empty() {
            tracing::debug!("Skipping mirror provider '{}': track has no ISRC", provider);
            continue;
        }
        let resolved = provider.replace("%ISRC%", isrc).replace("%QUERY%", &query);
        if let Some(src) = manager.sources.iter().find(|s| s.can_handle(&resolved)) {
            if src.is_mirror() {
                tracing::warn!(
                    "Skipping mirror provider '{}': '{}' is a Mirror-type source",
                    resolved,
                    src.name()
                );
                continue;
            }
            if Some(src.name().to_string()) == original_source_name {
                tracing::debug!(
                    "Skipping mirror provider '{}': would loop back to '{}'",
                    resolved,
                    src.name()
                );
                continue;
            }
        }
        if is_isrc_provider {
            isrc_providers.push(resolved);
        } else if cfg
            .throttled_prefixes
            .iter()
            .any(|p| resolved.starts_with(p.as_str()))
        {
            throttled_providers.push(resolved);
        } else {
            free_providers.push(resolved);
        }
    }
    if !isrc_providers.is_empty() {
        let mut futs: FuturesOrdered<_> = isrc_providers
            .iter()
            .map(|p| search_provider(manager, track_info, p, routeplanner.clone(), cfg, true))
            .collect();
        while let Some(result) = futs.next().await {
            if let Some(mr) = result {
                tracing::info!(
                    "[Mirror] ISRC match \"{}\" | {} | {} => {} | score: {:.3}",
                    track_info.title,
                    track_info.author,
                    fmt_ms(track_info.length),
                    mr.provider,
                    mr.score,
                );
                return Ok(mr.track);
            }
        }
    }
    let mut global_best: Option<MirrorResult> = None;
    if !free_providers.is_empty() {
        let mut futs: FuturesUnordered<_> = free_providers
            .iter()
            .map(|p| search_provider(manager, track_info, p, routeplanner.clone(), cfg, false))
            .collect();
        while let Some(result) = futs.next().await {
            if let Some(mr) = result {
                tracing::info!(
                    "[Mirror] \"{}\" | {} | {} => {} | score: {:.3}",
                    track_info.title,
                    track_info.author,
                    fmt_ms(track_info.length),
                    mr.provider,
                    mr.score,
                );
                if mr.score >= cfg.immediate_use {
                    return Ok(mr.track);
                }
                if global_best.as_ref().is_none_or(|b| mr.score > b.score) {
                    global_best = Some(mr);
                }
            }
        }
    }
    for provider in &throttled_providers {
        if let Some(mr) = search_provider(
            manager,
            track_info,
            provider,
            routeplanner.clone(),
            cfg,
            true,
        )
        .await
        {
            tracing::info!(
                "[Mirror] throttled match \"{}\" via {} (score {:.3})",
                track_info.title,
                mr.provider,
                mr.score
            );
            return Ok(mr.track);
        }
    }
    if let Some(best) = global_best {
        tracing::info!(
            "[Mirror] fallback match \"{}\" via {} (score {:.3})",
            track_info.title,
            best.provider,
            best.score
        );
        return Ok(best.track);
    }
    tracing::warn!(
        "[Mirror] no valid mirror found for \"{}\" | {}",
        track_info.title,
        track_info.author
    );
    Err(format!(
        "No mirror found for track: {} - {}",
        track_info.title, track_info.author
    ))
}
async fn search_provider(
    manager: &SourceManager,
    original: &crate::protocol::tracks::TrackInfo,
    resolved_provider: &str,
    routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    cfg: &crate::config::server::BestMatchConfig,
    trust_any: bool,
) -> Option<MirrorResult> {
    use crate::protocol::tracks::LoadResult;
    let candidates: Vec<crate::protocol::tracks::TrackInfo> =
        match manager.load(resolved_provider, routeplanner.clone()).await {
            LoadResult::Track(t) => vec![t.info],
            LoadResult::Search(tracks) => tracks.into_iter().take(10).map(|t| t.info).collect(),
            _ => return None,
        };
    if candidates.is_empty() {
        return None;
    }
    let mut scored: Vec<(f64, crate::protocol::tracks::TrackInfo)> = candidates
        .into_iter()
        .map(|info| {
            let s = score_match(
                &original.title,
                &original.author,
                original.length,
                &info.title,
                &info.author,
                info.length,
                cfg,
            );
            (s, info)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let top_score = scored[0].0;
    let (limit, threshold): (usize, f64) = if trust_any {
        (scored.len(), 0.0)
    } else if top_score >= cfg.immediate_use {
        (1, cfg.immediate_use)
    } else if top_score >= cfg.high_confidence {
        (2, cfg.high_confidence)
    } else {
        (3, cfg.min_similarity)
    };
    for (score, info) in scored.into_iter().take(limit) {
        if score < threshold {
            break;
        }
        let id = info.uri.as_deref().unwrap_or(&info.identifier);
        if let Some(track) =
            super::resolver::resolve_nested_track(manager, id, routeplanner.clone()).await
        {
            return Some(MirrorResult {
                track,
                score,
                provider: resolved_provider.to_string(),
            });
        }
    }
    None
}
}
pub mod registration {
use std::sync::Arc;
use crate::{
    common::HttpClientPool,
    sources::{
        amazonmusic::AmazonMusicSource,
        anghami::AnghamiSource,
        applemusic::AppleMusicSource,
        audiomack::AudiomackSource,
        audius::AudiusSource,
        bandcamp::BandcampSource,
        deezer::DeezerSource,
        flowery::FlowerySource,
        gaana::GaanaSource,
        google_tts::GoogleTtsSource,
        http::HttpSource,
        jiosaavn::JioSaavnSource,
        lastfm::LastFMSource,
        local::LocalSource,
        mixcloud::MixcloudSource,
        netease::NeteaseSource,
        pandora::PandoraSource,
        playable_track::BoxedSource,
        qobuz::QobuzSource,
        reddit::RedditSource,
        shazam::ShazamSource,
        soundcloud::SoundCloudSource,
        spotify::SpotifySource,
        tidal::TidalSource,
        twitch::TwitchSource,
        vkmusic::VkMusicSource,
        yandexmusic::YandexMusicSource,
        youtube::{YouTubeSource, YoutubeStreamContext, cipher::YouTubeCipherManager},
    },
};
pub fn register_all(
    sources: &mut Vec<BoxedSource>,
    config: &crate::config::AppConfig,
    http_pool: &Arc<HttpClientPool>,
) -> (
    Option<Arc<YouTubeCipherManager>>,
    Option<Arc<YoutubeStreamContext>>,
) {
    let yt_ctx = register_core_sources(sources, config, http_pool);
    register_extra_sources(sources, config);
    yt_ctx
}
fn register_core_sources(
    sources: &mut Vec<BoxedSource>,
    config: &crate::config::AppConfig,
    http_pool: &Arc<HttpClientPool>,
) -> (
    Option<Arc<YouTubeCipherManager>>,
    Option<Arc<YoutubeStreamContext>>,
) {
    let mut yt_ctx = (None, None);
    macro_rules! register {
        ($enabled:expr, $name:literal, $proxy:expr, $ctor:expr) => {
            if $enabled {
                if let Some(p) = &$proxy {
                    tracing::info!(
                        "Loading {} with proxy: {}",
                        $name,
                        p.url.as_ref().unwrap_or(&"enabled".to_owned())
                    );
                }
                match $ctor {
                    Ok(src) => {
                        tracing::info!("Loaded source: {}", $name);
                        sources.push(Box::new(src));
                    }
                    Err(e) => {
                        tracing::error!("{} source failed to initialize: {}", $name, e);
                    }
                }
            }
        };
    }
    if config.sources.youtube.as_ref().is_some_and(|c| c.enabled) {
        tracing::info!("Loaded source: YouTube");
        let yt_client = http_pool.get(None);
        let yt = YouTubeSource::new(config.sources.youtube.clone(), yt_client);
        yt_ctx = (Some(yt.cipher_manager()), Some(yt.stream_context()));
        sources.push(Box::new(yt));
    }
    let soundcloud_proxy = config
        .sources
        .soundcloud
        .as_ref()
        .and_then(|c| c.proxy.clone());
    register!(
        config
            .sources
            .soundcloud
            .as_ref()
            .is_some_and(|c| c.enabled),
        "SoundCloud",
        soundcloud_proxy,
        SoundCloudSource::new(
            config.sources.soundcloud.clone().unwrap(),
            http_pool.get(soundcloud_proxy.clone())
        )
    );
    register!(
        config.sources.spotify.as_ref().is_some_and(|c| c.enabled),
        "Spotify",
        None::<crate::config::HttpProxyConfig>,
        SpotifySource::new(config.sources.spotify.clone(), http_pool.get(None))
    );
    let jiosaavn_proxy = config
        .sources
        .jiosaavn
        .as_ref()
        .and_then(|c| c.proxy.clone());
    register!(
        config.sources.jiosaavn.as_ref().is_some_and(|c| c.enabled),
        "JioSaavn",
        jiosaavn_proxy,
        JioSaavnSource::new(
            config.sources.jiosaavn.clone(),
            http_pool.get(jiosaavn_proxy.clone())
        )
    );
    register_deezer(sources, config, http_pool);
    let apple_proxy = config
        .sources
        .applemusic
        .as_ref()
        .and_then(|c| c.proxy.clone());
    register!(
        config
            .sources
            .applemusic
            .as_ref()
            .is_some_and(|c| c.enabled),
        "Apple Music",
        apple_proxy,
        AppleMusicSource::new(
            config.sources.applemusic.clone(),
            http_pool.get(apple_proxy.clone())
        )
    );
    let gaana_proxy = config.sources.gaana.as_ref().and_then(|c| c.proxy.clone());
    register!(
        config.sources.gaana.as_ref().is_some_and(|c| c.enabled),
        "Gaana",
        gaana_proxy,
        GaanaSource::new(
            config.sources.gaana.clone(),
            http_pool.get(gaana_proxy.clone())
        )
    );
    let tidal_proxy = config.sources.tidal.as_ref().and_then(|c| c.proxy.clone());
    register!(
        config.sources.tidal.as_ref().is_some_and(|c| c.enabled),
        "Tidal",
        tidal_proxy,
        TidalSource::new(
            config.sources.tidal.clone(),
            http_pool.get(tidal_proxy.clone())
        )
    );
    let audiomack_proxy = config
        .sources
        .audiomack
        .as_ref()
        .and_then(|c| c.proxy.clone());
    register!(
        config.sources.audiomack.as_ref().is_some_and(|c| c.enabled),
        "Audiomack",
        audiomack_proxy,
        AudiomackSource::new(
            config.sources.audiomack.clone(),
            http_pool.get(audiomack_proxy.clone())
        )
    );
    let pandora_proxy = config
        .sources
        .pandora
        .as_ref()
        .and_then(|c| c.proxy.clone());
    register!(
        config.sources.pandora.as_ref().is_some_and(|c| c.enabled),
        "Pandora",
        pandora_proxy,
        PandoraSource::new(
            config.sources.pandora.clone(),
            http_pool.get(pandora_proxy.clone())
        )
    );
    let qobuz_proxy = config.sources.qobuz.as_ref().and_then(|c| c.proxy.clone());
    if config.sources.qobuz.as_ref().is_some_and(|c| c.enabled) {
        let token_provided = config
            .sources
            .qobuz
            .as_ref()
            .and_then(|c| c.user_token.as_ref())
            .is_some_and(|t| !t.is_empty());
        if !token_provided {
            tracing::warn!("Qobuz user_token is missing; all playback will fall back to mirrors.");
        }
        register!(
            true,
            "Qobuz",
            qobuz_proxy,
            QobuzSource::new(config, http_pool.get(qobuz_proxy.clone()))
        );
    }
    let anghami_proxy = config
        .sources
        .anghami
        .as_ref()
        .and_then(|c| c.proxy.clone());
    register!(
        config.sources.anghami.as_ref().is_some_and(|c| c.enabled),
        "Anghami",
        anghami_proxy,
        AnghamiSource::new(config, http_pool.get(anghami_proxy.clone()))
    );
    let shazam_proxy = config.sources.shazam.as_ref().and_then(|c| c.proxy.clone());
    register!(
        config.sources.shazam.as_ref().is_some_and(|c| c.enabled),
        "Shazam",
        shazam_proxy,
        ShazamSource::new(config, http_pool.get(shazam_proxy.clone()))
    );
    let mixcloud_proxy = config
        .sources
        .mixcloud
        .as_ref()
        .and_then(|c| c.proxy.clone());
    register!(
        config.sources.mixcloud.as_ref().is_some_and(|c| c.enabled),
        "Mixcloud",
        mixcloud_proxy,
        MixcloudSource::new(
            config.sources.mixcloud.clone(),
            http_pool.get(mixcloud_proxy.clone())
        )
    );
    let bandcamp_proxy = config
        .sources
        .bandcamp
        .as_ref()
        .and_then(|c| c.proxy.clone());
    register!(
        config.sources.bandcamp.as_ref().is_some_and(|c| c.enabled),
        "Bandcamp",
        bandcamp_proxy,
        BandcampSource::new(
            config.sources.bandcamp.clone(),
            http_pool.get(bandcamp_proxy.clone())
        )
    );
    let reddit_proxy = config.sources.reddit.as_ref().and_then(|c| c.proxy.clone());
    register!(
        config.sources.reddit.as_ref().is_some_and(|c| c.enabled),
        "Reddit",
        reddit_proxy,
        RedditSource::new(
            config.sources.reddit.clone(),
            http_pool.get(reddit_proxy.clone())
        )
    );
    register!(
        config.sources.lastfm.as_ref().is_some_and(|c| c.enabled),
        "Last.fm",
        None::<crate::config::HttpProxyConfig>,
        LastFMSource::new(config.sources.lastfm.clone(), http_pool.get(None))
    );
    let audius_proxy = config.sources.audius.as_ref().and_then(|c| c.proxy.clone());
    register!(
        config.sources.audius.as_ref().is_some_and(|c| c.enabled),
        "Audius",
        audius_proxy,
        AudiusSource::new(
            config.sources.audius.clone(),
            http_pool.get(audius_proxy.clone())
        )
    );
    register_yandex(sources, config, http_pool);
    register_vkmusic(sources, config, http_pool);
    register_netease(sources, config, http_pool);
    register_twitch(sources, config, http_pool);
    register_amazonmusic(sources, config, http_pool);
    if config.sources.http.as_ref().is_some_and(|c| c.enabled) {
        tracing::info!("Loaded source: http");
        sources.push(Box::new(HttpSource::new()));
    }
    yt_ctx
}
fn register_deezer(
    sources: &mut Vec<BoxedSource>,
    config: &crate::config::AppConfig,
    http_pool: &Arc<HttpClientPool>,
) {
    let (token_provided, key_provided) = if let Some(c) = config.sources.deezer.as_ref() {
        let arls_provided = c
            .arls
            .as_ref()
            .is_some_and(|a| !a.is_empty() && a.iter().any(|s| !s.is_empty()));
        let key_provided = c
            .master_decryption_key
            .as_ref()
            .is_some_and(|k| !k.is_empty());
        (arls_provided, key_provided)
    } else {
        (false, false)
    };
    if config.sources.deezer.as_ref().is_some_and(|c| c.enabled) {
        if !token_provided || !key_provided {
            let mut missing = Vec::new();
            if !token_provided {
                missing.push("arls");
            }
            if !key_provided {
                missing.push("master_decryption_key");
            }
            tracing::warn!(
                "Deezer source is enabled but {} {} missing; it will be disabled.",
                missing.join(" and "),
                if missing.len() > 1 { "are" } else { "is" }
            );
        } else {
            let proxy = config.sources.deezer.as_ref().and_then(|c| c.proxy.clone());
            let source = DeezerSource::new(
                config.sources.deezer.clone().unwrap(),
                http_pool.get(proxy.clone()),
            );
            match source {
                Ok(src) => {
                    tracing::info!("Loaded source: Deezer");
                    sources.push(Box::new(src));
                }
                Err(e) => {
                    tracing::error!("Deezer source failed to initialize: {}", e);
                }
            }
        }
    }
}
fn register_yandex(
    sources: &mut Vec<BoxedSource>,
    config: &crate::config::AppConfig,
    http_pool: &Arc<HttpClientPool>,
) {
    if let Some(c) = config.sources.yandexmusic.as_ref()
        && c.enabled
    {
        if c.access_token.is_none() {
            tracing::warn!(
                "Yandex Music source is enabled but the access_token is missing; it will be disabled."
            );
        } else {
            let proxy = c.proxy.clone();
            let source = YandexMusicSource::new(
                config.sources.yandexmusic.clone(),
                http_pool.get(proxy.clone()),
            );
            match source {
                Ok(src) => {
                    tracing::info!("Loaded source: Yandex Music");
                    sources.push(Box::new(src));
                }
                Err(e) => {
                    tracing::error!("Yandex Music source failed to initialize: {}", e);
                }
            }
        }
    }
}
fn register_vkmusic(
    sources: &mut Vec<BoxedSource>,
    config: &crate::config::AppConfig,
    http_pool: &Arc<HttpClientPool>,
) {
    if let Some(c) = config.sources.vkmusic.as_ref()
        && c.enabled
    {
        if c.user_token.is_none() && c.user_cookie.is_none() {
            tracing::warn!(
                "VK Music source is enabled but neither user_token nor user_cookie is set; API calls will fail."
            );
        }
        let proxy = c.proxy.clone();
        match VkMusicSource::new(config.sources.vkmusic.clone(), http_pool.get(proxy.clone())) {
            Ok(src) => {
                tracing::info!("Loaded source: VK Music");
                sources.push(Box::new(src));
            }
            Err(e) => {
                tracing::error!("VK Music source failed to initialize: {}", e);
            }
        }
    }
}
fn register_netease(
    sources: &mut Vec<BoxedSource>,
    config: &crate::config::AppConfig,
    http_pool: &Arc<HttpClientPool>,
) {
    if let Some(c) = config.sources.netease.as_ref()
        && c.enabled
    {
        let proxy = c.proxy.clone();
        match NeteaseSource::new(config.sources.netease.clone(), http_pool.get(proxy.clone())) {
            Ok(src) => {
                tracing::info!("Loaded source: Netease Music");
                sources.push(Box::new(src));
            }
            Err(e) => {
                tracing::error!("Netease Music source failed to initialize: {}", e);
            }
        }
    }
}
fn register_twitch(
    sources: &mut Vec<BoxedSource>,
    config: &crate::config::AppConfig,
    http_pool: &Arc<HttpClientPool>,
) {
    if let Some(c) = config.sources.twitch.as_ref()
        && c.enabled
    {
        let proxy = c.proxy.clone();
        tracing::info!("Loaded source: Twitch");
        sources.push(Box::new(TwitchSource::new(c.clone(), http_pool.get(proxy))));
    }
}
fn register_amazonmusic(
    sources: &mut Vec<BoxedSource>,
    config: &crate::config::AppConfig,
    http_pool: &Arc<HttpClientPool>,
) {
    if let Some(c) = config.sources.amazonmusic.as_ref()
        && c.enabled
    {
        let proxy = c.proxy.clone();
        match AmazonMusicSource::new(c.clone(), http_pool.get(proxy)) {
            Ok(src) => {
                tracing::info!("Loaded source: Amazon Music");
                sources.push(Box::new(src));
            }
            Err(e) => {
                tracing::error!("Amazon Music source failed to initialize: {}", e);
            }
        }
    }
}
fn register_extra_sources(sources: &mut Vec<BoxedSource>, config: &crate::config::AppConfig) {
    if let Some(c) = config.sources.google_tts.as_ref()
        && c.enabled
    {
        tracing::info!("Loaded source: Google TTS");
        sources.push(Box::new(GoogleTtsSource::new(c.clone())));
    }
    if let Some(c) = config.sources.flowery.as_ref()
        && c.enabled
    {
        tracing::info!("Loaded source: Flowery");
        sources.push(Box::new(FlowerySource::new(c.clone())));
    }
    if config.sources.local.as_ref().is_some_and(|c| c.enabled) {
        tracing::info!("Loaded source: local");
        sources.push(Box::new(LocalSource::new()));
    }
}
}
use std::sync::Arc;
use crate::{
    common::HttpClientPool,
    sources::playable_track::{BoxedSource, BoxedTrack},
};
pub struct SourceManager {
    pub sources: Vec<BoxedSource>,
    pub mirrors: crate::config::server::MirrorsConfig,
    pub youtube_cipher_manager: Option<Arc<crate::sources::youtube::cipher::YouTubeCipherManager>>,
    pub youtube_stream_ctx: Option<Arc<crate::sources::youtube::YoutubeStreamContext>>,
    pub http_pool: Arc<HttpClientPool>,
    pub player_config: crate::config::player::PlayerConfig,
}
impl SourceManager {
    pub fn new(config: &crate::config::AppConfig) -> Self {
        let http_pool = Arc::new(HttpClientPool::new());
        let mut sources = Vec::new();
        let (youtube_cipher_manager, youtube_stream_ctx) =
            registration::register_all(&mut sources, config, &http_pool);
        Self {
            sources,
            mirrors: config.player.mirrors.clone(),
            youtube_cipher_manager,
            youtube_stream_ctx,
            http_pool,
            player_config: config.player.clone(),
        }
    }
    pub async fn load(
        &self,
        identifier: &str,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> crate::protocol::tracks::LoadResult {
        for source in &self.sources {
            if source.can_handle(identifier) {
                tracing::debug!(
                    "SourceManager: Loading '{}' with source: {}",
                    identifier,
                    source.name()
                );
                return source.load(identifier, routeplanner.clone()).await;
            }
        }
        tracing::debug!(
            "SourceManager: No source matched identifier: '{}'",
            identifier
        );
        crate::protocol::tracks::LoadResult::Empty {}
    }
    pub async fn load_search(
        &self,
        query: &str,
        types: &[String],
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Option<crate::protocol::tracks::SearchResult> {
        for source in &self.sources {
            if source.can_handle(query) {
                tracing::trace!("Loading search '{}' with source: {}", query, source.name());
                return source.load_search(query, types, routeplanner.clone()).await;
            }
        }
        tracing::debug!("No source could handle search query: {}", query);
        None
    }
    pub async fn resolve_track(
        &self,
        track_info: &crate::protocol::tracks::TrackInfo,
        routeplanner: Option<Arc<dyn crate::routeplanner::RoutePlanner>>,
    ) -> Result<BoxedTrack, String> {
        let identifier = track_info.uri.as_deref().unwrap_or(&track_info.identifier);
        for source in &self.sources {
            if source.can_handle(identifier) {
                tracing::trace!(
                    "Resolving playable track for '{}' with source: {}",
                    identifier,
                    source.name()
                );
                if let Some(track) = source.get_track(identifier, routeplanner.clone()).await {
                    return Ok(track);
                }
                break;
            }
        }
        resolver::resolve_with_mirrors(
            self,
            track_info,
            identifier,
            &self.mirrors,
            routeplanner,
        )
        .await
    }
    pub fn source_names(&self) -> Vec<String> {
        self.sources.iter().map(|s| s.name().to_string()).collect()
    }
    pub fn get_proxy_config(&self, source_name: &str) -> Option<crate::config::HttpProxyConfig> {
        self.sources
            .iter()
            .find(|s| s.name() == source_name)
            .and_then(|s| s.get_proxy_config())
    }
}