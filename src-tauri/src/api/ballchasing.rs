use crate::api::models::{ReplayData, ReplayListItem, ReplayListResponse, ReplaySearchParams, SearchIdentity};
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

    /// Runs a search from `first`, following `next` pages until `limit` replays
    /// are collected or the results run out.
    pub async fn search_pages(&self, first: reqwest::Url, limit: usize) -> Result<Vec<ReplayListItem>, String> {
        let mut out = Vec::new();
        let mut url = first;
        loop {
            let response = self.send(Endpoint::List, url).await?;
            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(match status.as_u16() {
                    401 => "Invalid API key. Check your ballchasing.com API key.".to_string(),
                    _ => format!("Search error {}: {}", status, body),
                });
            }
            let page = response
                .json::<ReplayListResponse>()
                .await
                .map_err(|e| format!("Failed to parse search results: {}", e))?;
            let empty = page.list.is_empty();
            out.extend(page.list);
            match page.next {
                Some(next) if out.len() < limit && !empty => url = checked_next(&next)?,
                _ => break,
            }
        }
        out.truncate(limit);
        Ok(out)
    }
}

/// A `next` cursor is only followed if it points back at the ballchasing
/// replays endpoint, since the API key is sent with the request.
fn checked_next(next: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(next).map_err(|e| format!("Invalid next-page URL: {}", e))?;
    let expected = reqwest::Url::parse(BASE_URL).map_err(|e| e.to_string())?;
    let same_api = url.scheme() == "https"
        && url.host_str() == expected.host_str()
        && url.path().starts_with("/api/replays");
    if !same_api {
        return Err("Refusing to follow a next-page URL outside the ballchasing replays API.".to_string());
    }
    Ok(url)
}

/// Filters from the site search that pass straight through to `GET /api/replays`.
const SITE_FILTERS: &[&str] = &[
    "title", "player-name", "player-id", "season", "min-rank", "max-rank", "pro", "uploader", "group", "map",
    "created-before", "created-after", "replay-date-before", "replay-date-after",
];

/// A search pasted from the ballchasing.com site (e.g. `https://ballchasing.com/?player-name=x`),
/// turned into API filters for coaching.
#[derive(Debug, Clone, PartialEq)]
pub struct SiteSearch {
    pub filters: Vec<(String, String)>,
    pub names: Vec<String>,
    pub ids: Vec<String>,
    /// Parameters left out, with the reason
    pub ignored: Vec<String>,
}

/// Parses a pasted ballchasing.com search. Only the site's own host is accepted;
/// the API request is always built from scratch, so nothing else in the URL is sent
/// anywhere. Coaching compares against ranked 2v2 benchmarks, so the playlist is
/// forced to ranked-doubles, and a win/loss filter is dropped because a one-sided
/// sample would skew every stat.
pub fn parse_site_search(text: &str) -> Result<SiteSearch, String> {
    let url = reqwest::Url::parse(text.trim()).map_err(|_| "That isn't a link. Paste a ballchasing.com search URL.".to_string())?;
    let host = url.host_str().unwrap_or("");
    if !matches!(url.scheme(), "https" | "http") || !matches!(host, "ballchasing.com" | "www.ballchasing.com") {
        return Err("Only ballchasing.com search links are supported.".to_string());
    }
    if !matches!(url.path(), "/" | "" | "/replays") {
        return Err("Paste a link from ballchasing.com's replay search (the 'all replays' page).".to_string());
    }
    let mut filters = Vec::new();
    let (mut names, mut ids, mut ignored) = (Vec::new(), Vec::new(), Vec::new());
    let mut playlists = Vec::new();
    for (k, v) in url.query_pairs() {
        let (k, v) = (k.to_string(), v.trim().to_string());
        if v.is_empty() {
            continue;
        }
        match k.as_str() {
            "playlist" => playlists.push(v),
            "match-result" => ignored.push(format!("{k}={v}: coaching needs wins and losses")),
            "season" => filters.push((k, normalize_season(&v)?)),
            "player-name" => {
                names.push(v.clone());
                filters.push((k, v));
            }
            "player-id" => {
                ids.push(v.clone());
                filters.push((k, v));
            }
            _ if SITE_FILTERS.contains(&k.as_str()) => filters.push((k, v)),
            "sort-by" | "sort-dir" | "count" | "after" | "page" => {}
            _ => ignored.push(format!("{k}={v}: not a replay filter")),
        }
    }
    if names.is_empty() && ids.is_empty() {
        return Err("The search needs a player name or player id, e.g. ballchasing.com/?player-name=you".to_string());
    }
    if !playlists.is_empty() && !playlists.iter().any(|p| p == "ranked-doubles") {
        return Err(format!(
            "This search is for {}. Coaching compares ranked 2v2 games, so include ranked-doubles or remove the playlist filter.",
            playlists.join(", ")
        ));
    }
    filters.push(("playlist".to_string(), "ranked-doubles".to_string()));
    Ok(SiteSearch { filters, names, ids, ignored })
}

/// First page of a site search, newest games first.
pub fn site_search_url(search: &SiteSearch, count: u32) -> Result<reqwest::Url, String> {
    let mut query = search.filters.clone();
    query.push(("count".to_string(), count.clamp(1, 200).to_string()));
    query.push(("sort-by".to_string(), "replay-date".to_string()));
    query.push(("sort-dir".to_string(), "desc".to_string()));
    reqwest::Url::parse_with_params(&format!("{}/replays", BASE_URL), &query).map_err(|e| e.to_string())
}

/// Who a site search could be: every id listed under a searched name (exact, ignoring
/// case), plus any searched id, with how many of the replays each appears in. A name is
/// not an identity, so the user confirms which of these are them.
pub fn search_identities(replays: &[ReplayListItem], search: &SiteSearch) -> Vec<SearchIdentity> {
    let names: Vec<String> = search.names.iter().map(|n| n.to_lowercase()).collect();
    let mut found: Vec<SearchIdentity> = Vec::new();
    for r in replays {
        let mut seen = std::collections::HashSet::new();
        for (id, name) in r.players() {
            let by_name = name.is_some_and(|n| names.contains(&n.to_lowercase()));
            if !(by_name || search.ids.contains(&id)) || !seen.insert(id.clone()) {
                continue;
            }
            match found.iter_mut().find(|f| f.player_id == id) {
                Some(f) => f.games += 1,
                None => found.push(SearchIdentity { player_id: id, name: name.unwrap_or("").to_string(), games: 1 }),
            }
        }
    }
    found.sort_by(|a, b| b.games.cmp(&a.games).then(a.player_id.cmp(&b.player_id)));
    found
}

/// Builds the `GET /replays` URL for a search.
///
/// A `next` cursor from a previous page is only followed if it points back at the
/// ballchasing replays endpoint, since the API key is sent with the request.
pub(crate) fn search_url(params: &ReplaySearchParams) -> Result<reqwest::Url, String> {
    if let Some(next) = &params.next {
        return checked_next(next);
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
    fn site_search_from_player_name() {
        let s = parse_site_search("https://ballchasing.com/?player-name=distortion2112").unwrap();
        assert_eq!(s.names, vec!["distortion2112"]);
        let url = site_search_url(&s, 30).unwrap();
        assert_eq!(
            url.as_str(),
            "https://ballchasing.com/api/replays?player-name=distortion2112&playlist=ranked-doubles\
             &count=30&sort-by=replay-date&sort-dir=desc"
        );
    }

    #[test]
    fn site_search_keeps_filters_and_reports_what_it_drops() {
        let s = parse_site_search(
            "https://www.ballchasing.com/?player-name=A+b&season=F24&match-result=win&sort-by=upload-date&foo=1&map=",
        )
        .unwrap();
        assert_eq!(s.names, vec!["A b"]);
        assert!(s.filters.contains(&("season".into(), "f24".into())));
        assert!(!s.filters.iter().any(|(k, _)| k == "match-result" || k == "sort-by" || k == "map"));
        assert_eq!(s.ignored.len(), 2, "{:?}", s.ignored);
        assert!(s.ignored[0].starts_with("match-result=win"));
    }

    #[test]
    fn site_search_playlist_must_allow_ranked_doubles() {
        assert!(parse_site_search("https://ballchasing.com/?player-name=x&playlist=ranked-duels").is_err());
        let s = parse_site_search("https://ballchasing.com/?player-name=x&playlist=ranked-duels&playlist=ranked-doubles").unwrap();
        let playlists: Vec<_> = s.filters.iter().filter(|(k, _)| k == "playlist").collect();
        assert_eq!(playlists, vec![&("playlist".to_string(), "ranked-doubles".to_string())]);
    }

    #[test]
    fn site_search_rejects_other_sites_and_searches_without_a_player() {
        for bad in [
            "https://evil.example/?player-name=x",
            "https://ballchasing.com.evil.example/?player-name=x",
            "https://ballchasing.com/player/epic/x",
            "ftp://ballchasing.com/?player-name=x",
            "not a url",
            "https://ballchasing.com/?season=f24",
        ] {
            assert!(parse_site_search(bad).is_err(), "{bad} should be rejected");
        }
    }

    fn listed(id: &str, players: &[(&str, &str)]) -> ReplayListItem {
        let team = |ps: &[(&str, &str)]| {
            Some(crate::api::models::ListTeam {
                players: Some(
                    ps.iter()
                        .map(|(pid, name)| {
                            let (platform, id) = pid.split_once(':').unwrap();
                            crate::api::models::ListPlayer {
                                name: Some(name.to_string()),
                                id: Some(crate::api::models::PlayerId {
                                    platform: Some(platform.into()),
                                    id: Some(id.into()),
                                }),
                            }
                        })
                        .collect(),
                ),
            })
        };
        serde_json::from_value::<ReplayListItem>(serde_json::json!({ "id": id }))
            .map(|mut r| {
                r.blue = team(&players[..1]);
                r.orange = team(&players[1..]);
                r
            })
            .unwrap()
    }

    #[test]
    fn identities_count_exact_name_matches_only() {
        let s = parse_site_search("https://ballchasing.com/?player-name=Distortion2112").unwrap();
        let replays = vec![
            listed("a", &[("epic:me", "distortion2112"), ("epic:x", "Other")]),
            listed("b", &[("epic:me", "Distortion2112"), ("steam:1", "distortion2112x")]),
            listed("c", &[("steam:old", "distortion2112"), ("epic:y", "Y")]),
        ];
        assert_eq!(
            search_identities(&replays, &s),
            vec![
                SearchIdentity { player_id: "epic:me".into(), name: "distortion2112".into(), games: 2 },
                SearchIdentity { player_id: "steam:old".into(), name: "distortion2112".into(), games: 1 },
            ]
        );
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
