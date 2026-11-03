use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::models::{Benchmarks, RankBenchmark, Source, StatMeta};

pub const SCHEMA_VERSION: u32 = 1;

/// ballchasing rank ids, lowest to highest. Index = `tier_index`.
pub const TIERS: [&str; 22] = [
    "bronze-1", "bronze-2", "bronze-3",
    "silver-1", "silver-2", "silver-3",
    "gold-1", "gold-2", "gold-3",
    "platinum-1", "platinum-2", "platinum-3",
    "diamond-1", "diamond-2", "diamond-3",
    "champion-1", "champion-2", "champion-3",
    "grand-champion-1", "grand-champion-2", "grand-champion-3",
    "supersonic-legend",
];

/// Parses `benchmarks.json`, rejecting schema versions this build doesn't know.
pub fn parse(json: &str) -> Result<Benchmarks, String> {
    #[derive(Deserialize)]
    struct Header {
        schema_version: u32,
    }
    let header: Header = serde_json::from_str(json).map_err(|e| format!("Invalid benchmarks file: {}", e))?;
    if header.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "Unsupported benchmarks schema_version {} (this build reads {}). Update the app or regenerate the file.",
            header.schema_version, SCHEMA_VERSION
        ));
    }
    let mut data: Benchmarks =
        serde_json::from_str(json).map_err(|e| format!("Invalid benchmarks file: {}", e))?;
    data.tiers.sort_by_key(|t| t.tier_index);
    Ok(data)
}

/// Benchmark for one rank, with neighbors for context.
#[derive(Debug, Clone, Serialize)]
pub struct RankLookup {
    /// The rank asked for
    pub requested: String,
    /// The rank actually returned. Differs from `requested` when that tier has no data.
    pub benchmark: RankBenchmark,
    pub substituted: bool,
    /// Nearest tier with data below / above `benchmark`
    pub below: Option<RankBenchmark>,
    pub above: Option<RankBenchmark>,
    pub stats: HashMap<String, StatMeta>,
    pub source: Source,
    pub generated_at: String,
}

/// Looks up `rank` (a ballchasing rank id). Falls back to the nearest tier with
/// data, preferring the lower one on a tie, and reports the substitution.
pub fn lookup(data: &Benchmarks, rank: &str) -> Result<RankLookup, String> {
    let wanted = TIERS
        .iter()
        .position(|t| *t == rank)
        .ok_or_else(|| format!("Unknown rank '{}'", rank))? as i32;
    let best = data
        .tiers
        .iter()
        .enumerate()
        .min_by_key(|(_, t)| ((t.tier_index as i32 - wanted).abs(), t.tier_index))
        .map(|(i, _)| i)
        .ok_or("The benchmarks file has no tiers.")?;
    let benchmark = data.tiers[best].clone();
    Ok(RankLookup {
        requested: rank.to_string(),
        substituted: benchmark.rank != rank,
        below: best.checked_sub(1).map(|i| data.tiers[i].clone()),
        above: data.tiers.get(best + 1).cloned(),
        benchmark,
        stats: data.stats.clone(),
        source: data.source.clone(),
        generated_at: data.generated_at.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tiers: &[u8]) -> String {
        let tier_json: Vec<String> = tiers
            .iter()
            .map(|&i| {
                format!(
                    r#"{{"rank":"{}","tier_index":{},"sample_size":40,"games":12,"low_sample":false,
                    "stats":{{"boost_bpm":{{"n":40,"mean":{m},"stddev":50,"p5":1,"p10":2,"p25":3,"p50":{m},"p75":5,"p90":6,"p95":7}}}}}}"#,
                    TIERS[i as usize],
                    i,
                    m = 300 + i as u32 * 10
                )
            })
            .collect();
        format!(
            r#"{{"schema_version":1,"generated_at":"2026-11-02T23:00:00+00:00",
            "source":{{"site":"ballchasing.com","playlist":"ranked-doubles","seasons":["f22"],"rank_source":"x","row_filter":"y"}},
            "stats":{{"boost_bpm":{{"group":"boost","unit":"as_is","rank_signal":{{"spearman":0.36,"direction":"higher","weight":0.36}}}},
                      "core_goals":{{"group":"core","unit":"per_minute"}}}},
            "tiers":[{}]}}"#,
            tier_json.join(",")
        )
    }

    #[test]
    fn parses_schema_v1() {
        let data = parse(&fixture(&[13, 12])).unwrap();
        assert_eq!(data.tiers[0].rank, "diamond-1", "tiers sorted by index");
        assert_eq!(data.stats["core_goals"].unit, super::super::StatUnit::PerMinute);
        assert!(data.stats["core_goals"].rank_signal.is_none());
        assert_eq!(data.stats["boost_bpm"].rank_signal.as_ref().unwrap().direction, super::super::Direction::Higher);
    }

    #[test]
    fn rejects_unknown_schema() {
        let json = fixture(&[13]).replace("\"schema_version\":1", "\"schema_version\":2");
        let err = parse(&json).unwrap_err();
        assert!(err.contains("schema_version 2"), "{err}");
    }

    #[test]
    fn rejects_malformed() {
        assert!(parse("{not json").is_err());
    }

    #[test]
    fn exact_match_with_neighbors() {
        let data = parse(&fixture(&[12, 13, 14])).unwrap();
        let r = lookup(&data, "diamond-2").unwrap();
        assert_eq!(r.benchmark.rank, "diamond-2");
        assert!(!r.substituted);
        assert_eq!(r.below.unwrap().rank, "diamond-1");
        assert_eq!(r.above.unwrap().rank, "diamond-3");
    }

    #[test]
    fn gap_falls_back_to_nearest_and_reports_it() {
        // diamond-2 (13) missing; diamond-1 (12) and champion-1 (15)
        let data = parse(&fixture(&[12, 15])).unwrap();
        let r = lookup(&data, "diamond-2").unwrap();
        assert_eq!(r.benchmark.rank, "diamond-1");
        assert!(r.substituted);
        assert_eq!(r.requested, "diamond-2");
    }

    #[test]
    fn tie_prefers_lower_tier() {
        let data = parse(&fixture(&[12, 14])).unwrap();
        assert_eq!(lookup(&data, "diamond-2").unwrap().benchmark.rank, "diamond-1");
    }

    #[test]
    fn ends_of_the_ladder() {
        let data = parse(&fixture(&[6, 7, 20])).unwrap();
        let low = lookup(&data, "bronze-1").unwrap();
        assert_eq!(low.benchmark.rank, "gold-1");
        assert!(low.below.is_none());
        let high = lookup(&data, "supersonic-legend").unwrap();
        assert_eq!(high.benchmark.rank, "grand-champion-3");
        assert!(high.above.is_none());
    }

    #[test]
    fn unknown_rank_is_an_error() {
        let data = parse(&fixture(&[12])).unwrap();
        assert!(lookup(&data, "diamond-9").is_err());
    }
}
