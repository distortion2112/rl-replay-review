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
