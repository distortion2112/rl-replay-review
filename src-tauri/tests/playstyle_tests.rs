use std::collections::BTreeMap;

use rl_replay_review_lib::benchmark::parse;
use rl_replay_review_lib::profile::{compute_playstyle, Axis, GameStats, PlayerAggregate};

fn bench() -> rl_replay_review_lib::benchmark::Benchmarks {
    let path = format!("{}/resources/benchmarks.json", env!("CARGO_MANIFEST_DIR"));
    parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn player(n: usize, f: impl Fn(usize) -> Vec<(&'static str, f64)>) -> PlayerAggregate {
    let games: Vec<GameStats> = (0..n)
        .map(|i| GameStats {
            replay_id: format!("r{i}"),
            date: None,
            team: "blue".into(),
            won: false,
            score_diff: 0,
            duration_s: 300.0,
            overtime: false,
            values: f(i).into_iter().map(|(k, v)| (k.to_string(), v)).collect::<BTreeMap<_, _>>(),
        })
        .collect();
    PlayerAggregate {
        player: "epic:t".into(),
        name: None,
        game_count: games.len() as u32,
        win_count: 0,
        first_date: None,
        last_date: None,
        games,
        skipped: vec![],
    }
}

const ALL: [&str; 16] = [
    "positioning_percent_offensive_third", "core_shots", "demo_inflicted", "positioning_avg_distance_to_ball",
    "positioning_percent_infront_ball", "movement_percent_high_air", "movement_percent_supersonic_speed",
    "movement_avg_powerslide_duration", "boost_count_collected_small", "boost_amount_overfill",
    "boost_percent_zero_boost", "core_assists", "core_saves", "positioning_avg_distance_to_mates",
    "positioning_percent_most_back", "movement_percent_low_air",
];

fn axis(p: &rl_replay_review_lib::profile::PlaystyleProfile, a: Axis) -> &rl_replay_review_lib::profile::AxisScore {
    p.axes.iter().find(|s| s.axis == a).unwrap()
}

#[test]
fn fewer_than_five_games() {
    let p = compute_playstyle(&player(4, |_| vec![("core_shots", 1.0)]), "champion-1", &bench()).unwrap();
    assert!(p.axes.is_empty());
    assert!(p.archetype.is_none());
    assert!(p.archetype_reason.contains("4 of 5"));
}

#[test]
fn all_zero_stats_stay_in_range() {
    let mut stats: Vec<(&str, f64)> = ALL.iter().map(|s| (*s, 0.0)).collect();
    stats.push(("movement_percent_ground", 0.0));
    let p = compute_playstyle(&player(20, |_| stats.clone()), "champion-1", &bench()).unwrap();
    assert_eq!(p.axes.len(), 5);
    for s in &p.axes {
        assert!((0.0..=100.0).contains(&s.value), "{:?} {}", s.axis, s.value);
        assert!(s.interval.0 <= s.value + 1e-9 && s.value <= s.interval.1 + 1e-9, "{:?}", s);
    }
    // Zero distance to ball counts toward aggression (inverse input): not zero
    assert!(axis(&p, Axis::Aggression).value > 0.0);
}

#[test]
fn extreme_values_clamp() {
    let p = compute_playstyle(&player(20, |_| ALL.iter().map(|s| (*s, 1e9)).collect()), "champion-1", &bench()).unwrap();
    for s in &p.axes {
        assert!((0.0..=100.0).contains(&s.value), "{:?} {}", s.axis, s.value);
    }
    // All "more" inputs at the top: Mechanical has no inverse inputs, so it's 100
    assert_eq!(axis(&p, Axis::Mechanical).value, 100.0);
}

#[test]
fn missing_inputs_renormalize_not_zero() {
    // Only one Aggression input present, at the tier's top: axis is high, not diluted
    let p = compute_playstyle(&player(20, |_| vec![("positioning_percent_offensive_third", 1e9)]), "champion-1", &bench()).unwrap();
    let a = axis(&p, Axis::Aggression);
    assert_eq!(a.inputs_used, vec!["positioning_percent_offensive_third"]);
    assert_eq!(a.value, 100.0);
    // Axes with no inputs at all are left out rather than reported as 0
    assert!(p.axes.iter().all(|s| s.axis == Axis::Aggression));
}

#[test]
fn consistent_games_give_narrow_intervals_noisy_games_wide() {
    let steady = compute_playstyle(&player(30, |_| vec![("core_shots", 0.7)]), "champion-1", &bench()).unwrap();
    let noisy = compute_playstyle(&player(30, |i| vec![("core_shots", if i % 2 == 0 { 0.0 } else { 1.4 })]), "champion-1", &bench()).unwrap();
    let w = |p: &rl_replay_review_lib::profile::PlaystyleProfile| { let s = axis(p, Axis::Aggression); s.interval.1 - s.interval.0 };
    assert!(w(&steady) < 1e-9);
    assert!(w(&noisy) > 5.0, "{}", w(&noisy));
}

#[test]
fn deterministic() {
    let a = player(25, |i| ALL.iter().map(|s| (*s, (i % 7) as f64 * 3.0)).collect());
    assert_eq!(compute_playstyle(&a, "diamond-2", &bench()).unwrap(), compute_playstyle(&a, "diamond-2", &bench()).unwrap());
}

#[test]
fn no_archetype_below_ten_games_even_if_axes_are_extreme() {
    // Everything maxed would otherwise be a confident archetype
    let p = compute_playstyle(&player(9, |_| ALL.iter().map(|s| (*s, 1e9)).collect()), "champion-1", &bench()).unwrap();
    assert_eq!(p.axes.len(), 5);
    assert!(p.archetype.is_none());
    assert!(p.archetype_reason.contains("9 of 10"), "{}", p.archetype_reason);
}
