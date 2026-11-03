use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayData {
    pub id: String,
    pub title: Option<String>,
    pub map_code: Option<String>,
    pub map_name: Option<String>,
    pub playlist_name: Option<String>,
    pub duration: Option<u32>,
    pub overtime: Option<bool>,
    pub overtime_seconds: Option<u32>,
    pub season: Option<u32>,
    pub date: Option<String>,
    pub blue: Option<TeamData>,
    pub orange: Option<TeamData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamData {
    pub name: Option<String>,
    pub players: Option<Vec<PlayerData>>,
    pub stats: Option<TeamStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStats {
    pub core: Option<CoreStats>,
    pub boost: Option<BoostStats>,
    pub movement: Option<MovementStats>,
    pub positioning: Option<PositioningStats>,
    pub demo: Option<DemoStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub name: Option<String>,
    pub id: Option<PlayerId>,
    pub car_name: Option<String>,
    pub mvp: Option<bool>,
    pub stats: Option<PlayerStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerId {
    pub platform: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub core: Option<CoreStats>,
    pub boost: Option<BoostStats>,
    pub movement: Option<MovementStats>,
    pub positioning: Option<PositioningStats>,
    pub demo: Option<DemoStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreStats {
    pub shots: Option<u32>,
    pub shots_against: Option<u32>,
    pub goals: Option<u32>,
    pub goals_against: Option<u32>,
    pub saves: Option<u32>,
    pub assists: Option<u32>,
    pub score: Option<u32>,
    pub shooting_percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoostStats {
    pub bpm: Option<f64>,
    pub bcpm: Option<f64>,
    pub avg_amount: Option<f64>,
    pub amount_collected: Option<f64>,
    pub amount_stolen: Option<f64>,
    pub amount_collected_big: Option<f64>,
    pub amount_stolen_big: Option<f64>,
    pub amount_collected_small: Option<f64>,
    pub amount_stolen_small: Option<f64>,
    pub count_collected_big: Option<u32>,
    pub count_stolen_big: Option<u32>,
    pub count_collected_small: Option<u32>,
    pub count_stolen_small: Option<u32>,
    pub amount_overfill: Option<f64>,
    pub amount_overfill_stolen: Option<f64>,
    pub amount_used_while_supersonic: Option<f64>,
    pub time_zero_boost: Option<f64>,
    pub percent_zero_boost: Option<f64>,
    pub time_full_boost: Option<f64>,
    pub percent_full_boost: Option<f64>,
    pub time_boost_0_25: Option<f64>,
    pub time_boost_25_50: Option<f64>,
    pub time_boost_50_75: Option<f64>,
    pub time_boost_75_100: Option<f64>,
    pub percent_boost_0_25: Option<f64>,
    pub percent_boost_25_50: Option<f64>,
    pub percent_boost_50_75: Option<f64>,
    pub percent_boost_75_100: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementStats {
    pub avg_speed: Option<f64>,
    pub total_distance: Option<f64>,
    pub time_supersonic_speed: Option<f64>,
    pub time_boost_speed: Option<f64>,
    pub time_slow_speed: Option<f64>,
    pub time_ground: Option<f64>,
    pub time_low_air: Option<f64>,
    pub time_high_air: Option<f64>,
    pub time_powerslide: Option<f64>,
    pub count_powerslide: Option<u32>,
    pub avg_powerslide_duration: Option<f64>,
    pub avg_speed_percentage: Option<f64>,
    pub percent_slow_speed: Option<f64>,
    pub percent_boost_speed: Option<f64>,
    pub percent_supersonic_speed: Option<f64>,
    pub percent_ground: Option<f64>,
    pub percent_low_air: Option<f64>,
    pub percent_high_air: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositioningStats {
    pub avg_distance_to_ball: Option<f64>,
    pub avg_distance_to_ball_possession: Option<f64>,
    pub avg_distance_to_ball_no_possession: Option<f64>,
    pub avg_distance_to_mates: Option<f64>,
    pub time_defensive_third: Option<f64>,
    pub time_neutral_third: Option<f64>,
    pub time_offensive_third: Option<f64>,
    pub time_defensive_half: Option<f64>,
    pub time_offensive_half: Option<f64>,
    pub time_behind_ball: Option<f64>,
    pub time_infront_ball: Option<f64>,
    pub time_most_back: Option<f64>,
    pub time_most_forward: Option<f64>,
    pub time_closest_to_ball: Option<f64>,
    pub time_farthest_from_ball: Option<f64>,
    pub percent_defensive_third: Option<f64>,
    pub percent_offensive_third: Option<f64>,
    pub percent_neutral_third: Option<f64>,
    pub percent_defensive_half: Option<f64>,
    pub percent_offensive_half: Option<f64>,
    pub percent_behind_ball: Option<f64>,
    pub percent_infront_ball: Option<f64>,
    pub percent_most_back: Option<f64>,
    pub percent_most_forward: Option<f64>,
    pub percent_closest_to_ball: Option<f64>,
    pub percent_farthest_from_ball: Option<f64>,
    pub goals_against_while_last_defender: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoStats {
    pub inflicted: Option<u32>,
    pub taken: Option<u32>,
}

// --- Replay search/list types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayListResponse {
    pub list: Vec<ReplayListItem>,
    pub count: Option<u32>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayListItem {
    pub id: String,
    pub date: Option<String>,
    pub playlist_id: Option<String>,
    pub playlist_name: Option<String>,
    pub duration: Option<u32>,
    pub overtime: Option<bool>,
    pub overtime_seconds: Option<u32>,
    pub map_code: Option<String>,
    pub map_name: Option<String>,
    pub min_rank: Option<RankInfo>,
    pub max_rank: Option<RankInfo>,
    pub season: Option<u32>,
    pub replay_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankInfo {
    pub id: Option<String>,
    pub tier: Option<u32>,
    pub division: Option<u32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySearchParams {
    pub player_id: Option<String>,
    pub playlist: Option<String>,
    /// `1`-`14` for legacy seasons, `f1`, `f2`, ... for free-to-play seasons.
    pub season: Option<String>,
    pub count: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub next: Option<String>,
}
