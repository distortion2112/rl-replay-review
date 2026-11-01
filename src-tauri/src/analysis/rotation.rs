use serde::Serialize;

use crate::api::models::PlayerData;

#[derive(Debug, Clone, Serialize)]
pub struct RotationInsight {
    pub player_name: String,
    pub is_ball_chasing: bool,
    pub offensive_third_pct: f64,
    pub avg_distance_to_ball: f64,
    pub team_avg_distance_to_ball: f64,
    pub closest_to_ball_pct: f64,
    pub severity: String,
    pub explanation: String,
}

pub fn analyze_rotation(players: &[PlayerData]) -> Vec<RotationInsight> {
    let team_avg_ball_dist: f64 = players
        .iter()
        .filter_map(|p| {
            p.stats
                .as_ref()
                .and_then(|s| s.positioning.as_ref())
                .and_then(|pos| pos.avg_distance_to_ball)
        })
        .sum::<f64>()
        / players.len().max(1) as f64;

    players
        .iter()
        .map(|player| {
            let name = player.name.clone().unwrap_or("Unknown".into());
            let positioning = player
                .stats
                .as_ref()
                .and_then(|s| s.positioning.as_ref());

            let offensive_pct = positioning
                .and_then(|p| p.percent_offensive_third)
                .unwrap_or(0.0);
            let avg_dist = positioning
                .and_then(|p| p.avg_distance_to_ball)
                .unwrap_or(0.0);
            let closest_pct = positioning
                .and_then(|p| p.percent_closest_to_ball)
                .unwrap_or(0.0);

            let dist_ratio = if team_avg_ball_dist > 0.0 {
                avg_dist / team_avg_ball_dist
            } else {
                1.0
            };

            let is_ball_chasing = offensive_pct > 40.0 && dist_ratio < 0.85 && closest_pct > 40.0;

            let (severity, explanation) = if is_ball_chasing {
                if offensive_pct > 55.0 && dist_ratio < 0.7 {
                    (
                        "high".to_string(),
                        format!(
                            "{} spent {:.0}% in the offensive third with {:.0}% closest to ball. \
                             Avg ball distance is {:.0}% below team average — strong ball-chasing pattern.",
                            name,
                            offensive_pct,
                            closest_pct,
                            (1.0 - dist_ratio) * 100.0
                        ),
                    )
                } else {
                    (
                        "moderate".to_string(),
                        format!(
                            "{} shows some ball-chasing tendencies: {:.0}% offensive third, \
                             {:.0}% closest to ball.",
                            name, offensive_pct, closest_pct
                        ),
                    )
                }
            } else {
                (
                    "none".to_string(),
                    format!("{} has healthy rotation patterns.", name),
                )
            };

            RotationInsight {
                player_name: name,
                is_ball_chasing,
                offensive_third_pct: offensive_pct,
                avg_distance_to_ball: avg_dist,
                team_avg_distance_to_ball: team_avg_ball_dist,
                closest_to_ball_pct: closest_pct,
                severity,
                explanation,
            }
        })
        .collect()
}
