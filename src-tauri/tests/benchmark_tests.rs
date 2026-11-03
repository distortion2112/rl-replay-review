use rl_replay_review_lib::benchmark::{lookup, parse, Direction, StatUnit, TIERS};

fn bundled() -> String {
    let path = format!("{}/resources/benchmarks.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path, e))
}

#[test]
fn bundled_file_parses() {
    let data = parse(&bundled()).unwrap();
    assert!(!data.tiers.is_empty());
    assert!(data.stats.len() > 30, "only {} stats", data.stats.len());
    assert!(!data.source.seasons.is_empty());
}

#[test]
fn every_rank_resolves() {
    let data = parse(&bundled()).unwrap();
    for rank in TIERS {
        let r = lookup(&data, rank).unwrap();
        assert!(!r.benchmark.stats.is_empty(), "{rank} has no stats");
    }
}

#[test]
fn distributions_are_ordered() {
    let data = parse(&bundled()).unwrap();
    for tier in &data.tiers {
        for (name, d) in &tier.stats {
            let q = [d.p5, d.p10, d.p25, d.p50, d.p75, d.p90, d.p95];
            assert!(q.windows(2).all(|w| w[0] <= w[1]), "{} {}: quantiles not ascending {:?}", tier.rank, name, q);
            assert!(d.stddev >= 0.0);
        }
    }
}

#[test]
fn speed_rises_with_rank() {
    // Sanity check on real data: higher ranks play faster.
    let data = parse(&bundled()).unwrap();
    let meta = &data.stats["movement_avg_speed"];
    assert_eq!(meta.unit, StatUnit::AsIs);
    assert_eq!(meta.rank_signal.as_ref().unwrap().direction, Direction::Higher);
    let gold = &lookup(&data, "gold-2").unwrap().benchmark.stats["movement_avg_speed"];
    let gc = &lookup(&data, "grand-champion-2").unwrap().benchmark.stats["movement_avg_speed"];
    assert!(gc.p50 > gold.p50, "GC2 median speed {} <= Gold 2 {}", gc.p50, gold.p50);
}

#[test]
fn every_tier_records_its_source() {
    let data = parse(&bundled()).unwrap();
    for tier in &data.tiers {
        let season = tier.season.as_deref().unwrap_or_else(|| panic!("{} has no season", tier.rank));
        assert!(data.source.seasons.iter().any(|s| s == season), "{}: season {} not in source.seasons", tier.rank, season);
        assert!(tier.rank_source.is_some(), "{} has no rank_source", tier.rank);
    }
}

#[test]
fn champion_and_gc_use_current_season_anchors() {
    let data = parse(&bundled()).unwrap();
    for rank in ["champion-1", "champion-2", "champion-3", "grand-champion-1", "grand-champion-2", "grand-champion-3"] {
        let tier = lookup(&data, rank).unwrap().benchmark;
        assert_eq!(tier.season.as_deref(), Some("f24"), "{rank}");
        assert!(tier.rank_source.as_deref().unwrap_or("").contains("manual anchor"), "{rank}");
        assert!(!tier.low_sample, "{rank}");
    }
}

#[test]
fn files_without_tier_sources_still_parse() {
    // Older single-season files have no per-tier season / rank_source
    let mut v: serde_json::Value = serde_json::from_str(&bundled()).unwrap();
    for t in v["tiers"].as_array_mut().unwrap() {
        t.as_object_mut().unwrap().remove("season");
        t.as_object_mut().unwrap().remove("rank_source");
    }
    let data = parse(&v.to_string()).unwrap();
    assert!(data.tiers.iter().all(|t| t.season.is_none()));
}
