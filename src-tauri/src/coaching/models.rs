use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerRef {
    /// `<platform>:<id>`
    pub player: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    /// Whole 80% interval is better than the tier's 75th percentile
    Strength,
    /// Whole 80% interval is worse than the tier's 25th percentile
    Weakness,
    /// Estimate is outside the 25-75 band but the interval crosses it:
    /// not enough games to tell
    Uncertain,
}

/// Pointer to a moment in a replay that shows a finding. Empty in v1; the hook
/// for frame-level evidence later (#31).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClipRef {
    pub replay_id: String,
    pub time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    /// Stable id, e.g. `boost.percent_zero_boost`
    pub id: String,
    /// Benchmark stat key, e.g. `boost_percent_zero_boost`
    pub stat_key: String,
    pub kind: FindingKind,
    /// Player's estimate, in benchmark units (shrunk toward the tier median for small samples)
    pub value: f64,
    /// 80% interval around `value`
    pub interval: (f64, f64),
    /// Percentile of `value` at the current tier, oriented so higher is better
    pub pct_current_tier: f64,
    /// Percentile of `value` at the target tier, oriented so higher is better
    pub pct_target_tier: f64,
    pub current_tier_median: f64,
    pub target_tier_median: f64,
    /// True when a higher value is better for this stat
    pub higher_is_better: bool,
    /// |Spearman correlation with rank| from the benchmarks
    pub rank_signal: f64,
    /// Larger = more worth acting on (weaknesses) or highlighting (strengths)
    pub priority: f64,
    /// Games the estimate is based on
    pub games: u32,
    pub clips: Vec<ClipRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidencePack {
    pub schema_version: u32,
    pub player: PlayerRef,
    /// Rank asked for, and the benchmark tier actually used (differs if that tier had no data)
    pub current_tier: String,
    pub current_tier_used: String,
    /// Next tier up (or the same tier at the top of the ladder)
    pub target_tier: String,
    pub target_tier_used: String,
    pub games: u32,
    /// False when there are fewer than `min_games` games: findings are empty
    pub enough_data: bool,
    pub min_games: u32,
    /// Strengths, weaknesses and uncertain findings, highest priority first
    pub findings: Vec<Finding>,
    /// Ids of the top 3 by priority, excluding stats with negligible rank signal
    pub top_strengths: Vec<String>,
    pub top_weaknesses: Vec<String>,
    /// Seasons of the benchmark tiers used, for showing data freshness
    pub benchmark_seasons: Vec<String>,
}
