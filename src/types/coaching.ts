/** Mirrors `src-tauri/src/coaching/models.rs` (evidence pack schema_version 1). */

export type FindingKind = "strength" | "weakness" | "uncertain";

export interface ClipRef {
  replay_id: string;
  time: number;
}

export interface Finding {
  /** stable id, e.g. "boost.percent_zero_boost" */
  id: string;
  stat_key: string;
  kind: FindingKind;
  /** player's estimate in benchmark units (shrunk toward the tier median for small samples) */
  value: number;
  /** 80% interval [low, high] */
  interval: [number, number];
  /** percentile at the current tier, oriented so higher is better */
  pct_current_tier: number;
  /** percentile at the target tier, oriented so higher is better */
  pct_target_tier: number;
  current_tier_median: number;
  target_tier_median: number;
  higher_is_better: boolean;
  /** |Spearman correlation with rank| */
  rank_signal: number;
  priority: number;
  games: number;
  clips: ClipRef[];
}

export interface EvidencePack {
  schema_version: number;
  player: { player: string; name: string | null };
  current_tier: string;
  current_tier_used: string;
  target_tier: string;
  target_tier_used: string;
  games: number;
  /** false below `min_games`: findings are empty */
  enough_data: boolean;
  min_games: number;
  /** highest priority first */
  findings: Finding[];
  top_strengths: string[];
  top_weaknesses: string[];
  benchmark_seasons: string[];
}

/** Mirrors `src-tauri/src/coaching/text.rs`. */
export interface ReportItem {
  finding_id: string;
  kind: FindingKind;
  title: string;
  explanation: string;
  /** what to practice: weaknesses with mapped advice only */
  action: string | null;
  /** in the pack's top 3 for its kind */
  top: boolean;
}

export interface CoachingReport {
  pack_schema_version: number;
  summary: string;
  /** top weaknesses, top strengths, other weaknesses and strengths, then uncertain */
  items: ReportItem[];
  /** "templates" */
  source: string;
}
