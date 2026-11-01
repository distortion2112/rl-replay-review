use serde::Serialize;

use crate::api::models::PlayerData;

#[derive(Debug, Clone, Serialize)]
pub struct BoostInsight {
    pub player_name: String,
    pub efficiency_score: f64,
    pub small_pad_ratio: f64,
    pub overfill_pct: f64,
    pub zero_boost_pct: f64,
    pub stolen_amount: f64,
    pub rating: String,
    pub explanation: String,
}

pub fn analyze_boost(players: &[PlayerData]) -> Vec<BoostInsight> {
    players
        .iter()
        .map(|player| {
            let name = player.name.clone().unwrap_or("Unknown".into());
            let boost = player.stats.as_ref().and_then(|s| s.boost.as_ref());

            let small_collected = boost
                .and_then(|b| b.count_collected_small)
                .unwrap_or(0) as f64;
            let big_collected = boost
                .and_then(|b| b.count_collected_big)
                .unwrap_or(0) as f64;
            let total_pads = small_collected + big_collected;

            let small_pad_ratio = if total_pads > 0.0 {
                small_collected / total_pads
            } else {
                0.0
            };

            let amount_collected = boost.and_then(|b| b.amount_collected).unwrap_or(0.0);
            let overfill = boost.and_then(|b| b.amount_overfill).unwrap_or(0.0);
            let overfill_pct = if amount_collected > 0.0 {
                (overfill / amount_collected) * 100.0
            } else {
                0.0
            };

            let zero_boost_pct = boost.and_then(|b| b.percent_zero_boost).unwrap_or(0.0);
            let stolen = boost.and_then(|b| b.amount_stolen).unwrap_or(0.0);

            // Efficiency score: 0-100
            // Rewards: high small pad ratio, low overfill, low zero-boost time, boost stealing
            let small_pad_score = small_pad_ratio * 30.0;
            let overfill_score = (1.0 - (overfill_pct / 100.0).min(1.0)) * 25.0;
            let zero_boost_score = (1.0 - (zero_boost_pct / 100.0).min(1.0)) * 25.0;
            let steal_score = (stolen / 500.0).min(1.0) * 20.0;

            let efficiency_score =
                (small_pad_score + overfill_score + zero_boost_score + steal_score).min(100.0);

            let (rating, explanation) = if efficiency_score >= 75.0 {
                (
                    "excellent".to_string(),
                    format!(
                        "{} has excellent boost management. Small pad ratio: {:.0}%, overfill: {:.0}%, \
                         zero boost: {:.0}%.",
                        name,
                        small_pad_ratio * 100.0,
                        overfill_pct,
                        zero_boost_pct
                    ),
                )
            } else if efficiency_score >= 50.0 {
                (
                    "good".to_string(),
                    format!(
                        "{} has decent boost efficiency ({:.0}/100). Consider using more small pads \
                         (current ratio: {:.0}%).",
                        name,
                        efficiency_score,
                        small_pad_ratio * 100.0
                    ),
                )
            } else if efficiency_score >= 25.0 {
                (
                    "needs_work".to_string(),
                    format!(
                        "{} could improve boost management ({:.0}/100). High overfill ({:.0}%) and \
                         {:.0}% time with zero boost suggest over-reliance on big pads.",
                        name, efficiency_score, overfill_pct, zero_boost_pct
                    ),
                )
            } else {
                (
                    "poor".to_string(),
                    format!(
                        "{} has poor boost efficiency ({:.0}/100). Focus on collecting small pads \
                         and reducing wasted boost.",
                        name, efficiency_score
                    ),
                )
            };

            BoostInsight {
                player_name: name,
                efficiency_score,
                small_pad_ratio,
                overfill_pct,
                zero_boost_pct,
                stolen_amount: stolen,
                rating,
                explanation,
            }
        })
        .collect()
}
