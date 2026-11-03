import { describe, expect, it } from "vitest";
import { extractReplayId } from "../lib/tauri-commands";

const ID = "542d459c-a3a4-451e-a06b-017c77a74a70";

describe("extractReplayId", () => {
  it.each([
    ["bare id", ID],
    ["id with surrounding whitespace", `  ${ID}\n`],
    ["replay page URL", `https://ballchasing.com/replay/${ID}`],
    ["replay page URL with fragment", `https://ballchasing.com/replay/${ID}#stats`],
    ["API URL", `https://ballchasing.com/api/replays/${ID}`],
    [
      "error message containing the API URL",
      `Request failed: error sending request for url (https://ballchasing.com/api/replays/${ID})`,
    ],
    ["uppercase id", ID.toUpperCase()],
  ])("extracts the id from %s", (_label, input) => {
    expect(extractReplayId(input)).toBe(ID);
  });

  it.each([
    ["plain text", "hello"],
    ["empty string", ""],
    ["URL with a non-UUID id", "https://ballchasing.com/replay/abc123"],
  ])("returns null for %s", (_label, input) => {
    expect(extractReplayId(input)).toBeNull();
  });
});
