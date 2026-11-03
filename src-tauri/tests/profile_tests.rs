use rl_replay_review_lib::profile::{player_game, PlayerAggregate};

fn fixture() -> serde_json::Value {
    let path = format!("{}/tests/fixtures/ballchasing_replay_f22.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

const STEAM: &str = "steam:76561198210682145";

#[test]
fn real_replay_values_in_benchmark_units() {
    // 189s game: totals are divided by 3.15 minutes, rates and percentages kept
    let g = player_game(&fixture(), STEAM).unwrap();
    assert_eq!(g.team, "blue");
    assert!(g.won);
    assert_eq!(g.score_diff, 3);
    assert!((g.values["core_goals"] - 3.0 / 3.15).abs() < 1e-9);
    assert!((g.values["boost_amount_collected"] - 993.0 / 3.15).abs() < 1e-9);
    assert_eq!(g.values["boost_bpm"], 264.0);
    assert_eq!(g.values["movement_avg_speed"], 1432.0);
    assert!(!g.values.contains_key("core_mvp"));
}

#[test]
fn keys_match_bundled_benchmarks() {
    // Every stat the benchmarks know about should come out of a real replay
    // under the same key, or comparisons silently find nothing.
    let path = format!("{}/resources/benchmarks.json", env!("CARGO_MANIFEST_DIR"));
    let bench = rl_replay_review_lib::benchmark::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    let g = player_game(&fixture(), STEAM).unwrap();
    // f24-only stats (aerial hits, clears, ...) don't exist in this f22 replay
    let f22_stats: Vec<&String> = bench
        .stats
        .keys()
        .filter(|k| bench.tiers.iter().any(|t| t.season.as_deref() == Some("f22") && t.stats.contains_key(*k)))
        .collect();
    let missing: Vec<_> = f22_stats.iter().filter(|k| !g.values.contains_key(k.as_str())).collect();
    assert!(missing.is_empty(), "benchmark stats not produced from a replay: {missing:?}");
}

#[test]
fn aggregate_over_real_replay() {
    let detail = fixture();
    let a = PlayerAggregate::from_details(STEAM, [&detail]);
    assert_eq!(a.game_count, 1);
    assert_eq!(a.win_count, 1);
    assert!(a.name.is_some());
}
