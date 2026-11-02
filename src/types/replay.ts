export interface ReplayData {
  id: string;
  title?: string;
  map_code?: string;
  map_name?: string;
  playlist_name?: string;
  duration?: number;
  overtime?: boolean;
  overtime_seconds?: number;
  season?: number;
  date?: string;
  blue?: TeamData;
  orange?: TeamData;
}

export interface TeamData {
  name?: string;
  players?: PlayerData[];
  stats?: TeamStats;
}

export interface TeamStats {
  core?: CoreStats;
  boost?: BoostStats;
  movement?: MovementStats;
  positioning?: PositioningStats;
  demo?: DemoStats;
}

export interface PlayerData {
  name?: string;
  id?: PlayerId;
  car_name?: string;
  mvp?: boolean;
  stats?: PlayerStats;
}

export interface PlayerId {
  platform?: string;
  id?: string;
}

export interface PlayerStats {
  core?: CoreStats;
  boost?: BoostStats;
  movement?: MovementStats;
  positioning?: PositioningStats;
  demo?: DemoStats;
}

export interface CoreStats {
  shots?: number;
  shots_against?: number;
  goals?: number;
  goals_against?: number;
  saves?: number;
  assists?: number;
  score?: number;
  shooting_percentage?: number;
}

export interface BoostStats {
  bpm?: number;
  bcpm?: number;
  avg_amount?: number;
  amount_collected?: number;
  amount_stolen?: number;
  amount_collected_big?: number;
  amount_stolen_big?: number;
  amount_collected_small?: number;
  amount_stolen_small?: number;
  count_collected_big?: number;
  count_stolen_big?: number;
  count_collected_small?: number;
  count_stolen_small?: number;
  amount_overfill?: number;
  amount_overfill_stolen?: number;
  amount_used_while_supersonic?: number;
  time_zero_boost?: number;
  percent_zero_boost?: number;
  time_full_boost?: number;
  percent_full_boost?: number;
  time_boost_0_25?: number;
  time_boost_25_50?: number;
  time_boost_50_75?: number;
  time_boost_75_100?: number;
  percent_boost_0_25?: number;
  percent_boost_25_50?: number;
  percent_boost_50_75?: number;
  percent_boost_75_100?: number;
}

export interface MovementStats {
  avg_speed?: number;
  total_distance?: number;
  time_supersonic_speed?: number;
  time_boost_speed?: number;
  time_slow_speed?: number;
  time_ground?: number;
  time_low_air?: number;
  time_high_air?: number;
  time_powerslide?: number;
  count_powerslide?: number;
  avg_powerslide_duration?: number;
  avg_speed_percentage?: number;
  percent_slow_speed?: number;
  percent_boost_speed?: number;
  percent_supersonic_speed?: number;
  percent_ground?: number;
  percent_low_air?: number;
  percent_high_air?: number;
}

export interface PositioningStats {
  avg_distance_to_ball?: number;
  avg_distance_to_ball_possession?: number;
  avg_distance_to_ball_no_possession?: number;
  avg_distance_to_mates?: number;
  time_defensive_third?: number;
  time_neutral_third?: number;
  time_offensive_third?: number;
  time_defensive_half?: number;
  time_offensive_half?: number;
  time_behind_ball?: number;
  time_infront_ball?: number;
  time_most_back?: number;
  time_most_forward?: number;
  time_closest_to_ball?: number;
  time_farthest_from_ball?: number;
  percent_defensive_third?: number;
  percent_offensive_third?: number;
  percent_neutral_third?: number;
  percent_defensive_half?: number;
  percent_offensive_half?: number;
  percent_behind_ball?: number;
  percent_infront_ball?: number;
  percent_most_back?: number;
  percent_most_forward?: number;
  percent_closest_to_ball?: number;
  percent_farthest_from_ball?: number;
  goals_against_while_last_defender?: number;
}

export interface DemoStats {
  inflicted?: number;
  taken?: number;
}

// --- Replay search/list types ---

export interface ReplayListResponse {
  list: ReplayListItem[];
  count?: number;
  next?: string;
}

export interface ReplayListItem {
  id: string;
  date?: string;
  playlist_id?: string;
  playlist_name?: string;
  duration?: number;
  overtime?: boolean;
  overtime_seconds?: number;
  map_code?: string;
  map_name?: string;
  min_rank?: RankInfo;
  max_rank?: RankInfo;
  season?: number;
  replay_title?: string;
}

export interface RankInfo {
  id?: string;
  tier?: number;
  division?: number;
  name?: string;
}

export interface ReplaySearchParams {
  player_id?: string;
  playlist?: string;
  season?: number;
  count?: number;
  sort_by?: string;
  sort_dir?: string;
  next?: string;
}
