use super::extractor::{DeviceAuthResponse, TidalToken, TokenResponse};
use regex::Regex;
use reqwest::Client;
use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};
use tokio::sync::RwLock;
use tracing::{error, info};

const CLIENT_ID: &str = "fX2JxdmntZWK0ixT";
const CLIENT_SECRET: &str = "1Nn9AfDAjxrgJFJbKNWLeAyKGVGmINuXPPLHVXAvxAg=";

pub struct TidalOAuth {
    pub client: Client,
    pub access_token: RwLock<Option<String>>,
    pub refresh_token: RwLock<Option<String>>,
}

impl TidalOAuth {
    pub fn new(refresh_token: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("TIDAL/3704 CFNetwork/1220.1 Darwin/20.3.0")
                .build()
                .unwrap_or_default(),
            access_token: RwLock::new(None),
            refresh_token: RwLock::new(refresh_token),
        }
    }

    pub async fn get_access_token(&self) -> Option<String> {
        if let Some(token) = &*self.access_token.read().await {
            return Some(token.clone());
        }
        if self.refresh_oauth_token().await.is_ok() {
            return self.access_token.read().await.clone();
        }
        None
    }

    pub async fn get_refresh_token(&self) -> Option<String> {
        self.refresh_token.read().await.clone()
    }

    pub async fn initialize_access_token(self: Arc<Self>) {
        if self.refresh_token.read().await.is_some() {
            return;
        }
        info!("Starting Tidal device authorization flow...");
        let form = [("client_id", CLIENT_ID), ("scope", "r_usr w_usr w_sub")];
        let resp = match self
            .client
            .post("https://auth.tidal.com/v1/oauth2/device_authorization")
            .form(&form)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                error!("Tidal device authorization request failed: {}", e);
                return;
            }
        };
        if !resp.status().is_success() {
            error!("Tidal device authorization failed: {}", resp.status());
            return;
        }
        let data: DeviceAuthResponse = match resp.json().await {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse device auth response: {}", e);
                return;
            }
        };
        crate::log_println!(
            "\n  ┌────────────────────────────────────────────────────────────┐\n  │                     TIDAL OAUTH LOGIN                      │\n  ├────────────────────────────────────────────────────────────┤\n  │ 1. Visit: {:<48} │\n  │ 2. Log in and authorize the application.                   │\n  └────────────────────────────────────────────────────────────┘\n",
            data.verification_uri_complete
        );
        let oauth = self.clone();
        tokio::spawn(async move {
            oauth.poll_token(data.device_code, data.interval).await;
        });
    }

    async fn poll_token(&self, device_code: String, interval: u64) {
        let mut interval_timer = tokio::time::interval(Duration::from_secs(interval.max(1)));
        loop {
            interval_timer.tick().await;
            let form = [
                ("client_id", CLIENT_ID),
                ("device_code", &device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("scope", "r_usr w_usr w_sub"),
            ];
            let resp = match self
                .client
                .post("https://auth.tidal.com/v1/oauth2/token")
                .basic_auth(CLIENT_ID, Some(CLIENT_SECRET))
                .form(&form)
                .send()
                .await
            {
                Ok(r) => r,
                Err(_) => continue,
            };
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<TokenResponse>().await {
                    let mut at_lock = self.access_token.write().await;
                    let mut rt_lock = self.refresh_token.write().await;
                    *at_lock = Some(data.access_token);
                    *rt_lock = data.refresh_token;
                    info!("Successfully authorized Tidal OAuth");
                    if let Some(ref rt) = *rt_lock {
                        info!("Tidal Refresh Token: {}", rt);
                    }
                    return;
                }
            } else if let Ok(body) = resp.json::<serde_json::Value>().await {
                let error = body["error"].as_str().unwrap_or_default();
                match error {
                    "authorization_pending" => continue,
                    "slow_down" => {
                        interval_timer = tokio::time::interval(Duration::from_secs(interval + 3));
                    }
                    _ => {
                        error!("Tidal OAuth polling failed: {}", error);
                        return;
                    }
                }
            }
        }
    }

    pub async fn refresh_oauth_token(&self) -> Result<(), String> {
        let refresh_token = self.get_refresh_token().await;
        let rt = match refresh_token {
            Some(t) => t,
            None => return Err("No refresh token available".to_string()),
        };
        info!("Refreshing Tidal OAuth token...");
        let form = [
            ("client_id", CLIENT_ID),
            ("refresh_token", &rt),
            ("grant_type", "refresh_token"),
            ("scope", "r_usr w_usr w_sub"),
        ];
        let resp = self
            .client
            .post("https://auth.tidal.com/v1/oauth2/token")
            .basic_auth(CLIENT_ID, Some(CLIENT_SECRET))
            .form(&form)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("Tidal token refresh failed ({}): {}", status, body);
            return Err(format!("Refresh failed: {}", status));
        }
        let data: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
        let mut at_lock = self.access_token.write().await;
        let mut rt_lock = self.refresh_token.write().await;
        *at_lock = Some(data.access_token);
        if data.refresh_token.is_some() {
            *rt_lock = data.refresh_token;
        }
        info!("Successfully refreshed Tidal OAuth token");
        Ok(())
    }
}

fn script_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"src="(/assets/index-[^"]+\.js)""#).unwrap())
}

fn client_id_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"clientId\s*[:=]\s*"([^"]+)""#).unwrap())
}

pub struct TidalTokenTracker {
    pub token: RwLock<Option<TidalToken>>,
    pub client: Arc<reqwest::Client>,
    pub oauth: Arc<TidalOAuth>,
}

impl TidalTokenTracker {
    pub fn new(client: Arc<reqwest::Client>, oauth: Arc<TidalOAuth>) -> Self {
        Self {
            token: RwLock::new(None),
            client,
            oauth,
        }
    }

    pub async fn get_scraper_token(&self) -> Option<String> {
        {
            let lock = self.token.read().await;
            if let Some(token) = &*lock {
                if self.is_valid(token) {
                    return Some(token.access_token.clone());
                }
            }
        }
        self.refresh_token().await
    }

    pub async fn get_oauth_token(&self) -> Option<String> {
        self.oauth.get_access_token().await
    }

    fn is_valid(&self, token: &TidalToken) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        token.expiry_ms > now + 10_000
    }

    async fn refresh_token(&self) -> Option<String> {
        info!("Fetching new Tidal API token via scraper...");
        let listen_url = "https://listen.tidal.com";
        let resp = self.client.get(listen_url).send().await.ok()?;
        if !resp.status().is_success() {
            error!("Tidal listen page returned status: {}", resp.status());
            return None;
        }
        let html = resp.text().await.unwrap_or_default();
        let script_path = script_regex().captures(&html)?.get(1)?.as_str();
        let script_url = format!("https://listen.tidal.com{}", script_path);
        let js_resp = self.client.get(&script_url).send().await.ok()?;
        let js_content = js_resp.text().await.unwrap_or_default();
        let mut matches = client_id_regex().captures_iter(&js_content);
        matches.next();
        let token_str = matches.next()?.get(1)?.as_str().to_owned();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let token = TidalToken {
            access_token: token_str.clone(),
            expiry_ms: now + (24 * 60 * 60 * 1000),
        };
        let mut lock = self.token.write().await;
        *lock = Some(token);
        info!("Successfully refreshed Tidal scraper token");
        Some(token_str)
    }

    pub fn init(self: Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            this.get_scraper_token().await;
        });
    }

    pub async fn has_oauth_refresh_token(&self) -> bool {
        self.oauth.get_refresh_token().await.is_some()
    }
}
