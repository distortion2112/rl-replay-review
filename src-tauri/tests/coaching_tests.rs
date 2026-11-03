use std::collections::BTreeMap;

use rl_replay_review_lib::benchmark::parse;
use rl_replay_review_lib::coaching::{generate_report, render, verify, FindingKind};
use rl_replay_review_lib::profile::{GameStats, PlayerAggregate};

/// Two tiers. Quantiles p5..p95 per stat; the target tier is "better" on every stat.
fn bench() -> rl_replay_review_lib::benchmark::Benchmarks {
    let tier = |rank: &str, idx: u8, shift: f64| {
        // Quantiles always ascend; `better` moves the better tier up (+1) or down (-1)
        let d = |base: f64, step: f64, better: f64| {
            let q: Vec<f64> = [-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0].iter().map(|k| base + better * shift * step + k * step).collect();
            format!(r#"{{"n":200,"mean":{m},"stddev":{s},"p5":{},"p10":{},"p25":{},"p50":{},"p75":{},"p90":{},"p95":{}}}"#,
                    q[0], q[1], q[2], q[3], q[4], q[5], q[6], m = q[3], s = step)
        };
        format!(
            r#"{{"rank":"{rank}","tier_index":{idx},"sample_size":200,"games":100,"low_sample":false,"season":"f24","rank_source":"test",
               "stats":{{"boost_bpm":{},"boost_percent_zero_boost":{},"demo_inflicted":{},"core_saves":{}}}}}"#,
            d(400.0, 20.0, 1.0),
            // lower is better: the better tier has lower zero-boost time
            d(10.0, 1.0, -1.0),
            d(1.0, 0.2, 1.0),
            d(0.5, 0.05, 1.0)
        )
    };
    let json = format!(
        r#"{{"schema_version":1,"generated_at":"t","source":{{"site":"s","playlist":"p","seasons":["f24"],"rank_source":"r","row_filter":"f"}},
           "stats":{{
             "boost_bpm":{{"group":"boost","unit":"as_is","rank_signal":{{"spearman":0.5,"direction":"higher","weight":0.5}}}},
             "boost_percent_zero_boost":{{"group":"boost","unit":"as_is","rank_signal":{{"spearman":-0.4,"direction":"lower","weight":0.4}}}},
             "demo_inflicted":{{"group":"demo","unit":"per_minute","rank_signal":{{"spearman":0.02,"direction":"higher","weight":0.02}}}},
             "core_saves":{{"group":"core","unit":"per_minute","rank_signal":{{"spearman":0.3,"direction":"higher","weight":0.3}}}}}},
           "tiers":[{},{}]}}"#,
        tier("diamond-2", 13, 0.0),
        tier("diamond-3", 14, 1.0)
    );
    parse(&json).unwrap()
}

/// `n` games; each stat alternates value ± spread.
fn player(n: usize, stats: &[(&str, f64, f64)]) -> PlayerAggregate {
    let games = (0..n)
        .map(|i| {
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            let values: BTreeMap<String, f64> =
                stats.iter().map(|(k, v, spread)| (k.to_string(), v + sign * spread)).collect();
            GameStats {
                replay_id: format!("r{i}"),
                date: Some(format!("2026-10-{:02}T00:00:00Z", i % 28 + 1)),
                team: "blue".into(),
                won: i % 2 == 0,
                score_diff: 0,
                duration_s: 300.0,
                overtime: false,
                values,
            }
        })
        .collect::<Vec<_>>();
    PlayerAggregate {
        player: "epic:test".into(),
        name: Some("Tester".into()),
        game_count: games.len() as u32,
        win_count: 0,
        first_date: None,
        last_date: None,
        games,
        skipped: vec![],
    }
}

#[test]
fn fixture_quantiles_ascend() {
    for t in &bench().tiers {
        for (k, d) in &t.stats {
            let q = [d.p5, d.p10, d.p25, d.p50, d.p75, d.p90, d.p95];
            assert!(q.windows(2).all(|w| w[0] <= w[1]), "{} {k}: {q:?}", t.rank);
        }
    }
}

#[test]
fn not_enough_games_returns_empty_pack() {
    let pack = generate_report(&player(9, &[("boost_bpm", 300.0, 1.0)]), "diamond-2", &bench()).unwrap();
    assert!(!pack.enough_data);
    assert_eq!(pack.games, 9);
    assert!(pack.findings.is_empty());
}

#[test]
fn clear_weakness_and_strength() {
    // bpm far below D2 p25 (380), consistent; saves far above p75 (0.55)
    let a = player(40, &[("boost_bpm", 330.0, 2.0), ("core_saves", 0.8, 0.01)]);
    let pack = generate_report(&a, "diamond-2", &bench()).unwrap();
    let get = |id: &str| pack.findings.iter().find(|f| f.id == id).unwrap();
    assert_eq!(get("boost.bpm").kind, FindingKind::Weakness);
    assert_eq!(get("core.saves").kind, FindingKind::Strength);
    assert_eq!(pack.target_tier, "diamond-3");
    assert_eq!(pack.top_weaknesses, vec!["boost.bpm"]);
    assert_eq!(pack.top_strengths, vec!["core.saves"]);
}

#[test]
fn noisy_estimate_is_uncertain_not_weakness() {
    // Mean below p25 but per-game spread is huge with only 10 games
    let pack = generate_report(&player(10, &[("boost_bpm", 360.0, 150.0)]), "diamond-2", &bench()).unwrap();
    let f = pack.findings.iter().find(|f| f.id == "boost.bpm").unwrap();
    assert_eq!(f.kind, FindingKind::Uncertain);
    assert!(f.interval.0 < 380.0 && f.interval.1 > 380.0);
    assert!(pack.top_weaknesses.is_empty());
}

#[test]
fn typical_values_produce_no_finding() {
    let pack = generate_report(&player(30, &[("boost_bpm", 400.0, 5.0)]), "diamond-2", &bench()).unwrap();
    assert!(pack.findings.iter().all(|f| f.id != "boost.bpm"));
}

#[test]
fn lower_is_better_polarity() {
    // High zero-boost time is bad: D2 p50 is 10 and p95 is 13, so 16 is well past it
    let pack = generate_report(&player(40, &[("boost_percent_zero_boost", 16.0, 0.5)]), "diamond-2", &bench()).unwrap();
    let f = pack.findings.iter().find(|f| f.id == "boost.percent_zero_boost").unwrap();
    assert!(!f.higher_is_better);
    assert_eq!(f.kind, FindingKind::Weakness);
    assert!(f.pct_current_tier < 25.0, "oriented percentile should be low: {}", f.pct_current_tier);
}

#[test]
fn negligible_rank_signal_never_in_top() {
    // Huge demo gap, but demos barely relate to rank (weight 0.02)
    let a = player(40, &[("demo_inflicted", 0.0, 0.0), ("boost_bpm", 340.0, 1.0)]);
    let pack = generate_report(&a, "diamond-2", &bench()).unwrap();
    assert!(pack.findings.iter().any(|f| f.id == "demo.inflicted" && f.kind == FindingKind::Weakness));
    assert!(!pack.top_weaknesses.contains(&"demo.inflicted".to_string()));
    assert_eq!(pack.top_weaknesses[0], "boost.bpm");
}

#[test]
fn top_of_ladder_targets_itself() {
    let pack = generate_report(&player(10, &[]), "supersonic-legend", &bench()).unwrap();
    assert_eq!(pack.target_tier, "supersonic-legend");
    // Fixture has no SSL tier: nearest available is used and reported
    assert_eq!(pack.current_tier_used, "diamond-3");
}

#[test]
fn evidence_pack_v1_snapshot() {
    // Locks the JSON shape #11 and the UI depend on. If this changes on purpose,
    // bump coaching::SCHEMA_VERSION and regenerate the fixture.
    let a = player(20, &[("boost_bpm", 330.0, 4.0), ("core_saves", 0.8, 0.02), ("boost_percent_zero_boost", 10.0, 0.5)]);
    let pack = generate_report(&a, "diamond-2", &bench()).unwrap();
    let actual = serde_json::to_string_pretty(&pack).unwrap();
    let path = format!("{}/tests/fixtures/evidence_pack_v1.json", env!("CARGO_MANIFEST_DIR"));
    if std::env::var("UPDATE_SNAPSHOTS").is_ok() {
        std::fs::write(&path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path).expect("run with UPDATE_SNAPSHOTS=1 to create the fixture");
    assert_eq!(actual, expected);
}

#[test]
fn real_benchmarks_produce_finite_priorities_and_skip_rare_events() {
    // A player with rare events (bicycle hits) and normal stats, against the
    // bundled benchmarks: rare-event stats are skipped, priorities stay finite.
    let path = format!("{}/resources/benchmarks.json", env!("CARGO_MANIFEST_DIR"));
    let bench = parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    let a = player(30, &[
        ("core_bicycle_hits", 0.05, 0.05),
        ("demo_inflicted_in_defensive_half", 0.05, 0.05),
        ("boost_bpm", 300.0, 20.0),
        ("movement_percent_supersonic_speed", 30.0, 2.0),
    ]);
    let pack = generate_report(&a, "grand-champion-2", &bench).unwrap();
    assert!(pack.findings.iter().all(|f| f.priority.is_finite() && f.priority < 100.0), "{:?}",
        pack.findings.iter().map(|f| (&f.id, f.priority)).collect::<Vec<_>>());
    assert!(pack.findings.iter().all(|f| f.id != "core.bicycle_hits" && f.id != "demo.inflicted_in_defensive_half"));
    let tops: Vec<&String> = pack.top_weaknesses.iter().chain(&pack.top_strengths).collect();
    for id in tops {
        let f = pack.findings.iter().find(|f| &f.id == id).unwrap();
        assert!(f.rank_signal >= rl_replay_review_lib::coaching::engine::MIN_RANK_SIGNAL, "{id} {}", f.rank_signal);
    }
}

fn sample_pack() -> rl_replay_review_lib::coaching::EvidencePack {
    let a = player(20, &[("boost_bpm", 330.0, 4.0), ("core_saves", 0.8, 0.02), ("boost_percent_zero_boost", 10.0, 0.5)]);
    generate_report(&a, "diamond-2", &bench()).unwrap()
}

#[test]
fn templates_render_every_kind_and_pass_the_verifier() {
    let mut pack = sample_pack();
    // Add an uncertain finding (on a stat the sample pack has no finding for)
    let noisy = generate_report(&player(10, &[("boost_percent_zero_boost", 14.0, 12.0)]), "diamond-2", &bench()).unwrap();
    assert!(noisy.findings.iter().any(|f| f.kind == FindingKind::Uncertain), "{:?}", noisy.findings);
    pack.findings.extend(noisy.findings);
    let report = render(&pack);
    let kinds: Vec<FindingKind> = report.items.iter().map(|i| i.kind).collect();
    for k in [FindingKind::Weakness, FindingKind::Strength, FindingKind::Uncertain] {
        assert!(kinds.contains(&k), "{k:?} not rendered");
    }
    assert!(report.items[0].top && report.items[0].kind == FindingKind::Weakness);
    assert!(report.items[0].action.is_some(), "boost.bpm has mapped advice");
    for text in std::iter::once(&report.summary).chain(report.items.iter().flat_map(|i| [&i.title, &i.explanation])) {
        let v = verify(text, &pack, false);
        assert!(v.is_empty(), "template text failed verification: {text:?} {v:?}");
    }
}

#[test]
fn not_enough_data_summary() {
    let pack = generate_report(&player(9, &[]), "diamond-2", &bench()).unwrap();
    let report = render(&pack);
    assert!(report.items.is_empty());
    assert!(report.summary.contains("at least 10 games") && report.summary.contains("there are 9"), "{}", report.summary);
    assert!(verify(&report.summary, &pack, false).is_empty());
}

#[test]
fn verifier_rejects_bad_narration() {
    let pack = sample_pack();
    let good = "Your boost use is 344 per minute [boost.bpm], short of Diamond 3 players around 420 [boost.bpm].";
    assert!(verify(good, &pack, true).is_empty(), "{:?}", verify(good, &pack, true));

    let rules = |text: &str| verify(text, &pack, true).into_iter().map(|v| v.rule).collect::<Vec<_>>();
    assert_eq!(rules("Your boost use is 512 per minute [boost.bpm]."), vec!["unknown_number"]);
    assert_eq!(rules("Champion 1 players use 344 boost per minute [boost.bpm]."), vec!["extra_tier"]);
    assert_eq!(rules("You use 344 boost per minute [boost.made_up]."), vec!["unknown_citation"]);
    assert_eq!(rules("You use 344 boost per minute."), vec!["uncited_claim"]);
    // Rounding: 344 shown as 344.0 is fine, 344.4 is not (value is exactly 344)
    assert!(rules("Boost use 344.0 per minute [boost.bpm].").is_empty());
    assert_eq!(rules("Boost use 344.4 per minute [boost.bpm]."), vec!["unknown_number"]);
}

#[test]
fn every_benchmark_stat_renders_cleanly() {
    // A finding of each kind for every stat in the shipped benchmarks: readable
    // labels (no key fragments or digits) and nothing the verifier rejects.
    use rl_replay_review_lib::coaching::{EvidencePack, Finding};
    let path = format!("{}/resources/benchmarks.json", env!("CARGO_MANIFEST_DIR"));
    let bench = parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut findings = Vec::new();
    for (i, stat) in bench.stats.keys().enumerate() {
        let kind = [FindingKind::Weakness, FindingKind::Strength, FindingKind::Uncertain][i % 3];
        findings.push(Finding {
            id: stat.replacen('_', ".", 1),
            stat_key: stat.clone(),
            kind,
            value: 1234.567,
            interval: (1200.25, 1270.75),
            pct_current_tier: 12.4,
            pct_target_tier: 8.6,
            current_tier_median: 1500.5,
            target_tier_median: 1600.25,
            higher_is_better: true,
            rank_signal: 0.5,
            priority: 1.0,
            games: 30,
            clips: vec![],
        });
    }
    let pack = EvidencePack {
        schema_version: 1,
        player: rl_replay_review_lib::coaching::PlayerRef { player: "epic:x".into(), name: None },
        current_tier: "grand-champion-1".into(),
        current_tier_used: "grand-champion-1".into(),
        target_tier: "grand-champion-2".into(),
        target_tier_used: "grand-champion-2".into(),
        games: 30,
        enough_data: true,
        min_games: 10,
        findings,
        top_strengths: vec![],
        top_weaknesses: vec![],
        benchmark_seasons: vec!["f24".into()],
    };
    let report = render(&pack);
    assert_eq!(report.items.len(), bench.stats.len());
    for item in &report.items {
        for text in [&item.title, &item.explanation] {
            assert!(!text.contains('_'), "key fragment in {text:?}");
            let v = verify(text, &pack, false);
            assert!(v.is_empty(), "{}: {text:?} {v:?}", item.finding_id);
        }
    }
}

#[test]
fn weak_signal_findings_are_described_as_unusual() {
    // demo_inflicted has rank signal 0.02 in the synthetic benchmarks
    let pack = generate_report(&player(40, &[("demo_inflicted", 0.0, 0.0)]), "diamond-2", &bench()).unwrap();
    let report = render(&pack);
    let item = report.items.iter().find(|i| i.finding_id == "demo.inflicted").unwrap();
    assert!(item.title.starts_with("Unusual for your rank"), "{}", item.title);
    assert!(item.explanation.contains("may simply be your style"));
    assert!(item.action.is_none());
    assert!(verify(&item.explanation, &pack, false).is_empty());
}
