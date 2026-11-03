use crate::api::models::{ReplayData, ReplayListResponse, ReplaySearchParams};
use crate::api::rate_limit::{retry_after, ApiTier, Endpoint, RateLimiter};
use reqwest::{Client, IntoUrl, Response};
use tokio::time::Duration;

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

    /// Reads the account's patron tier from `GET /api/` once per session so the
    /// limiter uses that tier's limits. Failure leaves the conservative default.
    async fn ensure_tier(&self) {
        if !self.rate_limiter.needs_tier_detection().await {
            return;
        }
        #[derive(serde::Deserialize)]
        struct Account {
            #[serde(rename = "type")]
            kind: Option<String>,
        }
        let account = self
            .client
            .get(format!("{}/", BASE_URL))
            .header("Authorization", &self.api_key)
            .send()
            .await
            .ok()
            .filter(|r| r.status().is_success());
        if let Some(resp) = account
            && let Ok(account) = resp.json::<Account>().await
        {
            let tier = ApiTier::from_api_type(account.kind.as_deref().unwrap_or(""));
            eprintln!("ballchasing tier: {:?}", tier);
            self.rate_limiter.set_tier(tier).await;
        }
    }

    /// Sends an authorized GET within the endpoint group's limits. A 429 pushes
    /// the group's next call back by `Retry-After` (default 30s) and is returned
    /// as an error with the wait time.
    async fn send(&self, endpoint: Endpoint, url: impl IntoUrl) -> Result<Response, String> {
        self.ensure_tier().await;
        self.rate_limiter.acquire(endpoint).await;
        let response = self
            .client
            .get(url)
            .header("Authorization", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().as_u16() == 429 {
            let wait = retry_after(response.headers()).unwrap_or(Duration::from_secs(30));
            self.rate_limiter.back_off(endpoint, wait).await;
            return Err(format!(
                "ballchasing.com rate limit hit. Try again in {} seconds.",
                wait.as_secs()
            ));
        }
        Ok(response)
    }

    pub async fn fetch_replay(&self, replay_id: &str) -> Result<ReplayData, String> {
        let response = self
            .send(Endpoint::Get, format!("{}/replays/{}", BASE_URL, replay_id))
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => "Invalid API key. Check your ballchasing.com API key.".to_string(),
                404 => format!("Replay '{}' not found.", replay_id),
                _ => format!("API error {}: {}", status, body),
            });
        }

        response
            .json::<ReplayData>()
            .await
            .map_err(|e| format!("Failed to parse replay data: {}", e))
    }

    /// Replay detail as raw JSON. The aggregate reads stats generically from
    /// this, so stats ballchasing adds later are picked up without model changes.
    pub async fn fetch_replay_raw(&self, replay_id: &str) -> Result<serde_json::Value, String> {
        let response = self
            .send(Endpoint::Get, format!("{}/replays/{}", BASE_URL, replay_id))
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 => "Invalid API key. Check your ballchasing.com API key.".to_string(),
                404 => format!("Replay '{}' not found.", replay_id),
                _ => format!("API error {}", status),
            });
        }
        response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse replay data: {}", e))
    }

    pub async fn download_replay(&self, replay_id: &str) -> Result<Vec<u8>, String> {
        let response = self
            .send(Endpoint::File, format!("{}/replays/{}/file", BASE_URL, replay_id))
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 => "Invalid API key.".to_string(),
                404 => format!("Replay file '{}' not found.", replay_id),
                _ => format!("Download error {}", status),
            });
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read replay file: {}", e))
    }

    pub async fn search_replays(&self, params: &ReplaySearchParams) -> Result<ReplayListResponse, String> {
        let response = self.send(Endpoint::List, search_url(params)?).await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => "Invalid API key. Check your ballchasing.com API key.".to_string(),
                _ => format!("Search error {}: {}", status, body),
            });
        }

        response
            .json::<ReplayListResponse>()
            .await
            .map_err(|e| format!("Failed to parse search results: {}", e))
    }
}

/// Builds the `GET /replays` URL for a search.
///
/// A `next` cursor from a previous page is only followed if it points back at the
/// ballchasing replays endpoint, since the API key is sent with the request.
pub(crate) fn search_url(params: &ReplaySearchParams) -> Result<reqwest::Url, String> {
    if let Some(next) = &params.next {
        let url = reqwest::Url::parse(next).map_err(|e| format!("Invalid next-page URL: {}", e))?;
        let expected = reqwest::Url::parse(BASE_URL).map_err(|e| e.to_string())?;
        let same_api = url.scheme() == "https"
            && url.host_str() == expected.host_str()
            && url.path().starts_with("/api/replays");
        if !same_api {
            return Err("Refusing to follow a next-page URL outside the ballchasing replays API.".to_string());
        }
        return Ok(url);
    }

    let mut query: Vec<(&str, String)> = Vec::new();
    if let Some(player_id) = &params.player_id {
        query.push(("player-id", player_id.clone()));
    }
    if let Some(playlist) = &params.playlist {
        query.push(("playlist", playlist.clone()));
    }
    if let Some(season) = &params.season {
        query.push(("season", normalize_season(season)?));
    }
    if let Some(count) = params.count {
        query.push(("count", count.clamp(1, 200).to_string()));
    }
    if let Some(sort_by) = &params.sort_by {
        query.push(("sort-by", sort_by.clone()));
    }
    if let Some(sort_dir) = &params.sort_dir {
        query.push(("sort-dir", sort_dir.clone()));
    }
    reqwest::Url::parse_with_params(&format!("{}/replays", BASE_URL), &query).map_err(|e| e.to_string())
}

/// ballchasing takes legacy seasons as `1`..`14` and free-to-play seasons as `f1`, `f2`, ...
/// A bare number above 14 is almost certainly a free-to-play season and would silently
/// return no results, so it is rejected instead.
fn normalize_season(season: &str) -> Result<String, String> {
    let s = season.trim().to_lowercase();
    let valid = match s.strip_prefix('f') {
        Some(n) => !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()),
        None => s.parse::<u32>().is_ok_and(|n| (1..=14).contains(&n)),
    };
    if valid {
        Ok(s)
    } else {
        Err(format!(
            "Invalid season '{}'. Use 1-14 for legacy seasons or f1, f2, ... for free-to-play seasons (e.g. f24).",
            season
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> ReplaySearchParams {
        ReplaySearchParams {
            player_id: None,
            playlist: None,
            season: None,
            count: None,
            sort_by: None,
            sort_dir: None,
            next: None,
        }
    }

    #[test]
    fn builds_encoded_query() {
        let url = search_url(&ReplaySearchParams {
            player_id: Some("steam:7656 1198".into()),
            playlist: Some("ranked-doubles".into()),
            season: Some("f24".into()),
            count: Some(500),
            sort_by: Some("replay-date".into()),
            sort_dir: Some("desc".into()),
            ..params()
        })
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://ballchasing.com/api/replays?player-id=steam%3A7656+1198&playlist=ranked-doubles\
             &season=f24&count=200&sort-by=replay-date&sort-dir=desc"
        );
    }

    #[test]
    fn season_formats() {
        assert_eq!(normalize_season("F24").unwrap(), "f24");
        assert_eq!(normalize_season("14").unwrap(), "14");
        assert!(normalize_season("24").is_err(), "bare free-to-play number silently returns nothing");
        assert!(normalize_season("f").is_err());
        assert!(normalize_season("season 3").is_err());
    }

    #[test]
    fn count_is_clamped() {
        let url = search_url(&ReplaySearchParams { count: Some(0), ..params() }).unwrap();
        assert_eq!(url.query(), Some("count=1"));
    }

    #[test]
    fn follows_ballchasing_next_cursor() {
        let next = "https://ballchasing.com/api/replays?after=abc&count=200";
        let url = search_url(&ReplaySearchParams { next: Some(next.into()), ..params() }).unwrap();
        assert_eq!(url.as_str(), next);
    }

    #[test]
    fn rejects_foreign_next_cursor() {
        for next in [
            "https://evil.example/api/replays?x=1",
            "http://ballchasing.com/api/replays?x=1",
            "https://ballchasing.com.evil.example/api/replays",
            "https://ballchasing.com/api/maps",
        ] {
            let result = search_url(&ReplaySearchParams { next: Some(next.into()), ..params() });
            assert!(result.is_err(), "{next} should be rejected");
        }
    }
}
