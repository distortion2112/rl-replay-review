// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";

const commands = vi.hoisted(() => ({
  searchPlayerReplays: vi.fn(),
  searchSiteReplays: vi.fn(),
  buildPlayerAggregate: vi.fn(),
  generateCoachingReport: vi.fn(),
  computePlaystyle: vi.fn(),
  renderCoachingReport: vi.fn(),
}));
const listeners = vi.hoisted(() => ({ handler: null as null | ((e: { payload: unknown }) => void), unlisten: vi.fn() }));

vi.mock("../lib/tauri-commands", () => commands);
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_name: string, handler: (e: { payload: unknown }) => void) => {
    listeners.handler = handler;
    return listeners.unlisten;
  }),
}));

import { useCoachingStore } from "../hooks/useCoaching";

const profile = { playerId: "epic:me", name: null, rank: "champion-1", games: 3 };

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  useCoachingStore.getState().clearProfile();
});

describe("useCoaching", () => {
  it("persists the profile", () => {
    useCoachingStore.getState().setProfile(profile);
    expect(JSON.parse(localStorage.getItem("coaching-profile")!)).toEqual(profile);
    useCoachingStore.getState().clearProfile();
    expect(localStorage.getItem("coaching-profile")).toBeNull();
  });

  it("runs search -> aggregate -> report + playstyle -> text, and learns the player's name", async () => {
    commands.searchPlayerReplays.mockResolvedValue({ list: [{ id: "a" }, { id: "b" }, { id: "c" }] });
    commands.buildPlayerAggregate.mockImplementation(async () => {
      listeners.handler?.({ payload: { current: 2, total: 3, replay_id: "b" } });
      expect(useCoachingStore.getState().progress).toEqual({ current: 2, total: 3 });
      return { player: "epic:me", name: "Me", games: [], game_count: 3, skipped: [] };
    });
    commands.generateCoachingReport.mockResolvedValue({ games: 3 });
    commands.computePlaystyle.mockResolvedValue({ axes: [] });
    commands.renderCoachingReport.mockResolvedValue({ summary: "ok", items: [] });

    useCoachingStore.getState().setProfile(profile);
    await useCoachingStore.getState().generate();

    expect(commands.searchPlayerReplays).toHaveBeenCalledWith(
      expect.objectContaining({ player_id: "epic:me", playlist: "ranked-doubles", count: 3, sort_by: "replay-date" }),
    );
    expect(commands.buildPlayerAggregate).toHaveBeenCalledWith("epic:me", ["a", "b", "c"], []);
    expect(commands.searchSiteReplays).not.toHaveBeenCalled();
    expect(commands.generateCoachingReport).toHaveBeenCalledWith(expect.objectContaining({ name: "Me" }), "champion-1");
    expect(listeners.unlisten).toHaveBeenCalled();
    const s = useCoachingStore.getState();
    expect(s.status).toBe("done");
    expect(s.report?.summary).toBe("ok");
    expect(s.profile?.name).toBe("Me");
    expect(JSON.parse(localStorage.getItem("coaching-profile")!).name).toBe("Me");
  });

  it("adds games from a pasted search, newest first, keeping only the player's accounts", async () => {
    const game = (id: string, date: string, player: string) => ({
      id,
      date,
      blue: { players: [{ name: "x", id: { platform: player.split(":")[0], id: player.split(":")[1] } }] },
    });
    commands.searchPlayerReplays.mockResolvedValue({
      list: [game("a", "2026-10-03T10:00:00Z", "epic:me"), game("b", "2026-10-01T10:00:00Z", "epic:me")],
    });
    commands.searchSiteReplays.mockResolvedValue({
      replays: [
        game("c", "2026-10-03T08:00:00-05:00", "steam:old"), // 13:00Z: newer than "a", though it sorts before it as text
        game("a", "2026-10-03T10:00:00Z", "epic:me"),
        game("d", "2026-10-04T10:00:00Z", "epic:stranger"),
      ],
      identities: [],
      ignored: [],
    });
    commands.buildPlayerAggregate.mockResolvedValue({ player: "epic:me", name: "Me", games: [], game_count: 3, skipped: [] });
    commands.generateCoachingReport.mockResolvedValue({ games: 3 });
    commands.computePlaystyle.mockResolvedValue({ axes: [] });
    commands.renderCoachingReport.mockResolvedValue({ summary: "ok", items: [] });

    const url = "https://ballchasing.com/?player-name=me";
    useCoachingStore.getState().setProfile({ ...profile, searchUrl: url, aliases: ["steam:old"] });
    await useCoachingStore.getState().generate();

    expect(commands.searchSiteReplays).toHaveBeenCalledWith(url, 3);
    expect(commands.buildPlayerAggregate).toHaveBeenCalledWith("epic:me", ["c", "a", "b"], ["steam:old"]);
    expect(useCoachingStore.getState().status).toBe("done");
  });

  it("explains when ballchasing has no games for the player", async () => {
    commands.searchPlayerReplays.mockResolvedValue({ list: [] });
    useCoachingStore.getState().setProfile(profile);
    await useCoachingStore.getState().generate();
    const s = useCoachingStore.getState();
    expect(s.status).toBe("error");
    expect(s.error).toMatch(/No ranked 2v2 games/);
    expect(commands.buildPlayerAggregate).not.toHaveBeenCalled();
  });

  it("surfaces backend errors and stops listening for progress", async () => {
    commands.searchPlayerReplays.mockResolvedValue({ list: [{ id: "a" }] });
    commands.buildPlayerAggregate.mockRejectedValue("No API key configured.");
    useCoachingStore.getState().setProfile(profile);
    await useCoachingStore.getState().generate();
    expect(useCoachingStore.getState()).toMatchObject({ status: "error", error: "No API key configured." });
    expect(listeners.unlisten).toHaveBeenCalled();
  });
});
