//! Deterministic coaching text from an evidence pack (#11). Works offline and
//! never adds facts: every number shown comes from the pack.

use serde::{Deserialize, Serialize};

use super::engine::MIN_RANK_SIGNAL;
use super::models::{EvidencePack, Finding, FindingKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportItem {
    pub finding_id: String,
    pub kind: FindingKind,
    pub title: String,
    pub explanation: String,
    /// What to practice. Weaknesses only, and only for findings with mapped advice.
    pub action: Option<String>,
    /// In the pack's top 3 for its kind
    pub top: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoachingReport {
    pub pack_schema_version: u32,
    pub summary: String,
    /// Top weaknesses, top strengths, other weaknesses and strengths, then uncertain
    pub items: Vec<ReportItem>,
    /// `"templates"` here; the optional LLM narrator will set its own source
    pub source: String,
}

/// How a stat's value is shown. Units appear once, after the number or range.
#[derive(Clone, Copy)]
enum Fmt {
    /// 0-100 share of the game: "12.3%"
    Percent,
    /// Per-minute count or amount: "0.82 per minute"
    PerMinute,
    /// Whole number with a unit: "1,523 uu/s"
    Whole(&'static str),
    /// Seconds: "0.12 s"
    Seconds,
    /// Seconds per minute of game: "2.09 s per minute"
    SecondsPerMinute,
    /// Whole distance units per minute: "34,210 uu per minute"
    WholeUuPerMinute,
    /// Plain number with one decimal: "53.9"
    Plain,
}

/// Display name and format for every benchmark stat. Labels contain no digits,
/// so the verifier never mistakes them for claims. Unlisted stats (ones
/// ballchasing adds later) fall back to a name derived from the key.
const LABELS: &[(&str, &str, Fmt)] = &[
    ("boost_amount_collected", "boost collected", Fmt::PerMinute),
    ("boost_amount_collected_big", "boost from big pads", Fmt::PerMinute),
    ("boost_amount_collected_small", "boost from small pads", Fmt::PerMinute),
    ("boost_amount_overfill", "boost wasted by overfilling", Fmt::PerMinute),
    ("boost_amount_overfill_stolen", "stolen boost wasted by overfilling", Fmt::PerMinute),
    ("boost_amount_stolen", "boost stolen", Fmt::PerMinute),
    ("boost_amount_stolen_big", "boost stolen from big pads", Fmt::PerMinute),
    ("boost_amount_stolen_small", "boost stolen from small pads", Fmt::PerMinute),
    ("boost_amount_used_while_supersonic", "boost used while already supersonic", Fmt::PerMinute),
    ("boost_avg_amount", "average boost held", Fmt::Plain),
    ("boost_bcpm", "boost collected per minute", Fmt::Whole("")),
    ("boost_bpm", "boost used per minute", Fmt::Whole("")),
    ("boost_count_collected_big", "big pads collected", Fmt::PerMinute),
    ("boost_count_collected_small", "small pads collected", Fmt::PerMinute),
    ("boost_count_stolen_big", "big pads stolen", Fmt::PerMinute),
    ("boost_count_stolen_small", "small pads stolen", Fmt::PerMinute),
    ("boost_percent_boost_0_25", "time with a quarter tank or less", Fmt::Percent),
    ("boost_percent_boost_25_50", "time with a quarter to half a tank", Fmt::Percent),
    ("boost_percent_boost_50_75", "time with half to three quarters of a tank", Fmt::Percent),
    ("boost_percent_boost_75_100", "time with three quarters of a tank or more", Fmt::Percent),
    ("boost_percent_full_boost", "time on full boost", Fmt::Percent),
    ("boost_percent_zero_boost", "time on zero boost", Fmt::Percent),
    ("core_aerial_hits", "aerial hits", Fmt::PerMinute),
    ("core_assists", "assists", Fmt::PerMinute),
    ("core_bicycle_hits", "bicycle hits", Fmt::PerMinute),
    ("core_centers", "centers", Fmt::PerMinute),
    ("core_clears", "clears", Fmt::PerMinute),
    ("core_crossbar_hits", "crossbar hits", Fmt::PerMinute),
    ("core_epic_saves", "epic saves", Fmt::PerMinute),
    ("core_first_touches", "first touches", Fmt::PerMinute),
    ("core_flip_resets", "flip resets", Fmt::PerMinute),
    ("core_goals", "goals", Fmt::PerMinute),
    ("core_goals_against", "goals against", Fmt::PerMinute),
    ("core_juggle_hits", "juggle hits", Fmt::PerMinute),
    ("core_saves", "saves", Fmt::PerMinute),
    ("core_score", "in-game score", Fmt::PerMinute),
    ("core_shooting_percentage", "shooting percentage", Fmt::Percent),
    ("core_shots", "shots", Fmt::PerMinute),
    ("core_shots_against", "shots against", Fmt::PerMinute),
    ("demo_inflicted", "demos", Fmt::PerMinute),
    ("demo_inflicted_in_defensive_half", "demos in your own half", Fmt::PerMinute),
    ("demo_inflicted_in_offensive_half", "demos in the attacking half", Fmt::PerMinute),
    ("demo_taken", "times demolished", Fmt::PerMinute),
    ("demo_taken_in_defensive_half", "times demolished in your own half", Fmt::PerMinute),
    ("demo_taken_in_offensive_half", "times demolished in the attacking half", Fmt::PerMinute),
    ("movement_avg_powerslide_duration", "average powerslide length", Fmt::Seconds),
    ("movement_avg_speed", "average speed", Fmt::Whole("uu/s")),
    ("movement_avg_speed_percentage", "average speed as a share of max", Fmt::Percent),
    ("movement_count_powerslide", "powerslides", Fmt::PerMinute),
    ("movement_percent_boost_speed", "time at boost speed", Fmt::Percent),
    ("movement_percent_ground", "time on the ground", Fmt::Percent),
    ("movement_percent_high_air", "time high in the air", Fmt::Percent),
    ("movement_percent_low_air", "time low in the air", Fmt::Percent),
    ("movement_percent_slow_speed", "time at slow speed", Fmt::Percent),
    ("movement_percent_supersonic_speed", "time at supersonic speed", Fmt::Percent),
    ("movement_time_powerslide", "powerslide time", Fmt::SecondsPerMinute),
    ("movement_total_distance", "distance covered", Fmt::WholeUuPerMinute),
    ("positioning_avg_distance_to_ball", "average distance to the ball", Fmt::Whole("uu")),
    ("positioning_avg_distance_to_ball_no_possession", "average distance to the ball when the opponents have it", Fmt::Whole("uu")),
    ("positioning_avg_distance_to_ball_possession", "average distance to the ball when your team has it", Fmt::Whole("uu")),
    ("positioning_avg_distance_to_mates", "average distance to your teammate", Fmt::Whole("uu")),
    ("positioning_goals_against_while_last_defender", "goals conceded as last defender", Fmt::PerMinute),
    ("positioning_percent_behind_ball", "time behind the ball", Fmt::Percent),
    ("positioning_percent_closest_to_ball", "time closest to the ball", Fmt::Percent),
    ("positioning_percent_defensive_half", "time in your own half", Fmt::Percent),
    ("positioning_percent_defensive_third", "time in your defensive third", Fmt::Percent),
    ("positioning_percent_farthest_from_ball", "time farthest from the ball", Fmt::Percent),
    ("positioning_percent_infront_ball", "time in front of the ball", Fmt::Percent),
    ("positioning_percent_most_back", "time as the furthest-back player", Fmt::Percent),
    ("positioning_percent_most_forward", "time as the furthest-forward player", Fmt::Percent),
    ("positioning_percent_neutral_third", "time in the middle third", Fmt::Percent),
    ("positioning_percent_offensive_half", "time in the attacking half", Fmt::Percent),
    ("positioning_percent_offensive_third", "time in the attacking third", Fmt::Percent),
];

/// What to practice, by finding id. Keyed by id so the narrator's citations
/// and the verifier can refer to the same entries.
pub const ADVICE: &[(&str, &str)] = &[
    ("movement.avg_speed", "Keep your speed up between touches: boost or flip toward the play instead of driving at it, and land facing where you want to go next."),
    ("movement.percent_supersonic_speed", "Chain speed instead of resetting it: stay on boost through rotations and use flips to hold supersonic rather than braking into turns."),
    ("movement.percent_boost_speed", "Spend less time at base speed: flip forward when you're not boosting so you stay at boost speed."),
    ("movement.percent_slow_speed", "Avoid stopping and three-point turns: powerslide-turn, half-flip to reverse direction, and wavedash out of landings."),
    ("movement.count_powerslide", "Use powerslide for tighter turns, especially when turning back on defense and when lining up a shot."),
    ("movement.time_powerslide", "Use powerslide for tighter turns, especially when turning back on defense and when lining up a shot."),
    ("movement.avg_powerslide_duration", "Use short powerslide taps instead of long slides; long slides bleed speed."),
    ("movement.total_distance", "Keep moving and rotating instead of waiting in one spot. Covering more ground keeps you in the play."),
    ("movement.percent_high_air", "Work on fast aerials in training packs, then challenge high balls sooner instead of waiting for them to drop."),
    ("movement.percent_low_air", "Contest bouncing balls with small jumps instead of waiting for them to land."),
    ("movement.percent_ground", "Get off the ground more: jump-challenge bouncing balls and meet 50s in the air."),
    ("core.aerial_hits", "Add aerial training packs to your warmup and go for touches you'd normally let drop."),
    ("boost.bpm", "Use more of your boost: hold it to keep speed through rotations and challenges instead of saving it at 100."),
    ("boost.bcpm", "Collect more boost along your path: route rotations through pads instead of driving straight back."),
    ("boost.amount_collected", "Collect more boost along your path: route rotations through pads instead of driving straight back."),
    ("boost.count_collected_small", "Grab small pads on every rotation back. They refill you without a detour to a big boost."),
    ("boost.amount_collected_small", "Grab small pads on every rotation back. They refill you without a detour to a big boost."),
    ("boost.count_collected_big", "Take corner and midfield big boosts when your rotation passes them, so you arrive at the next play with boost."),
    ("boost.amount_collected_big", "Take corner and midfield big boosts when your rotation passes them, so you arrive at the next play with boost."),
    ("boost.amount_stolen", "When you're the forward player, take the opponents' corner or back-wall boost before rotating out: it fuels you and starves them."),
    ("boost.count_stolen_small", "When you're the forward player, take the opponents' small pads on your way out of their half."),
    ("boost.amount_stolen_big", "When you're the forward player, take the opponents' corner boost before rotating out."),
    ("boost.percent_zero_boost", "Avoid running empty: pick up small pads on the way back and feather boost instead of holding it down."),
    ("boost.amount_overfill", "Skip big boosts when you're already high and leave them for your teammate; take small pads instead."),
    ("core.shots", "Look for more shooting chances: follow up your teammate's touches and shoot open balls on your first touch."),
    ("core.saves", "Shadow-defend on the way back so you're in position, and practice saves in defensive training packs."),
    ("positioning.percent_neutral_third", "As second player, hold midfield more often instead of retreating to net or overcommitting forward."),
    ("positioning.percent_defensive_third", "Once the ball is cleared, push up to midfield with your teammate instead of staying deep."),
    ("demo.inflicted", "Go for demos on the last defender or goalie when you're already at speed; it can open the net."),
    ("demo.inflicted_in_offensive_half", "Go for demos on the last defender or goalie when you're already at speed; it can open the net."),
    ("demo.taken", "Watch for demo threats: keep moving on defense and avoid stalling in predictable spots."),
    ("demo.taken_in_defensive_half", "Keep moving in net and avoid sitting still on the goal line, where you're easy to demo."),
];

pub fn advice_for(id: &str) -> Option<&'static str> {
    ADVICE.iter().find(|(k, _)| *k == id).map(|(_, a)| *a)
}

fn label_and_fmt(stat: &str) -> (String, Fmt) {
    if let Some((_, label, fmt)) = LABELS.iter().find(|(k, _, _)| *k == stat) {
        return ((*label).to_string(), *fmt);
    }
    // Fallback for stats added later: a readable name without digits, and a
    // format from how the benchmark expresses the stat.
    let short = stat.split_once('_').map_or(stat, |(_, s)| s);
    let fmt = if short.starts_with("percent_") {
        Fmt::Percent
    } else if crate::profile::stat_unit(stat) == crate::benchmark::StatUnit::PerMinute {
        Fmt::PerMinute
    } else {
        Fmt::Plain
    };
    let words: Vec<&str> = short
        .trim_start_matches("percent_")
        .split('_')
        .filter(|w| !w.chars().any(|c| c.is_ascii_digit()))
        .collect();
    (words.join(" "), fmt)
}

fn thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// The number alone, and the unit that follows it.
fn parts(v: f64, fmt: Fmt) -> (String, String) {
    match fmt {
        Fmt::Percent => (format!("{v:.1}"), "%".into()),
        Fmt::PerMinute => (format!("{v:.2}"), " per minute".into()),
        Fmt::Whole("") => (thousands(v.round() as i64), String::new()),
        Fmt::Whole(unit) => (thousands(v.round() as i64), format!(" {unit}")),
        Fmt::Seconds => (format!("{v:.2}"), " s".into()),
        Fmt::SecondsPerMinute => (format!("{v:.2}"), " s per minute".into()),
        Fmt::WholeUuPerMinute => (thousands(v.round() as i64), " uu per minute".into()),
        Fmt::Plain => (format!("{v:.1}"), String::new()),
    }
}

fn fmt_value(v: f64, fmt: Fmt) -> String {
    let (n, unit) = parts(v, fmt);
    format!("{n}{unit}")
}

/// "15.79 to 16.75 per minute", "30.2% to 32.3%"
fn fmt_range(lo: f64, hi: f64, fmt: Fmt) -> String {
    let ((a, unit), (b, _)) = (parts(lo, fmt), parts(hi, fmt));
    if unit == "%" { format!("{a}% to {b}%") } else { format!("{a} to {b}{unit}") }
}

/// "diamond-2" -> "Diamond 2", "grand-champion-1" -> "Grand Champion 1"
pub fn tier_name(rank: &str) -> String {
    if rank == "supersonic-legend" {
        return "Supersonic Legend".into();
    }
    rank.split('-')
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn item(f: &Finding, pack: &EvidencePack, top: bool) -> ReportItem {
    let (label, fmt) = label_and_fmt(&f.stat_key);
    let name = capitalize(&label);
    let cur = tier_name(&pack.current_tier_used);
    let tgt = tier_name(&pack.target_tier_used);
    let v = fmt_value(f.value, fmt);
    let range = format!("80% range {}", fmt_range(f.interval.0, f.interval.1, fmt));
    // A stat that barely separates ranks can't be "holding you back" or a
    // "strength" for climbing: describe it as different, not better or worse.
    let weak_signal = f.rank_signal < MIN_RANK_SIGNAL && f.kind != FindingKind::Uncertain;
    let (title, explanation) = match f.kind {
        _ if weak_signal => (
            format!("Unusual for your rank: {label}"),
            format!(
                "{name}: {v} ({range}), compared with around {} for {cur} players. This stat doesn't strongly separate ranks, so it may simply be your style.",
                fmt_value(f.current_tier_median, fmt)
            ),
        ),
        FindingKind::Weakness => (
            if top { format!("Holding you back: {label}") } else { format!("Below your rank: {label}") },
            format!(
                "{name}: {v} ({range}). {cur} players are around {}, and {tgt} players around {}. You're ahead of only about {:.0}% of {cur} players here.",
                fmt_value(f.current_tier_median, fmt),
                fmt_value(f.target_tier_median, fmt),
                f.pct_current_tier
            ),
        ),
        FindingKind::Strength => (
            format!("Strength: {label}"),
            format!(
                "{name}: {v} ({range}). That's ahead of about {:.0}% of {cur} players and {:.0}% of {tgt} players.",
                f.pct_current_tier, f.pct_target_tier
            ),
        ),
        FindingKind::Uncertain => (
            format!("Not enough games to tell: {label}"),
            format!(
                "{name}: {v}, which looks {} for {cur}. But with {} games the {range} still overlaps the usual range, so a few more games will tell.",
                if f.pct_current_tier < 50.0 { "weak" } else { "strong" },
                f.games
            ),
        ),
    };
    ReportItem {
        finding_id: f.id.clone(),
        kind: f.kind,
        title,
        explanation,
        action: (f.kind == FindingKind::Weakness && !weak_signal).then(|| advice_for(&f.id).map(str::to_string)).flatten(),
        top,
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

pub fn render(pack: &EvidencePack) -> CoachingReport {
    let cur = tier_name(&pack.current_tier_used);
    let tgt = tier_name(&pack.target_tier_used);
    let mut report = CoachingReport {
        pack_schema_version: pack.schema_version,
        summary: String::new(),
        items: Vec::new(),
        source: "templates".into(),
    };
    if !pack.enough_data {
        report.summary = format!(
            "Not enough data yet: coaching needs at least {} games and there are {}. Play a few more and check back.",
            pack.min_games, pack.games
        );
        return report;
    }

    let by_id = |id: &String| pack.findings.iter().find(|f| &f.id == id);
    let label = |id: &String| by_id(id).map(|f| label_and_fmt(&f.stat_key).0).unwrap_or_default();
    let strengths: Vec<String> = pack.top_strengths.iter().map(label).collect();
    let weaknesses: Vec<String> = pack.top_weaknesses.iter().map(label).collect();
    let target_phrase = if pack.target_tier_used == pack.current_tier_used { cur.clone() } else { tgt.clone() };
    report.summary = match (strengths.is_empty(), weaknesses.is_empty()) {
        (true, true) => format!(
            "Across {} games, nothing stands out against {cur} players on the stats that best separate ranks. Your play is typical for your rank.",
            pack.games
        ),
        (false, true) => format!(
            "Across {} games, your clearest strengths against {cur} players are {}. No stat that separates ranks is clearly holding you back.",
            pack.games,
            list(&strengths)
        ),
        (true, false) => format!(
            "Across {} games, the biggest gaps between you and {target_phrase} are {}.",
            pack.games,
            list(&weaknesses)
        ),
        (false, false) => format!(
            "Across {} games, your clearest strengths against {cur} players are {}. The biggest gaps between you and {target_phrase} are {}.",
            pack.games,
            list(&strengths),
            list(&weaknesses)
        ),
    };

    let mut push = |f: &Finding, top: bool| report.items.push(item(f, pack, top));
    for id in pack.top_weaknesses.iter().chain(&pack.top_strengths) {
        if let Some(f) = by_id(id) {
            push(f, true);
        }
    }
    let is_top = |f: &Finding| pack.top_weaknesses.contains(&f.id) || pack.top_strengths.contains(&f.id);
    for kind in [FindingKind::Weakness, FindingKind::Strength, FindingKind::Uncertain] {
        for f in pack.findings.iter().filter(|f| f.kind == kind && !is_top(f)) {
            push(f, false);
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_names() {
        assert_eq!(tier_name("diamond-2"), "Diamond 2");
        assert_eq!(tier_name("grand-champion-1"), "Grand Champion 1");
        assert_eq!(tier_name("supersonic-legend"), "Supersonic Legend");
    }

    #[test]
    fn value_formats() {
        assert_eq!(fmt_value(12.345, Fmt::Percent), "12.3%");
        assert_eq!(fmt_value(0.8234, Fmt::PerMinute), "0.82 per minute");
        assert_eq!(fmt_value(1523.4, Fmt::Whole("uu/s")), "1,523 uu/s");
        assert_eq!(fmt_value(2678.6, Fmt::Whole("uu")), "2,679 uu");
        assert_eq!(fmt_value(473.6, Fmt::Whole("")), "474");
        assert_eq!(fmt_value(2.093, Fmt::SecondsPerMinute), "2.09 s per minute");
        assert_eq!(fmt_value(34210.4, Fmt::WholeUuPerMinute), "34,210 uu per minute");
        assert_eq!(fmt_range(15.79, 16.75, Fmt::PerMinute), "15.79 to 16.75 per minute");
        assert_eq!(fmt_range(30.21, 32.33, Fmt::Percent), "30.2% to 32.3%");
    }

    #[test]
    fn at_least_fifteen_finding_ids_have_advice() {
        assert!(ADVICE.len() >= 15);
        let mut ids: Vec<&str> = ADVICE.iter().map(|(k, _)| *k).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), ADVICE.len(), "duplicate advice ids");
    }

    #[test]
    fn fallback_labels_have_no_digits() {
        assert_eq!(label_and_fmt("boost_percent_boost_10_20").0, "boost");
        assert_eq!(label_and_fmt("core_new_stat").0, "new stat");
        assert!(matches!(label_and_fmt("positioning_avg_new_distance").1, Fmt::Plain));
    }

    #[test]
    fn labels_are_clean() {
        for (k, label, _) in LABELS {
            assert!(!label.chars().any(|c| c.is_ascii_digit() || c == '_'), "{k}: {label}");
        }
    }
}
