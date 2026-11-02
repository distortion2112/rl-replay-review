use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiTier {
    Free,
    Patron,
}

impl ApiTier {
    pub fn tokens_per_second(&self) -> f64 {
        match self {
            ApiTier::Free => 2.0,
            ApiTier::Patron => 16.0,
        }
    }

    pub fn weekly_budget(&self) -> u32 {
        match self {
            ApiTier::Free => 70,
            ApiTier::Patron => u32::MAX,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RateLimitStatus {
    pub tier: ApiTier,
    pub calls_used: u32,
    pub calls_remaining: u32,
    pub weekly_budget: u32,
    pub week_resets_at: String,
}

#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    rate: f64, // tokens per second
}

impl TokenBucket {
    fn new(rate: f64) -> Self {
        Self {
            tokens: rate, // start full
            last_refill: Instant::now(),
            rate,
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate).min(self.rate);
        self.last_refill = now;
    }

    fn try_acquire(&mut self) -> Option<Duration> {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            None // no wait needed
        } else {
            let wait = (1.0 - self.tokens) / self.rate;
            Some(Duration::from_secs_f64(wait))
        }
    }

    fn set_rate(&mut self, rate: f64) {
        self.refill();
        self.rate = rate;
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<RateLimiterInner>>,
}

#[derive(Debug)]
struct RateLimiterInner {
    bucket: TokenBucket,
    tier: ApiTier,
    calls_used: u32,
    week_start: String,
}

fn current_week_start() -> String {
    // Use Monday as week start, format as YYYY-MM-DD
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Unix epoch (1970-01-01) was a Thursday. Days since epoch:
    let days = now / 86400;
    // day_of_week: 0=Thu, 1=Fri, ... 3=Mon
    let day_of_week = (days + 3) % 7; // 0=Mon
    let monday = days - day_of_week;
    let monday_secs = monday * 86400;

    // Convert to date string
    let (y, m, d) = unix_days_to_ymd(monday_secs / 86400);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn unix_days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RateLimiterInner {
                bucket: TokenBucket::new(ApiTier::Free.tokens_per_second()),
                tier: ApiTier::Free,
                calls_used: 0,
                week_start: current_week_start(),
            })),
        }
    }

    /// Load persisted state from the store
    pub fn load_from_store(&self, store: &tauri_plugin_store::Store<tauri::Wry>) {
        let inner = self.inner.clone();
        let tier = store
            .get("rate_limit_tier")
            .and_then(|v| serde_json::from_value::<ApiTier>(v).ok())
            .unwrap_or(ApiTier::Free);
        let calls_used = store
            .get("rate_limit_calls_used")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let week_start = store
            .get("rate_limit_week_start")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        // Use blocking lock since this is called during init
        let mut guard = inner.blocking_lock();
        guard.tier = tier;
        guard.bucket.set_rate(tier.tokens_per_second());

        let current_week = current_week_start();
        if week_start == current_week {
            guard.calls_used = calls_used;
            guard.week_start = week_start;
        } else {
            // New week — reset counter
            guard.calls_used = 0;
            guard.week_start = current_week;
        }
    }

    /// Save state to the store
    fn save_to_store(
        tier: ApiTier,
        calls_used: u32,
        week_start: &str,
        store: &tauri_plugin_store::Store<tauri::Wry>,
    ) {
        store.set(
            "rate_limit_tier",
            serde_json::to_value(tier).unwrap_or_default(),
        );
        store.set("rate_limit_calls_used", serde_json::json!(calls_used));
        store.set(
            "rate_limit_week_start",
            serde_json::json!(week_start),
        );
        let _ = store.save();
    }

    /// Wait for rate limit token, increment weekly counter, persist.
    /// Returns error if weekly budget is exhausted.
    pub async fn acquire(
        &self,
        store: &tauri_plugin_store::Store<tauri::Wry>,
    ) -> Result<(), String> {
        let wait_duration;
        {
            let mut inner = self.inner.lock().await;

            // Check weekly budget
            let current_week = current_week_start();
            if inner.week_start != current_week {
                inner.calls_used = 0;
                inner.week_start = current_week;
            }

            if inner.calls_used >= inner.tier.weekly_budget() {
                return Err(format!(
                    "Weekly API budget exhausted ({}/{} calls). Resets next Monday.",
                    inner.calls_used,
                    inner.tier.weekly_budget()
                ));
            }

            wait_duration = inner.bucket.try_acquire();
        }

        // Wait outside the lock so other tasks aren't blocked
        if let Some(wait) = wait_duration {
            tokio::time::sleep(wait).await;
            // Re-acquire the token after waiting
            let mut inner = self.inner.lock().await;
            // The bucket will have refilled by now
            inner.bucket.refill();
            inner.bucket.tokens -= 1.0;
        }

        // Increment counter and persist
        {
            let mut inner = self.inner.lock().await;
            inner.calls_used += 1;
            Self::save_to_store(inner.tier, inner.calls_used, &inner.week_start, store);
        }

        Ok(())
    }

    /// Update tier based on response headers
    pub async fn update_from_headers(&self, headers: &reqwest::header::HeaderMap) {
        // ballchasing.com returns X-RateLimit-Limit header
        if let Some(limit) = headers
            .get("x-ratelimit-limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u32>().ok())
        {
            let mut inner = self.inner.lock().await;
            let new_tier = if limit > 2 {
                ApiTier::Patron
            } else {
                ApiTier::Free
            };
            if new_tier != inner.tier {
                eprintln!("API tier detected: {:?} (limit={})", new_tier, limit);
                inner.tier = new_tier;
                inner.bucket.set_rate(new_tier.tokens_per_second());
            }
        }
    }

    pub async fn status(&self) -> RateLimitStatus {
        let inner = self.inner.lock().await;
        let budget = inner.tier.weekly_budget();
        let remaining = budget.saturating_sub(inner.calls_used);

        // Calculate next Monday
        let reset = next_monday_from(&inner.week_start);

        RateLimitStatus {
            tier: inner.tier,
            calls_used: inner.calls_used,
            calls_remaining: remaining,
            weekly_budget: budget,
            week_resets_at: reset,
        }
    }
}

fn next_monday_from(week_start: &str) -> String {
    // week_start is a Monday date string; next Monday is +7 days
    // Simple approach: parse, add 7 days
    let parts: Vec<u64> = week_start
        .split('-')
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() != 3 {
        return "unknown".to_string();
    }

    // Convert to days, add 7, convert back
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    let days = ymd_to_unix_days(y, m, d) + 7;
    let (ny, nm, nd) = unix_days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", ny, nm, nd)
}

fn ymd_to_unix_days(y: u64, m: u64, d: u64) -> u64 {
    // Inverse of unix_days_to_ymd
    let y = if m <= 2 { y - 1 } else { y };
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_bucket_starts_full() {
        let mut bucket = TokenBucket::new(2.0);
        assert!(bucket.try_acquire().is_none()); // first token free
        assert!(bucket.try_acquire().is_none()); // second token free (rate=2)
    }

    #[test]
    fn token_bucket_depletes() {
        let mut bucket = TokenBucket::new(2.0);
        bucket.try_acquire();
        bucket.try_acquire();
        let wait = bucket.try_acquire();
        assert!(wait.is_some()); // should need to wait
    }

    #[test]
    fn week_start_is_monday() {
        let ws = current_week_start();
        assert_eq!(ws.len(), 10); // YYYY-MM-DD
    }

    #[test]
    fn date_round_trip() {
        let days = ymd_to_unix_days(2026, 11, 1);
        let (y, m, d) = unix_days_to_ymd(days);
        assert_eq!((y, m, d), (2026, 11, 1));
    }

    #[test]
    fn next_monday_calculation() {
        let next = next_monday_from("2026-10-26"); // Monday Oct 26
        assert_eq!(next, "2026-11-02");
    }

    #[test]
    fn tier_rates() {
        assert_eq!(ApiTier::Free.tokens_per_second(), 2.0);
        assert_eq!(ApiTier::Patron.tokens_per_second(), 16.0);
        assert_eq!(ApiTier::Free.weekly_budget(), 70);
    }
}
