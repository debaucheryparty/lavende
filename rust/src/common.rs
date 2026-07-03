pub mod errors {
use serde::{Deserialize, Serialize};
use crate::common::utils::now_ms;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Common,
    Suspicious,
    Fault,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LavendeError {
    pub timestamp: u64,
    pub status: u16,
    pub error: String,
    pub message: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
}
impl LavendeError {
    pub fn bad_request(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(400, "Bad Request", message, path)
    }
    pub fn not_found(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(404, "Not Found", message, path)
    }
    pub fn new(
        status: u16,
        error: impl Into<String>,
        message: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: now_ms(),
            status,
            error: error.into(),
            message: message.into(),
            path: path.into(),
            trace: None,
        }
    }
}
}
pub mod types {
use std::{ops::Deref, sync::Arc};
use rand::{Rng, distributions::Alphanumeric};
use tokio::sync::{Mutex, RwLock};
pub type Shared<T> = Arc<Mutex<T>>;
pub type SharedRw<T> = Arc<RwLock<T>>;
pub type AnyError = Box<dyn std::error::Error + Send + Sync>;
pub type AnyResult<T> = std::result::Result<T, AnyError>;
macro_rules! define_id {
    ($name:ident, $type:ty) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub $type);
        impl From<$type> for $name {
            fn from(val: $type) -> Self {
                Self(val)
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
    ($name:ident, $type:ty, copy) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub $type);
        impl From<$type> for $name {
            fn from(val: $type) -> Self {
                Self(val)
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}
define_id!(GuildId, String);
define_id!(SessionId, String);
define_id!(UserId, u64, copy);
define_id!(ChannelId, u64, copy);
impl Deref for GuildId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Deref for SessionId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl SessionId {
    pub fn generate() -> Self {
        let rng = rand::thread_rng();
        let s: String = rng
            .sample_iter(&Alphanumeric)
            .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            .take(16)
            .map(char::from)
            .collect();
        Self(s)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AudioFormat {
    Aac,
    Opus,
    Webm,
    Mp4,
    Mp3,
    Ogg,
    Flac,
    Wav,
    Unknown,
}
impl AudioFormat {
    pub fn as_ext(&self) -> &'static str {
        match self {
            Self::Aac => "aac",
            Self::Opus => "opus",
            Self::Webm => "webm",
            Self::Mp4 => "mp4",
            Self::Mp3 => "mp3",
            Self::Ogg => "ogg",
            Self::Flac => "flac",
            Self::Wav => "wav",
            Self::Unknown => "",
        }
    }
    pub fn from_ext(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "aac" => Self::Aac,
            "opus" => Self::Opus,
            "webm" => Self::Webm,
            "mp4" | "m4a" => Self::Mp4,
            "mp3" => Self::Mp3,
            "ogg" => Self::Ogg,
            "flac" => Self::Flac,
            "wav" => Self::Wav,
            _ => Self::Unknown,
        }
    }
    pub fn from_url(url: &str) -> Self {
        if url.contains(".m3u8") || url.contains("/playlist") {
            return Self::Aac;
        }
        if let Some(itag) = extract_youtube_itag(url) {
            match itag {
                249..=251 => return Self::Webm,
                139..=141 => return Self::Mp4,
                _ => {}
            }
        }
        if url.contains("mime=audio%2Fwebm") || url.contains("mime=audio/webm") {
            return Self::Webm;
        }
        if url.contains("mime=audio%2Fmp4") || url.contains("mime=audio/mp4") {
            return Self::Mp4;
        }
        let from_path = url
            .split('?')
            .next()
            .and_then(|path| std::path::Path::new(path).extension())
            .and_then(|ext| ext.to_str())
            .map(Self::from_ext)
            .unwrap_or(Self::Unknown);
        if from_path != Self::Unknown {
            return from_path;
        }
        if url.contains(".mp4") || url.contains(".m4a") {
            return Self::Mp4;
        }
        if url.contains(".flac") {
            return Self::Flac;
        }
        if url.contains(".mp3") {
            return Self::Mp3;
        }
        if url.contains(".ogg") {
            return Self::Ogg;
        }
        if url.contains(".webm") {
            return Self::Webm;
        }
        Self::Unknown
    }
    pub fn is_opus_passthrough(&self) -> bool {
        matches!(self, Self::Webm | Self::Ogg | Self::Opus)
    }
}
fn extract_youtube_itag(url: &str) -> Option<u32> {
    url.split('?').nth(1)?.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == "itag" { v.parse().ok() } else { None }
    })
}
}

pub mod http {
use std::{sync::Arc, time::Duration};
use dashmap::DashMap;
use reqwest::{Client, Proxy};
use tracing::warn;
use crate::{common::utils::default_user_agent, config::HttpProxyConfig};
pub struct HttpClientPool {
    clients: DashMap<Option<HttpProxyConfig>, Arc<Client>>,
}
impl HttpClientPool {
    pub fn new() -> Self {
        Self {
            clients: DashMap::new(),
        }
    }
    pub fn get(&self, proxy: Option<HttpProxyConfig>) -> Arc<Client> {
        self.clients
            .entry(proxy.clone())
            .or_insert_with(|| Arc::new(self.create_client(proxy)))
            .clone()
    }
    fn create_client(&self, proxy: Option<HttpProxyConfig>) -> Client {
        let mut builder = Client::builder()
            .user_agent(default_user_agent())
            .gzip(true)
            .deflate(true)
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .tcp_nodelay(true)
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(70));
        if let Some(url) = proxy.as_ref().and_then(|config| config.url.as_ref()) {
            match Proxy::all(url) {
                Ok(mut proxy_obj) => {
                    if let Some((u, p)) = proxy
                        .as_ref()
                        .and_then(|c| c.username.as_ref().zip(c.password.as_ref()))
                    {
                        proxy_obj = proxy_obj.basic_auth(u, p);
                    }
                    builder = builder.proxy(proxy_obj);
                }
                Err(e) => {
                    warn!(
                        "HttpClientPool: failed to parse proxy URL '{}': {} — proxy will be ignored",
                        url, e
                    );
                }
            }
        }
        match builder.build() {
            Ok(client) => client,
            Err(e) => {
                warn!(
                    "HttpClientPool: failed to build client ({}), falling back to default",
                    e
                );
                Client::new()
            }
        }
    }
}
impl Default for HttpClientPool {
    fn default() -> Self {
        Self::new()
    }
}
}
pub mod utils {
use std::time::{SystemTime, UNIX_EPOCH};
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ORANGE: &str = "\x1b[38;5;208m";
pub const GREEN: &str = "\x1b[32m";
pub const CYAN: &str = "\x1b[36m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const RED: &str = "\x1b[31m";
pub const COLOR_ERROR: &str = RED;
pub const COLOR_WARN: &str = YELLOW;
pub const COLOR_INFO: &str = GREEN;
pub const COLOR_DEBUG: &str = BLUE;
pub const COLOR_TRACE: &str = MAGENTA;
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}
pub fn strip_ansi_escapes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some('[') = chars.peek() {
                chars.next();
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}
pub fn memory_usage_report() -> String {
    "0 B".to_string()
}
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
pub fn default_user_agent() -> String {
    DEFAULT_USER_AGENT.to_owned()
}
pub fn shorten_error_cause(err: &str) -> String {
    let mut scrubbed = err;
    if let Some(idx) = scrubbed.find(" for https://") {
        scrubbed = &scrubbed[..idx];
    } else if let Some(idx) = scrubbed.find(" for http://") {
        scrubbed = &scrubbed[..idx];
    } else if let Some(idx) = scrubbed.find(" (https://") {
        scrubbed = &scrubbed[..idx];
    } else if let Some(idx) = scrubbed.find(" (http://") {
        scrubbed = &scrubbed[..idx];
    } else if scrubbed.contains("error sending request for url") {
        return "error sending request for url".to_string();
    }
    if let Some(line) = scrubbed.lines().next() {
        if line.len() > 100 {
            return format!("{}...", &line[..97]);
        }
        return line.to_string();
    }
    scrubbed.to_string()
}
}
pub mod logger {
pub mod formatter {
use core::fmt::{self as core_fmt};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    fmt::{
        self, FmtContext,
        format::{FormatEvent, FormatFields},
    },
    registry::LookupSpan,
};
use crate::common::utils::{
    BOLD, COLOR_DEBUG, COLOR_ERROR, COLOR_INFO, COLOR_TRACE, COLOR_WARN, DIM, RESET,
    memory_usage_report,
};
pub struct CustomFormatter {
    use_ansi: bool,
}
impl CustomFormatter {
    pub fn new(use_ansi: bool) -> Self {
        Self { use_ansi }
    }
    fn write_timestamp(&self, writer: &mut fmt::format::Writer<'_>) -> core_fmt::Result {
        let format = time::macros::format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
        );
        let now =
            time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
        let timestamp = now
            .format(&format)
            .unwrap_or_else(|_| "Unknown Time".to_string());
        if self.use_ansi {
            write!(writer, "{DIM}[{timestamp}]{RESET} ")
        } else {
            write!(writer, "[{timestamp}] ")
        }
    }
    fn write_level(&self, writer: &mut fmt::format::Writer<'_>, level: &Level) -> core_fmt::Result {
        let level_str = format!("{: <5}", level);
        if self.use_ansi {
            let color = match *level {
                Level::ERROR => COLOR_ERROR,
                Level::WARN => COLOR_WARN,
                Level::INFO => COLOR_INFO,
                Level::DEBUG => COLOR_DEBUG,
                Level::TRACE => COLOR_TRACE,
            };
            write!(writer, "{color}{BOLD}{level_str}{RESET} ")
        } else {
            write!(writer, "{level_str} ")
        }
    }
}
impl<S, N> FormatEvent<S, N> for CustomFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: fmt::format::Writer<'_>,
        event: &Event<'_>,
    ) -> core_fmt::Result {
        let (reset, dim) = if self.use_ansi {
            (RESET, DIM)
        } else {
            ("", "")
        };
        write!(writer, "{dim}[{}]{reset} ", memory_usage_report())?;
        self.write_timestamp(&mut writer)?;
        let metadata = event.metadata();
        self.write_level(&mut writer, metadata.level())?;
        let target = metadata.target();
        let line = metadata
            .line()
            .map(|l| l.to_string())
            .unwrap_or_else(|| "??".to_string());
        write!(writer, "{dim}{target}: {line}{reset} > ")?;
        ctx.format_fields(writer.by_ref(), event)?;
        write!(writer, "{reset}")?;
        writeln!(writer)
    }
}
}
pub mod writer {
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use parking_lot::Mutex;
fn today_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86400) as u32;
    let (y, m, d) = days_to_ymd(days);
    format!("{y:04}-{m:02}-{d:02}")
}
fn days_to_ymd(mut days: u32) -> (u32, u32, u32) {
    days += 719468;
    let era = days / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
fn resolve_path(base_path: &str, rotate_daily: bool, date: &str) -> String {
    if !rotate_daily {
        return base_path.to_string();
    }
    let base = Path::new(base_path);
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("lavende");
    let dir: PathBuf = base.parent().unwrap_or(Path::new(".")).into();
    dir.join(format!("{stem}-{date}.log"))
        .to_string_lossy()
        .into_owned()
}
#[derive(Clone)]
pub struct CircularFileWriter {
    base_path: String,
    max_lines: u32,
    max_files: u32,
    rotate_daily: bool,
    state: Arc<Mutex<WriterState>>,
}
struct WriterState {
    file: Option<File>,
    current_date: Option<String>,
    lines_since_prune: u32,
    is_pruning: bool,
}
impl CircularFileWriter {
    pub fn new(path: String, max_lines: u32, max_files: u32, rotate_daily: bool) -> Self {
        Self {
            base_path: path,
            max_lines,
            max_files,
            rotate_daily,
            state: Arc::new(Mutex::new(WriterState {
                file: None,
                current_date: None,
                lines_since_prune: 0,
                is_pruning: false,
            })),
        }
    }
    fn current_path(&self) -> String {
        if self.rotate_daily {
            resolve_path(&self.base_path, true, &today_date())
        } else {
            self.base_path.clone()
        }
    }
    fn ensure_file_open<'a>(&self, state: &'a mut WriterState) -> io::Result<&'a mut File> {
        let today = if self.rotate_daily {
            Some(today_date())
        } else {
            None
        };
        let need_rotate = state.file.is_none()
            || match (&state.current_date, &today) {
                (Some(curr), Some(new)) => curr != new,
                _ => false,
            };
        if need_rotate {
            state.file = None;
            let path = if self.rotate_daily {
                let d = today.as_deref().unwrap_or("");
                resolve_path(&self.base_path, true, d)
            } else {
                self.base_path.clone()
            };
            if let Some(parent) = Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            state.file = Some(OpenOptions::new().create(true).append(true).open(&path)?);
            state.current_date = today;
            if self.rotate_daily && self.max_files > 0 {
                self.cleanup_old_files();
            }
        }
        Ok(state.file.as_mut().expect("file was just opened"))
    }
    fn spawn_prune(&self) {
        let path = self.current_path();
        let max_lines = self.max_lines;
        let state_arc = self.state.clone();
        std::thread::spawn(move || {
            if let Err(e) = Self::do_prune(&path, max_lines) {
                eprintln!("Failed to prune log file '{}': {}", path, e);
            }
            let mut state = state_arc.lock();
            state.is_pruning = false;
        });
    }
    fn cleanup_old_files(&self) {
        let base = Path::new(&self.base_path);
        let dir = base.parent().unwrap_or(Path::new("."));
        let stem = base
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("lavende");
        let max_files = self.max_files as usize;
        let mut log_files: Vec<std::path::PathBuf> = match std::fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().and_then(|e| e.to_str()) == Some("log")
                        && p.file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.starts_with(stem) && s != stem)
                            .unwrap_or(false)
                })
                .collect(),
            Err(_) => return,
        };
        if log_files.len() <= max_files {
            return;
        }
        log_files.sort();
        let to_delete = log_files.len() - max_files;
        for path in log_files.iter().take(to_delete) {
            if let Err(e) = std::fs::remove_file(path) {
                eprintln!("Failed to delete old log file '{}': {}", path.display(), e);
            }
        }
    }
    fn do_prune(path: &str, max_lines: u32) -> io::Result<()> {
        if !Path::new(path).exists() {
            return Ok(());
        }
        let lines: Vec<String> = {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            reader.lines().collect::<Result<_, _>>()?
        };
        if lines.len() > max_lines as usize {
            let start = lines.len() - max_lines as usize;
            let tmp_path = format!("{}.tmp", path);
            {
                let mut file = File::create(&tmp_path)?;
                for line in &lines[start..] {
                    writeln!(file, "{}", line)?;
                }
            }
            std::fs::rename(tmp_path, path)?;
        }
        Ok(())
    }
}
impl io::Write for CircularFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut state = self.state.lock();
        let file = self.ensure_file_open(&mut state)?;
        file.write_all(buf)?;
        let new_lines = buf.iter().filter(|&&b| b == b'\n').count() as u32;
        state.lines_since_prune += new_lines;
        let prune_threshold = (self.max_lines / 10).max(50);
        if state.lines_since_prune >= prune_threshold && !state.is_pruning {
            state.is_pruning = true;
            state.lines_since_prune = 0;
            state.file = None; 
            self.spawn_prune();
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        let mut state = self.state.lock();
        if let Some(file) = &mut state.file {
            file.flush()?;
        }
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CircularFileWriter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
#[cfg(test)]
mod tests {
    use std::fs;
    use tracing_subscriber::fmt::MakeWriter;
    use super::*;
    fn cleanup_test_file(path: &str) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(format!("{}.tmp", path));
    }
    #[test]
    fn test_circular_file_writer_new() {
        let writer = CircularFileWriter::new("test_new.log".to_string(), 100, 0, false);
        let state = writer.state.lock();
        assert_eq!(state.lines_since_prune, 0);
        assert!(!state.is_pruning);
        assert!(state.file.is_none());
        cleanup_test_file("test_new.log");
    }
    #[test]
    fn test_write_creates_file() {
        let path = "test_create.log";
        cleanup_test_file(path);
        let mut writer = CircularFileWriter::new(path.to_string(), 100, 0, false);
        let data = b"test line\n";
        let result = writer.write(data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len());
        assert!(Path::new(path).exists());
        cleanup_test_file(path);
    }
    #[test]
    fn test_write_counts_newlines() {
        let path = "test_newlines.log";
        cleanup_test_file(path);
        let mut writer = CircularFileWriter::new(path.to_string(), 1000, 0, false);
        writer.write(b"line1\nline2\nline3\n").unwrap();
        let state = writer.state.lock();
        assert_eq!(state.lines_since_prune, 3);
        cleanup_test_file(path);
    }
    #[test]
    fn test_write_no_newlines() {
        let path = "test_no_newlines.log";
        cleanup_test_file(path);
        let mut writer = CircularFileWriter::new(path.to_string(), 1000, 0, false);
        writer.write(b"no newline here").unwrap();
        let state = writer.state.lock();
        assert_eq!(state.lines_since_prune, 0);
        cleanup_test_file(path);
    }
    #[test]
    fn test_flush() {
        let path = "test_flush.log";
        cleanup_test_file(path);
        let mut writer = CircularFileWriter::new(path.to_string(), 100, 0, false);
        writer.write(b"test\n").unwrap();
        let result = writer.flush();
        assert!(result.is_ok());
        cleanup_test_file(path);
    }
    #[test]
    fn test_flush_without_file() {
        let mut writer =
            CircularFileWriter::new("test_flush_no_file.log".to_string(), 100, 0, false);
        let result = writer.flush();
        assert!(result.is_ok());
        cleanup_test_file("test_flush_no_file.log");
    }
    #[test]
    fn test_clone() {
        let writer = CircularFileWriter::new("test_clone.log".to_string(), 100, 0, false);
        let cloned = writer.clone();
        assert!(Arc::ptr_eq(&writer.state, &cloned.state));
        cleanup_test_file("test_clone.log");
    }
    #[test]
    fn test_make_writer() {
        let writer = CircularFileWriter::new("test_make_writer.log".to_string(), 100, 0, false);
        let made = writer.make_writer();
        assert!(Arc::ptr_eq(&writer.state, &made.state));
        cleanup_test_file("test_make_writer.log");
    }
    #[test]
    fn test_do_prune_nonexistent_file() {
        let result = CircularFileWriter::do_prune("nonexistent_prune.log", 10);
        assert!(result.is_ok());
    }
    #[test]
    fn test_do_prune_small_file() {
        let path = "test_prune_small.log";
        cleanup_test_file(path);
        fs::write(path, "line1\nline2\nline3\n").unwrap();
        let result = CircularFileWriter::do_prune(path, 10);
        assert!(result.is_ok());
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content.lines().count(), 3);
        cleanup_test_file(path);
    }
    #[test]
    fn test_do_prune_large_file() {
        let path = "test_prune_large.log";
        cleanup_test_file(path);
        let mut content = String::new();
        for i in 1..=20 {
            content.push_str(&format!("line{}\n", i));
        }
        fs::write(path, content).unwrap();
        let result = CircularFileWriter::do_prune(path, 10);
        assert!(result.is_ok());
        let pruned = fs::read_to_string(path).unwrap();
        let lines: Vec<&str> = pruned.lines().collect();
        assert_eq!(lines.len(), 10);
        assert_eq!(lines[0], "line11");
        assert_eq!(lines[9], "line20");
        cleanup_test_file(path);
    }
    #[test]
    fn test_resolve_path_no_rotate() {
        let p = resolve_path("./logs/lavende.log", false, "2026-03-13");
        assert_eq!(p, "./logs/lavende.log");
    }
    #[test]
    fn test_resolve_path_rotate() {
        let p = resolve_path("./logs/lavende.log", true, "2026-03-13");
        assert!(p.contains("2026-03-13"));
        assert!(p.ends_with(".log"));
    }
    #[test]
    fn test_today_date_format() {
        let d = today_date();
        let parts: Vec<&str> = d.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 4); 
        assert_eq!(parts[1].len(), 2); 
        assert_eq!(parts[2].len(), 2); 
    }
    #[test]
    fn test_prune_threshold_calculation() {
        let _writer = CircularFileWriter::new("test.log".to_string(), 1000, 0, false);
        let threshold = (1000 / 10).max(50);
        assert_eq!(threshold, 100);
        let _writer = CircularFileWriter::new("test.log".to_string(), 100, 0, false);
        let threshold = (100 / 10).max(50);
        assert_eq!(threshold, 50);
        let _writer = CircularFileWriter::new("test.log".to_string(), 10, 0, false);
        let threshold = (10 / 10).max(50);
        assert_eq!(threshold, 50);
        cleanup_test_file("test.log");
    }
}
}
use std::{fs, path::Path, sync::OnceLock};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use crate::{common::utils::strip_ansi_escapes, config::LoggingConfig};
pub use formatter::CustomFormatter;
pub use writer::CircularFileWriter;
pub(crate) static GLOBAL_FILE_WRITER: OnceLock<CircularFileWriter> = OnceLock::new();
#[macro_export]
macro_rules! log_print {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        std::print!("{}", msg);
        $crate::common::logger::append_to_file_raw(&msg);
    }};
}
#[macro_export]
macro_rules! log_println {
    () => {{
        std::println!();
        $crate::common::logger::append_to_file_raw("\n");
    }};
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        std::println!("{}", msg);
        $crate::common::logger::append_to_file_raw(&format!("{}\n", msg));
    }};
}
pub fn append_to_file_raw(msg: &str) {
    if let Some(mut writer) = GLOBAL_FILE_WRITER.get().cloned() {
        use std::io::Write;
        let clean_msg = strip_ansi_escapes(msg);
        let _ = writer.write_all(clean_msg.as_bytes());
    }
}
pub fn init(config: &LoggingConfig) {
    let _ = tracing_log::LogTracer::init();
    let log_level = config.level.as_deref().unwrap_or("info");
    let filter_str = match config.filters.as_deref() {
        Some(f) if !f.is_empty() => format!("{log_level},{f}"),
        _ => log_level.to_string(),
    };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter_str));
    let stdout_layer = fmt::layer()
        .event_format(CustomFormatter::new(true))
        .with_ansi(true);
    let file_layer = config.file.as_ref().map(|file_config| {
        if let Some(parent) = Path::new(&file_config.path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let writer = CircularFileWriter::new(
            file_config.path.clone(),
            file_config.max_lines,
            file_config.max_files,
            file_config.rotate_daily,
        );
        let _ = GLOBAL_FILE_WRITER.set(writer.clone());
        fmt::layer()
            .with_writer(writer)
            .event_format(CustomFormatter::new(false))
            .with_ansi(false)
    });
    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .try_init();
}
}
pub use errors::*;
pub use http::*;
pub use logger::*;
pub use types::*;
pub use utils::*;