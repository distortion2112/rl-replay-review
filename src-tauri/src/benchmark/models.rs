use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Root of `benchmarks.json` (schema_version 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benchmarks {
    pub schema_version: u32,
    pub generated_at: String,
    pub source: Source,
    /// Stat name (`<group>_<stat>`) -> metadata
    pub stats: HashMap<String, StatMeta>,
    /// Tiers present in the data, ascending by `tier_index`. Some may be missing.
    pub tiers: Vec<RankBenchmark>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub site: String,
    pub playlist: String,
    pub seasons: Vec<String>,
    pub rank_source: String,
    pub row_filter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatMeta {
    pub group: String,
    pub unit: StatUnit,
    #[serde(default)]
    pub rank_signal: Option<RankSignal>,
}

/// How a player's stat must be expressed before comparing to the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatUnit {
    /// Percentages, averages and rates: compare directly.
    AsIs,
    /// Totals: divide by game minutes first.
    PerMinute,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankSignal {
    /// Spearman correlation of the stat with rank across all tiers
    pub spearman: f64,
    pub direction: Direction,
    /// |spearman|; how strongly the stat separates ranks
    pub weight: f64,
}

/// Whether a stat rises or falls at higher ranks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Higher,
    Lower,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankBenchmark {
    /// ballchasing rank id, e.g. `diamond-2`
    pub rank: String,
    /// 0 = bronze-1 ... 21 = supersonic-legend
    pub tier_index: u8,
    pub sample_size: u32,
    pub games: u32,
    /// Fewer than 30 rows: show with low confidence
    pub low_sample: bool,
    /// Season this tier's rows come from (tiers can mix seasons), e.g. `f24`
    #[serde(default)]
    pub season: Option<String>,
    /// How this tier's ranks were known: ballchasing per-player ranks, or
    /// manual anchors looked up on tracker.gg
    #[serde(default)]
    pub rank_source: Option<String>,
    /// Stats can differ by tier: newer seasons report stats older ones don't.
    pub stats: HashMap<String, StatDistribution>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatDistribution {
    pub n: u32,
    pub mean: f64,
    pub stddev: f64,
    pub p5: f64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
}
