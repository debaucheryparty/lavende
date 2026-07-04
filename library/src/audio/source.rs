pub mod traits {
    use std::io::{Read, Seek};
    use symphonia::core::io::MediaSource;
    pub trait AudioSource: Read + Seek + MediaSource + Send {
        fn content_type(&self) -> Option<String> {
            None
        }
        fn seekable(&self) -> bool {
            self.is_seekable()
        }
    }
}
pub mod segmented {
    use super::AudioSource;
    use crate::{
        audio::constants::{
            CHUNK_SIZE, FETCH_WAIT_MS, MAX_CONCURRENT_FETCHES, MAX_FETCH_RETRIES, PREFETCH_CHUNKS,
            PROBE_TIMEOUT_SECS, WORKER_IDLE_MS,
        },
        common::types::AnyResult,
    };
    use bytes::Bytes;
    use parking_lot::{Condvar, Mutex};
    use std::{
        collections::HashMap,
        io::{Read, Seek, SeekFrom},
        sync::Arc,
        time::Duration,
    };
    use symphonia::core::io::MediaSource;
    use tracing::{debug, trace, warn};
    #[derive(Clone)]
    enum ChunkState {
        Empty(u32),
        Downloading,
        Ready(Bytes),
    }
    struct ReaderState {
        chunks: HashMap<usize, ChunkState>,
        current_pos: u64,
        total_len: u64,
        is_terminated: bool,
        fatal_error: Option<String>,
    }
    pub struct SegmentedSource {
        pos: u64,
        len: u64,
        content_type: Option<Arc<str>>,
        shared: Arc<(Mutex<ReaderState>, Condvar)>,
    }
    impl SegmentedSource {
        pub async fn new(client: reqwest::Client, url: &str) -> AnyResult<Self> {
            let probe = client
                .get(url)
                .header("Range", "bytes=0-0")
                .header("Connection", "close")
                .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
                .send()
                .await?;
            let len = probe
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split('/').next_back())
                .and_then(|v| v.parse::<u64>().ok())
                .or_else(|| probe.content_length())
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "SegmentedSource: could not determine content length",
                    )
                })?;
            let content_type: Option<Arc<str>> = probe
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(Arc::from);
            debug!(
                "SegmentedSource opened: len={}, type={:?}",
                len, content_type
            );
            let mut chunks = HashMap::new();
            chunks.insert(0, ChunkState::Empty(0));
            let shared = Arc::new((
                Mutex::new(ReaderState {
                    chunks,
                    current_pos: 0,
                    total_len: len,
                    is_terminated: false,
                    fatal_error: None,
                }),
                Condvar::new(),
            ));
            for worker_id in 0..MAX_CONCURRENT_FETCHES {
                let shared_clone = shared.clone();
                let client_clone = client.clone();
                let url_str = url.to_string();
                tokio::spawn(async move {
                    fetch_worker(worker_id, shared_clone, client_clone, url_str).await;
                });
            }
            Ok(Self {
                pos: 0,
                len,
                content_type,
                shared,
            })
        }
    }
    impl AudioSource for SegmentedSource {
        fn content_type(&self) -> Option<String> {
            self.content_type.as_deref().map(str::to_string)
        }
    }
    impl Read for SegmentedSource {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.pos >= self.len {
                return Ok(0);
            }
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            state.current_pos = self.pos;
            loop {
                if let Some(ref err) = state.fatal_error {
                    return Err(std::io::Error::other(err.clone()));
                }
                let chunk_idx = (self.pos / CHUNK_SIZE as u64) as usize;
                let offset_in_chunk = (self.pos % CHUNK_SIZE as u64) as usize;
                match state.chunks.get(&chunk_idx) {
                    Some(ChunkState::Ready(bytes)) => {
                        let bytes = bytes.clone();
                        let available = bytes.len().saturating_sub(offset_in_chunk);
                        if available == 0 {
                            self.pos = ((chunk_idx + 1) * CHUNK_SIZE) as u64;
                            state.current_pos = self.pos;
                            continue;
                        }
                        let n = buf.len().min(available);
                        buf[..n].copy_from_slice(&bytes[offset_in_chunk..offset_in_chunk + n]);
                        self.pos += n as u64;
                        state.current_pos = self.pos;
                        if chunk_idx > 1 {
                            state.chunks.retain(|&idx, _| idx >= chunk_idx - 1);
                        }
                        return Ok(n);
                    }
                    Some(ChunkState::Downloading) | Some(ChunkState::Empty(_)) => {
                        cvar.notify_all();
                        trace!("SegmentedSource: waiting for chunk {}", chunk_idx);
                        cvar.wait_for(&mut state, Duration::from_millis(FETCH_WAIT_MS));
                    }
                    None => {
                        state.chunks.insert(chunk_idx, ChunkState::Empty(0));
                        cvar.notify_all();
                    }
                }
            }
        }
    }
    impl Seek for SegmentedSource {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            let new_pos = match pos {
                SeekFrom::Start(p) => p,
                SeekFrom::Current(delta) => self.pos.saturating_add_signed(delta),
                SeekFrom::End(delta) => self.len.saturating_add_signed(delta),
            };
            self.pos = new_pos.min(self.len);
            debug!("SegmentedSource: seek → {}", self.pos);
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            state.current_pos = self.pos;
            cvar.notify_all();
            Ok(self.pos)
        }
    }
    impl MediaSource for SegmentedSource {
        fn is_seekable(&self) -> bool {
            true
        }
        fn byte_len(&self) -> Option<u64> {
            Some(self.len)
        }
    }
    impl Drop for SegmentedSource {
        fn drop(&mut self) {
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            state.is_terminated = true;
            cvar.notify_all();
        }
    }
    async fn fetch_chunk(
        client: &reqwest::Client,
        url: &str,
        offset: u64,
        size: u64,
    ) -> AnyResult<Bytes> {
        let range = format!("bytes={}-{}", offset, offset + size - 1);
        let res = client
            .get(url)
            .header("Range", range)
            .header("Accept", "*/*")
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(format!("fetch_chunk: HTTP {}", res.status()).into());
        }
        Ok(res.bytes().await?)
    }
    async fn fetch_worker(
        worker_id: usize,
        shared: Arc<(Mutex<ReaderState>, Condvar)>,
        client: reqwest::Client,
        url: String,
    ) {
        let (lock, cvar) = &*shared;
        loop {
            let target = {
                let mut state = lock.lock();
                if state.is_terminated {
                    break;
                }
                let current_chunk = (state.current_pos / CHUNK_SIZE as u64) as usize;
                let total_len = state.total_len;
                let claimed = try_claim_chunk(&mut state, current_chunk, total_len);
                if claimed.is_none() {
                    let cursor_ready =
                        matches!(state.chunks.get(&current_chunk), Some(ChunkState::Ready(_)));
                    let window = if cursor_ready { PREFETCH_CHUNKS } else { 2 };
                    let mut found = None;
                    for j in 1..window {
                        let idx = current_chunk + j;
                        if (idx * CHUNK_SIZE) as u64 >= total_len {
                            break;
                        }
                        if let Some(c) = try_claim_chunk(&mut state, idx, total_len) {
                            found = Some(c);
                            break;
                        }
                    }
                    found
                } else {
                    claimed
                }
                .map(|(idx, retries)| {
                    debug!(
                        "Worker {}: claiming chunk {} (retry={})",
                        worker_id, idx, retries
                    );
                    state.chunks.insert(idx, ChunkState::Downloading);
                    (idx, retries, total_len)
                })
            };
            let (idx, prior_retries, total_len) = match target {
                Some(t) => t,
                None => {
                    tokio::time::sleep(Duration::from_millis(WORKER_IDLE_MS)).await;
                    continue;
                }
            };
            let offset = (idx * CHUNK_SIZE) as u64;
            let size = CHUNK_SIZE.min((total_len - offset) as usize) as u64;
            trace!(
                "Worker {}: requesting chunk {} (offset={}, size={})",
                worker_id, idx, offset, size
            );
            match fetch_chunk(&client, &url, offset, size).await {
                Ok(bytes) => {
                    let actual = bytes.len() as u64;
                    if actual != size {
                        warn!(
                            "Worker {}: partial fetch for chunk {} (got {}/{} bytes)",
                            worker_id, idx, actual, size
                        );
                        requeue_or_fatal(
                            lock,
                            cvar,
                            idx,
                            prior_retries,
                            &format!("partial fetch: {}/{} bytes", actual, size),
                        );
                        tokio::time::sleep(Duration::from_millis(FETCH_WAIT_MS)).await;
                        continue;
                    }
                    let mut state = lock.lock();
                    state.chunks.insert(idx, ChunkState::Ready(bytes));
                    trace!(
                        "Worker {}: filled chunk {} ({} bytes)",
                        worker_id, idx, actual
                    );
                    cvar.notify_all();
                }
                Err(e) => {
                    warn!(
                        "Worker {}: fetch failed for chunk {}: {}",
                        worker_id, idx, e
                    );
                    requeue_or_fatal(lock, cvar, idx, prior_retries, &e.to_string());
                    tokio::time::sleep(Duration::from_millis(FETCH_WAIT_MS)).await;
                }
            }
        }
    }
    #[inline]
    fn try_claim_chunk(
        state: &mut ReaderState,
        idx: usize,
        total_len: u64,
    ) -> Option<(usize, u32)> {
        if (idx * CHUNK_SIZE) as u64 >= total_len {
            return None;
        }
        match state.chunks.get(&idx) {
            Some(ChunkState::Empty(r)) => Some((idx, *r)),
            None => Some((idx, 0)),
            _ => None,
        }
    }
    #[inline]
    fn requeue_or_fatal(
        lock: &Mutex<ReaderState>,
        cvar: &Condvar,
        idx: usize,
        prior_retries: u32,
        error: &str,
    ) {
        let mut state = lock.lock();
        if prior_retries >= MAX_FETCH_RETRIES {
            let msg = format!(
                "Chunk {}: permanently failed after {} retries: {}",
                idx, prior_retries, error
            );
            warn!("SegmentedSource: fatal error - {}", msg);
            state.fatal_error = Some(msg);
        } else {
            state
                .chunks
                .insert(idx, ChunkState::Empty(prior_retries + 1));
        }
        cvar.notify_all();
    }
}
pub mod client {
    use crate::{common::types::AnyResult, config::HttpProxyConfig};
    use reqwest::{Client, Proxy, header::HeaderMap};
    use std::{net::IpAddr, time::Duration};
    use tracing::warn;
    pub fn create_client(
        user_agent: String,
        local_addr: Option<IpAddr>,
        proxy: Option<HttpProxyConfig>,
        headers: Option<HeaderMap>,
    ) -> AnyResult<Client> {
        let mut builder = Client::builder()
            .user_agent(user_agent)
            .connect_timeout(Duration::from_secs(5))
            .read_timeout(Duration::from_secs(8))
            .tcp_nodelay(true)
            .tcp_keepalive(Duration::from_secs(25))
            .pool_max_idle_per_host(64)
            .pool_idle_timeout(Duration::from_secs(70));
        if let Some(headers) = headers {
            builder = builder.default_headers(headers);
        }
        if let Some(ip) = local_addr {
            builder = builder.local_address(ip);
        }
        if let Some(p_cfg) = proxy
            && let Some(p_url) = p_cfg.url
        {
            match Proxy::all(&p_url) {
                Ok(mut p) => {
                    if let (Some(u), Some(pw)) = (p_cfg.username, p_cfg.password) {
                        p = p.basic_auth(&u, &pw);
                    }
                    builder = builder.proxy(p);
                }
                Err(e) => warn!("Failed to parse proxy URL '{}': {}", p_url, e),
            }
        }
        Ok(builder.build()?)
    }
}
pub mod http {
    pub mod prefetcher {
        use super::HttpSource;
        use crate::audio::constants::{MAX_FETCH_RETRIES, MAX_HTTP_BUF_BYTES};
        use bytes::Bytes;
        use parking_lot::{Condvar, Mutex};
        use std::{sync::Arc, time::Duration};
        use tracing::{debug, warn};
        #[derive(Debug)]
        pub enum PrefetchCommand {
            Continue,
            Seek(u64),
            Stop,
        }
        pub struct SharedState {
            pub chunks: std::collections::VecDeque<Bytes>,
            pub buffered: usize,
            pub done: bool,
            pub error: Option<String>,
            pub command: PrefetchCommand,
        }
        impl Default for SharedState {
            fn default() -> Self {
                Self::new()
            }
        }
        impl SharedState {
            pub fn new() -> Self {
                Self {
                    chunks: std::collections::VecDeque::with_capacity(64),
                    buffered: 0,
                    done: false,
                    error: None,
                    command: PrefetchCommand::Continue,
                }
            }
            pub fn drain_into(&mut self, dst: &mut [u8]) -> usize {
                let mut written = 0;
                while written < dst.len() {
                    let Some(front) = self.chunks.front_mut() else {
                        break;
                    };
                    let want = dst.len() - written;
                    let have = front.len();
                    if have <= want {
                        dst[written..written + have].copy_from_slice(front);
                        written += have;
                        self.buffered -= have;
                        self.chunks.pop_front();
                    } else {
                        dst[written..].copy_from_slice(&front[..want]);
                        *front = front.slice(want..);
                        self.buffered -= want;
                        written += want;
                        break;
                    }
                }
                written
            }
            pub fn skip(&mut self, mut n: usize) -> usize {
                let total = n;
                while n > 0 {
                    let Some(front) = self.chunks.front_mut() else {
                        break;
                    };
                    let have = front.len();
                    if have <= n {
                        n -= have;
                        self.buffered -= have;
                        self.chunks.pop_front();
                    } else {
                        *front = front.slice(n..);
                        self.buffered -= n;
                        n = 0;
                    }
                }
                total - n
            }
        }
        const SLEEP_SLICE_MS: u64 = 50;
        async fn interruptible_sleep(
            shared: &Arc<(Mutex<SharedState>, Condvar)>,
            total_ms: u64,
        ) -> bool {
            let slices = (total_ms / SLEEP_SLICE_MS).max(1);
            for _ in 0..slices {
                tokio::time::sleep(Duration::from_millis(SLEEP_SLICE_MS)).await;
                if matches!(shared.0.lock().command, PrefetchCommand::Stop) {
                    return true;
                }
            }
            false
        }
        pub async fn prefetch_loop(
            shared: Arc<(Mutex<SharedState>, Condvar)>,
            client: reqwest::Client,
            url: String,
            mut current_pos: u64,
            mut response: Option<reqwest::Response>,
            total_len: Option<u64>,
        ) {
            let mut retry_count: u32 = 0;
            'outer: loop {
                let seek_target: Option<u64> = {
                    let (lock, cvar) = &*shared;
                    let mut state = lock.lock();
                    loop {
                        match std::mem::replace(&mut state.command, PrefetchCommand::Continue) {
                            PrefetchCommand::Stop => break 'outer,
                            PrefetchCommand::Seek(pos) => {
                                state.done = false;
                                state.chunks.clear();
                                state.buffered = 0;
                                cvar.notify_all();
                                break Some(pos);
                            }
                            PrefetchCommand::Continue => {
                                if state.buffered >= MAX_HTTP_BUF_BYTES || state.done {
                                    cvar.wait_for(&mut state, Duration::from_millis(200));
                                    continue;
                                }
                                break None;
                            }
                        }
                    }
                };
                if let Some(target) = seek_target {
                    let forward = target.saturating_sub(current_pos);
                    if forward > 0 && forward <= 256 * 1024 && response.is_some() {
                        debug!("prefetch: socket-skip {} bytes", forward);
                        let mut leftover: Option<Bytes> = None;
                        let res = response.take().unwrap();
                        let skip_result = async {
                            let mut res = res;
                            let mut remaining = forward;
                            while remaining > 0 {
                                match res.chunk().await {
                                    Ok(Some(chunk)) => {
                                        if chunk.len() as u64 <= remaining {
                                            remaining -= chunk.len() as u64;
                                        } else {
                                            leftover = Some(chunk.slice(remaining as usize..));
                                            remaining = 0;
                                        }
                                    }
                                    _ => return Err(()),
                                }
                            }
                            Ok(res)
                        }
                        .await;
                        match skip_result {
                            Ok(r) => {
                                current_pos = target;
                                response = Some(r);
                                if let Some(lo) = leftover {
                                    let (lock, cvar) = &*shared;
                                    let mut state = lock.lock();
                                    state.buffered += lo.len();
                                    state.chunks.push_front(lo);
                                    cvar.notify_all();
                                }
                            }
                            Err(_) => {
                                current_pos = target;
                                response = None;
                            }
                        }
                    } else {
                        current_pos = target;
                        response = None;
                    }
                    retry_count = 0;
                }
                if response.is_none() {
                    match HttpSource::fetch_stream(&client, &url, current_pos, None).await {
                        Ok(r) => {
                            response = Some(r);
                            retry_count = 0;
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            if msg.contains("416") {
                                debug!("prefetch: 416 – reached end of stream");
                                let (lock, cvar) = &*shared;
                                let mut state = lock.lock();
                                state.done = true;
                                cvar.notify_all();
                                while state.done
                                    && matches!(state.command, PrefetchCommand::Continue)
                                {
                                    cvar.wait_for(&mut state, Duration::from_millis(200));
                                }
                                continue;
                            }
                            retry_count += 1;
                            if retry_count > MAX_FETCH_RETRIES {
                                warn!(
                                    "prefetch: fetch failed fatally after {} retries: {}",
                                    MAX_FETCH_RETRIES, e
                                );
                                let (lock, cvar) = &*shared;
                                let mut state = lock.lock();
                                state.error = Some(msg);
                                cvar.notify_all();
                                break 'outer;
                            }
                            let backoff_ms = 100u64 << (retry_count - 1).min(5);
                            warn!(
                                "prefetch: fetch failed (retry {}/{}): {} — backing off {}ms",
                                retry_count, MAX_FETCH_RETRIES, e, backoff_ms
                            );
                            if interruptible_sleep(&shared, backoff_ms).await {
                                break 'outer;
                            }
                            continue;
                        }
                    }
                }
                {
                    let (lock, cvar) = &*shared;
                    let mut state = lock.lock();
                    while state.buffered >= MAX_HTTP_BUF_BYTES
                        && matches!(state.command, PrefetchCommand::Continue)
                        && !state.done
                    {
                        cvar.wait_for(&mut state, Duration::from_millis(100));
                    }
                    if !matches!(state.command, PrefetchCommand::Continue) {
                        continue;
                    }
                }
                let res = response.as_mut().unwrap();
                match res.chunk().await {
                    Ok(Some(chunk)) => {
                        let n = chunk.len();
                        let (lock, cvar) = &*shared;
                        let mut state = lock.lock();
                        if !matches!(state.command, PrefetchCommand::Continue) {
                            continue;
                        }
                        current_pos += n as u64;
                        state.buffered += n;
                        state.chunks.push_back(chunk);
                        cvar.notify_all();
                    }
                    Ok(None) => {
                        response = None;
                        retry_count = 0;
                        let is_eof = total_len.is_none_or(|l| current_pos >= l);
                        if is_eof {
                            let (lock, cvar) = &*shared;
                            let mut state = lock.lock();
                            state.done = true;
                            cvar.notify_all();
                            while state.done && matches!(state.command, PrefetchCommand::Continue) {
                                cvar.wait_for(&mut state, Duration::from_millis(200));
                            }
                        }
                    }
                    Err(e) => {
                        response = None;
                        retry_count += 1;
                        if retry_count > MAX_FETCH_RETRIES {
                            warn!(
                                "prefetch: read failed fatally after {} retries: {}",
                                MAX_FETCH_RETRIES, e
                            );
                            let (lock, cvar) = &*shared;
                            let mut state = lock.lock();
                            state.error = Some(e.to_string());
                            cvar.notify_all();
                            break 'outer;
                        }
                        let backoff_ms = 50u64 << (retry_count - 1).min(5);
                        warn!(
                            "prefetch: read error (retry {}/{}): {} — backing off {}ms",
                            retry_count, MAX_FETCH_RETRIES, e, backoff_ms
                        );
                        if interruptible_sleep(&shared, backoff_ms).await {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
    use super::AudioSource;
    use crate::common::types::AnyResult;
    use parking_lot::{Condvar, Mutex};
    use prefetcher::{PrefetchCommand, SharedState, prefetch_loop};
    use std::{
        io::{Read, Seek, SeekFrom},
        sync::Arc,
        thread,
    };
    use symphonia::core::io::MediaSource;
    use tracing::debug;
    pub struct HttpSource {
        pos: u64,
        len: Option<u64>,
        content_type: Option<String>,
        shared: Arc<(Mutex<SharedState>, Condvar)>,
    }
    impl HttpSource {
        pub async fn new(client: reqwest::Client, url: &str) -> AnyResult<Self> {
            let response = Self::fetch_stream(&client, url, 0, None).await?;
            let len = response
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split('/').next_back())
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| response.content_length());
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);
            debug!("HttpSource opened: {} (len={:?})", url, len);
            let shared = Arc::new((Mutex::new(SharedState::new()), Condvar::new()));
            let shared_clone = Arc::clone(&shared);
            let url_clone = url.to_string();
            thread::Builder::new()
                .name("http-prefetch".into())
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(prefetch_loop(
                        shared_clone,
                        client,
                        url_clone,
                        0,
                        Some(response),
                        len,
                    ));
                })?;
            Ok(Self {
                pos: 0,
                len,
                content_type,
                shared,
            })
        }
        pub(crate) async fn fetch_stream(
            client: &reqwest::Client,
            url: &str,
            offset: u64,
            limit: Option<u64>,
        ) -> AnyResult<reqwest::Response> {
            let range = match limit {
                Some(l) => format!("bytes={}-{}", offset, offset + l - 1),
                None => format!("bytes={}-", offset),
            };
            let res = client
                .get(url)
                .header("Accept", "*/*")
                .header("Accept-Encoding", "identity")
                .header("Connection", "keep-alive")
                .header("Range", &range)
                .send()
                .await?;
            if !res.status().is_success() {
                return Err(format!("HTTP {} for {}", res.status(), url).into());
            }
            Ok(res)
        }
    }
    impl AudioSource for HttpSource {
        fn content_type(&self) -> Option<String> {
            self.content_type.clone()
        }
    }
    impl Read for HttpSource {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            loop {
                if !state.chunks.is_empty() || state.done || state.error.is_some() {
                    break;
                }
                cvar.wait_for(&mut state, std::time::Duration::from_millis(100));
            }
            if let Some(err) = state.error.take() {
                return Err(std::io::Error::other(err));
            }
            let n = state.drain_into(buf);
            if state.buffered < crate::audio::constants::HTTP_PREFETCH_BUFFER_SIZE {
                cvar.notify_one();
            }
            self.pos += n as u64;
            Ok(n)
        }
    }
    impl Seek for HttpSource {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            let new_pos = match pos {
                SeekFrom::Start(p) => p,
                SeekFrom::Current(d) => self.pos.saturating_add_signed(d),
                SeekFrom::End(d) => {
                    let l = self.len.ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::Unsupported,
                            "stream length unknown",
                        )
                    })?;
                    l.saturating_add_signed(d)
                }
            };
            if new_pos == self.pos {
                return Ok(self.pos);
            }
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            let forward = new_pos.saturating_sub(self.pos);
            if forward > 0 && forward <= state.buffered as u64 {
                debug!("HttpSource: in-memory seek +{} bytes", forward);
                state.skip(forward as usize);
                self.pos = new_pos;
                return Ok(self.pos);
            }
            debug!("HttpSource: hard seek {} → {}", self.pos, new_pos);
            state.chunks.clear();
            state.buffered = 0;
            state.done = false;
            state.error = None;
            state.command = PrefetchCommand::Seek(new_pos);
            cvar.notify_all();
            self.pos = new_pos;
            Ok(self.pos)
        }
    }
    impl MediaSource for HttpSource {
        fn is_seekable(&self) -> bool {
            self.len.is_some()
        }
        fn byte_len(&self) -> Option<u64> {
            self.len
        }
    }
    impl Drop for HttpSource {
        fn drop(&mut self) {
            let (lock, cvar) = &*self.shared;
            let mut state = lock.lock();
            state.command = PrefetchCommand::Stop;
            cvar.notify_all();
        }
    }
}
pub use client::create_client;
pub use http::HttpSource;
pub use segmented::SegmentedSource;
pub use traits::AudioSource;
