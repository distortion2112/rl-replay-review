/** Pure helpers for the coaching screen. */
import type { ReplayListItem } from "../types/replay";

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

/**
 * A search copied from ballchasing.com's replay list, e.g.
 * https://ballchasing.com/?player-name=you. The backend validates it fully.
 */
export function isSiteSearchUrl(text: string): boolean {
  try {
    const url = new URL(text.trim());
    const host = url.hostname.replace(/^www\./, "");
    return (
      host === "ballchasing.com" &&
      (url.pathname === "/" || url.pathname === "/replays") &&
      (url.searchParams.has("player-name") || url.searchParams.has("player-id"))
    );
  } catch {
    return false;
  }
}

/**
 * The ballchasing search for what's typed in the player-name box: a pasted search
 * link as is, otherwise a search for that exact name. Null when empty or another link.
 */
export function nameSearchUrl(text: string): string | null {
  const t = text.trim();
  if (!t) return null;
  if (isSiteSearchUrl(t)) return t;
  if (/^[a-z]+:\/\//i.test(t) || /ballchasing\.com/i.test(t)) return null;
  return `https://ballchasing.com/?player-name=${encodeURIComponent(t)}`;
}

/** What to show in the player-name box for a saved search: the bare name if that's all it is. */
export function nameSearchText(url: string): string {
  try {
    const u = new URL(url);
    const names = u.searchParams.getAll("player-name");
    return names.length === 1 && [...u.searchParams.keys()].length === 1 ? names[0] : url;
  } catch {
    return url;
  }
}

/** "<platform>:<id>" of every player a search lists for a replay. */
export function listedIds(r: ReplayListItem): string[] {
  return [...(r.blue?.players ?? []), ...(r.orange?.players ?? [])].flatMap((p) =>
    p.id?.platform && p.id?.id ? [`${p.id.platform}:${p.id.id}`] : [],
  );
}

/**
 * Replays from several searches as one list: each replay once, newest first, at most
 * `limit`. Replays a search lists without any of `ids` are dropped before any details
 * are fetched (a name search also finds other people with the same name).
 */
export function mergeReplays(lists: ReplayListItem[][], ids: string[], limit: number): ReplayListItem[] {
  const byId = new Map<string, ReplayListItem>();
  for (const r of lists.flat()) {
    const listed = listedIds(r);
    if (listed.length > 0 && !listed.some((id) => ids.includes(id))) continue;
    if (!byId.has(r.id)) byId.set(r.id, r);
  }
  const time = (r: ReplayListItem) => (r.date ? Date.parse(r.date) || 0 : 0); // dates carry their own offsets
  return [...byId.values()].sort((a, b) => time(b) - time(a)).slice(0, limit);
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
