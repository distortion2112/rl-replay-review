import { create } from "zustand";
import type { ReplayData, PlayerData } from "../types/replay";
import type { RotationInsight, BoostInsight } from "../types/analysis";
import {
  fetchReplay,
  setApiKey,
  getApiKey,
  analyzeRotation,
  analyzeBoost,
  extractReplayId,
} from "../lib/tauri-commands";

interface ReplayStore {
  apiKey: string | null;
  replay: ReplayData | null;
  loading: boolean;
  error: string | null;
  selectedPlayer: PlayerData | null;
  selectedTeam: "blue" | "orange";
  rotationInsights: RotationInsight[];
  boostInsights: BoostInsight[];
  initApiKey: () => Promise<void>;
  saveApiKey: (key: string) => Promise<void>;
  loadReplay: (input: string) => Promise<void>;
  selectPlayer: (player: PlayerData | null) => void;
  selectTeam: (team: "blue" | "orange") => void;
  allPlayers: () => PlayerData[];
  teamPlayers: (team: "blue" | "orange") => PlayerData[];
}

export const useReplayStore = create<ReplayStore>((set, get) => ({
  apiKey: null,
  replay: null,
  loading: false,
  error: null,
  selectedPlayer: null,
  selectedTeam: "blue",
  rotationInsights: [],
  boostInsights: [],

  initApiKey: async () => {
    try {
      const key = await getApiKey();
      set({ apiKey: key });
    } catch {
      set({ apiKey: null });
    }
  },

  saveApiKey: async (key: string) => {
    await setApiKey(key);
    set({ apiKey: key });
  },

  loadReplay: async (input: string) => {
    set({ loading: true, error: null });
    try {
      const id = extractReplayId(input);
      if (!id) {
        set({
          error:
            "Couldn't find a replay ID. Paste a ballchasing.com replay link or its ID (like 542d459c-a3a4-451e-a06b-017c77a74a70).",
          loading: false,
        });
        return;
      }
      const replay = await fetchReplay(id);
      set({ replay, loading: false });

      // Run analysis on all players
      const allPlayers = [
        ...(replay.blue?.players ?? []),
        ...(replay.orange?.players ?? []),
      ];
      const [rotation, boost] = await Promise.all([
        analyzeRotation(allPlayers),
        analyzeBoost(allPlayers),
      ]);
      set({ rotationInsights: rotation, boostInsights: boost });

      // Auto-select first player
      if (allPlayers.length > 0) {
        set({ selectedPlayer: allPlayers[0] });
      }
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  selectPlayer: (player) => set({ selectedPlayer: player }),
  selectTeam: (team) => set({ selectedTeam: team }),

  allPlayers: () => {
    const r = get().replay;
    return [...(r?.blue?.players ?? []), ...(r?.orange?.players ?? [])];
  },

  teamPlayers: (team) => {
    const r = get().replay;
    return (team === "blue" ? r?.blue?.players : r?.orange?.players) ?? [];
  },
}));
