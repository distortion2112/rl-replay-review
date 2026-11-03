use crate::benchmark::{self, Benchmarks, Direction, RankBenchmark, StatDistribution, TIERS};
use crate::profile::PlayerAggregate;

use super::models::*;

/// No findings below this many games.
pub const MIN_GAMES: u32 = 10;
/// z for a two-sided 80% interval.
const Z80: f64 = 1.2816;
/// Prior weight, in games, of the tier median when estimating a player's
/// stat: with n games the estimate is n/(n+K) player mean + K/(n+K) median.
const SHRINK_GAMES: f64 = 5.0;
/// Stats whose rank signal is below this never make the top 3. At ~0.1 signals
/// contradict each other on real data (e.g. "most forward" and "most back"
/// pointing opposite ways), so the bar sits above that.
pub const MIN_RANK_SIGNAL: f64 = 0.15;
const TOP_N: usize = 3;

/// Stats left out of coaching entirely, with the reason. Everything else with a
/// rank signal is eligible, and its polarity comes from the signal's direction.
const EXCLUDED: &[(&str, &str)] = &[
    ("core_score", "in-game score is a composite of other stats"),
    ("movement_avg_speed_percentage", "duplicate of movement_avg_speed"),
    ("core_mvp", "a game outcome, not a stat"),
    // Opponent-driven: these rise with rank because games get faster, so the
    // rank signal would call *fewer* shots or goals against a weakness.
    ("core_shots_against", "driven by opponents and game pace, not the player"),
    ("core_goals_against", "driven by opponents and team play, not the player"),
];

/// Stats where the rank-signal direction is the wrong guide to "better".
/// (Kept small on purpose: the signal is the default.)
const POLARITY_OVERRIDES: &[(&str, bool)] = &[
    // Getting demolished is never good, whatever rank correlation says
    ("demo_taken", false),
    ("demo_taken_in_defensive_half", false),
    ("demo_taken_in_offensive_half", false),
];

/// Empirical percentile (0-100) of `value` from a distribution's quantiles,
/// interpolated linearly between p5..p95 and extrapolated (clamped) beyond.
/// Runs of equal quantiles map to the middle of their percentile range.
pub fn percentile(value: f64, d: &StatDistribution) -> f64 {
    let pts = [(5.0, d.p5), (10.0, d.p10), (25.0, d.p25), (50.0, d.p50), (75.0, d.p75), (90.0, d.p90), (95.0, d.p95)];
    let (lo_p, lo_q) = pts[0];
    let (hi_p, hi_q) = pts[6];
    if value < lo_q {
        let slope = (pts[1].1 - lo_q) / (pts[1].0 - lo_p);
        return if slope > 0.0 { (lo_p - (lo_q - value) / slope).max(0.0) } else { 0.0 };
    }
    if value > hi_q {
        let slope = (hi_q - pts[5].1) / (hi_p - pts[5].0);
        return if slope > 0.0 { (hi_p + (value - hi_q) / slope).min(100.0) } else { 100.0 };
    }
    // All quantile points equal to value: middle of that run
    let equal: Vec<f64> = pts.iter().filter(|(_, q)| *q == value).map(|(p, _)| *p).collect();
    if !equal.is_empty() {
        return (equal[0] + equal[equal.len() - 1]) / 2.0;
    }
    for w in pts.windows(2) {
        let ((p0, q0), (p1, q1)) = (w[0], w[1]);
        if q0 < value && value < q1 {
            return p0 + (p1 - p0) * (value - q0) / (q1 - q0);
        }
    }
    50.0 // unreachable for ordered quantiles
}

/// Shrunk estimate and 80% interval of a player's per-game values.
fn estimate(values: &[f64], prior_median: f64) -> (f64, (f64, f64)) {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let sd = if values.len() > 1 {
        (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    } else {
        0.0
    };
    let w = n / (n + SHRINK_GAMES);
    let est = w * mean + (1.0 - w) * prior_median;
    let half = Z80 * w * sd / n.sqrt();
    (est, (est - half, est + half))
}

/// Scale for gaps: the interquartile range, or the standard deviation when the
/// middle half is flat, never zero.
fn spread(d: &StatDistribution) -> f64 {
    let iqr = d.p75 - d.p25;
    if iqr > 0.0 {
        iqr
    } else if d.stddev > 0.0 {
        d.stddev
    } else {
        1.0
    }
}

fn finding_id(stat: &str) -> String {
    match stat.split_once('_') {
        Some((group, rest)) => format!("{group}.{rest}"),
        None => stat.to_string(),
    }
}

fn tier_index(rank: &str) -> Result<usize, String> {
    TIERS.iter().position(|t| *t == rank).ok_or_else(|| format!("Unknown rank '{}'", rank))
}

/// Builds the evidence pack for `aggregate` at `rank` (ballchasing rank id).
pub fn generate_report(aggregate: &PlayerAggregate, rank: &str, bench: &Benchmarks) -> Result<EvidencePack, String> {
    let current = benchmark::lookup(bench, rank)?;
    let target_rank = TIERS[(tier_index(rank)? + 1).min(TIERS.len() - 1)];
    let target = benchmark::lookup(bench, target_rank)?;
    let mut seasons: Vec<String> = [&current.benchmark, &target.benchmark]
        .iter()
        .filter_map(|t| t.season.clone())
        .collect();
    seasons.dedup();

    let mut pack = EvidencePack {
        schema_version: SCHEMA_VERSION,
        player: PlayerRef { player: aggregate.player.clone(), name: aggregate.name.clone() },
        current_tier: rank.to_string(),
        current_tier_used: current.benchmark.rank.clone(),
        target_tier: target_rank.to_string(),
        target_tier_used: target.benchmark.rank.clone(),
        games: aggregate.game_count,
        enough_data: aggregate.game_count >= MIN_GAMES,
        min_games: MIN_GAMES,
        findings: Vec::new(),
        top_strengths: Vec::new(),
        top_weaknesses: Vec::new(),
        benchmark_seasons: seasons,
    };
    if !pack.enough_data {
        return Ok(pack);
    }

    let mut stats: Vec<&String> = bench.stats.keys().collect();
    stats.sort();
    for stat in stats {
        if let Some(f) = assess(stat, aggregate, bench, &current.benchmark, &target.benchmark) {
            pack.findings.push(f);
        }
    }
    pack.findings.sort_by(|a, b| b.priority.total_cmp(&a.priority).then(a.id.cmp(&b.id)));
    let top = |kind: FindingKind| -> Vec<String> {
        pack.findings
            .iter()
            .filter(|f| f.kind == kind && f.rank_signal >= MIN_RANK_SIGNAL)
            .take(TOP_N)
            .map(|f| f.id.clone())
            .collect()
    };
    pack.top_strengths = top(FindingKind::Strength);
    pack.top_weaknesses = top(FindingKind::Weakness);
    Ok(pack)
}

fn assess(
    stat: &str,
    aggregate: &PlayerAggregate,
    bench: &Benchmarks,
    current: &RankBenchmark,
    target: &RankBenchmark,
) -> Option<Finding> {
    if EXCLUDED.iter().any(|(s, _)| *s == stat) {
        return None;
    }
    let signal = bench.stats.get(stat)?.rank_signal.as_ref()?;
    let (cur, tgt) = (current.stats.get(stat)?, target.stats.get(stat)?);
    // Rare-event stats (bicycle hits, defensive-half demos): most players at
    // the tier have the same value, so the 25-75 band can't separate anyone.
    if cur.p75 <= cur.p25 {
        return None;
    }
    let values = aggregate.values(stat);
    if values.is_empty() {
        return None;
    }
    let higher_is_better = POLARITY_OVERRIDES
        .iter()
        .find(|(s, _)| *s == stat)
        .map_or(signal.direction == Direction::Higher, |(_, h)| *h);
    // Percentiles oriented so that higher is always better
    let good = |pct: f64| if higher_is_better { pct } else { 100.0 - pct };

    let (value, (lo, hi)) = estimate(&values, cur.p50);
    let (good_lo, good_hi) = {
        let (a, b) = (good(percentile(lo, cur)), good(percentile(hi, cur)));
        (a.min(b), a.max(b))
    };
    let pct_current = good(percentile(value, cur));
    let kind = if good_hi < 25.0 {
        FindingKind::Weakness
    } else if good_lo > 75.0 {
        FindingKind::Strength
    } else if pct_current < 25.0 || pct_current > 75.0 {
        FindingKind::Uncertain
    } else {
        return None; // within the normal range for the tier
    };

    // Gap in units of the tier's spread (IQR, else stddev), toward "better"
    let sign = if higher_is_better { 1.0 } else { -1.0 };
    let gap = |median: f64, d: &StatDistribution| sign * (median - value) / spread(d);
    let priority = match kind {
        FindingKind::Weakness => gap(tgt.p50, tgt).max(0.0) * signal.weight,
        FindingKind::Strength => (-gap(cur.p50, cur)).max(0.0) * signal.weight,
        FindingKind::Uncertain => 0.0,
    };

    Some(Finding {
        id: finding_id(stat),
        stat_key: stat.to_string(),
        kind,
        value,
        interval: (lo, hi),
        pct_current_tier: pct_current,
        pct_target_tier: good(percentile(value, tgt)),
        current_tier_median: cur.p50,
        target_tier_median: tgt.p50,
        higher_is_better,
        rank_signal: signal.weight,
        priority,
        games: values.len() as u32,
        clips: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(p: [f64; 7]) -> StatDistribution {
        StatDistribution { n: 100, mean: p[3], stddev: 1.0, p5: p[0], p10: p[1], p25: p[2], p50: p[3], p75: p[4], p90: p[5], p95: p[6] }
    }

    #[test]
    fn percentile_at_known_quantiles() {
        let d = dist([10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0]);
        assert_eq!(percentile(10.0, &d), 5.0);
        assert_eq!(percentile(30.0, &d), 25.0);
        assert_eq!(percentile(40.0, &d), 50.0);
        assert_eq!(percentile(70.0, &d), 95.0);
        assert_eq!(percentile(35.0, &d), 37.5);
    }

    #[test]
    fn percentile_tails_extrapolate_and_clamp() {
        let d = dist([10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0]);
        assert!((percentile(5.0, &d) - 2.5).abs() < 1e-9);
        assert_eq!(percentile(-100.0, &d), 0.0);
        assert_eq!(percentile(1000.0, &d), 100.0);
    }

    #[test]
    fn percentile_flat_runs() {
        // Zero-inflated stat: p5..p25 all 0
        let d = dist([0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0]);
        assert_eq!(percentile(0.0, &d), 15.0);
        assert_eq!(percentile(0.5, &d), 37.5);
    }

    #[test]
    fn small_samples_shrink_toward_median() {
        let (few, _) = estimate(&[100.0, 100.0], 0.0);
        let (many, _) = estimate(&[100.0; 50], 0.0);
        assert!(few < many);
        assert!((many - 100.0 * 50.0 / 55.0).abs() < 1e-9);
    }

    #[test]
    fn interval_narrows_with_more_games() {
        let noisy: Vec<f64> = (0..10).map(|i| if i % 2 == 0 { 0.0 } else { 10.0 }).collect();
        let more: Vec<f64> = (0..40).map(|i| if i % 2 == 0 { 0.0 } else { 10.0 }).collect();
        let w = |v: &[f64]| { let (_, (a, b)) = estimate(v, 5.0); b - a };
        assert!(w(&more) < w(&noisy));
    }

    #[test]
    fn spread_never_zero() {
        let flat = StatDistribution { stddev: 0.0, ..dist([0.0; 7]) };
        assert_eq!(spread(&flat), 1.0);
        assert_eq!(spread(&StatDistribution { stddev: 0.3, ..dist([0.0; 7]) }), 0.3);
        assert_eq!(spread(&dist([0.0, 1.0, 2.0, 3.0, 5.0, 6.0, 7.0])), 3.0);
    }

    #[test]
    fn finding_ids() {
        assert_eq!(finding_id("boost_percent_zero_boost"), "boost.percent_zero_boost");
    }
}
