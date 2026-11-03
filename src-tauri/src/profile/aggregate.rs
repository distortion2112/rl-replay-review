use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::benchmark::StatUnit;

/// Share of the game a player must be present for to count, matching the
/// benchmark generator's row filter.
pub const FULL_GAME_SHARE: f64 = 0.9;

/// How a ballchasing stat is expressed in the benchmarks. Mirrors
/// `stat_unit` in `tools/benchmark-generator/benchmarks.py`: percentages,
/// averages and rates as-is; totals per minute of game time.
pub fn stat_unit(name: &str) -> StatUnit {
    const RATES: [&str; 3] = ["boost_bpm", "boost_bcpm", "core_shooting_percentage"];
    let short = name.split_once('_').map_or("", |(_, s)| s);
    if RATES.contains(&name) || short.starts_with("percent_") || short.starts_with("avg_") {
        StatUnit::AsIs
    } else {
        StatUnit::PerMinute
    }
}

/// One game for one player. `values` are keyed `<group>_<stat>` in benchmark units.
#[derive(Debug, Clone, Serialize)]
pub struct GameStats {
    pub replay_id: String,
    pub date: Option<String>,
    pub team: String,
    pub won: bool,
    pub score_diff: i64,
    pub duration_s: f64,
    pub overtime: bool,
    pub values: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Skipped {
    pub replay_id: String,
    pub reason: String,
}

/// Why a replay contributes no game for the player.
fn skip(detail: &Value, reason: impl Into<String>) -> Skipped {
    Skipped {
        replay_id: detail.get("id").and_then(Value::as_str).unwrap_or("?").to_string(),
        reason: reason.into(),
    }
}

/// Extracts `player` (`<platform>:<id>`) from a ballchasing replay detail
/// (`GET /replays/{id}`), with every numeric stat flattened to `<group>_<stat>`
/// and totals converted to per-minute.
pub fn player_game(detail: &Value, player: &str) -> Result<GameStats, Skipped> {
    if detail.get("status").and_then(Value::as_str) != Some("ok") {
        return Err(skip(detail, "replay still processing on ballchasing"));
    }
    let duration = detail.get("duration").and_then(Value::as_f64).unwrap_or(0.0);
    if duration <= 0.0 {
        return Err(skip(detail, "replay has no duration"));
    }
    let goals = |color: &str| {
        detail
            .pointer(&format!("/{color}/stats/core/goals"))
            .and_then(Value::as_i64)
            .unwrap_or(0)
    };
    for (color, other) in [("blue", "orange"), ("orange", "blue")] {
        let players = detail.pointer(&format!("/{color}/players")).and_then(Value::as_array);
        for p in players.into_iter().flatten() {
            let id = format!(
                "{}:{}",
                p.pointer("/id/platform").and_then(Value::as_str).unwrap_or(""),
                p.pointer("/id/id").and_then(Value::as_str).unwrap_or("")
            );
            if id != player {
                continue;
            }
            let played = p.get("end_time").and_then(Value::as_f64).unwrap_or(0.0)
                - p.get("start_time").and_then(Value::as_f64).unwrap_or(0.0);
            if played < FULL_GAME_SHARE * duration {
                return Err(skip(detail, "player missed part of the game"));
            }
            let minutes = duration / 60.0;
            let mut values = BTreeMap::new();
            if let Some(groups) = p.get("stats").and_then(Value::as_object) {
                for (group, stats) in groups {
                    for (k, v) in stats.as_object().into_iter().flatten() {
                        // Numbers only; booleans like core.mvp are not stats
                        let Some(x) = v.as_f64().filter(|_| v.is_number()) else { continue };
                        let key = format!("{group}_{k}");
                        let x = match stat_unit(&key) {
                            StatUnit::AsIs => x,
                            StatUnit::PerMinute => x / minutes,
                        };
                        values.insert(key, x);
                    }
                }
            }
            let (mine, theirs) = (goals(color), goals(other));
            return Ok(GameStats {
                replay_id: detail.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
                date: detail.get("date").and_then(Value::as_str).map(str::to_string),
                team: color.to_string(),
                won: mine > theirs,
                score_diff: mine - theirs,
                duration_s: duration,
                overtime: detail.get("overtime").and_then(Value::as_bool).unwrap_or(false),
                values,
            });
        }
    }
    Err(skip(detail, "player not in this replay"))
}

/// A player's games, plus which replays were left out and why.
#[derive(Debug, Clone, Serialize)]
pub struct PlayerAggregate {
    /// `<platform>:<id>`
    pub player: String,
    pub name: Option<String>,
    pub games: Vec<GameStats>,
    pub game_count: u32,
    pub win_count: u32,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
    pub skipped: Vec<Skipped>,
}

impl PlayerAggregate {
    /// Builds from replay details. Replays without a usable game for the
    /// player are listed in `skipped` rather than failing the whole aggregate.
    pub fn from_details<'a>(player: &str, details: impl IntoIterator<Item = &'a Value>) -> Self {
        let mut games = Vec::new();
        let mut skipped = Vec::new();
        let mut name = None;
        for detail in details {
            match player_game(detail, player) {
                Ok(g) => {
                    if name.is_none() {
                        name = find_name(detail, player);
                    }
                    games.push(g);
                }
                Err(s) => skipped.push(s),
            }
        }
        games.sort_by(|a, b| a.date.cmp(&b.date));
        PlayerAggregate {
            player: player.to_string(),
            name,
            game_count: games.len() as u32,
            win_count: games.iter().filter(|g| g.won).count() as u32,
            first_date: games.first().and_then(|g| g.date.clone()),
            last_date: games.last().and_then(|g| g.date.clone()),
            games,
            skipped,
        }
    }

    /// Per-game values for one stat, from the games that report it.
    pub fn values(&self, stat: &str) -> Vec<f64> {
        self.games.iter().filter_map(|g| g.values.get(stat).copied()).collect()
    }

    pub fn mean(&self, stat: &str) -> Option<f64> {
        let v = self.values(stat);
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    }
}

fn find_name(detail: &Value, player: &str) -> Option<String> {
    ["blue", "orange"].iter().find_map(|c| {
        detail.pointer(&format!("/{c}/players"))?.as_array()?.iter().find_map(|p| {
            let id = format!(
                "{}:{}",
                p.pointer("/id/platform")?.as_str()?,
                p.pointer("/id/id")?.as_str()?
            );
            (id == player).then(|| p.get("name")?.as_str().map(str::to_string)).flatten()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn units_match_benchmark_generator() {
        assert_eq!(stat_unit("core_goals"), StatUnit::PerMinute);
        assert_eq!(stat_unit("positioning_percent_behind_ball"), StatUnit::AsIs);
        assert_eq!(stat_unit("positioning_avg_distance_to_ball"), StatUnit::AsIs);
        assert_eq!(stat_unit("boost_bpm"), StatUnit::AsIs);
        assert_eq!(stat_unit("core_shooting_percentage"), StatUnit::AsIs);
        assert_eq!(stat_unit("demo_inflicted"), StatUnit::PerMinute);
    }

    fn detail(status: &str, start: f64) -> Value {
        json!({
            "id": "r1", "status": status, "duration": 300, "date": "2026-10-01T00:00:00Z", "overtime": false,
            "blue": { "stats": { "core": { "goals": 1 } }, "players": [
                { "id": { "platform": "epic", "id": "me" }, "name": "Me", "start_time": start, "end_time": 300,
                  "stats": { "core": { "goals": 1, "mvp": true }, "boost": { "bpm": 400 } } } ] },
            "orange": { "stats": { "core": { "goals": 2 } }, "players": [] }
        })
    }

    #[test]
    fn skips_with_reasons() {
        assert!(player_game(&detail("pending", 0.0), "epic:me").unwrap_err().reason.contains("processing"));
        assert!(player_game(&detail("ok", 100.0), "epic:me").unwrap_err().reason.contains("missed"));
        assert!(player_game(&detail("ok", 0.0), "epic:other").unwrap_err().reason.contains("not in"));
    }

    #[test]
    fn booleans_are_not_stats_and_loss_is_recorded() {
        let g = player_game(&detail("ok", 0.0), "epic:me").unwrap();
        assert!(!g.values.contains_key("core_mvp"));
        assert_eq!(g.values["core_goals"], 1.0 / 5.0);
        assert!(!g.won);
        assert_eq!(g.score_diff, -1);
    }

    #[test]
    fn aggregate_keeps_partial_results() {
        let a = PlayerAggregate::from_details("epic:me", [&detail("ok", 0.0), &detail("ok", 100.0)]);
        assert_eq!(a.game_count, 1);
        assert_eq!(a.skipped.len(), 1);
        assert_eq!(a.name.as_deref(), Some("Me"));
        assert_eq!(a.mean("boost_bpm"), Some(400.0));
        assert_eq!(a.mean("boost_missing"), None);
    }
}
