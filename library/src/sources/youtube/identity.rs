pub mod ua {
    pub const ANDROID: &str = "com.google.android.youtube/20.01.35 (Linux; U; Android 14) identity";
    pub const ANDROID_VR: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8 Pro Build/UQ1A.240205.002; wv) \
         AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 \
         Chrome/121.0.6167.164 Mobile Safari/537.36 YouTubeVR/1.42.15 (gzip)";
    pub const IOS: &str =
        "com.google.ios.youtube/21.02.1 (iPhone16,2; U; CPU iOS 18_2 like Mac OS X;)";
    pub const TVHTML5: &str = "Mozilla/5.0 (Fuchsia) AppleWebKit/537.36 (KHTML, like Gecko) \
         Chrome/140.0.0.0 Safari/537.36 CrKey/1.56.500000";
    pub const MWEB: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_2 like Mac OS X) \
         AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
    pub const WEB_EMBEDDED: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36,gzip(gfe)";
    pub const TVHTML5_SIMPLY: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36";
    pub const TVHTML5_UNPLUGGED: &str = "Mozilla/5.0 (Linux armeabi-v7a; Android 7.1.2; Fire OS 6.0) \
         Cobalt/22.lts.3.306369-gold (unlike Gecko) v8/8.8.278.8-jit gles Starboard/13, \
         Amazon_ATV_mediatek8695_2019/NS6294 (Amazon, AFTMM, Wireless) \
         com.amazon.firetv.youtube/22.3.r2.v66.0";
    pub const WEB: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
         (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";
}

pub fn cdn_user_agent(stream_url: &str) -> Option<&'static str> {
    if !stream_url.contains("googlevideo.com") && !stream_url.contains("youtube.com") {
        return None;
    }
    let client = url_query_param(stream_url, "c")?;
    match client {
        "ANDROID" => Some(ua::ANDROID),
        "ANDROID_VR" => Some(ua::ANDROID_VR),
        "IOS" => Some(ua::IOS),
        "TVHTML5" => Some(ua::TVHTML5),
        "MWEB" => Some(ua::MWEB),
        "WEB_EMBEDDED_PLAYER" => Some(ua::WEB_EMBEDDED),
        "TVHTML5_SIMPLY" => Some(ua::TVHTML5_SIMPLY),
        "TVHTML5_UNPLUGGED" => Some(ua::TVHTML5_UNPLUGGED),
        _ => None,
    }
}

fn url_query_param<'a>(url: &'a str, key: &str) -> Option<&'a str> {
    let query = url.split_once('?')?.1;
    let prefix = format!("{}=", key);
    for part in query.split('&') {
        if let Some(val) = part.strip_prefix(prefix.as_str()) {
            return Some(val.split('#').next().unwrap_or(val));
        }
    }
    None
}
