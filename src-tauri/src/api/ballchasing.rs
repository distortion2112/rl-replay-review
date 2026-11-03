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

        let url = search_url(params)?;

        let response = self
            .client
            .get(url)
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
