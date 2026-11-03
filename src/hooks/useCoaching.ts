import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import {
  buildPlayerAggregate,
  computePlaystyle,
  generateCoachingReport,
  renderCoachingReport,
  searchPlayerReplays,
  searchSiteReplays,
} from "../lib/tauri-commands";
import { mergeReplays } from "../lib/coaching";
import type { AggregateProgress, PlayerAggregate, PlaystyleProfile } from "../types/profile";
import type { CoachingReport, EvidencePack } from "../types/coaching";

export interface CoachingProfile {
  /** ballchasing "<platform>:<id>" */
  playerId: string;
  /** shown in the header; from the replay the player was picked from, if any */
  name: string | null;
  /** ballchasing rank id; chosen by the player (ballchasing no longer reports ranks) */
  rank: string;
  /** how many recent ranked 2v2 games to analyze */
  games: number;
  /** a ballchasing.com search whose replays are pulled in too, e.g. ?player-name=… */
  searchUrl?: string;
  /** other "<platform>:<id>"s the player confirmed are theirs (from the search) */
  aliases?: string[];
  /** true when `playerId` was picked from the search's accounts rather than typed */
  idFromSearch?: boolean;
}

export type CoachingStatus = "idle" | "searching" | "aggregating" | "analyzing" | "done" | "error";

interface CoachingStore {
  profile: CoachingProfile | null;
  status: CoachingStatus;
  progress: { current: number; total: number } | null;
  error: string | null;
  aggregate: PlayerAggregate | null;
  pack: EvidencePack | null;
  report: CoachingReport | null;
  playstyle: PlaystyleProfile | null;
  generatedAt: string | null;

  setProfile: (profile: CoachingProfile) => void;
  clearProfile: () => void;
  generate: () => Promise<void>;
}

const PROFILE_KEY = "coaching-profile";

function loadProfile(): CoachingProfile | null {
  try {
    const raw = localStorage.getItem(PROFILE_KEY);
    return raw ? (JSON.parse(raw) as CoachingProfile) : null;
  } catch {
    return null;
  }
}

function saveProfile(profile: CoachingProfile | null) {
  try {
    if (profile) localStorage.setItem(PROFILE_KEY, JSON.stringify(profile));
    else localStorage.removeItem(PROFILE_KEY);
  } catch {
    // Storage unavailable: the profile just won't persist
  }
}

const emptyResults = { aggregate: null, pack: null, report: null, playstyle: null, generatedAt: null };

export const useCoachingStore = create<CoachingStore>((set, get) => ({
  profile: loadProfile(),
  status: "idle",
  progress: null,
  error: null,
  ...emptyResults,

  setProfile: (profile) => {
    saveProfile(profile);
    set({ profile, status: "idle", error: null, progress: null, ...emptyResults });
  },

  clearProfile: () => {
    saveProfile(null);
    set({ profile: null, status: "idle", error: null, progress: null, ...emptyResults });
  },

  generate: async () => {
    const profile = get().profile;
    if (!profile) return;
    set({ status: "searching", error: null, progress: null });
    try {
      const aliases = profile.aliases ?? [];
      const [byId, bySearch] = await Promise.all([
        searchPlayerReplays({
          player_id: profile.playerId,
          playlist: "ranked-doubles",
          count: profile.games,
          sort_by: "replay-date",
          sort_dir: "desc",
        }),
        profile.searchUrl ? searchSiteReplays(profile.searchUrl, profile.games) : Promise.resolve(null),
      ]);
      const merged = mergeReplays([byId.list, bySearch?.replays ?? []], [profile.playerId, ...aliases], profile.games);
      const ids = merged.map((r) => r.id);
      if (ids.length === 0) {
        throw new Error(
          "No ranked 2v2 games found on ballchasing for this player. Replays need to be uploaded to ballchasing (public or unlisted) to be analyzed.",
        );
      }

      set({ status: "aggregating", progress: { current: 0, total: ids.length } });
      const unlisten = await listen<AggregateProgress>("aggregate-progress", (e) =>
        set({ progress: { current: e.payload.current, total: e.payload.total } }),
      );
      let aggregate: PlayerAggregate;
      try {
        aggregate = await buildPlayerAggregate(profile.playerId, ids, aliases);
      } finally {
        unlisten();
      }

      set({ status: "analyzing" });
      const [pack, playstyle] = await Promise.all([
        generateCoachingReport(aggregate, profile.rank),
        computePlaystyle(aggregate, profile.rank),
      ]);
      const report = await renderCoachingReport(pack);
      set({ status: "done", aggregate, pack, report, playstyle, generatedAt: new Date().toISOString() });
      if (!profile.name && aggregate.name) {
        const named = { ...profile, name: aggregate.name };
        saveProfile(named);
        set({ profile: named });
      }
    } catch (e) {
      set({ status: "error", error: e instanceof Error ? e.message : String(e) });
    }
  },
}));
