export interface RotationInsight {
  player_name: string;
  is_ball_chasing: boolean;
  offensive_third_pct: number;
  avg_distance_to_ball: number;
  team_avg_distance_to_ball: number;
  closest_to_ball_pct: number;
  severity: "none" | "moderate" | "high";
  explanation: string;
}

export interface BoostInsight {
  player_name: string;
  efficiency_score: number;
  small_pad_ratio: number;
  overfill_pct: number;
  zero_boost_pct: number;
  stolen_amount: number;
  rating: "excellent" | "good" | "needs_work" | "poor";
  explanation: string;
}
