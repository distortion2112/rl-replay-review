/** Mirrors `src-tauri/src/benchmark/models.rs` (benchmarks.json schema_version 1). */

export type StatUnit = "as_is" | "per_minute";
export type Direction = "higher" | "lower";

export interface RankSignal {
  /** Spearman correlation of the stat with rank across all tiers */
  spearman: number;
  direction: Direction;
  /** |spearman|: how strongly the stat separates ranks */
  weight: number;
}

export interface StatMeta {
  group: string;
  /** "per_minute": divide the player's total by game minutes before comparing */
  unit: StatUnit;
  rank_signal?: RankSignal | null;
}

export interface StatDistribution {
  n: number;
  mean: number;
  stddev: number;
  p5: number;
  p10: number;
  p25: number;
  p50: number;
  p75: number;
  p90: number;
  p95: number;
}

export interface RankBenchmark {
  /** ballchasing rank id, e.g. "diamond-2" */
  rank: string;
  /** 0 = bronze-1 ... 21 = supersonic-legend */
  tier_index: number;
  sample_size: number;
  games: number;
  /** fewer than 30 rows: show with low confidence */
  low_sample: boolean;
  stats: Record<string, StatDistribution>;
}

export interface BenchmarkSource {
  site: string;
  playlist: string;
  seasons: string[];
  rank_source: string;
  row_filter: string;
}

export interface RankLookup {
  requested: string;
  benchmark: RankBenchmark;
  /** true when `requested` had no data and the nearest tier was used */
  substituted: boolean;
  below: RankBenchmark | null;
  above: RankBenchmark | null;
  stats: Record<string, StatMeta>;
  source: BenchmarkSource;
  generated_at: string;
}
