//! Client-side throttling that mirrors ballchasing.com's published limits.
//!
//! ballchasing limits reads per second and per hour, separately for the list,
//! get and file endpoint groups, and the limits depend on the account's patron
//! tier (https://ballchasing.com/doc/api). There is no weekly read budget; the
//! 70-per-week figure ballchasing reports is the *upload* quota.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

const HOUR: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiTier {
    Regular,
    Gold,
    Diamond,
    Champion,
    Gc,
}

impl ApiTier {
    /// Parses the `type` field of `GET /api/`. Unknown values fall back to the
    /// most conservative tier.
    pub fn from_api_type(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "gold" => ApiTier::Gold,
            "diamond" => ApiTier::Diamond,
            "champion" => ApiTier::Champion,
            "gc" => ApiTier::Gc,
            _ => ApiTier::Regular,
        }
    }

    pub fn limit(&self, endpoint: Endpoint) -> Limit {
        use ApiTier::*;
        use Endpoint::*;
        let (per_second, per_hour) = match (self, endpoint) {
            (Regular, List) => (2.0, Some(500)),
            (Regular, Get) => (2.0, Some(1000)),
            (Regular, File) => (1.0, Some(200)),
            (Gold, List) => (2.0, Some(1000)),
            (Gold, Get) => (2.0, Some(2000)),
            (Gold, File) => (2.0, Some(400)),
            (Diamond, List) => (4.0, Some(2000)),
            (Diamond, Get) => (4.0, Some(5000)),
            (Diamond, File) => (2.0, Some(1000)),
            (Champion, List | Get) => (8.0, None),
            (Champion, File) => (2.0, Some(2000)),
            (Gc, List | Get) => (16.0, None),
            (Gc, File) => (2.0, None),
        };
        Limit { per_second, per_hour }
    }
}

/// ballchasing endpoint groups that are limited independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Endpoint {
    /// `GET /replays`
    List,
    /// `GET /replays/{id}`
    Get,
    /// `GET /replays/{id}/file`
    File,
}

const ENDPOINTS: [Endpoint; 3] = [Endpoint::List, Endpoint::Get, Endpoint::File];

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Limit {
    pub per_second: f64,
    pub per_hour: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointStatus {
    pub endpoint: Endpoint,
    pub per_second: f64,
    pub per_hour: Option<u32>,
    pub used_last_hour: u32,
    pub remaining_this_hour: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateLimitStatus {
    pub tier: ApiTier,
    /// False until the tier has been read from the API this session.
    pub tier_detected: bool,
    pub endpoints: Vec<EndpointStatus>,
}

/// Reservation-based limiter for one endpoint group: calls are spaced at least
/// `1 / per_second` apart and at most `per_hour` start in any rolling hour.
#[derive(Debug)]
struct EndpointLimiter {
    limit: Limit,
    next_free: Option<Instant>,
    started: VecDeque<Instant>,
}

impl EndpointLimiter {
    fn new(limit: Limit) -> Self {
        Self { limit, next_free: None, started: VecDeque::new() }
    }

    /// Reserves the earliest allowed start time at or after `now` and records it.
    fn reserve(&mut self, now: Instant) -> Instant {
        while self.started.front().is_some_and(|t| now.saturating_duration_since(*t) >= HOUR) {
            self.started.pop_front();
        }
        let mut at = now.max(self.next_free.unwrap_or(now));
        if let Some(per_hour) = self.limit.per_hour {
            let per_hour = per_hour as usize;
            if self.started.len() >= per_hour {
                // The call that falls out of the window when this one starts
                let oldest_in_window = self.started[self.started.len() - per_hour];
                at = at.max(oldest_in_window + HOUR);
            }
        }
        self.next_free = Some(at + Duration::from_secs_f64(1.0 / self.limit.per_second));
        self.started.push_back(at);
        at
    }

    /// Delays the next call, e.g. after a 429 with `Retry-After`.
    fn back_off(&mut self, until: Instant) {
        self.next_free = Some(self.next_free.map_or(until, |n| n.max(until)));
    }

    fn used_last_hour(&self, now: Instant) -> u32 {
        self.started.iter().filter(|t| now.saturating_duration_since(**t) < HOUR && **t <= now).count() as u32
    }
}

#[derive(Debug)]
struct Inner {
    tier: ApiTier,
    tier_detected: bool,
    limiters: [EndpointLimiter; 3],
}

impl Inner {
    fn new(tier: ApiTier) -> Self {
        Self {
            tier,
            tier_detected: false,
            limiters: ENDPOINTS.map(|e| EndpointLimiter::new(tier.limit(e))),
        }
    }

    fn limiter(&mut self, endpoint: Endpoint) -> &mut EndpointLimiter {
        &mut self.limiters[endpoint as usize]
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<Inner>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(Inner::new(ApiTier::Regular))) }
    }

    /// Waits until a call to `endpoint` is allowed. Never refuses; long waits
    /// only happen once an hourly cap is reached.
    pub async fn acquire(&self, endpoint: Endpoint) {
        let at = self.inner.lock().await.limiter(endpoint).reserve(Instant::now());
        tokio::time::sleep_until(at).await;
    }

    pub async fn back_off(&self, endpoint: Endpoint, retry_after: Duration) {
        self.inner.lock().await.limiter(endpoint).back_off(Instant::now() + retry_after);
    }

    pub async fn needs_tier_detection(&self) -> bool {
        !self.inner.lock().await.tier_detected
    }

    /// Applies a detected tier, keeping each endpoint's call history.
    pub async fn set_tier(&self, tier: ApiTier) {
        let mut inner = self.inner.lock().await;
        inner.tier = tier;
        inner.tier_detected = true;
        for e in ENDPOINTS {
            inner.limiter(e).limit = tier.limit(e);
        }
    }

    /// Forces re-detection, e.g. after the API key changes.
    pub async fn reset_tier(&self) {
        self.inner.lock().await.tier_detected = false;
    }

    pub async fn status(&self) -> RateLimitStatus {
        let inner = self.inner.lock().await;
        let now = Instant::now();
        let endpoints = inner
            .limiters
            .iter()
            .zip(ENDPOINTS)
            .map(|(l, endpoint)| {
                let used = l.used_last_hour(now);
                EndpointStatus {
                    endpoint,
                    per_second: l.limit.per_second,
                    per_hour: l.limit.per_hour,
                    used_last_hour: used,
                    remaining_this_hour: l.limit.per_hour.map(|cap| cap.saturating_sub(used)),
                }
            })
            .collect();
        RateLimitStatus { tier: inner.tier, tier_detected: inner.tier_detected, endpoints }
    }
}

/// Parses a `Retry-After` header given in seconds.
pub fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter(per_second: f64, per_hour: Option<u32>) -> EndpointLimiter {
        EndpointLimiter::new(Limit { per_second, per_hour })
    }

    #[test]
    fn tier_parsing() {
        assert_eq!(ApiTier::from_api_type("regular"), ApiTier::Regular);
        assert_eq!(ApiTier::from_api_type("Diamond"), ApiTier::Diamond);
        assert_eq!(ApiTier::from_api_type("gc"), ApiTier::Gc);
        assert_eq!(ApiTier::from_api_type("something-new"), ApiTier::Regular);
    }

    #[test]
    fn tier_limits_match_published_table() {
        assert_eq!(ApiTier::Regular.limit(Endpoint::Get), Limit { per_second: 2.0, per_hour: Some(1000) });
        assert_eq!(ApiTier::Diamond.limit(Endpoint::List), Limit { per_second: 4.0, per_hour: Some(2000) });
        assert_eq!(ApiTier::Diamond.limit(Endpoint::File), Limit { per_second: 2.0, per_hour: Some(1000) });
        assert_eq!(ApiTier::Gc.limit(Endpoint::Get), Limit { per_second: 16.0, per_hour: None });
        assert_eq!(ApiTier::Gc.limit(Endpoint::File), Limit { per_second: 2.0, per_hour: None });
    }

    #[test]
    fn no_weekly_cap() {
        // Far more than the old 70-call "budget" goes through with only spacing.
        let mut l = limiter(2.0, Some(1000));
        let t0 = Instant::now();
        let last = (0..500).map(|_| l.reserve(t0)).last().unwrap();
        assert_eq!(last - t0, Duration::from_secs_f64(499.0 / 2.0));
    }

    #[test]
    fn spaces_calls_per_second() {
        let mut l = limiter(4.0, None);
        let t0 = Instant::now();
        assert_eq!(l.reserve(t0), t0);
        assert_eq!(l.reserve(t0), t0 + Duration::from_millis(250));
        // A call made later than the spacing goes straight through
        let later = t0 + Duration::from_secs(5);
        assert_eq!(l.reserve(later), later);
    }

    #[test]
    fn hourly_cap_waits_for_window() {
        let mut l = limiter(1000.0, Some(3));
        let t0 = Instant::now();
        for _ in 0..3 {
            l.reserve(t0);
        }
        // 4th call must wait until the first falls out of the rolling hour
        assert_eq!(l.reserve(t0), t0 + HOUR);
    }

    #[test]
    fn back_off_delays_next_call() {
        let mut l = limiter(2.0, None);
        let t0 = Instant::now();
        l.back_off(t0 + Duration::from_secs(30));
        assert_eq!(l.reserve(t0), t0 + Duration::from_secs(30));
    }

    #[test]
    fn used_last_hour_counts_only_started_calls() {
        let mut l = limiter(1.0, Some(100));
        let t0 = Instant::now();
        l.reserve(t0);
        l.reserve(t0); // scheduled for t0 + 1s
        assert_eq!(l.used_last_hour(t0), 1);
        assert_eq!(l.used_last_hour(t0 + Duration::from_secs(1)), 2);
        assert_eq!(l.used_last_hour(t0 + HOUR + Duration::from_secs(2)), 0);
    }

    #[test]
    fn retry_after_header() {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(reqwest::header::RETRY_AFTER, "12".parse().unwrap());
        assert_eq!(retry_after(&h), Some(Duration::from_secs(12)));
    }
}
