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

/** ballchasing.com patron tier, read from the API. */
export type ApiTier = "regular" | "gold" | "diamond" | "champion" | "gc";

export interface EndpointStatus {
  endpoint: "list" | "get" | "file";
  per_second: number;
  /** null when the tier has no hourly cap for this endpoint */
  per_hour: number | null;
  used_last_hour: number;
  remaining_this_hour: number | null;
}

export interface RateLimitStatus {
  tier: ApiTier;
  /** false until the tier has been read from the API this session */
  tier_detected: boolean;
  endpoints: EndpointStatus[];
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
