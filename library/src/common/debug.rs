use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, trace, warn};

static GLOBAL_START: AtomicU64 = AtomicU64::new(0);

pub fn init_debug_timer() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    GLOBAL_START.store(now, Ordering::Release);
}

fn elapsed_ms() -> u64 {
    let start = GLOBAL_START.load(Ordering::Acquire);
    if start == 0 {
        return 0;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    now.saturating_sub(start)
}

pub struct DebugTimer {
    start: Instant,
    label: &'static str,
}

impl DebugTimer {
    pub fn new(label: &'static str) -> Self {
        trace!(target: "lavende_core::timing", label, "Started");
        Self {
            start: Instant::now(),
            label,
        }
    }
}

impl Drop for DebugTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        if elapsed > Duration::from_millis(100) {
            warn!(
                target: "lavende_core::timing",
                label = self.label,
                duration_ms = elapsed.as_millis(),
                "Slow operation detected"
            );
        } else {
            trace!(
                target: "lavende_core::timing",
                label = self.label,
                duration_ms = elapsed.as_millis(),
                "Completed"
            );
        }
    }
}

#[macro_export]
macro_rules! debug_player {
    ($guild_id:expr, $event:expr, $($key:ident = $val:expr),* $(,)?) => {
        tracing::debug!(
            target: "lavende_core::player",
            guild_id = %$guild_id,
            event = $event,
            elapsed_ms = $crate::common::debug::elapsed_ms(),
            $($key = ?$val),*
        );
    };
    ($guild_id:expr, $event:expr) => {
        tracing::debug!(
            target: "lavende_core::player",
            guild_id = %$guild_id,
            event = $event,
            elapsed_ms = $crate::common::debug::elapsed_ms(),
        );
    };
}

#[macro_export]
macro_rules! debug_mixer {
    ($($key:ident = $val:expr),* $(,)?) => {
        tracing::trace!(
            target: "lavende_core::mixer",
            elapsed_ms = $crate::common::debug::elapsed_ms(),
            $($key = ?$val),*
        );
    };
}

#[macro_export]
macro_rules! debug_decoder {
    ($track_id:expr, $event:expr, $($key:ident = $val:expr),* $(,)?) => {
        tracing::debug!(
            target: "lavende_core::decoder",
            track_id = %$track_id,
            event = $event,
            elapsed_ms = $crate::common::debug::elapsed_ms(),
            $($key = ?$val),*
        );
    };
}

#[macro_export]
macro_rules! debug_http {
    ($url:expr, $event:expr, $($key:ident = $val:expr),* $(,)?) => {
        tracing::debug!(
            target: "lavende_core::http",
            url = %$url,
            event = $event,
            elapsed_ms = $crate::common::debug::elapsed_ms(),
            $($key = ?$val),*
        );
    };
}

#[macro_export]
macro_rules! debug_buffer {
    ($($key:ident = $val:expr),* $(,)?) => {
        tracing::trace!(
            target: "lavende_core::buffer",
            elapsed_ms = $crate::common::debug::elapsed_ms(),
            $($key = ?$val),*
        );
    };
}

#[macro_export]
macro_rules! error_with_context {
    ($error:expr, $context:expr, $($key:ident = $val:expr),* $(,)?) => {
        tracing::error!(
            target: "lavende_core::error",
            error = %$error,
            context = $context,
            elapsed_ms = $crate::common::debug::elapsed_ms(),
            $($key = ?$val),*
        );
    };
}

#[macro_export]
macro_rules! warn_with_context {
    ($message:expr, $context:expr, $($key:ident = $val:expr),* $(,)?) => {
        tracing::warn!(
            target: "lavende_core::warning",
            message = $message,
            context = $context,
            elapsed_ms = $crate::common::debug::elapsed_ms(),
            $($key = ?$val),*
        );
    };
}

pub fn log_player_state(
    guild_id: &str,
    state: &str,
    position: u64,
    volume: f32,
    paused: bool,
    buffering: bool,
) {
    debug!(
        target: "lavende_core::player::state",
        guild_id,
        state,
        position_ms = position,
        volume,
        paused,
        buffering,
        elapsed_ms = elapsed_ms(),
    );
}

pub fn log_mixer_stats(
    active_tracks: usize,
    active_layers: usize,
    frames_mixed: u64,
    underruns: u64,
) {
    trace!(
        target: "lavende_core::mixer::stats",
        active_tracks,
        active_layers,
        frames_mixed,
        underruns,
        elapsed_ms = elapsed_ms(),
    );
}

pub fn log_buffer_pool_stats(total_bytes: usize, buckets: usize, entries: usize) {
    trace!(
        target: "lavende_core::buffer::pool",
        total_bytes,
        buckets,
        entries,
        utilization_pct = (total_bytes as f64 / (4 * 1024 * 1024) as f64 * 100.0),
        elapsed_ms = elapsed_ms(),
    );
}

pub fn log_http_request(url: &str, method: &str, status: u16, duration_ms: u64, bytes: usize) {
    if status >= 400 {
        warn!(
            target: "lavende_core::http::request",
            url,
            method,
            status,
            duration_ms,
            bytes,
            elapsed_ms = elapsed_ms(),
        );
    } else {
        debug!(
            target: "lavende_core::http::request",
            url,
            method,
            status,
            duration_ms,
            bytes,
            elapsed_ms = elapsed_ms(),
        );
    }
}

pub fn log_decode_error(track_id: &str, error: &str, sample_rate: u32, channels: usize) {
    error!(
        target: "lavende_core::decoder::error",
        track_id,
        error,
        sample_rate,
        channels,
        elapsed_ms = elapsed_ms(),
    );
}

pub fn log_track_loaded(
    track_id: &str,
    source: &str,
    duration_ms: u64,
    sample_rate: u32,
    channels: usize,
    codec: &str,
) {
    info!(
        target: "lavende_core::track::loaded",
        track_id,
        source,
        duration_ms,
        sample_rate,
        channels,
        codec,
        elapsed_ms = elapsed_ms(),
    );
}

pub fn log_gateway_event(guild_id: &str, event: &str, speaking: bool, ssrc: u32) {
    debug!(
        target: "lavende_core::gateway::event",
        guild_id,
        event,
        speaking,
        ssrc,
        elapsed_ms = elapsed_ms(),
    );
}

pub fn log_voice_connection(guild_id: &str, endpoint: &str, connected: bool, latency_ms: i64) {
    if connected {
        info!(
            target: "lavende_core::voice::connection",
            guild_id,
            endpoint,
            connected,
            latency_ms,
            elapsed_ms = elapsed_ms(),
        );
    } else {
        warn!(
            target: "lavende_core::voice::connection",
            guild_id,
            endpoint,
            connected,
            elapsed_ms = elapsed_ms(),
        );
    }
}

pub fn log_memory_stats(heap_bytes: usize, pool_bytes: usize, buffer_count: usize) {
    trace!(
        target: "lavende_core::memory",
        heap_bytes,
        pool_bytes,
        buffer_count,
        total_mb = (heap_bytes + pool_bytes) as f64 / (1024.0 * 1024.0),
        elapsed_ms = elapsed_ms(),
    );
}

pub fn log_performance_warning(
    component: &str,
    operation: &str,
    duration_ms: u64,
    threshold_ms: u64,
) {
    warn!(
        target: "lavende_core::performance",
        component,
        operation,
        duration_ms,
        threshold_ms,
        overhead_pct = ((duration_ms as f64 / threshold_ms as f64 - 1.0) * 100.0),
        elapsed_ms = elapsed_ms(),
    );
}
