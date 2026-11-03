//! How a player plays, on five axes, relative to their own rank.
//!
//! Playstyle is **descriptive, not evaluative**: it says how someone plays,
//! not whether it's working. The coaching engine never treats an axis as a
//! weakness.

use serde::{Deserialize, Serialize};

use crate::benchmark::{self, Benchmarks, RankBenchmark};
use crate::coaching::percentile;

use super::PlayerAggregate;

/// Below this, no axes are computed.
pub const MIN_GAMES: u32 = 5;
/// Below this, no archetype: bootstrapping a handful of games gives intervals
/// far narrower than the truth (on real data, a 6-game interval of 61.7-62.7
/// missed the 30-game value of 55.1). Matches the coaching engine's minimum.
pub const MIN_ARCHETYPE_GAMES: u32 = 10;
const BOOTSTRAP_SAMPLES: usize = 400;
/// Fixed seed so the same games always give the same profile.
const BOOTSTRAP_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// One input to an axis: a benchmark stat, its weight, and whether a *lower*
/// value counts toward the axis (e.g. distance to ball for Aggression).
struct Input {
    stat: &'static str,
    weight: f64,
    inverse: bool,
}

const fn input(stat: &'static str, weight: f64, inverse: bool) -> Input {
    Input { stat, weight, inverse }
}

/// Axis definitions. Weights are a starting point (#9); they're renormalized
/// over whichever inputs a player has. Challenge them here.
const AXES: [(Axis, &[Input]); 5] = [
    (Axis::Aggression, &[
        input("positioning_percent_offensive_third", 30.0, false),
        input("core_shots", 20.0, false),
        input("demo_inflicted", 15.0, false),
        input("positioning_avg_distance_to_ball", 20.0, true),
        input("positioning_percent_infront_ball", 15.0, false),
    ]),
    (Axis::Mechanical, &[
        // shooting_percentage removed (#9): an outcome confounded by shot selection
        input("movement_percent_high_air", 25.0, false),
        input("movement_percent_supersonic_speed", 25.0, false),
        input("movement_avg_powerslide_duration", 25.0, false),
    ]),
    (Axis::BoostDiscipline, &[
        // small_pad_ratio has no benchmark distribution; small pads per minute stands in.
        // bpm removed (#9): its direction is ambiguous.
        input("boost_count_collected_small", 30.0, false),
        input("boost_amount_overfill", 25.0, true),
        input("boost_percent_zero_boost", 25.0, true),
    ]),
    (Axis::TeamPlay, &[
        input("core_assists", 25.0, false),
        input("core_saves", 25.0, false),
        input("positioning_avg_distance_to_mates", 25.0, true),
        input("positioning_percent_most_back", 25.0, false),
    ]),
    (Axis::AerialTendency, &[
        input("movement_percent_high_air", 40.0, false),
        input("movement_percent_low_air", 35.0, false),
        input("movement_percent_ground", 25.0, true),
    ]),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    Aggression,
    Mechanical,
    BoostDiscipline,
    TeamPlay,
    AerialTendency,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AxisScore {
    pub axis: Axis,
    /// 0-100: weighted percentile at the player's rank
    pub value: f64,
    /// 80% bootstrap interval over the player's games
    pub interval: (f64, f64),
    /// Inputs that contributed (others were missing for this player or tier)
    pub inputs_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaystyleProfile {
    pub player: String,
    pub rank: String,
    /// Benchmark tier actually used (nearest with data)
    pub rank_used: String,
    pub games: u32,
    /// Empty below `MIN_GAMES`
    pub axes: Vec<AxisScore>,
    /// Only when the axes that define it clear their thresholds with confidence
    pub archetype: Option<String>,
    /// Why there is or isn't an archetype, for the UI
    pub archetype_reason: String,
}

/// Small deterministic PRNG (xorshift64*), so no dependency is needed.
struct Rng(u64);
impl Rng {
    fn next_index(&mut self, n: usize) -> usize {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as usize % n
    }
}

/// Axis value from per-input means: weighted mean of oriented percentiles,
/// renormalized over inputs present for both player and tier.
fn axis_value(inputs: &[Input], means: &[Option<f64>], tier: &RankBenchmark) -> Option<(f64, Vec<String>)> {
    let mut total = 0.0;
    let mut weight = 0.0;
    let mut used = Vec::new();
    for (inp, mean) in inputs.iter().zip(means) {
        let (Some(mean), Some(dist)) = (mean, tier.stats.get(inp.stat)) else { continue };
        let pct = percentile(*mean, dist);
        total += inp.weight * if inp.inverse { 100.0 - pct } else { pct };
        weight += inp.weight;
        used.push(inp.stat.to_string());
    }
    (weight > 0.0).then(|| (total / weight, used))
}

fn mean_of(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, n) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    (n > 0).then(|| sum / n as f64)
}

pub fn compute_playstyle(aggregate: &PlayerAggregate, rank: &str, bench: &Benchmarks) -> Result<PlaystyleProfile, String> {
    let tier = benchmark::lookup(bench, rank)?.benchmark;
    let mut profile = PlaystyleProfile {
        player: aggregate.player.clone(),
        rank: rank.to_string(),
        rank_used: tier.rank.clone(),
        games: aggregate.game_count,
        axes: Vec::new(),
        archetype: None,
        archetype_reason: String::new(),
    };
    if aggregate.game_count < MIN_GAMES {
        profile.archetype_reason = format!("Not enough games yet ({} of {MIN_GAMES}).", aggregate.game_count);
        return Ok(profile);
    }

    let games = &aggregate.games;
    let mut rng = Rng(BOOTSTRAP_SEED);
    for (axis, inputs) in AXES.iter() {
        let means: Vec<Option<f64>> = inputs
            .iter()
            .map(|i| mean_of(games.iter().filter_map(|g| g.values.get(i.stat).copied())))
            .collect();
        let Some((value, used)) = axis_value(inputs, &means, &tier) else { continue };

        // Bootstrap: resample games, recompute input means and the axis
        let mut samples: Vec<f64> = (0..BOOTSTRAP_SAMPLES)
            .filter_map(|_| {
                let picks: Vec<usize> = (0..games.len()).map(|_| rng.next_index(games.len())).collect();
                let means: Vec<Option<f64>> = inputs
                    .iter()
                    .map(|i| mean_of(picks.iter().filter_map(|&k| games[k].values.get(i.stat).copied())))
                    .collect();
                axis_value(inputs, &means, &tier).map(|(v, _)| v)
            })
            .collect();
        samples.sort_by(f64::total_cmp);
        let at = |q: f64| samples[((samples.len() - 1) as f64 * q).round() as usize];
        profile.axes.push(AxisScore { axis: *axis, value, interval: (at(0.1), at(0.9)), inputs_used: used });
    }
    if aggregate.game_count < MIN_ARCHETYPE_GAMES {
        profile.archetype_reason = format!(
            "Not enough games for an archetype yet ({} of {MIN_ARCHETYPE_GAMES}). Axes are shown with their intervals.",
            aggregate.game_count
        );
        return Ok(profile);
    }
    let (archetype, reason) = archetype(&profile.axes);
    profile.archetype = archetype;
    profile.archetype_reason = reason;
    Ok(profile)
}

/// Rules from #9. A rule fires only when every axis it names has its whole
/// interval on the right side of the threshold.
fn archetype(axes: &[AxisScore]) -> (Option<String>, String) {
    let get = |a: Axis| axes.iter().find(|s| s.axis == a);
    let at_least = |a: Axis, t: f64| get(a).is_some_and(|s| s.interval.0 >= t);
    let at_most = |a: Axis, t: f64| get(a).is_some_and(|s| s.interval.1 <= t);
    use Axis::*;
    let rules: [(&str, bool); 4] = [
        ("Aggressive Striker", at_least(Aggression, 70.0) && at_least(Mechanical, 60.0)),
        ("Defensive Anchor", at_most(Aggression, 30.0) && at_least(TeamPlay, 70.0)),
        ("Efficient Rotator", at_least(BoostDiscipline, 75.0) && at_least(TeamPlay, 60.0)),
        ("Mechanical Specialist", at_least(Mechanical, 80.0) && at_least(AerialTendency, 70.0)),
    ];
    if let Some((name, _)) = rules.iter().find(|(_, hit)| *hit) {
        return (Some(name.to_string()), "Defining axes clear their thresholds across the 80% interval.".into());
    }
    if axes.len() == 5 && axes.iter().all(|s| s.interval.0 >= 40.0 && s.interval.1 <= 60.0) {
        return (Some("Balanced Generalist".into()), "Every axis is within 40-60 across its 80% interval.".into());
    }
    // Fallback: a clear leading axis, if its interval sits above every other axis's estimate
    let mut sorted: Vec<&AxisScore> = axes.iter().collect();
    sorted.sort_by(|a, b| b.value.total_cmp(&a.value));
    if let [top, second, ..] = sorted.as_slice()
        && top.interval.0 > second.value
        && top.value >= 60.0
    {
        return (
            Some(format!("{} leaning", label(top.axis))),
            format!("{} clearly leads the other axes.", label(top.axis)),
        );
    }
    (None, "No archetype yet: the axes that would define one aren't clear enough with these games.".into())
}

fn label(axis: Axis) -> &'static str {
    match axis {
        Axis::Aggression => "Aggression",
        Axis::Mechanical => "Mechanical",
        Axis::BoostDiscipline => "Boost discipline",
        Axis::TeamPlay => "Team play",
        Axis::AerialTendency => "Aerial",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(axis: Axis, value: f64, lo: f64, hi: f64) -> AxisScore {
        AxisScore { axis, value, interval: (lo, hi), inputs_used: vec![] }
    }

    #[test]
    fn archetype_requires_whole_interval() {
        use Axis::*;
        let confident = [score(Aggression, 80.0, 72.0, 88.0), score(Mechanical, 70.0, 62.0, 78.0)];
        assert_eq!(archetype(&confident).0.as_deref(), Some("Aggressive Striker"));
        // Same point estimates, but the interval dips under the threshold
        let unsure = [score(Aggression, 80.0, 65.0, 92.0), score(Mechanical, 70.0, 55.0, 80.0)];
        assert_ne!(archetype(&unsure).0.as_deref(), Some("Aggressive Striker"));
    }

    #[test]
    fn defensive_anchor_uses_upper_bound() {
        use Axis::*;
        let a = [score(Aggression, 20.0, 12.0, 28.0), score(TeamPlay, 80.0, 72.0, 88.0)];
        assert_eq!(archetype(&a).0.as_deref(), Some("Defensive Anchor"));
    }

    #[test]
    fn no_archetype_when_axes_overlap() {
        use Axis::*;
        let a = [score(Aggression, 55.0, 40.0, 70.0), score(TeamPlay, 52.0, 38.0, 66.0), score(Mechanical, 50.0, 35.0, 65.0)];
        assert_eq!(archetype(&a).0, None);
    }

    #[test]
    fn rng_is_deterministic_and_in_range() {
        let mut a = Rng(BOOTSTRAP_SEED);
        let mut b = Rng(BOOTSTRAP_SEED);
        for _ in 0..1000 {
            let (x, y) = (a.next_index(7), b.next_index(7));
            assert_eq!(x, y);
            assert!(x < 7);
        }
    }
}
