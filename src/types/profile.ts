/** Mirrors `src-tauri/src/profile/aggregate.rs`. */

export interface GameStats {
  replay_id: string;
  date: string | null;
  team: "blue" | "orange";
  won: boolean;
  score_diff: number;
  duration_s: number;
  overtime: boolean;
  /** `<group>_<stat>` in benchmark units: totals per minute, rates/percentages as-is */
  values: Record<string, number>;
}

export interface Skipped {
  replay_id: string;
  reason: string;
}

export interface PlayerAggregate {
  /** `<platform>:<id>` */
  player: string;
  name: string | null;
  games: GameStats[];
  game_count: number;
  win_count: number;
  first_date: string | null;
  last_date: string | null;
  /** replays with no usable full game for the player, with the reason */
  skipped: Skipped[];
}

/** Payload of the `aggregate-progress` event. */
export interface AggregateProgress {
  current: number;
  total: number;
  replay_id: string;
}

export type Axis = "aggression" | "mechanical" | "boost_discipline" | "team_play" | "aerial_tendency";

export interface AxisScore {
  axis: Axis;
  /** 0-100: weighted percentile at the player's rank */
  value: number;
  /** 80% bootstrap interval [low, high] */
  interval: [number, number];
  inputs_used: string[];
}

/** Mirrors `src-tauri/src/profile/playstyle.rs`. Descriptive, not evaluative. */
export interface PlaystyleProfile {
  player: string;
  rank: string;
  rank_used: string;
  games: number;
  /** empty below 5 games */
  axes: AxisScore[];
  /** null below 10 games, or unless its defining axes clear their thresholds with confidence */
  archetype: string | null;
  archetype_reason: string;
}
