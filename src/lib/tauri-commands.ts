import { invoke } from "@tauri-apps/api/core";
import type { ReplayData, PlayerData, ReplayListResponse, ReplaySearchParams } from "../types/replay";
import type { RotationInsight, BoostInsight, RateLimitStatus } from "../types/analysis";
import type { FrameData } from "../types/frames";

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

export async function getRateLimitStatus(): Promise<RateLimitStatus> {
  return invoke("get_rate_limit_status");
}

export function extractReplayId(input: string): string {
  const trimmed = input.trim();
  // Handle full URLs like https://ballchasing.com/replay/abc123
  const urlMatch = trimmed.match(/ballchasing\.com\/replay\/([a-zA-Z0-9-]+)/);
  if (urlMatch) return urlMatch[1];
  // Otherwise treat as raw ID
  return trimmed;
}
