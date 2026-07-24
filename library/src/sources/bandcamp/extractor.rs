use regex::Regex;
use std::sync::OnceLock;

pub fn stream_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https?://t4\.bcbits\.com/stream/[a-zA-Z0-9]+/mp3-128/\d+\?p=\d+&amp;ts=\d+&amp;t=[a-zA-Z0-9]+&amp;token=\d+_[a-zA-Z0-9]+").unwrap())
}

pub fn url_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?i)^https?://(?P<subdomain>[^/]+)\.bandcamp\.com/(?P<type>track|album)/(?P<slug>[^/?]+)").unwrap())
}

pub fn identifier_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^(?P<subdomain>[a-zA-Z0-9\-]+):(?P<slug>[a-zA-Z0-9\-]+)$").unwrap()
    })
}

pub fn result_blocks_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?s)<li class=.searchresult data-search.[\s\S]*?</li>").unwrap()
    })
}

pub fn art_url_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"<a class="artcont" href="([^"]+)">"#).unwrap())
}

pub fn title_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?s)<div class="heading">\s*<a[^>]*>\s*(.+?)\s*</a>"#).unwrap()
    })
}

pub fn subhead_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"(?s)<div class="subhead">([\s\S]*?)</div>"#).unwrap())
}

pub fn artwork_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"(?s)<div class="art">\s*<img src="([^"]+)""#).unwrap())
}

pub fn tralbum_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"data-tralbum=["'](.+?)["']"#).unwrap())
}

pub fn extract_stream_url(body: &str) -> Option<String> {
    stream_pattern()
        .find(body)
        .map(|m| m.as_str().replace("&amp;", "&"))
}

pub fn get_identifier_from_url(url: &str) -> String {
    if let Some(caps) = url_pattern().captures(url) {
        return format!("{}:{}", &caps["subdomain"], &caps["slug"]);
    }
    url.to_owned()
}
