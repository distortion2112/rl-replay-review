use crate::api::models::ReplayData;
use crate::api::rate_limit::RateLimiter;
use reqwest::Client;

const BASE_URL: &str = "https://ballchasing.com/api";

pub struct BallchasingClient {
    client: Client,
    api_key: String,
    rate_limiter: RateLimiter,
}

impl BallchasingClient {
    pub fn new(api_key: String, rate_limiter: RateLimiter) -> Self {
        Self {
            client: Client::new(),
            api_key,
            rate_limiter,
        }
    }

    pub async fn fetch_replay(
        &self,
        replay_id: &str,
        store: &tauri_plugin_store::Store<tauri::Wry>,
    ) -> Result<ReplayData, String> {
        self.rate_limiter.acquire(store).await?;

        let url = format!("{}/replays/{}", BASE_URL, replay_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        self.rate_limiter.update_from_headers(response.headers()).await;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => "Invalid API key. Check your ballchasing.com API key.".to_string(),
                404 => format!("Replay '{}' not found.", replay_id),
                429 => "Rate limited. Please wait a moment and try again.".to_string(),
                _ => format!("API error {}: {}", status, body),
            });
        }

        response
            .json::<ReplayData>()
            .await
            .map_err(|e| format!("Failed to parse replay data: {}", e))
    }

    pub async fn download_replay(
        &self,
        replay_id: &str,
        store: &tauri_plugin_store::Store<tauri::Wry>,
    ) -> Result<Vec<u8>, String> {
        self.rate_limiter.acquire(store).await?;

        let url = format!("{}/replays/{}/file", BASE_URL, replay_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        self.rate_limiter.update_from_headers(response.headers()).await;

        let status = response.status();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 => "Invalid API key.".to_string(),
                404 => format!("Replay file '{}' not found.", replay_id),
                429 => "Rate limited. Please wait.".to_string(),
                _ => format!("Download error {}", status),
            });
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read replay file: {}", e))
    }
}
