import { describe, expect, it } from "vitest";
import { RANKS, findingGroup, parsePlayerInput, rankName } from "../lib/coaching";

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
