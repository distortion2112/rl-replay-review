/** Pure helpers for the coaching screen. */

/** ballchasing rank ids, lowest to highest (same order as the benchmarks). */
export const RANKS = [
  "bronze-1", "bronze-2", "bronze-3",
  "silver-1", "silver-2", "silver-3",
  "gold-1", "gold-2", "gold-3",
  "platinum-1", "platinum-2", "platinum-3",
  "diamond-1", "diamond-2", "diamond-3",
  "champion-1", "champion-2", "champion-3",
  "grand-champion-1", "grand-champion-2", "grand-champion-3",
  "supersonic-legend",
] as const;

export type RankId = (typeof RANKS)[number];

/** "grand-champion-1" -> "Grand Champion 1" */
export function rankName(rank: string): string {
  if (rank === "supersonic-legend") return "Supersonic Legend";
  return rank
    .split("-")
    .map((w) => (w ? w[0].toUpperCase() + w.slice(1) : w))
    .join(" ");
}

const PLATFORMS = ["epic", "steam", "ps4", "xbox", "switch", "psynet"];

/**
 * A ballchasing player id ("<platform>:<id>") from what someone pastes:
 * "epic:abc123", "steam:7656…", or a ballchasing player page link like
 * https://ballchasing.com/player/epic/abc123. Returns null otherwise.
 */
export function parsePlayerInput(text: string): string | null {
  const t = text.trim();
  const url = t.match(/ballchasing\.com\/player\/([a-z0-9]+)\/([^/?#\s]+)/i);
  if (url && PLATFORMS.includes(url[1].toLowerCase())) {
    return `${url[1].toLowerCase()}:${decodeURIComponent(url[2])}`;
  }
  const pair = t.match(/^([a-z0-9]+):([^\s]+)$/i);
  if (pair && PLATFORMS.includes(pair[1].toLowerCase())) {
    return `${pair[1].toLowerCase()}:${pair[2]}`;
  }
  return null;
}

export const FINDING_GROUPS = ["movement", "boost", "positioning", "core", "demo"] as const;
export type FindingGroup = (typeof FINDING_GROUPS)[number];

/** "boost.percent_zero_boost" -> "boost" */
export function findingGroup(findingId: string): string {
  return findingId.split(".")[0];
}

export const GROUP_LABELS: Record<FindingGroup, string> = {
  movement: "Movement",
  boost: "Boost",
  positioning: "Positioning",
  core: "Core",
  demo: "Demos",
};
