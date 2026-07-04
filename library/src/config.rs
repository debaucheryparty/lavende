pub mod sources {
    pub mod amazonmusic {
        use crate::config::sources::HttpProxyConfig;
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone, Default)]
        #[serde(default)]
        pub struct AmazonMusicConfig {
            pub enabled: bool,
            #[serde(default = "default_search_limit")]
            pub search_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
            pub api_url: Option<String>,
        }
        fn default_search_limit() -> usize {
            3
        }
    }
    pub mod anghami {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_limit_10, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct AnghamiConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for AnghamiConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    search_limit: 10,
                    proxy: None,
                }
            }
        }
    }
    pub mod applemusic {
        use super::HttpProxyConfig;
        use crate::config::sources::{
            default_country_code, default_five, default_true, default_zero,
        };
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct AppleMusicConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_country_code")]
            pub country_code: String,
            pub media_api_token: Option<String>,
            #[serde(default = "default_zero")]
            pub playlist_load_limit: usize,
            #[serde(default = "default_zero")]
            pub album_load_limit: usize,
            #[serde(default = "default_five")]
            pub playlist_page_load_concurrency: usize,
            #[serde(default = "default_five")]
            pub album_page_load_concurrency: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for AppleMusicConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    country_code: default_country_code(),
                    media_api_token: None,
                    playlist_load_limit: 0,
                    album_load_limit: 0,
                    playlist_page_load_concurrency: 5,
                    album_page_load_concurrency: 5,
                    proxy: None,
                }
            }
        }
    }
    pub mod audiomack {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_limit_20, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct AudiomackConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_limit_20")]
            pub search_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for AudiomackConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    search_limit: 20,
                    proxy: None,
                }
            }
        }
    }
    pub mod audius {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_limit_10, default_limit_100, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct AudiusConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            #[serde(default = "default_limit_100")]
            pub playlist_load_limit: usize,
            #[serde(default = "default_limit_100")]
            pub album_load_limit: usize,
            #[serde(default)]
            pub app_name: Option<String>,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for AudiusConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    search_limit: 10,
                    playlist_load_limit: 100,
                    album_load_limit: 100,
                    app_name: None,
                    proxy: None,
                }
            }
        }
    }
    pub mod bandcamp {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_limit_10, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct BandcampConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for BandcampConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    search_limit: 10,
                    proxy: None,
                }
            }
        }
    }
    pub mod deezer {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_false, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct DeezerConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            pub arls: Option<Vec<String>>,
            pub master_decryption_key: Option<String>,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for DeezerConfig {
            fn default() -> Self {
                Self {
                    enabled: default_false(),
                    arls: None,
                    master_decryption_key: None,
                    proxy: None,
                }
            }
        }
    }
    pub mod flowery {
        use crate::config::sources::{default_false, default_zero};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct FloweryConfig {
            #[serde(default = "crate::config::sources::default_true")]
            pub enabled: bool,
            #[serde(default = "default_voice")]
            pub voice: String,
            #[serde(default = "default_false")]
            pub translate: bool,
            #[serde(default = "default_zero")]
            pub silence: usize,
            #[serde(default = "default_speed")]
            pub speed: f32,
            #[serde(default = "default_false")]
            pub enforce_config: bool,
        }
        impl Default for FloweryConfig {
            fn default() -> Self {
                Self {
                    enabled: false,
                    voice: default_voice(),
                    translate: false,
                    silence: 0,
                    speed: default_speed(),
                    enforce_config: false,
                }
            }
        }
        fn default_voice() -> String {
            "Salli".to_string()
        }
        fn default_speed() -> f32 {
            1.0
        }
    }
    pub mod gaana {
        use super::HttpProxyConfig;
        use crate::config::sources::{
            default_limit_10, default_limit_20, default_limit_50, default_true,
        };
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct GaanaConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            pub proxy: Option<HttpProxyConfig>,
            pub stream_quality: Option<String>,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            #[serde(default = "default_limit_50")]
            pub playlist_load_limit: usize,
            #[serde(default = "default_limit_50")]
            pub album_load_limit: usize,
            #[serde(default = "default_limit_20")]
            pub artist_load_limit: usize,
        }
        impl Default for GaanaConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    proxy: None,
                    stream_quality: None,
                    search_limit: 10,
                    playlist_load_limit: 50,
                    album_load_limit: 50,
                    artist_load_limit: 20,
                }
            }
        }
    }
    pub mod google_tts {
        use crate::config::sources::default_true;
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct GoogleTtsConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_language")]
            pub language: String,
        }
        impl Default for GoogleTtsConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    language: default_language(),
                }
            }
        }
        fn default_language() -> String {
            "en-US".to_string()
        }
    }
    pub mod http {
        use crate::config::sources::default_true;
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct HttpSourceConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
        }
        impl Default for HttpSourceConfig {
            fn default() -> Self {
                Self { enabled: true }
            }
        }
    }
    pub mod jiosaavn {
        use super::HttpProxyConfig;
        use crate::config::sources::{
            default_limit_10, default_limit_20, default_limit_50, default_true,
        };
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct JioSaavnConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(rename = "apiUrl")]
            pub api_url: Option<String>,
            pub decryption: Option<JioSaavnDecryptionConfig>,
            pub proxy: Option<HttpProxyConfig>,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            #[serde(default = "default_limit_10")]
            pub recommendations_limit: usize,
            #[serde(default = "default_limit_50")]
            pub playlist_load_limit: usize,
            #[serde(default = "default_limit_50")]
            pub album_load_limit: usize,
            #[serde(default = "default_limit_20")]
            pub artist_load_limit: usize,
        }
        impl Default for JioSaavnConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    api_url: None,
                    decryption: None,
                    proxy: None,
                    search_limit: 10,
                    recommendations_limit: 10,
                    playlist_load_limit: 50,
                    album_load_limit: 50,
                    artist_load_limit: 20,
                }
            }
        }
        #[derive(Debug, Deserialize, Serialize, Clone, Default)]
        pub struct JioSaavnDecryptionConfig {
            #[serde(rename = "secretKey")]
            pub secret_key: Option<String>,
        }
    }
    pub mod lastfm {
        use super::{default_false, default_limit_10};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone, Default)]
        #[serde(default)]
        pub struct LastFmConfig {
            #[serde(default = "default_false")]
            pub enabled: bool,
            pub api_key: Option<String>,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
        }
    }
    pub mod local {
        use crate::config::sources::default_true;
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct LocalSourceConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
        }
        impl Default for LocalSourceConfig {
            fn default() -> Self {
                Self { enabled: true }
            }
        }
    }
    pub mod mixcloud {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_limit_10, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct MixcloudConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for MixcloudConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    search_limit: 10,
                    proxy: None,
                }
            }
        }
    }
    pub mod netease {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_false, default_limit_10};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct NeteaseMusicConfig {
            #[serde(default = "default_false")]
            pub enabled: bool,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for NeteaseMusicConfig {
            fn default() -> Self {
                Self {
                    enabled: false,
                    search_limit: 10,
                    proxy: None,
                }
            }
        }
    }
    pub mod pandora {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_limit_10, default_limit_100, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct PandoraConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            pub proxy: Option<HttpProxyConfig>,
            pub csrf_token: Option<String>,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            #[serde(default = "default_limit_100")]
            pub playlist_load_limit: usize,
        }
        impl Default for PandoraConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    proxy: None,
                    csrf_token: None,
                    search_limit: 10,
                    playlist_load_limit: 100,
                }
            }
        }
    }
    pub mod qobuz {
        use super::HttpProxyConfig;
        use crate::config::sources::{
            default_limit_10, default_limit_20, default_limit_50, default_limit_100, default_true,
        };
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct QobuzConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            pub user_token: Option<String>,
            pub app_id: Option<String>,
            pub app_secret: Option<String>,
            pub proxy: Option<HttpProxyConfig>,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            #[serde(default = "default_limit_100")]
            pub playlist_load_limit: usize,
            #[serde(default = "default_limit_50")]
            pub album_load_limit: usize,
            #[serde(default = "default_limit_20")]
            pub artist_load_limit: usize,
        }
        impl Default for QobuzConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    user_token: None,
                    app_id: None,
                    app_secret: None,
                    proxy: None,
                    search_limit: 10,
                    playlist_load_limit: 100,
                    album_load_limit: 50,
                    artist_load_limit: 20,
                }
            }
        }
    }
    pub mod reddit {
        use crate::config::sources::HttpProxyConfig;
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone, Default)]
        #[serde(default)]
        pub struct RedditConfig {
            pub enabled: bool,
            pub proxy: Option<HttpProxyConfig>,
        }
    }
    pub mod shazam {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_limit_10, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct ShazamConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for ShazamConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    search_limit: 10,
                    proxy: None,
                }
            }
        }
    }
    pub mod soundcloud {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_limit_10, default_limit_100, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct SoundCloudConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            pub client_id: Option<String>,
            pub proxy: Option<HttpProxyConfig>,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            #[serde(default = "default_limit_100")]
            pub playlist_load_limit: usize,
        }
        impl Default for SoundCloudConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    client_id: None,
                    proxy: None,
                    search_limit: 10,
                    playlist_load_limit: 100,
                }
            }
        }
    }
    pub mod spotify {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_limit_10, default_limit_50, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct SpotifyConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_limit_6")]
            pub playlist_load_limit: usize,
            #[serde(default = "default_limit_6")]
            pub album_load_limit: usize,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            #[serde(default = "default_limit_10")]
            pub recommendations_limit: usize,
            #[serde(default = "default_limit_10")]
            pub playlist_page_load_concurrency: usize,
            #[serde(default = "default_limit_5")]
            pub album_page_load_concurrency: usize,
            #[serde(default = "default_limit_50")]
            pub track_resolve_concurrency: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for SpotifyConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    playlist_load_limit: 6,
                    album_load_limit: 6,
                    search_limit: 10,
                    recommendations_limit: 10,
                    playlist_page_load_concurrency: 10,
                    album_page_load_concurrency: 5,
                    track_resolve_concurrency: 50,
                    proxy: None,
                }
            }
        }
        fn default_limit_6() -> usize {
            6
        }
        fn default_limit_5() -> usize {
            5
        }
    }
    pub mod tidal {
        use super::HttpProxyConfig;
        use crate::config::sources::{
            default_country_code, default_false, default_limit_20, default_limit_50,
            default_tidal_quality, default_true,
        };
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct TidalConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default = "default_country_code")]
            pub country_code: String,
            #[serde(default = "default_tidal_quality")]
            pub quality: String,
            pub refresh_token: Option<String>,
            #[serde(default = "default_false")]
            pub get_oauth_token: bool,
            #[serde(default = "default_limit_50")]
            pub playlist_load_limit: usize,
            #[serde(default = "default_limit_50")]
            pub album_load_limit: usize,
            #[serde(default = "default_limit_20")]
            pub artist_load_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for TidalConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    country_code: default_country_code(),
                    quality: default_tidal_quality(),
                    refresh_token: None,
                    get_oauth_token: false,
                    playlist_load_limit: 50,
                    album_load_limit: 50,
                    artist_load_limit: 20,
                    proxy: None,
                }
            }
        }
    }
    pub mod twitch {
        use crate::config::sources::HttpProxyConfig;
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone, Default)]
        #[serde(default)]
        pub struct TwitchConfig {
            pub enabled: bool,
            pub client_id: Option<String>,
            pub proxy: Option<HttpProxyConfig>,
        }
    }
    pub mod vkmusic {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_false, default_limit_10};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct VkMusicConfig {
            #[serde(default = "default_false")]
            pub enabled: bool,
            pub user_token: Option<String>,
            pub user_cookie: Option<String>,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
            #[serde(default = "default_one")]
            pub playlist_load_limit: usize,
            #[serde(default = "default_one")]
            pub artist_load_limit: usize,
            #[serde(default = "default_limit_10")]
            pub recommendations_load_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
        }
        impl Default for VkMusicConfig {
            fn default() -> Self {
                Self {
                    enabled: false,
                    user_token: None,
                    user_cookie: None,
                    search_limit: 10,
                    playlist_load_limit: 1,
                    artist_load_limit: 1,
                    recommendations_load_limit: 10,
                    proxy: None,
                }
            }
        }
        fn default_one() -> usize {
            1
        }
    }
    pub mod yandexmusic {
        use super::HttpProxyConfig;
        use crate::config::sources::{default_false, default_limit_10, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct YandexMusicConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            pub access_token: Option<String>,
            #[serde(default = "default_limit_6")]
            pub playlist_load_limit: usize,
            #[serde(default = "default_limit_6")]
            pub album_load_limit: usize,
            #[serde(default = "default_limit_6")]
            pub artist_load_limit: usize,
            pub proxy: Option<HttpProxyConfig>,
            #[serde(default = "default_limit_10")]
            pub search_limit: usize,
        }
        impl Default for YandexMusicConfig {
            fn default() -> Self {
                Self {
                    enabled: default_false(),
                    access_token: None,
                    playlist_load_limit: 6,
                    album_load_limit: 6,
                    artist_load_limit: 6,
                    proxy: None,
                    search_limit: 10,
                }
            }
        }
        fn default_limit_6() -> usize {
            6
        }
    }
    pub mod youtube {
        use crate::config::sources::{default_false, default_true};
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct YouTubeConfig {
            #[serde(default = "default_true")]
            pub enabled: bool,
            #[serde(default)]
            pub clients: YouTubeClientsConfig,
            #[serde(default)]
            pub cipher: YouTubeCipherConfig,
            #[serde(default)]
            pub refresh_tokens: Vec<String>,
            #[serde(default = "default_false")]
            pub get_oauth_token: bool,
        }
        impl Default for YouTubeConfig {
            fn default() -> Self {
                Self {
                    enabled: true,
                    clients: YouTubeClientsConfig::default(),
                    cipher: YouTubeCipherConfig::default(),
                    refresh_tokens: Vec::new(),
                    get_oauth_token: false,
                }
            }
        }
        #[derive(Debug, Deserialize, Serialize, Clone, Default)]
        #[serde(default)]
        pub struct YouTubeCipherConfig {
            pub url: Option<String>,
            pub token: Option<String>,
        }
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct YouTubeClientsConfig {
            #[serde(default = "default_search_clients")]
            pub search: Vec<String>,
            #[serde(default = "default_playback_clients")]
            pub playback: Vec<String>,
            #[serde(default = "default_resolve_clients")]
            pub resolve: Vec<String>,
        }
        impl Default for YouTubeClientsConfig {
            fn default() -> Self {
                Self {
                    search: default_search_clients(),
                    playback: default_playback_clients(),
                    resolve: default_resolve_clients(),
                }
            }
        }
        fn default_search_clients() -> Vec<String> {
            vec![
                "MUSIC_ANDROID".to_string(),
                "MUSIC_WEB".to_string(),
                "ANDROID".to_string(),
                "WEB".to_string(),
            ]
        }
        fn default_playback_clients() -> Vec<String> {
            vec![
                "TV".to_string(),
                "ANDROID_MUSIC".to_string(),
                "WEB".to_string(),
                "IOS".to_string(),
                "ANDROID_VR".to_string(),
                "TV_CAST".to_string(),
                "WEB_EMBEDDED".to_string(),
            ]
        }
        fn default_resolve_clients() -> Vec<String> {
            vec![
                "WEB".to_string(),
                "MUSIC_WEB".to_string(),
                "ANDROID".to_string(),
                "TVHTML5_SIMPLY".to_string(),
            ]
        }
    }
    pub use amazonmusic::*;
    pub use anghami::*;
    pub use applemusic::*;
    pub use audiomack::*;
    pub use audius::*;
    pub use bandcamp::*;
    pub use deezer::*;
    pub use flowery::*;
    pub use gaana::*;
    pub use google_tts::*;
    pub use http::*;
    pub use jiosaavn::*;
    pub use lastfm::*;
    pub use local::*;
    pub use mixcloud::*;
    pub use netease::*;
    pub use pandora::*;
    pub use qobuz::*;
    pub use reddit::*;
    use serde::{Deserialize, Serialize};
    pub use shazam::*;
    pub use soundcloud::*;
    pub use spotify::*;
    pub use tidal::*;
    pub use twitch::*;
    pub use vkmusic::*;
    pub use yandexmusic::*;
    pub use youtube::*;
    #[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq, Eq, Hash)]
    pub struct HttpProxyConfig {
        pub url: Option<String>,
        pub username: Option<String>,
        pub password: Option<String>,
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    #[serde(default)]
    pub struct SourcesConfig {
        pub youtube: Option<YouTubeConfig>,
        pub spotify: Option<SpotifyConfig>,
        pub amazonmusic: Option<AmazonMusicConfig>,
        pub http: Option<HttpSourceConfig>,
        pub local: Option<LocalSourceConfig>,
        pub jiosaavn: Option<JioSaavnConfig>,
        pub deezer: Option<DeezerConfig>,
        pub applemusic: Option<AppleMusicConfig>,
        pub gaana: Option<GaanaConfig>,
        pub tidal: Option<TidalConfig>,
        pub soundcloud: Option<SoundCloudConfig>,
        pub audiomack: Option<AudiomackConfig>,
        pub audius: Option<AudiusConfig>,
        pub pandora: Option<PandoraConfig>,
        pub qobuz: Option<QobuzConfig>,
        pub anghami: Option<AnghamiConfig>,
        pub shazam: Option<ShazamConfig>,
        pub mixcloud: Option<MixcloudConfig>,
        pub bandcamp: Option<BandcampConfig>,
        pub twitch: Option<TwitchConfig>,
        pub netease: Option<NeteaseMusicConfig>,
        pub vkmusic: Option<VkMusicConfig>,
        pub yandexmusic: Option<YandexMusicConfig>,
        pub google_tts: Option<GoogleTtsConfig>,
        pub flowery: Option<FloweryConfig>,
        pub reddit: Option<RedditConfig>,
        pub lastfm: Option<LastFmConfig>,
    }
    pub fn default_true() -> bool {
        true
    }
    pub fn default_false() -> bool {
        false
    }
    pub fn default_limit_10() -> usize {
        10
    }
    pub fn default_limit_20() -> usize {
        20
    }
    pub fn default_limit_50() -> usize {
        50
    }
    pub fn default_limit_100() -> usize {
        100
    }
    pub fn default_limit_3000() -> usize {
        3000
    }
    pub fn default_country_code() -> String {
        "us".to_string()
    }
    pub fn default_zero() -> usize {
        0
    }
    pub fn default_five() -> usize {
        5
    }
    pub fn default_tidal_quality() -> String {
        "LOSSLESS".to_string()
    }
}
pub mod filters {
    use super::sources::default_true;
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct FiltersConfig {
        #[serde(default = "default_true")]
        pub volume: bool,
        #[serde(default = "default_true")]
        pub equalizer: bool,
        #[serde(default = "default_true")]
        pub karaoke: bool,
        #[serde(default = "default_true")]
        pub timescale: bool,
        #[serde(default = "default_true")]
        pub tremolo: bool,
        #[serde(default = "default_true")]
        pub vibrato: bool,
        #[serde(default = "default_true")]
        pub distortion: bool,
        #[serde(default = "default_true")]
        pub rotation: bool,
        #[serde(default = "default_true")]
        pub channel_mix: bool,
        #[serde(default = "default_true")]
        pub low_pass: bool,
        #[serde(default = "default_true")]
        pub echo: bool,
        #[serde(default = "default_true")]
        pub high_pass: bool,
        #[serde(default = "default_true")]
        pub normalization: bool,
        #[serde(default = "default_true")]
        pub chorus: bool,
        #[serde(default = "default_true")]
        pub compressor: bool,
        #[serde(default = "default_true")]
        pub flanger: bool,
        #[serde(default = "default_true")]
        pub phaser: bool,
        #[serde(default = "default_true")]
        pub phonograph: bool,
        #[serde(default = "default_true")]
        pub reverb: bool,
        #[serde(default = "default_true")]
        pub spatial: bool,
    }
    impl Default for FiltersConfig {
        fn default() -> Self {
            Self {
                volume: true,
                equalizer: true,
                karaoke: true,
                timescale: true,
                tremolo: true,
                vibrato: true,
                distortion: true,
                rotation: true,
                channel_mix: true,
                low_pass: true,
                echo: true,
                high_pass: true,
                normalization: true,
                chorus: true,
                compressor: true,
                flanger: true,
                phaser: true,
                phonograph: true,
                reverb: true,
                spatial: true,
            }
        }
    }
    impl FiltersConfig {
        pub fn is_enabled(&self, name: &str) -> bool {
            match name {
                "volume" => self.volume,
                "equalizer" => self.equalizer,
                "karaoke" => self.karaoke,
                "timescale" => self.timescale,
                "tremolo" => self.tremolo,
                "vibrato" => self.vibrato,
                "distortion" => self.distortion,
                "rotation" => self.rotation,
                "channel_mix" | "channelMix" => self.channel_mix,
                "low_pass" | "lowPass" => self.low_pass,
                "echo" => self.echo,
                "high_pass" | "highPass" => self.high_pass,
                "normalization" => self.normalization,
                "chorus" => self.chorus,
                "compressor" => self.compressor,
                "flanger" => self.flanger,
                "phaser" => self.phaser,
                "phonograph" => self.phonograph,
                "reverb" => self.reverb,
                "spatial" => self.spatial,
                _ => true,
            }
        }
    }
}
pub mod lyrics {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct LyricsConfig {
        #[serde(default)]
        pub youtubemusic: bool,
        #[serde(default)]
        pub lrclib: bool,
        #[serde(default)]
        pub genius: bool,
        #[serde(default)]
        pub deezer: bool,
        #[serde(default)]
        pub musixmatch: bool,
        #[serde(default)]
        pub letrasmus: bool,
        #[serde(default)]
        pub yandex: bool,
        #[serde(default)]
        pub netease: bool,
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    #[serde(default)]
    pub struct YandexLyricsConfig {
        pub access_token: Option<String>,
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    #[serde(default)]
    pub struct YandexConfig {
        pub lyrics: Option<YandexLyricsConfig>,
    }
}
pub mod metrics {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct MetricsConfig {
        #[serde(default)]
        pub prometheus: PrometheusConfig,
    }
    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct PrometheusConfig {
        #[serde(default)]
        pub enabled: bool,
        #[serde(default = "default_prometheus_endpoint")]
        pub endpoint: String,
    }
    impl Default for PrometheusConfig {
        fn default() -> Self {
            Self {
                enabled: false,
                endpoint: default_prometheus_endpoint(),
            }
        }
    }
    fn default_prometheus_endpoint() -> String {
        "/metrics".to_string()
    }
}
pub mod player {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct PlayerConfig {
        #[serde(default = "default_stuck_threshold_ms")]
        pub stuck_threshold_ms: u64,
        #[serde(default = "default_buffer_duration_ms")]
        pub buffer_duration_ms: u64,
        #[serde(default = "default_frame_buffer_duration_ms")]
        pub frame_buffer_duration_ms: u64,
        #[serde(default)]
        pub resampling_quality: ResamplingQuality,
        #[serde(default = "default_opus_encoding_quality")]
        pub opus_encoding_quality: u8,
        #[serde(default)]
        pub tape: TapeConfig,
        #[serde(default)]
        pub mirrors: crate::config::server::MirrorsConfig,
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default, Copy, PartialEq)]
    #[serde(rename_all = "lowercase")]
    pub enum ResamplingQuality {
        Low,
        #[default]
        Medium,
        High,
    }
    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct TapeConfig {
        #[serde(default)]
        pub tape_stop: bool,
        #[serde(default = "default_tape_stop_duration_ms")]
        pub tape_stop_duration_ms: u64,
        #[serde(default)]
        pub curve: TapeCurve,
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default, Copy, PartialEq)]
    #[serde(rename_all = "lowercase")]
    pub enum TapeCurve {
        Linear,
        Exponential,
        #[default]
        Sinusoidal,
    }
    impl TapeCurve {
        pub fn value(self, t: f32) -> f32 {
            match self {
                Self::Linear => t,
                Self::Exponential => t * t,
                Self::Sinusoidal => 0.5 * (1.0 - (t * std::f32::consts::PI).cos()),
            }
        }
    }
    impl Default for PlayerConfig {
        fn default() -> Self {
            Self {
                stuck_threshold_ms: default_stuck_threshold_ms(),
                buffer_duration_ms: default_buffer_duration_ms(),
                frame_buffer_duration_ms: default_frame_buffer_duration_ms(),
                resampling_quality: ResamplingQuality::default(),
                opus_encoding_quality: default_opus_encoding_quality(),
                tape: TapeConfig::default(),
                mirrors: crate::config::server::MirrorsConfig::default(),
            }
        }
    }
    impl Default for TapeConfig {
        fn default() -> Self {
            Self {
                tape_stop: false,
                tape_stop_duration_ms: default_tape_stop_duration_ms(),
                curve: TapeCurve::default(),
            }
        }
    }
    fn default_stuck_threshold_ms() -> u64 {
        10000
    }
    fn default_buffer_duration_ms() -> u64 {
        400
    }
    fn default_frame_buffer_duration_ms() -> u64 {
        5000
    }
    fn default_opus_encoding_quality() -> u8 {
        10
    }
    fn default_tape_stop_duration_ms() -> u64 {
        500
    }
}
pub mod server {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct ServerConfig {
        #[serde(default = "default_address")]
        pub address: String,
        #[serde(default = "default_port")]
        pub port: u16,
        #[serde(default = "default_authorization")]
        pub authorization: String,
        #[serde(default = "default_player_update_interval")]
        pub player_update_interval: u64,
        #[serde(default = "default_stats_interval")]
        pub stats_interval: u64,
        #[serde(default = "default_websocket_ping_interval")]
        pub websocket_ping_interval: u64,
        #[serde(default = "default_max_event_queue_size")]
        pub max_event_queue_size: usize,
    }
    impl Default for ServerConfig {
        fn default() -> Self {
            Self {
                address: default_address(),
                port: default_port(),
                authorization: default_authorization(),
                player_update_interval: default_player_update_interval(),
                stats_interval: default_stats_interval(),
                websocket_ping_interval: default_websocket_ping_interval(),
                max_event_queue_size: default_max_event_queue_size(),
            }
        }
    }
    fn default_address() -> String {
        "127.0.0.1".to_string()
    }
    fn default_port() -> u16 {
        2333
    }
    fn default_authorization() -> String {
        "youshallnotpass".to_string()
    }
    fn default_max_event_queue_size() -> usize {
        100
    }
    fn default_player_update_interval() -> u64 {
        5
    }
    fn default_stats_interval() -> u64 {
        30
    }
    fn default_websocket_ping_interval() -> u64 {
        20
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct LoggingConfig {
        pub level: Option<String>,
        pub filters: Option<String>,
        pub file: Option<LogFileConfig>,
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct LogFileConfig {
        pub path: String,
        pub max_lines: u32,
        #[serde(default)]
        pub max_files: u32,
        #[serde(default)]
        pub rotate_daily: bool,
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct RoutePlannerConfig {
        #[serde(default)]
        pub enabled: bool,
        #[serde(default)]
        pub cidrs: Vec<String>,
        #[serde(default)]
        pub excluded_ips: Vec<String>,
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    #[serde(default)]
    pub struct MirrorsConfig {
        pub providers: Vec<String>,
        pub best_match: BestMatchConfig,
    }
    #[derive(Debug, Deserialize, Serialize, Clone)]
    #[serde(default)]
    pub struct BestMatchConfig {
        pub scoring: bool,
        pub throttled_prefixes: Vec<String>,
        pub min_similarity: f64,
        pub high_confidence: f64,
        pub immediate_use: f64,
        pub weight_title: f64,
        pub weight_artist: f64,
        pub weight_duration: f64,
        pub duration_tolerance_ms: u64,
    }
    impl Default for BestMatchConfig {
        fn default() -> Self {
            Self {
                scoring: true,
                throttled_prefixes: vec!["ytmsearch:".into(), "ytsearch:".into()],
                min_similarity: 0.50,
                high_confidence: 0.75,
                immediate_use: 0.88,
                weight_title: 0.50,
                weight_artist: 0.30,
                weight_duration: 0.20,
                duration_tolerance_ms: 3_000,
            }
        }
    }
    #[derive(Debug, Deserialize, Serialize, Clone, Default)]
    pub struct ConfigServerConfig {
        pub url: String,
        pub username: Option<String>,
        pub password: Option<String>,
    }
}
use crate::common::types::AnyResult;
pub use filters::*;
pub use lyrics::*;
pub use metrics::*;
pub use player::*;
use serde::Deserialize;
pub use server::*;
pub use sources::*;
use std::{fs, path::Path};
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    #[serde(default)]
    pub route_planner: RoutePlannerConfig,
    #[serde(default)]
    pub sources: SourcesConfig,
    #[serde(default)]
    pub lyrics: LyricsConfig,
    pub logging: Option<LoggingConfig>,
    #[serde(default)]
    pub filters: FiltersConfig,
    #[serde(default)]
    pub player: PlayerConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub config_server: Option<ConfigServerConfig>,
}
impl AppConfig {
    pub async fn load() -> AnyResult<Self> {
        let config_path = if Path::new("config.toml").exists() {
            "config.toml"
        } else if Path::new("config.example.toml").exists() {
            "config.example.toml"
        } else {
            return Err("config.toml or config.example.toml not found — please create one from config.example.toml".into());
        };
        println!("Loading configuration from: {}", config_path);
        let raw = fs::read_to_string(config_path)?;
        if raw.is_empty() {
            return Err(format!("{} is empty", config_path).into());
        }
        let raw_val: toml::Value = toml::from_str(&raw)?;
        if let Some(cs_val) = raw_val.get("config_server") {
            let cs: ConfigServerConfig = cs_val.clone().try_into()?;
            let client = reqwest::Client::new();
            let mut request = client.get(&cs.url);
            if let (Some(u), Some(p)) = (&cs.username, &cs.password) {
                use base64::{Engine as _, engine::general_purpose};
                let auth = format!("{}:{}", u, p);
                let encoded = general_purpose::STANDARD.encode(auth);
                request = request.header("Authorization", format!("Basic {}", encoded));
            }
            let response = request.send().await?;
            if !response.status().is_success() {
                return Err(format!(
                    "Failed to fetch remote config: status {}",
                    response.status()
                )
                .into());
            }
            let remote_toml = response.text().await?;
            return Ok(toml::from_str(&remote_toml)?);
        }
        let config: Self = toml::from_str(&raw)?;
        Ok(config)
    }
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: Default::default(),
            route_planner: Default::default(),
            sources: Default::default(),
            lyrics: Default::default(),
            logging: None,
            filters: Default::default(),
            player: Default::default(),
            metrics: Default::default(),
            config_server: None,
        }
    }
}
