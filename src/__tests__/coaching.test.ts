import { describe, expect, it } from "vitest";
import { RANKS, findingGroup, isSiteSearchUrl, mergeReplays, nameSearchText, nameSearchUrl, parsePlayerInput, rankName } from "../lib/coaching";

describe("parsePlayerInput", () => {
  it.each([
    ["epic:fe3ca8ef10ca4033b26b8919e1ea51b7", "epic:fe3ca8ef10ca4033b26b8919e1ea51b7"],
    ["  steam:76561198210682145 ", "steam:76561198210682145"],
    ["EPIC:abc", "epic:abc"],
    ["https://ballchasing.com/player/steam/76561198210682145", "steam:76561198210682145"],
    ["https://ballchasing.com/player/epic/abc123?season=f24#stats", "epic:abc123"],
    ["https://ballchasing.com/player/ps4/Some%20Name", "ps4:Some Name"],
  ])("%s -> %s", (input, expected) => {
    expect(parsePlayerInput(input)).toBe(expected);
  });

  it.each(["", "hello", "abc123", "myspace:abc", "https://ballchasing.com/replay/542d459c-a3a4-451e-a06b-017c77a74a70"])(
    "rejects %s",
    (input) => {
      expect(parsePlayerInput(input)).toBeNull();
    },
  );
});

describe("ranks", () => {
  it("has all 22 tiers in order, matching the benchmarks", () => {
    expect(RANKS).toHaveLength(22);
    expect(RANKS[0]).toBe("bronze-1");
    expect(RANKS[13]).toBe("diamond-2");
    expect(RANKS[21]).toBe("supersonic-legend");
  });

  it("formats names", () => {
    expect(rankName("grand-champion-1")).toBe("Grand Champion 1");
    expect(rankName("supersonic-legend")).toBe("Supersonic Legend");
  });
});

it("groups finding ids", () => {
  expect(findingGroup("boost.percent_zero_boost")).toBe("boost");
});

describe("isSiteSearchUrl", () => {
  it("accepts ballchasing replay searches for a player", () => {
    expect(isSiteSearchUrl("https://ballchasing.com/?player-name=distortion2112")).toBe(true);
    expect(isSiteSearchUrl(" https://www.ballchasing.com/?season=f24&player-id=epic:abc ")).toBe(true);
  });
  it("rejects player pages, other sites and searches without a player", () => {
    expect(isSiteSearchUrl("https://ballchasing.com/player/epic/abc")).toBe(false);
    expect(isSiteSearchUrl("https://evil.example/?player-name=x")).toBe(false);
    expect(isSiteSearchUrl("https://ballchasing.com/?season=f24")).toBe(false);
    expect(isSiteSearchUrl("epic:abc")).toBe(false);
  });
});

describe("mergeReplays", () => {
  const r = (id: string, date: string, ...players: string[]) => ({
    id,
    date,
    blue: { players: players.map((p) => ({ id: { platform: p.split(":")[0], id: p.split(":")[1] } })) },
  });
  it("dedupes, drops replays without the player's accounts, sorts newest first and caps", () => {
    const out = mergeReplays(
      [
        [r("a", "2026-10-01T00:00:00Z", "epic:me"), r("b", "2026-10-03T00:00:00Z", "epic:me")],
        [r("a", "2026-10-01T00:00:00Z", "epic:me"), r("c", "2026-10-02T00:00:00Z", "steam:alt"), r("x", "2026-10-09T00:00:00Z", "epic:other")],
      ],
      ["epic:me", "steam:alt"],
      2,
    );
    expect(out.map((o) => o.id)).toEqual(["b", "c"]);
  });
  it("keeps replays a search lists without players (the aggregate checks them)", () => {
    expect(mergeReplays([[{ id: "a" }]], ["epic:me"], 5).map((o) => o.id)).toEqual(["a"]);
  });
});

describe("nameSearchUrl / nameSearchText", () => {
  it("searches a typed name and keeps a pasted search", () => {
    expect(nameSearchUrl(" distortion 2112 ")).toBe("https://ballchasing.com/?player-name=distortion%202112");
    const pasted = "https://ballchasing.com/?player-name=x&season=f24";
    expect(nameSearchUrl(pasted)).toBe(pasted);
    expect(nameSearchUrl("")).toBeNull();
    expect(nameSearchUrl("https://ballchasing.com/player/epic/abc")).toBeNull();
    expect(nameSearchUrl("https://evil.example/?player-name=x")).toBeNull();
  });
  it("shows a saved name search as the bare name, anything richer as the link", () => {
    expect(nameSearchText(nameSearchUrl("distortion 2112")!)).toBe("distortion 2112");
    const rich = "https://ballchasing.com/?player-name=x&season=f24";
    expect(nameSearchText(rich)).toBe(rich);
  });
});
