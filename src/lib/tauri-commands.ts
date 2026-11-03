import { invoke } from "@tauri-apps/api/core";
import type { ReplayData, PlayerData, ReplayListResponse, ReplaySearchParams } from "../types/replay";
import type { RotationInsight, BoostInsight, RateLimitStatus } from "../types/analysis";
import type { FrameData } from "../types/frames";
import type { RankLookup } from "../types/benchmark";
import type { PlayerAggregate, PlaystyleProfile } from "../types/profile";
import type { CoachingReport, EvidencePack } from "../types/coaching";

export async function fetchReplay(replayId: string): Promise<ReplayData> {
  return invoke("fetch_replay", { replayId });
}

export async function setApiKey(key: string): Promise<void> {
  return invoke("set_api_key", { key });
}

export async function getApiKey(): Promise<string | null> {
  return invoke("get_api_key");
}

export async function analyzeRotation(
  players: PlayerData[]
): Promise<RotationInsight[]> {
  return invoke("analyze_replay_rotation", { players });
}

export async function analyzeBoost(
  players: PlayerData[]
): Promise<BoostInsight[]> {
  return invoke("analyze_replay_boost", { players });
}

export async function downloadAndParseReplay(
  replayId: string
): Promise<FrameData> {
  return invoke("download_and_parse_replay", { replayId });
}

export async function searchPlayerReplays(
  params: ReplaySearchParams
): Promise<ReplayListResponse> {
  return invoke("search_player_replays", { params });
}

/** Benchmark for a ballchasing rank id (e.g. "diamond-2"), plus the nearest tiers around it. */
export async function getRankBenchmark(rank: string): Promise<RankLookup> {
  return invoke("get_rank_benchmark", { rank });
}

/**
 * A player's per-game stats across replays (cache-first). Listen for the
 * `aggregate-progress` event for progress.
 */
export async function buildPlayerAggregate(playerId: string, replayIds: string[]): Promise<PlayerAggregate> {
  return invoke("build_player_aggregate", { playerId, replayIds });
}

/** Compares an aggregate to the benchmarks at `rank` (e.g. "champion-1") and the tier above. */
export async function generateCoachingReport(aggregate: PlayerAggregate, rank: string): Promise<EvidencePack> {
  return invoke("generate_coaching_report", { aggregate, rank });
}

/** Five-axis playstyle relative to `rank`. Descriptive, not a judgment. */
export async function computePlaystyle(aggregate: PlayerAggregate, rank: string): Promise<PlaystyleProfile> {
  return invoke("compute_playstyle", { aggregate, rank });
}

/** Readable coaching from an evidence pack (deterministic templates). */
export async function renderCoachingReport(pack: EvidencePack): Promise<CoachingReport> {
  return invoke("render_coaching_report", { pack });
}

export async function getRateLimitStatus(): Promise<RateLimitStatus> {
  return invoke("get_rate_limit_status");
}

/** ballchasing replay IDs are UUIDs. */
const REPLAY_ID = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i;

/**
 * Finds a ballchasing replay ID anywhere in pasted text: a bare ID, a replay
 * page link, an API link (`/api/replays/<id>`), or text that merely contains
 * one. Returns null when there is none, so garbage is never sent to the API.
 */
export function extractReplayId(input: string): string | null {
  const match = input.match(REPLAY_ID);
  return match ? match[0].toLowerCase() : null;
}
