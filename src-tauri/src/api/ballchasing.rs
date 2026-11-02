use crate::api::models::{ReplayData, ReplayListResponse, ReplaySearchParams};
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

    pub async fn search_replays(
        &self,
        params: &ReplaySearchParams,
        store: &tauri_plugin_store::Store<tauri::Wry>,
    ) -> Result<ReplayListResponse, String> {
        self.rate_limiter.acquire(store).await?;

        // If we have a `next` cursor URL, use it directly; otherwise build from params
        let url = if let Some(next) = &params.next {
            next.clone()
        } else {
            let mut url = format!("{}/replays?", BASE_URL);
            let mut query_parts: Vec<String> = Vec::new();

            if let Some(player_id) = &params.player_id {
                query_parts.push(format!("player-id={}", player_id));
            }
            if let Some(playlist) = &params.playlist {
                query_parts.push(format!("playlist={}", playlist));
            }
            if let Some(season) = params.season {
                query_parts.push(format!("season={}", season));
            }
            if let Some(count) = params.count {
                query_parts.push(format!("count={}", count.min(200)));
            }
            if let Some(sort_by) = &params.sort_by {
                query_parts.push(format!("sort-by={}", sort_by));
            }
            if let Some(sort_dir) = &params.sort_dir {
                query_parts.push(format!("sort-dir={}", sort_dir));
            }

            url.push_str(&query_parts.join("&"));
            url
        };

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("Search request failed: {}", e))?;

        self.rate_limiter.update_from_headers(response.headers()).await;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => "Invalid API key. Check your ballchasing.com API key.".to_string(),
                429 => "Rate limited. Please wait a moment and try again.".to_string(),
                _ => format!("Search error {}: {}", status, body),
            });
        }

        response
            .json::<ReplayListResponse>()
            .await
            .map_err(|e| format!("Failed to parse search results: {}", e))
    }
}
