import { describe, expect, it } from "vitest";
import { interpolateFrame, MAX_FRAME_GAP_S } from "../lib/interpolation";
import type { GameFrame, PlayerFrame, Position } from "../types/frames";

const pos = (x: number, y = 0, z = 0): Position => ({ x, y, z });

const player = (player_index: number, position: Position | null): PlayerFrame => ({
  player_index,
  position,
  boost_amount: 33,
});

const frame = (time: number, ball: Position | null, players: PlayerFrame[]): GameFrame => ({
  time,
  ball,
  players,
});

describe("interpolateFrame", () => {
  it("exposes the goal-replay gap threshold", () => {
    expect(MAX_FRAME_GAP_S).toBe(0.5);
  });

  it("lerps ball and car positions halfway at the midpoint", () => {
    const frames = [
      frame(1.0, pos(0, 0, 0), [player(0, pos(100, 200, 0))]),
      frame(1.1, pos(100, 200, 50), [player(0, pos(200, 400, 20))]),
    ];
    const result = interpolateFrame(frames, 0, 1.05);
    expect(result).not.toBeNull();
    expect(result!.time).toBe(1.05);
    expect(result!.ball!.x).toBeCloseTo(50);
    expect(result!.ball!.y).toBeCloseTo(100);
    expect(result!.ball!.z).toBeCloseTo(25);
    expect(result!.players[0].position!.x).toBeCloseTo(150);
    expect(result!.players[0].position!.y).toBeCloseTo(300);
    expect(result!.players[0].position!.z).toBeCloseTo(10);
  });

  it("returns frames[index] itself when time equals its timestamp", () => {
    const frames = [frame(2, pos(0), []), frame(2.1, pos(10), [])];
    expect(interpolateFrame(frames, 0, 2)).toBe(frames[0]);
  });

  it("returns frame a unchanged across a gap longer than MAX_FRAME_GAP_S", () => {
    const frames = [
      frame(10, pos(0), [player(0, pos(0))]),
      frame(10 + MAX_FRAME_GAP_S + 0.1, pos(100), [player(0, pos(100))]),
    ];
    expect(interpolateFrame(frames, 0, 10.3)).toBe(frames[0]);
  });

  it("snaps to the start position when the implied speed exceeds 7000 uu/s", () => {
    // 1000 uu in 0.1s = 10000 uu/s: a teleport (respawn, kickoff reset).
    const frames = [
      frame(0, pos(0), [player(0, pos(0))]),
      frame(0.1, pos(1000), [player(0, pos(1000))]),
    ];
    const result = interpolateFrame(frames, 0, 0.05);
    expect(result!.ball).toEqual(pos(0));
    expect(result!.players[0].position).toEqual(pos(0));
  });

  it("keeps a's position for a player missing from b", () => {
    const frames = [
      frame(0, pos(0), [player(0, pos(0)), player(1, pos(500, 500, 17))]),
      frame(0.1, pos(10), [player(0, pos(100))]),
    ];
    const result = interpolateFrame(frames, 0, 0.05);
    expect(result!.players[0].position!.x).toBeCloseTo(50);
    expect(result!.players[1].position).toEqual(pos(500, 500, 17));
  });

  it("preserves extra frame fields such as ball_kinematics", () => {
    const kinematics = {
      rotation: { x: 0, y: 0, z: 0, w: 1 },
      linear_velocity: pos(100),
      angular_velocity: null,
      sleeping: false,
    };
    const a: GameFrame = { ...frame(0, pos(0), []), ball_kinematics: kinematics };
    const frames = [a, frame(0.1, pos(10), [])];
    const result = interpolateFrame(frames, 0, 0.05);
    expect(result).not.toBe(a);
    expect(result!.ball_kinematics).toBe(kinematics);
  });
});
